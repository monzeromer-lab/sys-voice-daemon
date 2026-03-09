use crossbeam_channel::Sender;
use log::{error, info, warn};
use std::thread;

/// Information about the currently focused application.
#[derive(Debug, Clone)]
pub struct AppInfo {
    pub wm_class: String,
    pub window_name: String,
}

/// Focus state sent to the daemon.
#[derive(Debug, Clone)]
pub enum FocusState {
    /// A text-entry field is active in the given application.
    TextFieldActive(AppInfo),
    /// No text field is focused.
    NotActive,
}

/// Detects the current display server and starts the appropriate focus monitor.
pub fn start_focus_monitor(tx: Sender<FocusState>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
        match session_type.as_str() {
            "x11" => {
                info!("Starting X11 focus monitor");
                if let Err(e) = x11_focus_loop(&tx) {
                    error!("X11 focus monitor error: {e}");
                }
            }
            "wayland" => {
                info!("Starting Wayland (AT-SPI2) focus monitor");
                if let Err(e) = wayland_focus_loop(&tx) {
                    error!("Wayland focus monitor error: {e}");
                }
            }
            other => {
                warn!("Unknown session type: {other:?}, defaulting to always-active mode");
                let _ = tx.send(FocusState::TextFieldActive(AppInfo {
                    wm_class: "unknown".to_string(),
                    window_name: "unknown".to_string(),
                }));
            }
        }
    })
}

/// X11 focus detection using x11rb.
fn x11_focus_loop(tx: &Sender<FocusState>) -> Result<(), Box<dyn std::error::Error>> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ConnectionExt, EventMask};
    use x11rb::protocol::Event;

    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    // Subscribe to property change events on the root window for active window changes
    conn.change_window_attributes(
        root,
        &x11rb::protocol::xproto::ChangeWindowAttributesAux::new()
            .event_mask(EventMask::PROPERTY_CHANGE),
    )?;
    conn.flush()?;

    // Get the _NET_ACTIVE_WINDOW atom
    let net_active_window = conn
        .intern_atom(false, b"_NET_ACTIVE_WINDOW")?
        .reply()?
        .atom;

    let mut last_window: u32 = 0;

    // Send initial state
    send_x11_focus_state(&conn, tx, net_active_window, &mut last_window)?;

    loop {
        let event = conn.wait_for_event()?;
        if let Event::PropertyNotify(evt) = event {
            if evt.atom == net_active_window {
                send_x11_focus_state(&conn, tx, net_active_window, &mut last_window)?;
            }
        }
    }
}

fn send_x11_focus_state(
    conn: &impl x11rb::connection::Connection,
    tx: &Sender<FocusState>,
    net_active_window: u32,
    last_window: &mut u32,
) -> Result<(), Box<dyn std::error::Error>> {
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};

    // Get the active window from the root
    let screen = &conn.setup().roots[0];
    let root = screen.root;

    let reply = conn
        .get_property(false, root, net_active_window, AtomEnum::WINDOW, 0, 1)?
        .reply()?;

    let window = if reply.length > 0 {
        reply.value32().and_then(|mut iter| iter.next()).unwrap_or(0)
    } else {
        0
    };

    if window == 0 || window == *last_window {
        return Ok(());
    }
    *last_window = window;

    // Get WM_CLASS property
    let wm_class_reply = conn
        .get_property(false, window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 1024)?
        .reply()?;

    let wm_class = String::from_utf8_lossy(&wm_class_reply.value)
        .replace('\0', " ")
        .trim()
        .to_string();

    // Get _NET_WM_NAME or WM_NAME for the window title
    let net_wm_name_atom = conn.intern_atom(false, b"_NET_WM_NAME")?.reply()?.atom;
    let utf8_atom = conn.intern_atom(false, b"UTF8_STRING")?.reply()?.atom;

    let name_reply = conn
        .get_property(false, window, net_wm_name_atom, utf8_atom, 0, 1024)?
        .reply()?;

    let window_name = if name_reply.length > 0 {
        String::from_utf8_lossy(&name_reply.value).to_string()
    } else {
        let name_reply = conn
            .get_property(false, window, AtomEnum::WM_NAME, AtomEnum::STRING, 0, 1024)?
            .reply()?;
        String::from_utf8_lossy(&name_reply.value).to_string()
    };

    info!("Focus changed: wm_class={wm_class:?}, name={window_name:?}");

    let state = FocusState::TextFieldActive(AppInfo {
        wm_class,
        window_name,
    });

    let _ = tx.send(state);
    Ok(())
}

/// Wayland focus detection via AT-SPI2 D-Bus interface.
fn wayland_focus_loop(tx: &Sender<FocusState>) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(wayland_focus_async(tx))
}

async fn wayland_focus_async(tx: &Sender<FocusState>) -> Result<(), Box<dyn std::error::Error>> {
    use futures_util::StreamExt;

    let connection = zbus::Connection::session().await?;

    // Register as an AT-SPI2 event listener
    let proxy: zbus::Proxy<'_> = zbus::proxy::Builder::new(&connection)
        .destination("org.a11y.atspi.Registry")?
        .path("/org/a11y/atspi/registry")?
        .interface("org.a11y.atspi.Registry")?
        .build()
        .await?;

    // Register for focus events
    let _: () = proxy.call("RegisterEvent", &("focus:")).await?;
    info!("Registered for AT-SPI2 focus events");

    // Connect to the accessibility bus
    let a11y_address = get_a11y_bus_address(&connection).await?;
    let a11y_addr: zbus::Address = a11y_address.parse()?;
    let a11y_conn = zbus::connection::Builder::address(a11y_addr)?
        .build()
        .await?;

    // Add a match rule for focus events
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.a11y.atspi.Event.Focus")?
        .build();

    let mut stream = zbus::MessageStream::for_match_rule(rule, &a11y_conn, None).await?;

    while let Some(msg) = stream.next().await {
        if let Ok(msg) = msg {
            let sender = msg
                .header()
                .sender()
                .map(|s| s.to_string())
                .unwrap_or_default();

            let state = FocusState::TextFieldActive(AppInfo {
                wm_class: sender.clone(),
                window_name: sender,
            });
            if tx.send(state).is_err() {
                break;
            }
        }
    }

    Ok(())
}

async fn get_a11y_bus_address(
    conn: &zbus::Connection,
) -> Result<String, Box<dyn std::error::Error>> {
    let proxy: zbus::Proxy<'_> = zbus::proxy::Builder::new(conn)
        .destination("org.a11y.Bus")?
        .path("/org/a11y/bus")?
        .interface("org.a11y.Bus")?
        .build()
        .await?;

    let address: String = proxy.call("GetAddress", &()).await?;
    Ok(address)
}

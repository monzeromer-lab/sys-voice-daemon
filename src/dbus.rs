use crossbeam_channel::Sender;
use log::info;
use std::sync::{Arc, Mutex};
use zbus::object_server::SignalEmitter;

use crate::daemon::DaemonState;

/// Commands sent from D-Bus to the daemon control loop.
#[derive(Debug, Clone)]
pub enum DaemonCommand {
    Enable,
    Disable,
    Quit,
}

fn state_to_str(s: DaemonState) -> &'static str {
    match s {
        DaemonState::Idle => "idle",
        DaemonState::Listening => "listening",
        DaemonState::Paused => "paused",
    }
}

/// D-Bus interface object for controlling the daemon.
pub struct DaemonInterface {
    ctrl_tx: Sender<DaemonCommand>,
    state: Arc<Mutex<DaemonState>>,
}

impl DaemonInterface {
    pub fn new(ctrl_tx: Sender<DaemonCommand>, state: Arc<Mutex<DaemonState>>) -> Self {
        Self { ctrl_tx, state }
    }

    /// Update the shared state and emit D-Bus notifications.
    fn set_state(&self, new_state: DaemonState) {
        if let Ok(mut s) = self.state.lock() {
            *s = new_state;
        }
    }
}

#[zbus::interface(name = "com.github.voskdictation")]
impl DaemonInterface {
    /// Enable the daemon (resume listening).
    async fn enable(
        &self,
        #[zbus(signal_context)] ctxt: SignalEmitter<'_>,
    ) {
        info!("D-Bus: Enable requested");
        let new_state = DaemonState::Idle;
        self.set_state(new_state);
        let _ = self.ctrl_tx.send(DaemonCommand::Enable);
        let _ = Self::status_changed(&ctxt, state_to_str(new_state)).await;
    }

    /// Disable the daemon (pause listening).
    async fn disable(
        &self,
        #[zbus(signal_context)] ctxt: SignalEmitter<'_>,
    ) {
        info!("D-Bus: Disable requested");
        let new_state = DaemonState::Paused;
        self.set_state(new_state);
        let _ = self.ctrl_tx.send(DaemonCommand::Disable);
        let _ = Self::status_changed(&ctxt, state_to_str(new_state)).await;
    }

    /// Gracefully shut down the daemon.
    async fn quit(&self) {
        info!("D-Bus: Quit requested");
        let _ = self.ctrl_tx.send(DaemonCommand::Quit);
    }

    /// Current daemon state as a string.
    #[zbus(property)]
    fn state(&self) -> String {
        let state = self.state.lock().unwrap();
        state_to_str(*state).to_string()
    }

    /// Signal emitted when the daemon state changes.
    #[zbus(signal)]
    async fn status_changed(signal_emitter: &SignalEmitter<'_>, state: &str) -> zbus::Result<()>;
}

/// Start the D-Bus service on the session bus.
/// This runs in a tokio async context and blocks until the connection is dropped.
pub async fn start_dbus_server(
    ctrl_tx: Sender<DaemonCommand>,
    state: Arc<Mutex<DaemonState>>,
) -> Result<zbus::Connection, zbus::Error> {
    let iface = DaemonInterface::new(ctrl_tx, state);

    let conn = zbus::connection::Builder::session()?
        .name("com.github.voskdictation")?
        .serve_at("/com/github/voskdictation", iface)?
        .build()
        .await?;

    info!("D-Bus service registered: com.github.voskdictation");
    Ok(conn)
}

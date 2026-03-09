use crossbeam_channel::{select, Receiver};
use log::{info, warn};
use std::sync::{Arc, Mutex};

use crate::dbus::DaemonCommand;
use crate::focus::FocusState;
use crate::injector;
use crate::stt::SttEvent;

/// The daemon's operating state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonState {
    /// Daemon is idle — no text field focused.
    Idle,
    /// A text field is focused and we are actively listening/transcribing.
    Listening,
    /// Daemon is paused by the user (global toggle off).
    Paused,
}

/// Runs the main daemon loop, coordinating focus events, STT output, and control commands.
pub fn run(
    stt_rx: Receiver<SttEvent>,
    focus_rx: Receiver<FocusState>,
    ctrl_rx: Receiver<DaemonCommand>,
    shared_state: Arc<Mutex<DaemonState>>,
) {
    let mut state = DaemonState::Idle;
    let mut virtual_kb = match injector::create_virtual_keyboard() {
        Ok(kb) => Some(kb),
        Err(e) => {
            warn!("Failed to create virtual keyboard: {e}");
            warn!("Text injection will be disabled. Transcriptions will be printed to stdout.");
            None
        }
    };

    info!("Daemon running");

    loop {
        select! {
            recv(ctrl_rx) -> msg => {
                match msg {
                    Ok(DaemonCommand::Enable) => {
                        info!("Daemon enabled");
                        state = DaemonState::Idle;
                        update_shared_state(&shared_state, state);
                    }
                    Ok(DaemonCommand::Disable) => {
                        info!("Daemon paused");
                        state = DaemonState::Paused;
                        update_shared_state(&shared_state, state);
                    }
                    Ok(DaemonCommand::Quit) => {
                        info!("Daemon quit requested");
                        break;
                    }
                    Err(_) => {
                        warn!("Control channel closed");
                        break;
                    }
                }
            },
            recv(focus_rx) -> msg => {
                match msg {
                    Ok(FocusState::TextFieldActive(app)) => {
                        if state != DaemonState::Paused {
                            info!("Text field active in: {} ({})", app.window_name, app.wm_class);
                            state = DaemonState::Listening;
                            update_shared_state(&shared_state, state);
                        }
                    }
                    Ok(FocusState::NotActive) => {
                        if state == DaemonState::Listening {
                            info!("No text field focused, going idle");
                            state = DaemonState::Idle;
                            update_shared_state(&shared_state, state);
                        }
                    }
                    Err(_) => {
                        warn!("Focus channel closed");
                        break;
                    }
                }
            },
            recv(stt_rx) -> msg => {
                match msg {
                    Ok(SttEvent::Partial(text)) => {
                        if state == DaemonState::Listening {
                            info!("[partial] {text}");
                        }
                    }
                    Ok(SttEvent::Final(text)) => {
                        if state == DaemonState::Listening {
                            info!("[final] {text}");
                            if let Some(ref mut kb) = virtual_kb {
                                injector::inject_text(kb, &text);
                                injector::inject_text(kb, " ");
                            } else {
                                println!("{text}");
                            }
                        }
                    }
                    Err(_) => {
                        warn!("STT channel closed");
                        break;
                    }
                }
            },
        }
    }

    info!("Daemon stopped");
}

fn update_shared_state(shared: &Arc<Mutex<DaemonState>>, new_state: DaemonState) {
    if let Ok(mut s) = shared.lock() {
        *s = new_state;
    }
}

mod audio;
mod config;
mod daemon;
mod dbus;
mod focus;
#[cfg(feature = "hud")]
mod hud;
mod injector;
mod stt;

use config::Config;
use daemon::DaemonState;
use log::info;
use std::sync::{Arc, Mutex};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Parse model path from CLI args or use default
    let model_path = std::env::args()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| Config::default().model_path);

    let config = Config::new(&model_path);

    info!("=== VoskDictation Voice-to-Text Daemon ===");
    info!("Model path: {}", config.model_path.display());
    info!("Sample rate: {} Hz", config.sample_rate);

    // Load Vosk model
    let model = stt::load_model(&config.model_path).expect("Failed to load Vosk model");

    // Create channels for inter-thread communication
    let (audio_tx, audio_rx) = crossbeam_channel::bounded(32);
    let (stt_tx, stt_rx) = crossbeam_channel::unbounded();
    let (focus_tx, focus_rx) = crossbeam_channel::unbounded();
    let (ctrl_tx, ctrl_rx) = crossbeam_channel::unbounded();

    // Shared daemon state for D-Bus property reads
    let shared_state = Arc::new(Mutex::new(DaemonState::Idle));

    // Start D-Bus service in a background tokio thread
    let dbus_ctrl_tx = ctrl_tx.clone();
    let dbus_state = Arc::clone(&shared_state);
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime for D-Bus");

        rt.block_on(async {
            match dbus::start_dbus_server(dbus_ctrl_tx, dbus_state).await {
                Ok(conn) => {
                    // Keep the connection alive
                    info!("D-Bus server running");
                    loop {
                        conn.monitor_activity().await;
                    }
                }
                Err(e) => {
                    log::error!("Failed to start D-Bus server: {e}");
                    log::warn!("GNOME panel integration will be unavailable");
                }
            }
        });
    });

    // Start audio capture thread
    let _audio_handle = audio::start_capture(config.sample_rate, audio_tx);

    // Start STT processing thread
    let _stt_handle = stt::start_stt(
        model,
        config.sample_rate as f32,
        audio_rx,
        stt_tx,
    );

    // Start focus detection thread
    let _focus_handle = focus::start_focus_monitor(focus_tx);

    info!("All subsystems started. Speak into the microphone.");

    // Run the main daemon loop (blocks)
    daemon::run(stt_rx, focus_rx, ctrl_rx, shared_state);
}

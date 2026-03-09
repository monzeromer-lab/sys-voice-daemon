use crossbeam_channel::Receiver;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Label};
use log::info;
use std::sync::{Arc, Mutex};

use crate::stt::SttEvent;

const APP_ID: &str = "com.assistant.vosk.hud";

/// Starts the GTK4 HUD overlay in a dedicated thread.
/// The HUD displays partial transcriptions and a "Listening..." indicator.
pub fn start_hud(stt_rx: Receiver<SttEvent>) {
    std::thread::spawn(move || {
        let app = Application::builder().application_id(APP_ID).build();

        let stt_rx = Arc::new(Mutex::new(stt_rx));

        app.connect_activate(move |app| {
            let window = ApplicationWindow::builder()
                .application(app)
                .title("Voice Assistant")
                .default_width(400)
                .default_height(60)
                .decorated(false)
                .build();

            // Make the window always on top
            window.set_mnemonics_visible(false);

            let label = Label::new(Some("Listening..."));
            label.set_margin_start(12);
            label.set_margin_end(12);
            label.set_margin_top(8);
            label.set_margin_bottom(8);
            window.set_child(Some(&label));

            // Poll for STT events using a GLib timeout
            let stt_rx = Arc::clone(&stt_rx);
            let label_clone = label.clone();
            gtk4::glib::timeout_add_local(
                std::time::Duration::from_millis(50),
                move || {
                    if let Ok(rx) = stt_rx.lock() {
                        while let Ok(event) = rx.try_recv() {
                            match event {
                                SttEvent::Partial(text) => {
                                    label_clone.set_text(&format!("... {text}"));
                                }
                                SttEvent::Final(text) => {
                                    label_clone.set_text(&text);
                                }
                            }
                        }
                    }
                    gtk4::glib::ControlFlow::Continue
                },
            );

            window.present();
            info!("HUD window presented");
        });

        app.run_with_args::<String>(&[]);
    });
}

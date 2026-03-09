use crossbeam_channel::{Receiver, Sender};
use log::{error, info, warn};
use std::path::Path;
use std::thread;
use vosk::{CompleteResult, DecodingState, Model, Recognizer};

/// Events emitted by the STT engine.
#[derive(Debug, Clone)]
pub enum SttEvent {
    /// Partial (non-final) transcription that may change.
    Partial(String),
    /// Final transcription for a completed utterance.
    Final(String),
}

/// Loads a Vosk model from the given path.
pub fn load_model(model_path: &Path) -> Result<Model, String> {
    info!("Loading Vosk model from: {}", model_path.display());
    Model::new(model_path.to_str().ok_or("Invalid model path encoding")?)
        .ok_or_else(|| format!("Failed to load Vosk model from {}", model_path.display()))
}

/// Starts the STT processing thread.
/// Receives i16 PCM audio chunks from `audio_rx` and sends transcription events to `stt_tx`.
pub fn start_stt(
    model: Model,
    sample_rate: f32,
    audio_rx: Receiver<Vec<i16>>,
    stt_tx: Sender<SttEvent>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        if let Err(e) = stt_loop(&model, sample_rate, &audio_rx, &stt_tx) {
            error!("STT engine error: {e}");
        }
    })
}

fn stt_loop(
    model: &Model,
    sample_rate: f32,
    audio_rx: &Receiver<Vec<i16>>,
    stt_tx: &Sender<SttEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut recognizer = Recognizer::new(model, sample_rate)
        .ok_or("Failed to create Vosk recognizer")?;

    recognizer.set_max_alternatives(0);
    recognizer.set_words(false);
    recognizer.set_partial_words(false);

    info!("Vosk recognizer ready (sample rate: {sample_rate}Hz)");

    for chunk in audio_rx.iter() {
        match recognizer.accept_waveform(&chunk) {
            Ok(DecodingState::Running) => {
                // Still decoding — send partial result
                let partial = recognizer.partial_result();
                if !partial.partial.is_empty() {
                    let _ = stt_tx.send(SttEvent::Partial(partial.partial.to_string()));
                }
            }
            Ok(DecodingState::Finalized) => {
                // Utterance complete (silence detected) — send final result
                if let Some(text) = extract_text(recognizer.result()) {
                    if !text.is_empty() {
                        let _ = stt_tx.send(SttEvent::Final(text));
                    }
                }
            }
            Ok(DecodingState::Failed) => {
                warn!("Vosk decoding failed for a chunk");
            }
            Err(e) => {
                error!("Vosk accept_waveform error: {e}");
            }
        }
    }

    // Flush any remaining audio
    if let Some(text) = extract_text(recognizer.final_result()) {
        if !text.is_empty() {
            let _ = stt_tx.send(SttEvent::Final(text));
        }
    }

    info!("STT engine stopped");
    Ok(())
}

fn extract_text(result: CompleteResult<'_>) -> Option<String> {
    match result {
        CompleteResult::Single(single) => Some(single.text.to_string()),
        CompleteResult::Multiple(multi) => {
            multi.alternatives.first().map(|alt| alt.text.to_string())
        }
    }
}

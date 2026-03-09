use crossbeam_channel::Sender;
use log::{error, info};
use rodio::microphone::MicrophoneBuilder;
use rodio::Source;
use std::num::NonZero;
use std::thread;

/// Size of each audio chunk sent to the STT engine (in samples).
/// At 16kHz mono, 4000 samples = 250ms of audio.
const CHUNK_SIZE: usize = 4000;

/// Starts capturing audio from the default microphone in a background thread.
/// Sends chunks of i16 PCM samples (mono, 16kHz) to the provided channel.
/// Returns a join handle for the capture thread.
pub fn start_capture(
    sample_rate: u32,
    tx: Sender<Vec<i16>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        if let Err(e) = capture_loop(sample_rate, &tx) {
            error!("Audio capture error: {e}");
        }
    })
}

fn capture_loop(
    sample_rate: u32,
    tx: &Sender<Vec<i16>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let sr = NonZero::new(sample_rate).ok_or("sample rate must be non-zero")?;

    let mic = MicrophoneBuilder::new()
        .default_device()?
        .default_config()?
        .prefer_sample_rates([sr])
        .prefer_channel_counts([NonZero::new(1).unwrap()]);

    let mic = mic.open_stream()?;

    let actual_sr = mic.sample_rate().get();
    let actual_ch = mic.channels().get();
    info!(
        "Microphone opened: {}Hz, {} channel(s)",
        actual_sr, actual_ch
    );

    let mut chunk = Vec::with_capacity(CHUNK_SIZE);

    for sample_f32 in mic {
        // Convert f32 [-1.0, 1.0] to i16 for Vosk
        let sample_i16 = f32_to_i16(sample_f32);
        chunk.push(sample_i16);

        if chunk.len() >= CHUNK_SIZE {
            if tx.send(chunk.clone()).is_err() {
                info!("Audio channel closed, stopping capture");
                break;
            }
            chunk.clear();
        }
    }

    Ok(())
}

/// Convert an f32 sample in [-1.0, 1.0] to i16.
fn f32_to_i16(sample: f32) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    (clamped * i16::MAX as f32) as i16
}

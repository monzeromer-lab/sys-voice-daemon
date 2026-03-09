use std::path::PathBuf;

/// Global daemon configuration.
pub struct Config {
    /// Path to the Vosk language model directory.
    pub model_path: PathBuf,
    /// Sample rate for audio capture and Vosk recognizer (must match).
    pub sample_rate: u32,
    /// Number of audio channels (Vosk requires mono = 1).
    pub channels: u16,
    /// Whether the daemon is globally enabled.
    pub enabled: bool,
}

impl Config {
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        Self {
            model_path: model_path.into(),
            sample_rate: 16_000,
            channels: 1,
            enabled: true,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        // Default model path: ~/.local/share/vosk/model
        let model_path = dirs_default();
        Self {
            model_path,
            sample_rate: 16_000,
            channels: 1,
            enabled: true,
        }
    }
}

fn dirs_default() -> PathBuf {
    if let Some(data_dir) = std::env::var_os("XDG_DATA_HOME") {
        PathBuf::from(data_dir).join("vosk").join("model")
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("vosk")
            .join("model")
    } else {
        PathBuf::from("model")
    }
}

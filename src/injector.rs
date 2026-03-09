use evdev::{uinput::VirtualDeviceBuilder, AttributeSet, InputEvent, Key};
use log::{error, info};
use std::thread;
use std::time::Duration;

/// Delay between key press and release events.
const KEY_DELAY: Duration = Duration::from_millis(5);
/// Delay between injecting individual characters.
const CHAR_DELAY: Duration = Duration::from_millis(10);

/// Creates and returns a virtual keyboard device via uinput.
pub fn create_virtual_keyboard() -> Result<evdev::uinput::VirtualDevice, std::io::Error> {
    let mut keys = AttributeSet::<Key>::new();

    // Letters A-Z
    for key in [
        Key::KEY_A, Key::KEY_B, Key::KEY_C, Key::KEY_D, Key::KEY_E,
        Key::KEY_F, Key::KEY_G, Key::KEY_H, Key::KEY_I, Key::KEY_J,
        Key::KEY_K, Key::KEY_L, Key::KEY_M, Key::KEY_N, Key::KEY_O,
        Key::KEY_P, Key::KEY_Q, Key::KEY_R, Key::KEY_S, Key::KEY_T,
        Key::KEY_U, Key::KEY_V, Key::KEY_W, Key::KEY_X, Key::KEY_Y,
        Key::KEY_Z,
    ] {
        keys.insert(key);
    }

    // Numbers 0-9
    for key in [
        Key::KEY_0, Key::KEY_1, Key::KEY_2, Key::KEY_3, Key::KEY_4,
        Key::KEY_5, Key::KEY_6, Key::KEY_7, Key::KEY_8, Key::KEY_9,
    ] {
        keys.insert(key);
    }

    // Punctuation and special keys
    for key in [
        Key::KEY_SPACE, Key::KEY_ENTER, Key::KEY_BACKSPACE, Key::KEY_TAB,
        Key::KEY_DOT, Key::KEY_COMMA, Key::KEY_SEMICOLON, Key::KEY_APOSTROPHE,
        Key::KEY_LEFTBRACE, Key::KEY_RIGHTBRACE, Key::KEY_MINUS, Key::KEY_EQUAL,
        Key::KEY_SLASH, Key::KEY_BACKSLASH, Key::KEY_GRAVE,
        Key::KEY_LEFTSHIFT, Key::KEY_RIGHTSHIFT,
    ] {
        keys.insert(key);
    }

    let device = VirtualDeviceBuilder::new()?
        .name("VoskDictation Virtual Keyboard")
        .with_keys(&keys)?
        .build()?;

    // Give udev a moment to register the device
    thread::sleep(Duration::from_millis(200));
    info!("Virtual keyboard device created");
    Ok(device)
}

/// Injects a string of text by emitting key events on the virtual keyboard.
pub fn inject_text(device: &mut evdev::uinput::VirtualDevice, text: &str) {
    for ch in text.chars() {
        if let Some((key, shifted)) = char_to_key(ch) {
            if shifted {
                press_key(device, Key::KEY_LEFTSHIFT, true);
            }
            press_key(device, key, true);
            thread::sleep(KEY_DELAY);
            press_key(device, key, false);
            if shifted {
                press_key(device, Key::KEY_LEFTSHIFT, false);
            }
            thread::sleep(CHAR_DELAY);
        } else {
            error!("No keycode mapping for character: {ch:?}");
        }
    }
}

fn press_key(device: &mut evdev::uinput::VirtualDevice, key: Key, pressed: bool) {
    let value = if pressed { 1 } else { 0 };
    let event = InputEvent::new_now(evdev::EventType::KEY, key.0, value);
    if let Err(e) = device.emit(&[event]) {
        error!("Failed to emit key event: {e}");
    }
}

/// Maps a character to its (Key, needs_shift) pair for US QWERTY layout.
fn char_to_key(ch: char) -> Option<(Key, bool)> {
    let result = match ch {
        'a' => (Key::KEY_A, false),
        'b' => (Key::KEY_B, false),
        'c' => (Key::KEY_C, false),
        'd' => (Key::KEY_D, false),
        'e' => (Key::KEY_E, false),
        'f' => (Key::KEY_F, false),
        'g' => (Key::KEY_G, false),
        'h' => (Key::KEY_H, false),
        'i' => (Key::KEY_I, false),
        'j' => (Key::KEY_J, false),
        'k' => (Key::KEY_K, false),
        'l' => (Key::KEY_L, false),
        'm' => (Key::KEY_M, false),
        'n' => (Key::KEY_N, false),
        'o' => (Key::KEY_O, false),
        'p' => (Key::KEY_P, false),
        'q' => (Key::KEY_Q, false),
        'r' => (Key::KEY_R, false),
        's' => (Key::KEY_S, false),
        't' => (Key::KEY_T, false),
        'u' => (Key::KEY_U, false),
        'v' => (Key::KEY_V, false),
        'w' => (Key::KEY_W, false),
        'x' => (Key::KEY_X, false),
        'y' => (Key::KEY_Y, false),
        'z' => (Key::KEY_Z, false),
        'A' => (Key::KEY_A, true),
        'B' => (Key::KEY_B, true),
        'C' => (Key::KEY_C, true),
        'D' => (Key::KEY_D, true),
        'E' => (Key::KEY_E, true),
        'F' => (Key::KEY_F, true),
        'G' => (Key::KEY_G, true),
        'H' => (Key::KEY_H, true),
        'I' => (Key::KEY_I, true),
        'J' => (Key::KEY_J, true),
        'K' => (Key::KEY_K, true),
        'L' => (Key::KEY_L, true),
        'M' => (Key::KEY_M, true),
        'N' => (Key::KEY_N, true),
        'O' => (Key::KEY_O, true),
        'P' => (Key::KEY_P, true),
        'Q' => (Key::KEY_Q, true),
        'R' => (Key::KEY_R, true),
        'S' => (Key::KEY_S, true),
        'T' => (Key::KEY_T, true),
        'U' => (Key::KEY_U, true),
        'V' => (Key::KEY_V, true),
        'W' => (Key::KEY_W, true),
        'X' => (Key::KEY_X, true),
        'Y' => (Key::KEY_Y, true),
        'Z' => (Key::KEY_Z, true),
        '0' => (Key::KEY_0, false),
        '1' => (Key::KEY_1, false),
        '2' => (Key::KEY_2, false),
        '3' => (Key::KEY_3, false),
        '4' => (Key::KEY_4, false),
        '5' => (Key::KEY_5, false),
        '6' => (Key::KEY_6, false),
        '7' => (Key::KEY_7, false),
        '8' => (Key::KEY_8, false),
        '9' => (Key::KEY_9, false),
        ' ' => (Key::KEY_SPACE, false),
        '\n' => (Key::KEY_ENTER, false),
        '\t' => (Key::KEY_TAB, false),
        '.' => (Key::KEY_DOT, false),
        ',' => (Key::KEY_COMMA, false),
        ';' => (Key::KEY_SEMICOLON, false),
        '\'' => (Key::KEY_APOSTROPHE, false),
        '-' => (Key::KEY_MINUS, false),
        '=' => (Key::KEY_EQUAL, false),
        '/' => (Key::KEY_SLASH, false),
        '\\' => (Key::KEY_BACKSLASH, false),
        '`' => (Key::KEY_GRAVE, false),
        '[' => (Key::KEY_LEFTBRACE, false),
        ']' => (Key::KEY_RIGHTBRACE, false),
        // Shifted punctuation
        '!' => (Key::KEY_1, true),
        '@' => (Key::KEY_2, true),
        '#' => (Key::KEY_3, true),
        '$' => (Key::KEY_4, true),
        '%' => (Key::KEY_5, true),
        '^' => (Key::KEY_6, true),
        '&' => (Key::KEY_7, true),
        '*' => (Key::KEY_8, true),
        '(' => (Key::KEY_9, true),
        ')' => (Key::KEY_0, true),
        '_' => (Key::KEY_MINUS, true),
        '+' => (Key::KEY_EQUAL, true),
        '{' => (Key::KEY_LEFTBRACE, true),
        '}' => (Key::KEY_RIGHTBRACE, true),
        ':' => (Key::KEY_SEMICOLON, true),
        '"' => (Key::KEY_APOSTROPHE, true),
        '<' => (Key::KEY_COMMA, true),
        '>' => (Key::KEY_DOT, true),
        '?' => (Key::KEY_SLASH, true),
        '|' => (Key::KEY_BACKSLASH, true),
        '~' => (Key::KEY_GRAVE, true),
        _ => return None,
    };
    Some(result)
}

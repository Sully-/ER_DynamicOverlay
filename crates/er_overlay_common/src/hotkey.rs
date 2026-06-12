use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotkeyBinding {
    pub key: OverlayKey,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKey {
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Key0,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    GraveAccent,
    Minus,
    Equal,
    LeftBracket,
    RightBracket,
    Backslash,
    Semicolon,
    Apostrophe,
    Comma,
    Period,
    Slash,
}

impl FromStr for OverlayKey {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "`" => Ok(Self::GraveAccent),
            "-" => Ok(Self::Minus),
            "=" | "+" => Ok(Self::Equal),
            "[" => Ok(Self::LeftBracket),
            "]" => Ok(Self::RightBracket),
            "\\" => Ok(Self::Backslash),
            ";" => Ok(Self::Semicolon),
            "'" => Ok(Self::Apostrophe),
            "," => Ok(Self::Comma),
            "." => Ok(Self::Period),
            "/" => Ok(Self::Slash),
            other => match other.to_ascii_uppercase().as_str() {
                "F1" => Ok(Self::F1),
                "F2" => Ok(Self::F2),
                "F3" => Ok(Self::F3),
                "F4" => Ok(Self::F4),
                "F5" => Ok(Self::F5),
                "F6" => Ok(Self::F6),
                "F7" => Ok(Self::F7),
                "F8" => Ok(Self::F8),
                "F9" => Ok(Self::F9),
                "F10" => Ok(Self::F10),
                "F11" => Ok(Self::F11),
                "F12" => Ok(Self::F12),
                "A" => Ok(Self::A),
                "B" => Ok(Self::B),
                "C" => Ok(Self::C),
                "D" => Ok(Self::D),
                "E" => Ok(Self::E),
                "F" => Ok(Self::F),
                "G" => Ok(Self::G),
                "H" => Ok(Self::H),
                "I" => Ok(Self::I),
                "J" => Ok(Self::J),
                "K" => Ok(Self::K),
                "L" => Ok(Self::L),
                "M" => Ok(Self::M),
                "N" => Ok(Self::N),
                "O" => Ok(Self::O),
                "P" => Ok(Self::P),
                "Q" => Ok(Self::Q),
                "R" => Ok(Self::R),
                "S" => Ok(Self::S),
                "T" => Ok(Self::T),
                "U" => Ok(Self::U),
                "V" => Ok(Self::V),
                "W" => Ok(Self::W),
                "X" => Ok(Self::X),
                "Y" => Ok(Self::Y),
                "Z" => Ok(Self::Z),
                "0" => Ok(Self::Key0),
                "1" => Ok(Self::Key1),
                "2" => Ok(Self::Key2),
                "3" => Ok(Self::Key3),
                "4" => Ok(Self::Key4),
                "5" => Ok(Self::Key5),
                "6" => Ok(Self::Key6),
                "7" => Ok(Self::Key7),
                "8" => Ok(Self::Key8),
                "9" => Ok(Self::Key9),
                "GRAVE" | "BACKQUOTE" | "TILDE" => Ok(Self::GraveAccent),
                "MINUS" | "DASH" | "HYPHEN" => Ok(Self::Minus),
                "EQUAL" | "EQUALS" | "PLUS" => Ok(Self::Equal),
                "LBRACKET" | "LEFTBRACKET" => Ok(Self::LeftBracket),
                "RBRACKET" | "RIGHTBRACKET" => Ok(Self::RightBracket),
                "BACKSLASH" => Ok(Self::Backslash),
                "SEMICOLON" => Ok(Self::Semicolon),
                "APOSTROPHE" | "QUOTE" => Ok(Self::Apostrophe),
                "COMMA" => Ok(Self::Comma),
                "PERIOD" | "DOT" => Ok(Self::Period),
                "SLASH" => Ok(Self::Slash),
                _ => Err(()),
            },
        }
    }
}

pub fn parse_hotkey(raw: &str) -> Option<HotkeyBinding> {
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut key_part: Option<String> = None;

    for part in raw.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        let lower = part.to_ascii_lowercase();
        match lower.as_str() {
            "ctrl" | "control" => ctrl = true,
            "alt" => alt = true,
            "shift" => shift = true,
            other => {
                if key_part.is_some() {
                    return None;
                }
                key_part = Some(other.to_string());
            }
        }
    }

    let key_str = key_part?;
    let key = OverlayKey::from_str(&key_str).ok()?;
    Some(HotkeyBinding {
        key,
        ctrl,
        alt,
        shift,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_f8() {
        let hk = parse_hotkey("F8").unwrap();
        assert_eq!(hk.key, OverlayKey::F8);
        assert!(!hk.ctrl);
    }

    #[test]
    fn parse_ctrl_shift_f1() {
        let hk = parse_hotkey("Ctrl+Shift+F1").unwrap();
        assert_eq!(hk.key, OverlayKey::F1);
        assert!(hk.ctrl);
        assert!(hk.shift);
        assert!(!hk.alt);
    }

    #[test]
    fn parse_symbol_keys() {
        assert_eq!(parse_hotkey("=").unwrap().key, OverlayKey::Equal);
        assert_eq!(parse_hotkey("-").unwrap().key, OverlayKey::Minus);
        assert_eq!(parse_hotkey("comma").unwrap().key, OverlayKey::Comma);
        let hk = parse_hotkey("Ctrl+=").unwrap();
        assert_eq!(hk.key, OverlayKey::Equal);
        assert!(hk.ctrl);
    }

    #[test]
    fn parse_invalid() {
        assert!(parse_hotkey("").is_none());
        assert!(parse_hotkey("Unknown").is_none());
        assert!(parse_hotkey("Ctrl+Alt").is_none());
    }
}

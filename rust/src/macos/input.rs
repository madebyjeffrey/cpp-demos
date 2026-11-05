//! AppKit virtual key codes and menu key-equivalent strings.
use crate::Key;

impl Key {
    pub(crate) fn equivalent(self) -> String {
        use Key::*;
        match self {
            A | B | C | D | E | F | G | H | I | J | K | L | M | N | O | P | Q | R | S | T | U
            | V | W | X | Y | Z => char::from_u32('a' as u32 + self as u32 - A as u32)
                .unwrap()
                .to_string(),
            Num0 | Num1 | Num2 | Num3 | Num4 | Num5 | Num6 | Num7 | Num8 | Num9 => {
                char::from_u32('0' as u32 + self as u32 - Num0 as u32)
                    .unwrap()
                    .to_string()
            }
            F1 | F2 | F3 | F4 | F5 | F6 | F7 | F8 | F9 | F10 | F11 | F12 => {
                char::from_u32(0xf704 + self as u32 - F1 as u32)
                    .unwrap()
                    .to_string()
            }
            Enter => "\r".into(),
            Escape => "\u{1b}".into(),
            Tab => "\t".into(),
            Space => " ".into(),
            Backspace => "\u{8}".into(),
            DeleteForward => "\u{7f}".into(),
            Left => "\u{f702}".into(),
            Right => "\u{f703}".into(),
            Up => "\u{f700}".into(),
            Down => "\u{f701}".into(),
            Home => "\u{f729}".into(),
            End => "\u{f72b}".into(),
            PageUp => "\u{f72c}".into(),
            PageDown => "\u{f72d}".into(),
            Unknown => String::new(),
        }
    }
    pub(crate) fn from_native(code: u16, text: &str) -> Self {
        use Key::*;
        match code {
            53 => Escape,
            36 | 76 => Enter,
            48 => Tab,
            49 => Space,
            51 => Backspace,
            117 => DeleteForward,
            123 => Left,
            124 => Right,
            125 => Down,
            126 => Up,
            115 => Home,
            119 => End,
            116 => PageUp,
            121 => PageDown,
            122 => F1,
            120 => F2,
            99 => F3,
            118 => F4,
            96 => F5,
            97 => F6,
            98 => F7,
            100 => F8,
            101 => F9,
            109 => F10,
            103 => F11,
            111 => F12,
            _ => {
                let lower = text.to_lowercase();
                if lower.chars().count() != 1 {
                    return Unknown;
                }
                let c = lower.chars().next().unwrap();
                const LETTERS: [Key; 26] = [
                    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
                ];
                const DIGITS: [Key; 10] =
                    [Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9];
                if c.is_ascii_lowercase() {
                    LETTERS[c as usize - 'a' as usize]
                } else if c.is_ascii_digit() {
                    DIGITS[c as usize - '0' as usize]
                } else {
                    Unknown
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn key_mapping_handles_layout_and_special_keys() {
        assert_eq!(Key::from_native(0, "A"), Key::A);
        assert_eq!(Key::from_native(0, "é"), Key::Unknown);
        assert_eq!(Key::from_native(123, ""), Key::Left);
        assert_eq!(Key::F12.equivalent(), "\u{f70f}");
    }
}

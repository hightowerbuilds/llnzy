//! Pure routing and encoding for scroll wheel input.
//!
//! A terminal application decides what wheel events mean. When it enables
//! mouse reporting (Claude Code, htop, ...) the wheel must reach the app as
//! mouse escape sequences so it can scroll its own view; when it runs on the
//! alternate screen with alternate scroll enabled (less, vim, ...) the wheel
//! becomes arrow keys; otherwise the wheel scrolls our scrollback. Shift
//! always bypasses the app and scrolls scrollback, matching xterm/alacritty.

/// Where a wheel event should be delivered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WheelRoute {
    /// Scroll the emulator's local scrollback display.
    Scrollback,
    /// Encode as mouse wheel reports and write to the PTY.
    MouseReport,
    /// Encode as arrow key presses and write to the PTY.
    AlternateScroll,
}

pub fn route_wheel(
    mouse_mode: bool,
    alt_screen: bool,
    alternate_scroll: bool,
    shift: bool,
) -> WheelRoute {
    if shift {
        WheelRoute::Scrollback
    } else if mouse_mode {
        WheelRoute::MouseReport
    } else if alt_screen && alternate_scroll {
        WheelRoute::AlternateScroll
    } else {
        WheelRoute::Scrollback
    }
}

/// Encode `lines` wheel steps (positive = up) as mouse reports at the given
/// zero-based grid cell. SGR encoding (`CSI < b ; x ; y M`) has no range
/// limit; legacy X10 encoding clamps to the 223-column/row addressable range
/// and drops reports for cells beyond it.
pub fn encode_wheel_reports(lines: i32, col: usize, row: usize, sgr: bool) -> Vec<u8> {
    if lines == 0 {
        return Vec::new();
    }
    let button = if lines > 0 { 64 } else { 65 };
    let count = lines.unsigned_abs() as usize;
    let mut bytes = Vec::new();
    for _ in 0..count {
        if sgr {
            bytes
                .extend_from_slice(format!("\x1b[<{};{};{}M", button, col + 1, row + 1).as_bytes());
        } else {
            // Legacy encoding stores 32 + (1-based coordinate) in one byte,
            // capped at 255, so coordinates past 223 are unaddressable.
            if col >= 223 || row >= 223 {
                return Vec::new();
            }
            bytes.extend_from_slice(&[
                0x1b,
                b'[',
                b'M',
                32 + button,
                (32 + col + 1) as u8,
                (32 + row + 1) as u8,
            ]);
        }
    }
    bytes
}

/// Encode `lines` wheel steps (positive = up) as arrow key presses for
/// alternate scroll mode, honoring application cursor key encoding.
pub fn encode_alternate_scroll(lines: i32, app_cursor: bool) -> Vec<u8> {
    if lines == 0 {
        return Vec::new();
    }
    let code = if lines > 0 { b'A' } else { b'B' };
    let prefix = if app_cursor { b'O' } else { b'[' };
    let count = lines.unsigned_abs() as usize;
    let mut bytes = Vec::with_capacity(count * 3);
    for _ in 0..count {
        bytes.extend_from_slice(&[0x1b, prefix, code]);
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_always_scrolls_scrollback() {
        assert_eq!(route_wheel(true, true, true, true), WheelRoute::Scrollback);
        assert_eq!(
            route_wheel(true, false, false, true),
            WheelRoute::Scrollback
        );
    }

    #[test]
    fn mouse_mode_routes_to_app() {
        assert_eq!(
            route_wheel(true, false, false, false),
            WheelRoute::MouseReport
        );
        assert_eq!(
            route_wheel(true, true, true, false),
            WheelRoute::MouseReport
        );
    }

    #[test]
    fn alt_screen_with_alternate_scroll_sends_arrows() {
        assert_eq!(
            route_wheel(false, true, true, false),
            WheelRoute::AlternateScroll
        );
        assert_eq!(
            route_wheel(false, true, false, false),
            WheelRoute::Scrollback
        );
        assert_eq!(
            route_wheel(false, false, true, false),
            WheelRoute::Scrollback
        );
    }

    #[test]
    fn primary_screen_without_mouse_mode_scrolls_scrollback() {
        assert_eq!(
            route_wheel(false, false, false, false),
            WheelRoute::Scrollback
        );
    }

    #[test]
    fn sgr_wheel_reports_are_one_based_and_repeated() {
        assert_eq!(
            encode_wheel_reports(2, 4, 9, true),
            b"\x1b[<64;5;10M\x1b[<64;5;10M".to_vec()
        );
        assert_eq!(
            encode_wheel_reports(-1, 0, 0, true),
            b"\x1b[<65;1;1M".to_vec()
        );
    }

    #[test]
    fn legacy_wheel_reports_offset_coordinates() {
        // button 64 -> 32 + 64 = 96 (`\x60`), col 0 -> 33 (`!`), row 0 -> 33.
        assert_eq!(
            encode_wheel_reports(1, 0, 0, false),
            vec![0x1b, b'[', b'M', 96, 33, 33]
        );
    }

    #[test]
    fn legacy_wheel_reports_drop_out_of_range_cells() {
        assert!(encode_wheel_reports(1, 223, 0, false).is_empty());
        assert!(encode_wheel_reports(1, 0, 223, false).is_empty());
    }

    #[test]
    fn zero_lines_encode_nothing() {
        assert!(encode_wheel_reports(0, 5, 5, true).is_empty());
        assert!(encode_alternate_scroll(0, false).is_empty());
    }

    #[test]
    fn alternate_scroll_honors_app_cursor_mode() {
        assert_eq!(encode_alternate_scroll(2, false), b"\x1b[A\x1b[A".to_vec());
        assert_eq!(encode_alternate_scroll(-1, true), b"\x1bOB".to_vec());
    }
}

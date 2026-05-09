use super::{Buffer, Position};

/// How this buffer indents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndentStyle {
    Tabs,
    Spaces(u8),
}

impl Default for IndentStyle {
    fn default() -> Self {
        IndentStyle::Spaces(4)
    }
}

impl IndentStyle {
    /// Detect indent style from file content.
    pub(super) fn detect(text: &str) -> Self {
        let mut tab_lines = 0u32;
        let mut space_lines = 0u32;
        let mut space_widths = [0u32; 9]; // index 1..8

        for line in text.lines().take(200) {
            if line.starts_with('\t') {
                tab_lines += 1;
            } else if line.starts_with(' ') {
                space_lines += 1;
                let spaces = line.len() - line.trim_start_matches(' ').len();
                if (1..=8).contains(&spaces) {
                    space_widths[spaces] += 1;
                }
            }
        }

        if tab_lines > space_lines {
            IndentStyle::Tabs
        } else {
            // Find the most common space width.
            let width = space_widths[1..=8]
                .iter()
                .enumerate()
                .max_by_key(|(_, &count)| count)
                .map(|(i, _)| i + 1)
                .unwrap_or(4) as u8;
            IndentStyle::Spaces(width)
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            IndentStyle::Tabs => "\t",
            IndentStyle::Spaces(1) => " ",
            IndentStyle::Spaces(2) => "  ",
            IndentStyle::Spaces(3) => "   ",
            IndentStyle::Spaces(4) => "    ",
            IndentStyle::Spaces(5) => "     ",
            IndentStyle::Spaces(6) => "      ",
            IndentStyle::Spaces(7) => "       ",
            IndentStyle::Spaces(8) => "        ",
            _ => "    ", // fallback
        }
    }

    pub fn width(self) -> usize {
        match self {
            IndentStyle::Tabs => 1,
            IndentStyle::Spaces(n) => n as usize,
        }
    }
}

pub(super) fn leading_whitespace_len(line: &str) -> usize {
    line.len() - line.trim_start_matches([' ', '\t']).len()
}

impl Buffer {
    /// Indent a range of lines by one level.
    pub fn indent_lines(&mut self, start_line: usize, end_line: usize) {
        let indent = self.indent_style.as_str().to_string();
        // Work backwards to keep positions stable.
        for line_idx in (start_line..=end_line.min(self.line_count().saturating_sub(1))).rev() {
            let pos = Position::new(line_idx, 0);
            self.insert(pos, &indent);
        }
    }

    /// Dedent a range of lines by one level.
    pub fn dedent_lines(&mut self, start_line: usize, end_line: usize) {
        let width = self.indent_style.width();
        // Work backwards to keep positions stable.
        for line_idx in (start_line..=end_line.min(self.line_count().saturating_sub(1))).rev() {
            let line = self.line(line_idx);
            let remove_count = if line.starts_with('\t') {
                1
            } else {
                let spaces = line.len() - line.trim_start_matches(' ').len();
                spaces.min(width)
            };
            if remove_count > 0 {
                self.delete(
                    Position::new(line_idx, 0),
                    Position::new(line_idx, remove_count),
                );
            }
        }
    }

    /// Get the indentation string of a line.
    pub fn line_indent(&self, line_idx: usize) -> &str {
        let line = self.line(line_idx);
        let trimmed = line.trim_start_matches([' ', '\t']);
        &line[..line.len() - trimmed.len()]
    }
}

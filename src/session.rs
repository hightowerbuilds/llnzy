use crate::config::Config;
use crate::pty::Pty;
use crate::terminal::Terminal;

pub struct Session {
    pub terminal: Terminal,
    pub pty: Pty,
    pub title: String,
    pub cwd: Option<String>,  // working directory from OSC 7 or title
    pub custom_name: Option<String>, // user-assigned session name
    pub exited: Option<i32>,  // exit code if shell has exited
}

impl Session {
    pub fn new(
        cols: u16,
        rows: u16,
        config: &Config,
        proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>,
    ) -> std::io::Result<Self> {
        Self::new_in_dir(cols, rows, config, proxy, None)
    }

    pub fn new_in_dir(
        cols: u16,
        rows: u16,
        config: &Config,
        proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>,
        cwd: Option<&str>,
    ) -> std::io::Result<Self> {
        let terminal = Terminal::new(cols, rows);
        let pty = Pty::spawn_in(&config.shell, cols, rows, proxy, cwd)?;
        Ok(Session {
            terminal,
            pty,
            title: "shell".to_string(),
            cwd: cwd.map(|s| s.to_string()),
            custom_name: None,
            exited: None,
        })
    }

    /// Process all available PTY output. Returns (data_changed, clipboard_text, bell_rang).
    pub fn process_output(&mut self) -> (bool, Option<String>, bool) {
        let mut all_bytes = Vec::new();
        while let Some(bytes) = self.pty.try_read() {
            all_bytes.extend_from_slice(&bytes);
        }
        let mut clipboard_text = None;
        let mut bell = false;
        if !all_bytes.is_empty() {
            self.terminal.process(&all_bytes);
            for event in self.terminal.drain_events() {
                match event {
                    crate::terminal::TerminalEvent::Title(t) => {
                        // Try to extract CWD from title (e.g. "user@host: /path" or just "/path")
                        if let Some(path) = extract_cwd_from_title(&t) {
                            self.cwd = Some(path);
                        }
                        self.title = t;
                    }
                    crate::terminal::TerminalEvent::ResetTitle => self.title = "shell".to_string(),
                    crate::terminal::TerminalEvent::PtyWrite(t) => {
                        self.pty.write(t.as_bytes());
                    }
                    crate::terminal::TerminalEvent::ClipboardStore(t) => {
                        clipboard_text = Some(t);
                    }
                    crate::terminal::TerminalEvent::Bell => {
                        bell = true;
                    }
                    crate::terminal::TerminalEvent::ChildExit(code) => {
                        self.exited = Some(code);
                    }
                }
            }
            (true, clipboard_text, bell)
        } else {
            (false, None, false)
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.terminal.resize(cols, rows);
        self.pty.resize(cols, rows);
    }

    /// Display name: custom name > title > "shell"
    pub fn display_name(&self) -> &str {
        if let Some(name) = &self.custom_name {
            name
        } else if !self.title.is_empty() && self.title != "shell" {
            &self.title
        } else {
            "shell"
        }
    }

    pub fn write(&mut self, data: &[u8]) {
        self.terminal.scroll_to_bottom();
        self.pty.write(data);
    }
}

/// Extract a working directory path from a terminal title string.
/// Handles common formats:
///   "user@host: /path/to/dir"
///   "user@host:/path/to/dir"
///   "/path/to/dir"
///   "~" or "~/subdir"
fn extract_cwd_from_title(title: &str) -> Option<String> {
    let title = title.trim();

    // Look for ": /path" or ":/path" pattern
    if let Some(pos) = title.find(": /").or_else(|| title.find(":/")) {
        let path = title[pos..].trim_start_matches(':').trim();
        if path.starts_with('/') {
            return Some(path.to_string());
        }
    }

    // Look for ": ~" pattern
    if let Some(pos) = title.find(": ~").or_else(|| title.find(":~")) {
        let path = title[pos..].trim_start_matches(':').trim();
        if path.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                let expanded = path.replacen('~', &home.to_string_lossy(), 1);
                return Some(expanded);
            }
        }
    }

    // Plain path
    if title.starts_with('/') {
        return Some(title.to_string());
    }
    if title.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            return Some(title.replacen('~', &home.to_string_lossy(), 1));
        }
    }

    None
}

// ── Split pane tree ──

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitDir {
    Horizontal,
    Vertical,
}

pub enum PaneNode {
    Leaf(Box<Session>),
    Split {
        dir: SplitDir,
        ratio: f32,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
        active_second: bool,
    },
}

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl PaneNode {
    /// Get mutable reference to the active (focused) session.
    pub fn active_mut(&mut self) -> &mut Session {
        match self {
            PaneNode::Leaf(s) => s,
            PaneNode::Split {
                first,
                second,
                active_second,
                ..
            } => {
                if *active_second {
                    second.active_mut()
                } else {
                    first.active_mut()
                }
            }
        }
    }

    pub fn active(&self) -> &Session {
        match self {
            PaneNode::Leaf(s) => s,
            PaneNode::Split {
                first,
                second,
                active_second,
                ..
            } => {
                if *active_second {
                    second.active()
                } else {
                    first.active()
                }
            }
        }
    }

    /// Process PTY output for ALL sessions in this tree.
    /// Returns (any_changed, Vec<clipboard_texts>, bell_rang).
    pub fn process_all(&mut self) -> (bool, Vec<String>, bool) {
        match self {
            PaneNode::Leaf(s) => {
                let (changed, clip, bell) = s.process_output();
                (changed, clip.into_iter().collect(), bell)
            }
            PaneNode::Split { first, second, .. } => {
                let (a, mut ca, bell_a) = first.process_all();
                let (b, cb, bell_b) = second.process_all();
                ca.extend(cb);
                (a || b, ca, bell_a || bell_b)
            }
        }
    }

    /// Resize all panes to fit the given rect.
    pub fn resize_all(&mut self, rect: Rect, cell_w: f32, cell_h: f32) {
        match self {
            PaneNode::Leaf(s) => {
                let cols = (rect.w / cell_w).max(1.0) as u16;
                let rows = (rect.h / cell_h).max(1.0) as u16;
                s.resize(cols, rows);
            }
            PaneNode::Split {
                dir,
                ratio,
                first,
                second,
                ..
            } => {
                let (r1, r2) = split_rect(rect, *dir, *ratio);
                first.resize_all(r1, cell_w, cell_h);
                second.resize_all(r2, cell_w, cell_h);
            }
        }
    }

    /// Collect all (session_ref, rect, is_active) for rendering.
    pub fn collect_panes(&self, rect: Rect, is_active: bool) -> Vec<(&Session, Rect, bool)> {
        match self {
            PaneNode::Leaf(s) => vec![(s, rect, is_active)],
            PaneNode::Split {
                dir,
                ratio,
                first,
                second,
                active_second,
                ..
            } => {
                let (r1, r2) = split_rect(rect, *dir, *ratio);
                let mut result = first.collect_panes(r1, is_active && !active_second);
                result.extend(second.collect_panes(r2, is_active && *active_second));
                result
            }
        }
    }

    /// Collect divider line rects for split borders.
    pub fn collect_dividers(&self, rect: Rect) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
        match self {
            PaneNode::Leaf(_) => vec![],
            PaneNode::Split {
                dir,
                ratio,
                first,
                second,
                ..
            } => {
                let (r1, r2) = split_rect(rect, *dir, *ratio);
                let divider_color = [0.3, 0.3, 0.35, 1.0];
                let mut rects = match dir {
                    SplitDir::Vertical => {
                        vec![(r1.x + r1.w, rect.y, 1.0, rect.h, divider_color)]
                    }
                    SplitDir::Horizontal => {
                        vec![(rect.x, r1.y + r1.h, rect.w, 1.0, divider_color)]
                    }
                };
                rects.extend(first.collect_dividers(r1));
                rects.extend(second.collect_dividers(r2));
                rects
            }
        }
    }

    /// Switch focus to the other child at the deepest split containing the active pane.
    pub fn cycle_focus(&mut self) {
        match self {
            PaneNode::Leaf(_) => {}
            PaneNode::Split {
                first,
                second,
                active_second,
                ..
            } => {
                // If the active child is a leaf, toggle. Otherwise recurse.
                let active_child = if *active_second { &**second } else { &**first };
                if matches!(active_child, PaneNode::Leaf(_)) {
                    *active_second = !*active_second;
                } else if *active_second {
                    second.cycle_focus();
                } else {
                    first.cycle_focus();
                }
            }
        }
    }

}

/// Split the active pane. Takes ownership and returns the new tree.
pub fn split_pane(
    node: PaneNode,
    dir: SplitDir,
    config: &Config,
    cols: u16,
    rows: u16,
    proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>,
) -> std::io::Result<PaneNode> {
    match node {
        PaneNode::Leaf(session) => {
            let new_session = Session::new(cols, rows, config, proxy)?;
            Ok(PaneNode::Split {
                dir,
                ratio: 0.5,
                first: Box::new(PaneNode::Leaf(session)),
                second: Box::new(PaneNode::Leaf(Box::new(new_session))),
                active_second: true,
            })
        }
        PaneNode::Split {
            dir: d,
            ratio,
            first,
            second,
            active_second,
        } => {
            if active_second {
                Ok(PaneNode::Split {
                    dir: d,
                    ratio,
                    first,
                    second: Box::new(split_pane(*second, dir, config, cols, rows, proxy)?),
                    active_second,
                })
            } else {
                Ok(PaneNode::Split {
                    dir: d,
                    ratio,
                    first: Box::new(split_pane(*first, dir, config, cols, rows, proxy)?),
                    second,
                    active_second,
                })
            }
        }
    }
}

fn split_rect(rect: Rect, dir: SplitDir, ratio: f32) -> (Rect, Rect) {
    let div = 1.0;
    match dir {
        SplitDir::Vertical => {
            let w1 = (rect.w * ratio - div / 2.0).max(0.0);
            let w2 = (rect.w * (1.0 - ratio) - div / 2.0).max(0.0);
            (
                Rect { w: w1, ..rect },
                Rect {
                    x: rect.x + w1 + div,
                    w: w2,
                    ..rect
                },
            )
        }
        SplitDir::Horizontal => {
            let h1 = (rect.h * ratio - div / 2.0).max(0.0);
            let h2 = (rect.h * (1.0 - ratio) - div / 2.0).max(0.0);
            (
                Rect { h: h1, ..rect },
                Rect {
                    y: rect.y + h1 + div,
                    h: h2,
                    ..rect
                },
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    // ── split_rect ──

    #[test]
    fn split_rect_vertical_even() {
        let r = rect(0.0, 0.0, 100.0, 50.0);
        let (r1, r2) = split_rect(r, SplitDir::Vertical, 0.5);
        // w1 = 100*0.5 - 0.5 = 49.5
        assert!((r1.w - 49.5).abs() < 0.01);
        assert!((r2.w - 49.5).abs() < 0.01);
        // r2 starts after r1 + 1px divider
        assert!((r2.x - 50.5).abs() < 0.01);
        // Heights unchanged
        assert_eq!(r1.h, 50.0);
        assert_eq!(r2.h, 50.0);
    }

    #[test]
    fn split_rect_horizontal_even() {
        let r = rect(0.0, 0.0, 80.0, 100.0);
        let (r1, r2) = split_rect(r, SplitDir::Horizontal, 0.5);
        assert!((r1.h - 49.5).abs() < 0.01);
        assert!((r2.h - 49.5).abs() < 0.01);
        assert!((r2.y - 50.5).abs() < 0.01);
        // Widths unchanged
        assert_eq!(r1.w, 80.0);
        assert_eq!(r2.w, 80.0);
    }

    #[test]
    fn split_rect_vertical_uneven_ratio() {
        let r = rect(10.0, 20.0, 200.0, 100.0);
        let (r1, r2) = split_rect(r, SplitDir::Vertical, 0.3);
        // w1 = 200*0.3 - 0.5 = 59.5
        assert!((r1.w - 59.5).abs() < 0.01);
        // w2 = 200*0.7 - 0.5 = 139.5
        assert!((r2.w - 139.5).abs() < 0.01);
        // r1 starts at same x as parent
        assert_eq!(r1.x, 10.0);
        // r2 starts after r1 + divider
        assert!((r2.x - (10.0 + 59.5 + 1.0)).abs() < 0.01);
    }

    #[test]
    fn split_rect_preserves_origin() {
        let r = rect(50.0, 100.0, 400.0, 300.0);
        let (r1, _) = split_rect(r, SplitDir::Vertical, 0.5);
        assert_eq!(r1.x, 50.0);
        assert_eq!(r1.y, 100.0);
    }

    #[test]
    fn split_rect_tiny_rect_clamps_to_zero() {
        let r = rect(0.0, 0.0, 1.0, 1.0);
        let (r1, r2) = split_rect(r, SplitDir::Vertical, 0.5);
        assert!(r1.w >= 0.0);
        assert!(r2.w >= 0.0);
    }

    // ── SplitDir equality ──

    #[test]
    fn split_dir_equality() {
        assert_eq!(SplitDir::Horizontal, SplitDir::Horizontal);
        assert_eq!(SplitDir::Vertical, SplitDir::Vertical);
        assert_ne!(SplitDir::Horizontal, SplitDir::Vertical);
    }
}

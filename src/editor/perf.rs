use std::collections::VecDeque;
use std::time::Instant;

/// Performance thresholds for the editor.

/// Files above this line count disable syntax highlighting.
pub const SYNTAX_LINE_LIMIT: usize = 100_000;

/// Files above this line count disable the minimap.
pub const MINIMAP_LINE_LIMIT: usize = 50_000;

/// Maximum number of lines to send for a single LSP didChange.
/// Files larger than this use full-document sync only on save.
pub const LSP_CHANGE_LINE_LIMIT: usize = 200_000;

/// Maximum number of completion items to display.
pub const MAX_COMPLETION_ITEMS: usize = 20;

/// Maximum number of fuzzy finder results.
pub const MAX_FINDER_RESULTS: usize = 30;

/// Minimum milliseconds between LSP didChange notifications.
pub const LSP_DEBOUNCE_MS: u64 = 100;

/// Maximum highlight query range (lines beyond viewport).
pub const HIGHLIGHT_BUFFER_LINES: usize = 50;

/// Check whether syntax highlighting should be active for a buffer.
pub fn syntax_enabled(line_count: usize) -> bool {
    line_count <= SYNTAX_LINE_LIMIT
}

/// Check whether the minimap should be rendered.
pub fn minimap_enabled(line_count: usize) -> bool {
    line_count <= MINIMAP_LINE_LIMIT
}

/// Tracks performance metrics for the profiling overlay.
pub struct PerfStats {
    /// Keystroke-to-render latency samples (ms).
    keystroke_latencies: VecDeque<f32>,
    /// Timestamp of the last keystroke (for latency measurement).
    last_keystroke: Option<Instant>,
    /// Per-subsystem frame time breakdown (ms).
    pub render_ms: f32,
    pub syntax_ms: f32,
    pub egui_ms: f32,
    /// Memory estimates.
    pub rope_bytes: usize,
    pub undo_depth: usize,
    pub tree_sitter_active: bool,
}

impl Default for PerfStats {
    fn default() -> Self {
        Self {
            keystroke_latencies: VecDeque::with_capacity(64),
            last_keystroke: None,
            render_ms: 0.0,
            syntax_ms: 0.0,
            egui_ms: 0.0,
            rope_bytes: 0,
            undo_depth: 0,
            tree_sitter_active: false,
        }
    }
}

impl PerfStats {
    /// Record that a keystroke occurred now. Call from the input handler.
    pub fn mark_keystroke(&mut self) {
        self.last_keystroke = Some(Instant::now());
    }

    /// Record that the frame rendered. If a keystroke was pending, compute its latency.
    pub fn mark_frame_rendered(&mut self) {
        if let Some(ts) = self.last_keystroke.take() {
            let latency_ms = ts.elapsed().as_secs_f32() * 1000.0;
            if self.keystroke_latencies.len() >= 64 {
                self.keystroke_latencies.pop_front();
            }
            self.keystroke_latencies.push_back(latency_ms);
        }
    }

    /// Average keystroke-to-pixel latency (ms), or None if no samples.
    pub fn avg_keystroke_latency(&self) -> Option<f32> {
        if self.keystroke_latencies.is_empty() {
            return None;
        }
        let sum: f32 = self.keystroke_latencies.iter().sum();
        Some(sum / self.keystroke_latencies.len() as f32)
    }

    /// P95 keystroke-to-pixel latency (ms), or None if not enough samples.
    pub fn p95_keystroke_latency(&self) -> Option<f32> {
        if self.keystroke_latencies.len() < 5 {
            return None;
        }
        let mut sorted: Vec<f32> = self.keystroke_latencies.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = (sorted.len() as f32 * 0.95) as usize;
        Some(sorted[idx.min(sorted.len() - 1)])
    }

    /// Update memory stats from the active buffer.
    pub fn update_buffer_stats(&mut self, rope_bytes: usize, undo_depth: usize, has_tree: bool) {
        self.rope_bytes = rope_bytes;
        self.undo_depth = undo_depth;
        self.tree_sitter_active = has_tree;
    }

    /// Format a summary string for the overlay.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        if let Some(avg) = self.avg_keystroke_latency() {
            let p95 = self.p95_keystroke_latency().unwrap_or(avg);
            parts.push(format!("Key: {avg:.1}ms avg / {p95:.1}ms p95"));
        }

        if self.render_ms > 0.0 {
            parts.push(format!("Render: {:.1}ms", self.render_ms));
        }
        if self.syntax_ms > 0.0 {
            parts.push(format!("Syntax: {:.1}ms", self.syntax_ms));
        }

        let rope_kb = self.rope_bytes as f64 / 1024.0;
        parts.push(format!("Rope: {rope_kb:.0}KB  Undo: {}  TS: {}", self.undo_depth,
            if self.tree_sitter_active { "on" } else { "off" }));

        parts.join("  |  ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keystroke_latency_tracking() {
        let mut stats = PerfStats::default();
        stats.mark_keystroke();
        std::thread::sleep(std::time::Duration::from_millis(2));
        stats.mark_frame_rendered();
        assert!(stats.avg_keystroke_latency().is_some());
        assert!(stats.avg_keystroke_latency().unwrap() >= 1.0);
    }

    #[test]
    fn no_latency_without_keystroke() {
        let mut stats = PerfStats::default();
        stats.mark_frame_rendered();
        assert!(stats.avg_keystroke_latency().is_none());
    }

    #[test]
    fn buffer_stats_update() {
        let mut stats = PerfStats::default();
        stats.update_buffer_stats(4096, 15, true);
        assert_eq!(stats.rope_bytes, 4096);
        assert_eq!(stats.undo_depth, 15);
        assert!(stats.tree_sitter_active);
    }

    #[test]
    fn summary_includes_rope_info() {
        let mut stats = PerfStats::default();
        stats.update_buffer_stats(2048, 5, true);
        let s = stats.summary();
        assert!(s.contains("Rope:"));
        assert!(s.contains("Undo: 5"));
        assert!(s.contains("TS: on"));
    }
}

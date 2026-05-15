use std::time::{Duration, Instant};

use llnzy::editor::buffer::{Buffer, Position};
use llnzy::editor::syntax::SyntaxEngine;
use llnzy::terminal::Terminal;

#[test]
#[ignore = "release-mode performance budget; run with cargo test --release --test performance_budgets -- --ignored --nocapture"]
fn editor_large_insert_budget() {
    let source = numbered_lines("let value = 1;", 20_000);
    let elapsed = measure(|| {
        let mut buffer = Buffer::empty();
        buffer.insert(Position::new(0, 0), &source);
        for _ in 0..100 {
            let end = buffer.char_to_pos(buffer.len_chars());
            buffer.insert(end, "x");
        }
        assert!(buffer.len_chars() >= source.chars().count());
    });

    assert_under("editor large insert", elapsed, Duration::from_millis(500));
}

#[test]
#[ignore = "release-mode performance budget; run with cargo test --release --test performance_budgets -- --ignored --nocapture"]
fn syntax_parse_rust_budget() {
    let source = rust_functions(3_000);
    let elapsed = measure(|| {
        let mut syntax = SyntaxEngine::new();
        let tree = syntax.parse("rust", &source);
        assert!(tree.is_some());
    });

    assert_under("rust syntax parse", elapsed, Duration::from_secs(1));
}

#[test]
#[ignore = "release-mode performance budget; run with cargo test --release --test performance_budgets -- --ignored --nocapture"]
fn terminal_output_throughput_budget() {
    let payload = terminal_payload(30_000);
    let elapsed = measure(|| {
        let mut terminal = Terminal::new(120, 40);
        terminal.process(payload.as_bytes());
        assert!(terminal.history_size() > 0);
    });

    assert_under(
        "terminal output throughput",
        elapsed,
        Duration::from_millis(500),
    );
}

fn measure(work: impl FnOnce()) -> Duration {
    let start = Instant::now();
    work();
    start.elapsed()
}

fn assert_under(name: &str, elapsed: Duration, budget: Duration) {
    eprintln!("{name}: {:.2?} budget {:.2?}", elapsed, budget);
    assert!(
        elapsed <= budget,
        "{name} took {:.2?}, over budget {:.2?}",
        elapsed,
        budget
    );
}

fn numbered_lines(line: &str, count: usize) -> String {
    let mut out = String::with_capacity((line.len() + 16) * count);
    for idx in 0..count {
        out.push_str(line);
        out.push_str(" // ");
        out.push_str(&idx.to_string());
        out.push('\n');
    }
    out
}

fn rust_functions(count: usize) -> String {
    let mut out = String::with_capacity(count * 64);
    for idx in 0..count {
        out.push_str("fn generated_");
        out.push_str(&idx.to_string());
        out.push_str("() -> usize { ");
        out.push_str(&idx.to_string());
        out.push_str(" }\n");
    }
    out
}

fn terminal_payload(lines: usize) -> String {
    let mut out = String::with_capacity(lines * 40);
    for idx in 0..lines {
        out.push_str("\x1b[32mstatus\x1b[0m ");
        out.push_str(&idx.to_string());
        out.push_str(" complete\r\n");
    }
    out
}

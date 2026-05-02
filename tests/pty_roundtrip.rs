//! PTY round-trip tests.
//!
//! These tests spawn a real PTY with a shell, send commands,
//! and verify output comes back through the terminal emulator.
//!
//! We use portable_pty directly (the same library llnzy uses) rather than
//! going through our Pty wrapper, which requires a winit EventLoopProxy
//! that can't be created in test threads on macOS.

use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use portable_pty::{native_pty_system, Child, CommandBuilder, PtySize};

use llnzy::terminal::Terminal;

/// Spawn a shell in a PTY and return the terminal, reader channel, and writer.
type PtyWriter = Box<dyn Write + Send>;
type PtyMaster = Box<dyn portable_pty::MasterPty + Send>;
type PtyChild = Box<dyn Child + Send + Sync>;

fn spawn_shell(cols: u16, rows: u16) -> (Terminal, mpsc::Receiver<Vec<u8>>, PtyWriter, PtyMaster) {
    let (terminal, rx, writer, master, _child) = spawn_shell_with_child(cols, rows);
    (terminal, rx, writer, master)
}

fn spawn_shell_with_child(
    cols: u16,
    rows: u16,
) -> (
    Terminal,
    mpsc::Receiver<Vec<u8>>,
    PtyWriter,
    PtyMaster,
    PtyChild,
) {
    let pty_system = native_pty_system();
    let size = PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = pty_system.openpty(size).expect("Failed to open PTY");

    let mut cmd = CommandBuilder::new("/bin/sh");
    cmd.env("TERM", "xterm-256color");

    let child = pair
        .slave
        .spawn_command(cmd)
        .expect("Failed to spawn shell");
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .expect("Failed to clone reader");
    let writer = pair.master.take_writer().expect("Failed to take writer");

    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let mut buf = [0u8; 65536];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let terminal = Terminal::new(cols, rows);
    (terminal, rx, writer, pair.master, child)
}

/// Drain all available output from the PTY channel into the terminal.
fn drain(terminal: &mut Terminal, rx: &mpsc::Receiver<Vec<u8>>) -> bool {
    let mut got_data = false;
    while let Ok(bytes) = rx.try_recv() {
        terminal.process(&bytes);
        got_data = true;
    }
    got_data
}

/// Wait for PTY output with timeout.
fn wait_for_output(
    terminal: &mut Terminal,
    rx: &mpsc::Receiver<Vec<u8>>,
    timeout: Duration,
) -> bool {
    let start = Instant::now();
    loop {
        if drain(terminal, rx) {
            return true;
        }
        if start.elapsed() > timeout {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn wait_for_text(
    terminal: &mut Terminal,
    rx: &mpsc::Receiver<Vec<u8>>,
    needle: &str,
    timeout: Duration,
) -> bool {
    let start = Instant::now();
    loop {
        drain(terminal, rx);
        if all_text(terminal).contains(needle) {
            return true;
        }
        if start.elapsed() > timeout {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn wait_for_child_exit(child: &mut PtyChild, timeout: Duration) -> Option<i32> {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("child status check failed") {
            return Some(status.exit_code() as i32);
        }
        if start.elapsed() > timeout {
            return None;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn wait_for_reader_disconnect(
    terminal: &mut Terminal,
    rx: &mpsc::Receiver<Vec<u8>>,
    timeout: Duration,
) -> bool {
    let start = Instant::now();
    loop {
        drain(terminal, rx);
        match rx.recv_timeout(Duration::from_millis(25)) {
            Ok(bytes) => terminal.process(&bytes),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return true,
        }
        if start.elapsed() > timeout {
            return false;
        }
    }
}

/// Read a terminal row as a trimmed string.
fn read_line(term: &Terminal, row: usize, cols: usize) -> String {
    (0..cols)
        .map(|c| term.cell_char(row, c))
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// Collect all terminal text into one string.
fn all_text(term: &Terminal) -> String {
    let (cols, rows) = term.size();
    (0..rows)
        .map(|r| read_line(term, r, cols))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn pty_spawn_and_read_prompt() {
    let (mut terminal, rx, _writer, _master) = spawn_shell(80, 24);

    let got_output = wait_for_output(&mut terminal, &rx, Duration::from_secs(3));
    assert!(got_output, "Shell should produce output (prompt)");

    let line = read_line(&terminal, 0, 80);
    assert!(
        !line.is_empty(),
        "Prompt should be non-empty, got: '{}'",
        line
    );
}

#[test]
fn pty_echo_command() {
    let (mut terminal, rx, mut writer, _master) = spawn_shell(80, 24);

    // Wait for initial prompt
    wait_for_output(&mut terminal, &rx, Duration::from_secs(3));

    // Send a command
    writer.write_all(b"echo HELLO_LLNZY_TEST\n").unwrap();
    writer.flush().unwrap();

    // Wait for output and let it settle
    wait_for_output(&mut terminal, &rx, Duration::from_secs(3));
    std::thread::sleep(Duration::from_millis(200));
    drain(&mut terminal, &rx);

    let text = all_text(&terminal);
    // The output line (not the command echo itself) should contain our marker
    let output_lines: Vec<&str> = text
        .lines()
        .filter(|l| {
            l.contains("HELLO_LLNZY_TEST") && !l.starts_with("echo") && !l.contains("echo ")
        })
        .collect();
    assert!(
        !output_lines.is_empty(),
        "echo output should appear in terminal. Full text:\n{}",
        text
    );
}

#[test]
fn pty_resize() {
    let (mut terminal, rx, _writer, master) = spawn_shell(80, 24);

    wait_for_output(&mut terminal, &rx, Duration::from_secs(3));

    // Resize should not panic
    let _ = master.resize(PtySize {
        rows: 40,
        cols: 120,
        pixel_width: 0,
        pixel_height: 0,
    });
    terminal.resize(120, 40);
    assert_eq!(terminal.size(), (120, 40));

    // PTY should still work after resize
    std::thread::sleep(Duration::from_millis(100));
    drain(&mut terminal, &rx);
}

#[test]
fn pty_write_and_read_multiple() {
    let (mut terminal, rx, mut writer, _master) = spawn_shell(80, 24);

    wait_for_output(&mut terminal, &rx, Duration::from_secs(3));

    // Send first command
    writer.write_all(b"echo AAA_MARKER\n").unwrap();
    writer.flush().unwrap();
    wait_for_output(&mut terminal, &rx, Duration::from_secs(2));
    std::thread::sleep(Duration::from_millis(200));
    drain(&mut terminal, &rx);

    // Send second command
    writer.write_all(b"echo BBB_MARKER\n").unwrap();
    writer.flush().unwrap();
    wait_for_output(&mut terminal, &rx, Duration::from_secs(2));
    std::thread::sleep(Duration::from_millis(200));
    drain(&mut terminal, &rx);

    let text = all_text(&terminal);
    assert!(
        text.contains("AAA_MARKER"),
        "First echo output missing. Full text:\n{}",
        text
    );
    assert!(
        text.contains("BBB_MARKER"),
        "Second echo output missing. Full text:\n{}",
        text
    );
}

#[test]
fn pty_reader_closes_on_shell_exit() {
    let (mut terminal, rx, mut writer, _master) = spawn_shell(80, 24);

    wait_for_output(&mut terminal, &rx, Duration::from_secs(3));

    // Tell the shell to exit
    writer.write_all(b"exit 0\n").unwrap();
    writer.flush().unwrap();

    // The reader thread should stop producing data after the shell exits.
    // We detect this by waiting until the channel has no more data and
    // subsequent waits time out (meaning the reader thread has ended).
    let start = Instant::now();
    let mut last_data = Instant::now();
    loop {
        if drain(&mut terminal, &rx) {
            last_data = Instant::now();
        }
        // If we haven't received data for 1 second after exit, reader has closed
        if last_data.elapsed() > Duration::from_secs(1) {
            break;
        }
        if start.elapsed() > Duration::from_secs(5) {
            panic!("Timed out waiting for reader to close after shell exit");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    // If we got here, the reader thread closed — test passes
}

#[test]
fn pty_exit_status_and_restart_are_clear() {
    let (mut terminal, rx, mut writer, _master, mut child) = spawn_shell_with_child(80, 24);

    wait_for_output(&mut terminal, &rx, Duration::from_secs(3));
    writer
        .write_all(b"printf 'EXIT_BEFORE_RESTART\\n'; exit 23\n")
        .unwrap();
    writer.flush().unwrap();

    assert!(
        wait_for_text(
            &mut terminal,
            &rx,
            "EXIT_BEFORE_RESTART",
            Duration::from_secs(3)
        ),
        "pre-exit marker missing. Full text:\n{}",
        all_text(&terminal)
    );
    assert_eq!(
        wait_for_child_exit(&mut child, Duration::from_secs(5)),
        Some(23),
        "first shell exit status should be observable"
    );
    assert!(
        wait_for_reader_disconnect(&mut terminal, &rx, Duration::from_secs(3)),
        "reader should disconnect after first shell exits"
    );

    let (
        mut restarted_terminal,
        restarted_rx,
        mut restarted_writer,
        _restarted_master,
        mut restarted_child,
    ) = spawn_shell_with_child(80, 24);
    wait_for_output(
        &mut restarted_terminal,
        &restarted_rx,
        Duration::from_secs(3),
    );
    restarted_writer
        .write_all(b"printf 'RESTARTED_SHELL_READY\\n'; exit 0\n")
        .unwrap();
    restarted_writer.flush().unwrap();

    assert!(
        wait_for_text(
            &mut restarted_terminal,
            &restarted_rx,
            "RESTARTED_SHELL_READY",
            Duration::from_secs(3),
        ),
        "restarted shell marker missing. Full text:\n{}",
        all_text(&restarted_terminal)
    );
    assert_eq!(
        wait_for_child_exit(&mut restarted_child, Duration::from_secs(5)),
        Some(0),
        "restarted shell should exit cleanly"
    );
}

#[test]
fn pty_kill_terminates_child_and_reader() {
    let (mut terminal, rx, _writer, _master, mut child) = spawn_shell_with_child(80, 24);

    wait_for_output(&mut terminal, &rx, Duration::from_secs(3));
    child.kill().expect("failed to kill shell");

    assert!(
        wait_for_child_exit(&mut child, Duration::from_secs(5)).is_some(),
        "killed shell should report an exit status"
    );
    assert!(
        wait_for_reader_disconnect(&mut terminal, &rx, Duration::from_secs(3)),
        "reader should disconnect after shell is killed"
    );
}

#[test]
fn pty_resize_while_output_streams() {
    let (mut terminal, rx, mut writer, master) = spawn_shell(80, 24);

    wait_for_output(&mut terminal, &rx, Duration::from_secs(3));
    writer
        .write_all(
            b"i=1; while [ $i -le 80 ]; do printf 'STREAM_%03d abcdefghijklmnopqrstuvwxyz\\n' \"$i\"; i=$((i + 1)); done; printf 'STREAM_DONE\\n'\n",
        )
        .unwrap();
    writer.flush().unwrap();

    for (cols, rows) in [(100, 30), (72, 18), (120, 36), (80, 24)] {
        std::thread::sleep(Duration::from_millis(20));
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        terminal.resize(cols, rows);
        drain(&mut terminal, &rx);
    }

    assert!(
        wait_for_text(&mut terminal, &rx, "STREAM_DONE", Duration::from_secs(5)),
        "stream should complete after repeated resizes. Full text:\n{}",
        all_text(&terminal)
    );
    assert_eq!(terminal.size(), (80, 24));
}

#[test]
fn pty_large_paste_reaches_shell_intact() {
    let (mut terminal, rx, mut writer, _master) = spawn_shell(100, 30);

    wait_for_output(&mut terminal, &rx, Duration::from_secs(3));

    let mut payload = String::new();
    for i in 0..1536 {
        payload.push_str(&format!(
            "LLNZY_LARGE_PASTE_{i:04}: abcdefghijklmnopqrstuvwxyz\n"
        ));
    }
    let expected_bytes = payload.len();

    let mut command = String::new();
    command.push_str("p=/tmp/llnzy_pty_large_$$\n");
    command.push_str("cat > \"$p\" <<'LLNZY_EOF'\n");
    command.push_str(&payload);
    command.push_str("LLNZY_EOF\n");
    command.push_str("printf 'LARGE_BYTES:%s\\n' \"$(wc -c < \"$p\" | tr -d ' ')\"\n");
    command.push_str("rm -f \"$p\"\n");
    command.push_str("printf 'LARGE_DONE\\n'\n");

    writer.write_all(command.as_bytes()).unwrap();
    writer.flush().unwrap();

    assert!(
        wait_for_text(&mut terminal, &rx, "LARGE_DONE", Duration::from_secs(10)),
        "large paste should finish. Full text:\n{}",
        all_text(&terminal)
    );

    let text = all_text(&terminal);
    assert!(
        text.contains(&format!("LARGE_BYTES:{expected_bytes}")),
        "shell should receive the full large paste. Expected {expected_bytes} bytes. Full text:\n{text}"
    );
}

#[test]
fn pty_osc_title_event_survives_real_pty_roundtrip() {
    use llnzy::terminal::TerminalEvent;

    let (mut terminal, rx, mut writer, _master) = spawn_shell(80, 24);

    wait_for_output(&mut terminal, &rx, Duration::from_secs(3));
    writer
        .write_all(b"printf '\\033]0;LLNZY_OSC_TITLE\\007'\n")
        .unwrap();
    writer.flush().unwrap();

    let start = Instant::now();
    let mut title_seen = false;
    while start.elapsed() <= Duration::from_secs(3) {
        drain(&mut terminal, &rx);
        if terminal
            .drain_events()
            .iter()
            .any(|event| matches!(event, TerminalEvent::Title(title) if title == "LLNZY_OSC_TITLE"))
        {
            title_seen = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    assert!(title_seen, "OSC title event should survive PTY roundtrip");
}

#[test]
fn pty_osc7_working_directory_event_survives_real_pty_roundtrip() {
    use llnzy::terminal::TerminalEvent;

    let (mut terminal, rx, mut writer, _master) = spawn_shell(80, 24);

    wait_for_output(&mut terminal, &rx, Duration::from_secs(3));
    writer
        .write_all(b"printf '\\033]7;file://localhost/tmp/llnzy_osc7_roundtrip\\007'\n")
        .unwrap();
    writer.flush().unwrap();

    let start = Instant::now();
    let mut cwd_seen = false;
    while start.elapsed() <= Duration::from_secs(3) {
        drain(&mut terminal, &rx);
        if terminal.drain_events().iter().any(|event| {
            matches!(
                event,
                TerminalEvent::WorkingDirectory(cwd) if cwd == "/tmp/llnzy_osc7_roundtrip"
            )
        }) {
            cwd_seen = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    assert!(
        cwd_seen,
        "OSC 7 working directory event should survive PTY roundtrip"
    );
}

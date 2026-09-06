//! Integration tests for LLNZY's own `Pty` wrapper.
//!
//! `pty_roundtrip.rs` deliberately drives `portable_pty` directly so it can
//! control the child lifetime and drain loop explicitly. The consequence is
//! that it exercises a parallel reimplementation of the PTY plumbing and never
//! touches `Pty`'s write queue, reader thread, wakeup notifier, or `Drop` —
//! the exact seams where input was being dropped. These tests cover the
//! production wrapper instead.

use llnzy::pty::{Pty, PtyReadResult};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(10);

/// Drain the PTY until `predicate` is satisfied by the accumulated output, or
/// the deadline passes. Returns everything read.
fn drain_until(pty: &mut Pty, predicate: impl Fn(&str) -> bool) -> String {
    let mut collected = Vec::new();
    let deadline = Instant::now() + TIMEOUT;
    loop {
        match pty.try_read() {
            PtyReadResult::Data(bytes) => collected.extend_from_slice(&bytes),
            PtyReadResult::Empty => std::thread::sleep(Duration::from_millis(5)),
            PtyReadResult::Disconnected(_) => break,
        }
        let text = String::from_utf8_lossy(&collected);
        if predicate(&text) {
            break;
        }
        if Instant::now() >= deadline {
            panic!("timed out; output so far: {text:?}");
        }
    }
    String::from_utf8_lossy(&collected).into_owned()
}

/// Byte-for-byte integrity of a burst far past the old 64-chunk write queue.
///
/// The previous implementation used `try_send` into a 64-slot `sync_channel`
/// and discarded any chunk that did not fit, so a burst like this lost input
/// with nothing but a log line. Writing one byte at a time maximises chunk
/// count for a given payload, which is precisely the shape that overflowed.
///
/// Receipt is verified through `wc -c` rather than the echoed command: the
/// tty echoes input wrapped to the window width (inserting ` \r`), so the
/// echo is not a contiguous copy of what was sent. The line is also kept well
/// under the canonical-mode line limit (`MAX_CANON`, 1024 bytes on macOS),
/// beyond which the line discipline itself beeps and discards — a kernel
/// bound that has nothing to do with our queue.
#[test]
fn burst_of_single_byte_writes_reaches_the_shell_intact() {
    let mut pty = Pty::spawn_in("/bin/sh", 80, 24, None).unwrap();

    const PAYLOAD_LEN: usize = 200;
    let payload = "A".repeat(PAYLOAD_LEN);
    let line = format!("echo {payload} | wc -c\n");

    // One write call per byte: ~220 chunks through a queue that held 64.
    assert!(line.len() > 64, "burst must exceed the old queue capacity");
    for byte in line.as_bytes() {
        pty.write(std::slice::from_ref(byte))
            .expect("no keystroke may be dropped");
    }

    // wc counts the payload plus the newline echo appends.
    let expected = (PAYLOAD_LEN + 1).to_string();
    let output = drain_until(&mut pty, |text| text.contains(&expected));
    assert!(
        output.contains(&expected),
        "expected wc -c to report {expected} bytes of received input"
    );
}

/// The precise condition that used to drop input: a child that has stopped
/// reading stdin.
///
/// The other burst tests exercise the contract but do not guarantee a
/// reproduction, because the writer thread usually drains faster than the
/// caller can enqueue. Here the shell sleeps without reading, so the kernel
/// input buffer fills, the writer thread parks inside `write_all`, and the
/// queue genuinely backs up. Under the old 64-slot `try_send` this silently
/// discarded everything past the cap; now every write is accepted and the
/// backlog is visible and bounded.
#[test]
fn writes_to_a_child_that_stopped_reading_are_queued_not_dropped() {
    let mut pty = Pty::spawn_in("/bin/sh", 80, 24, None).unwrap();

    // Occupy the shell so it stops consuming stdin.
    pty.write(b"sleep 3\n").unwrap();
    std::thread::sleep(Duration::from_millis(300));

    const CHUNKS: usize = 300;
    const CHUNK_LEN: usize = 100;
    let chunk = vec![b'C'; CHUNK_LEN];

    let mut peak_pending = 0;
    for _ in 0..CHUNKS {
        pty.write(&chunk)
            .expect("queued input must never be refused below the byte ceiling");
        peak_pending = peak_pending.max(pty.pending_write_bytes());
    }

    // The old queue could hold at most 64 chunks (6400 bytes here) before
    // discarding. Exceeding that proves the backlog was real and retained
    // rather than dropped.
    let old_capacity_bytes = 64 * CHUNK_LEN;
    assert!(
        peak_pending > old_capacity_bytes,
        "expected a genuine backlog past the old {old_capacity_bytes}-byte cap, \
         peaked at {peak_pending}; the child may not have blocked"
    );
    assert!(
        peak_pending <= CHUNKS * CHUNK_LEN,
        "accounting overshoot: {peak_pending}"
    );
}

/// A single large write (the paste path) must also survive. The old queue
/// bounded chunk *count*, not bytes, so one huge chunk passed while 65 tiny
/// ones failed — this pins the other side of that asymmetry.
///
/// The payload is split into many short lines because the tty line discipline
/// caps a single line at `MAX_CANON`; a real paste of source code has the
/// same shape.
#[test]
fn large_single_write_reaches_the_shell_intact() {
    let mut pty = Pty::spawn_in("/bin/sh", 80, 24, None).unwrap();

    const LINES: usize = 1000;
    const LINE_LEN: usize = 40;
    let body: String = std::iter::repeat_n(format!("{}\n", "B".repeat(LINE_LEN)), LINES).collect();
    let expected_bytes = LINES * (LINE_LEN + 1);

    // One write call for the whole payload: the paste path.
    pty.write(format!("cat <<'PASTE_EOF' | wc -c\n{body}PASTE_EOF\n").as_bytes())
        .unwrap();

    let expected = expected_bytes.to_string();
    let output = drain_until(&mut pty, |text| text.contains(&expected));
    assert!(
        output.contains(&expected),
        "expected wc -c to report {expected} bytes"
    );
}

/// Resizing while the child floods must not lose queued output or wedge the
/// reader thread, which now blocks on a bounded channel.
#[test]
fn resize_during_output_flood_keeps_the_stream_intact() {
    let mut pty = Pty::spawn_in("/bin/sh", 80, 24, None).unwrap();
    pty.write(b"i=0; while [ $i -lt 400 ]; do echo line-$i; i=$((i+1)); done\n")
        .unwrap();

    // The tty echoes the command back, so the completion marker must be
    // something only the *output* contains — "line-399" is produced by the
    // loop but never typed, unlike a literal sentinel in the command itself.
    let mut collected = Vec::new();
    let mut resizes = 0;
    let deadline = Instant::now() + TIMEOUT;
    loop {
        match pty.try_read() {
            PtyReadResult::Data(bytes) => {
                collected.extend_from_slice(&bytes);
                if resizes < 4 {
                    pty.resize(100 + resizes * 5, 30);
                    resizes += 1;
                }
            }
            PtyReadResult::Empty => std::thread::sleep(Duration::from_millis(5)),
            PtyReadResult::Disconnected(_) => break,
        }
        if String::from_utf8_lossy(&collected).contains("line-399") {
            break;
        }
        assert!(Instant::now() < deadline, "timed out during flood");
    }

    let text = String::from_utf8_lossy(&collected);
    assert!(
        text.contains("line-399"),
        "lost tail of flood during resize"
    );
    assert!(text.contains("line-0"), "lost head of flood during resize");
    assert_eq!(resizes, 4, "resizes did not run during the flood");
}

/// Output produced immediately before exit must be delivered. `try_read` used
/// to report `Disconnected` as soon as `try_wait` succeeded, which stranded
/// anything the reader thread had not yet pushed.
#[test]
fn output_queued_at_child_exit_is_still_delivered() {
    let mut pty = Pty::spawn_in("/bin/sh", 80, 24, None).unwrap();
    pty.write(b"printf 'TAIL_SENTINEL\\n'; exit 3\n").unwrap();

    let mut collected = Vec::new();
    let deadline = Instant::now() + TIMEOUT;
    let exit_code = loop {
        match pty.try_read() {
            PtyReadResult::Data(bytes) => collected.extend_from_slice(&bytes),
            PtyReadResult::Empty => std::thread::sleep(Duration::from_millis(5)),
            PtyReadResult::Disconnected(code) => break code,
        }
        assert!(Instant::now() < deadline, "timed out waiting for exit");
    };

    let text = String::from_utf8_lossy(&collected);
    assert!(
        text.contains("TAIL_SENTINEL"),
        "output written just before exit was lost; got: {text:?}"
    );
    assert_eq!(exit_code, Some(3), "exit code must survive the drain");
}

/// Dropping the wrapper must take the child with it rather than leaking the
/// process or its reader/writer threads.
#[test]
fn drop_kills_the_child_process() {
    let pid = {
        let mut pty = Pty::spawn_in("/bin/sh", 80, 24, None).unwrap();
        pty.write(b"sleep 60\n").unwrap();
        let pid = pty.process_id().expect("child pid");
        // Let the shell actually start the sleep before we drop.
        std::thread::sleep(Duration::from_millis(200));
        pid
    };

    // Give the OS a moment to reap after Drop killed the child.
    let deadline = Instant::now() + TIMEOUT;
    loop {
        // Signal 0 probes existence without delivering anything.
        let alive = unsafe { libc::kill(pid as libc::pid_t, 0) } == 0;
        if !alive {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "child {pid} still alive after Pty::drop"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

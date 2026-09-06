use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Notify;

use crate::platform::shell::ShellProfile;
use crate::platform::terminal_host::TerminalLaunchSpec;

/// Result of a non-blocking PTY read.
pub enum PtyReadResult {
    /// Data was available.
    Data(Vec<u8>),
    /// No data available right now, but the channel is still open.
    Empty,
    /// The PTY reader thread has exited (child process is gone).
    Disconnected(Option<i32>),
}

/// Ceiling on bytes queued for the PTY but not yet written to the child.
///
/// Terminal input is never dropped, so the write queue is unbounded in chunk
/// count; this byte ceiling is the only thing that can refuse a write, and it
/// exists solely to bound memory if a child stops draining its input
/// entirely. Ordinary typing and pastes are orders of magnitude below it — a
/// caller has to push 8 MiB of unconsumed input to see `WouldBlock`.
const PTY_WRITE_PENDING_BYTES_MAX: usize = 8 * 1024 * 1024;

/// Number of reader chunks buffered between the PTY and the UI drain.
///
/// The reader thread blocks once this many chunks are outstanding, which lets
/// the kernel apply flow control to the child instead of growing our memory
/// without bound. Reads use a 64 KiB buffer, so this bounds queued output at
/// roughly 8 MiB.
const PTY_OUTPUT_QUEUE_CAPACITY: usize = 128;

/// How long `try_read` will keep reporting `Empty` after the child has exited
/// while waiting for the reader thread to finish draining in-flight output.
/// Bounds the pathological case where the reader never observes EOF.
const PTY_EXIT_DRAIN_GRACE: Duration = Duration::from_millis(250);

pub struct Pty {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    process_id: Option<u32>,
    write_tx: mpsc::Sender<Vec<u8>>,
    /// Bytes handed to the writer thread but not yet written to the child.
    /// Incremented by `write`, decremented by the writer thread once the
    /// bytes reach the PTY. Backs the `PTY_WRITE_PENDING_BYTES_MAX` ceiling.
    pending_write_bytes: Arc<AtomicUsize>,
    output_rx: mpsc::Receiver<Vec<u8>>,
    /// Set by the reader thread just before it exits, on every exit path.
    /// Lets `try_read` distinguish "momentarily empty" from "fully drained",
    /// so the child's final output is never reported as gone early.
    reader_done: Arc<AtomicBool>,
    /// When the child was first observed exited with the reader still
    /// running. Bounds how long we wait for that drain.
    exit_observed_at: Option<Instant>,
    /// Signalled by the reader thread whenever a read completes (data
    /// arrived, EOF, or error). The GPUI render task awaits this notifier
    /// so the UI thread sleeps when the shell is idle instead of polling
    /// `try_read` at 60 Hz.
    wakeup: Arc<Notify>,
    /// Set to true once the reader channel disconnects (child exited).
    dead: bool,
}

impl Pty {
    pub fn spawn(shell: &str, cols: u16, rows: u16) -> io::Result<Self> {
        Self::spawn_in(shell, cols, rows, None)
    }

    pub fn spawn_in(shell: &str, cols: u16, rows: u16, cwd: Option<&str>) -> io::Result<Self> {
        Self::spawn_with_spec(launch_spec(shell, cols, rows, cwd))
    }

    pub fn spawn_with_spec(spec: TerminalLaunchSpec) -> io::Result<Self> {
        let pty_system = native_pty_system();
        let size = PtySize {
            rows: spec.rows,
            cols: spec.cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system.openpty(size).map_err(io::Error::other)?;

        let mut cmd = CommandBuilder::new(spec.program.to_string_lossy().as_ref());
        for arg in &spec.args {
            cmd.arg(arg);
        }
        for (key, value) in &spec.env {
            cmd.env(key, value);
        }
        if let Some(dir) = &spec.cwd {
            cmd.cwd(dir);
        }

        let child = pair.slave.spawn_command(cmd).map_err(io::Error::other)?;
        let process_id = child.process_id();

        // Close slave in parent process
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().map_err(io::Error::other)?;
        let mut writer = pair.master.take_writer().map_err(io::Error::other)?;

        // Output is bounded: `send` blocks once PTY_OUTPUT_QUEUE_CAPACITY
        // chunks are outstanding, so a flooding child is throttled by the
        // kernel rather than by our heap. Input is unbounded in chunk count
        // and bounded by pending bytes instead, so no keystroke is ever
        // dropped and `write` never blocks the render thread.
        let (read_tx, read_rx) = mpsc::sync_channel::<Vec<u8>>(PTY_OUTPUT_QUEUE_CAPACITY);
        let (write_tx, write_rx) = mpsc::channel::<Vec<u8>>();

        let wakeup = Arc::new(Notify::new());
        let wakeup_reader = wakeup.clone();
        let reader_done = Arc::new(AtomicBool::new(false));
        let reader_done_thread = reader_done.clone();
        let pending_write_bytes = Arc::new(AtomicUsize::new(0));
        let pending_write_bytes_thread = pending_write_bytes.clone();

        // Spawn a dedicated thread for reading PTY output. After every read
        // (data arrived, EOF, or error) the wakeup notifier is pulsed so the
        // GPUI render task can drain the channel without polling. The notify
        // is always called in the loop, even on the error/EOF paths, so the
        // task wakes up to observe the disconnect.
        //
        // `send` is blocking: when the UI falls behind, this thread parks and
        // stops draining the PTY, which propagates backpressure to the child.
        // It unparks when the drain catches up, or errors out when the
        // receiver is dropped, which is how this thread exits on teardown.
        std::thread::spawn(move || {
            let mut buf = [0u8; 65536];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if read_tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                        wakeup_reader.notify_one();
                    }
                    Err(_) => break,
                }
            }
            // Publish completion before the final pulse so a waking reader
            // observes a drained channel rather than racing the flag.
            reader_done_thread.store(true, Ordering::Release);
            wakeup_reader.notify_one();
        });

        // Spawn a dedicated thread for writing PTY input.
        // This prevents large pastes from blocking the main/render thread
        // (macOS PTY buffers are ~4KB; write_all blocks when the buffer is full).
        // Each chunk's bytes are released from the pending count only once
        // they have actually reached the PTY, so the ceiling in `write`
        // reflects genuinely outstanding input.
        std::thread::spawn(move || {
            while let Ok(data) = write_rx.recv() {
                let len = data.len();
                let result = writer.write_all(&data);
                let _ = writer.flush();
                pending_write_bytes_thread.fetch_sub(len, Ordering::AcqRel);
                if result.is_err() {
                    break;
                }
            }
        });

        Ok(Pty {
            master: pair.master,
            child,
            process_id,
            write_tx,
            pending_write_bytes,
            output_rx: read_rx,
            reader_done,
            exit_observed_at: None,
            wakeup,
            dead: false,
        })
    }

    /// Returns a clone of the wakeup notifier. The GPUI render task awaits
    /// `Notify::notified` on this handle; the reader thread pulses it after
    /// every PTY read, so the task only wakes when there is genuine work.
    pub fn wakeup_handle(&self) -> Arc<Notify> {
        self.wakeup.clone()
    }

    /// Non-blocking read of PTY output.
    /// Distinguishes between "no data yet" and "child process exited."
    pub fn try_read(&mut self) -> PtyReadResult {
        if self.dead {
            return PtyReadResult::Empty;
        }
        match self.output_rx.try_recv() {
            Ok(data) => PtyReadResult::Data(data),
            Err(mpsc::TryRecvError::Empty) => {
                // An empty channel does not mean the child is finished
                // talking: the reader thread may still be mid-read. Reporting
                // the exit here would strand whatever it pushes next, because
                // callers stop draining once they see `Disconnected`. Wait for
                // the reader to signal completion, and only override that on a
                // bounded grace period so a reader that never sees EOF cannot
                // leave the session looking alive forever.
                let Ok(Some(status)) = self.child.try_wait() else {
                    self.exit_observed_at = None;
                    return PtyReadResult::Empty;
                };
                let exit_code = Some(status.exit_code() as i32);

                if self.reader_done.load(Ordering::Acquire) {
                    // The reader may have pushed a final chunk and finished
                    // between the `try_recv` above and this load. Once
                    // `reader_done` is set no further sends can occur, so a
                    // single re-check is enough to confirm the channel is
                    // genuinely drained before we stop the caller draining.
                    if let Ok(data) = self.output_rx.try_recv() {
                        return PtyReadResult::Data(data);
                    }
                    self.dead = true;
                    return PtyReadResult::Disconnected(exit_code);
                }

                let first_seen = *self.exit_observed_at.get_or_insert_with(Instant::now);
                if first_seen.elapsed() >= PTY_EXIT_DRAIN_GRACE {
                    log::debug!(
                        "pty reader still running {PTY_EXIT_DRAIN_GRACE:?} after child exit; \
                         reporting disconnect"
                    );
                    self.dead = true;
                    PtyReadResult::Disconnected(exit_code)
                } else {
                    PtyReadResult::Empty
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.dead = true;
                let exit_code = self
                    .child
                    .try_wait()
                    .ok()
                    .flatten()
                    .map(|status| status.exit_code() as i32);
                PtyReadResult::Disconnected(exit_code)
            }
        }
    }

    /// Returns true if the reader channel has disconnected (child exited).
    pub fn is_dead(&self) -> bool {
        self.dead
    }

    pub fn process_id(&self) -> Option<u32> {
        self.process_id
    }

    pub fn kill(&mut self) -> io::Result<()> {
        self.child.kill()
    }

    /// Write input bytes to the PTY.
    ///
    /// Never blocks and never silently discards input: the bytes are queued
    /// for a background write thread that owns the potentially-blocking PTY
    /// write, keeping the render thread free. Every caller runs on the GPUI
    /// foreground thread, so blocking here would freeze the UI — the queue is
    /// therefore unbounded in chunk count and bounded by
    /// `PTY_WRITE_PENDING_BYTES_MAX` outstanding bytes instead.
    ///
    /// Returns an error rather than dropping data:
    /// - `BrokenPipe` if the child is gone.
    /// - `WouldBlock` if the child has stopped draining its input and the
    ///   pending ceiling is reached. Nothing was queued; the caller still owns
    ///   the data and can surface the condition to the user.
    pub fn write(&mut self, data: &[u8]) -> io::Result<()> {
        if self.dead {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "pty write on a closed session",
            ));
        }
        if data.is_empty() {
            return Ok(());
        }

        // Reserve first so concurrent progress on the writer thread can only
        // make the ceiling more permissive, never less.
        let pending = self.pending_write_bytes.load(Ordering::Acquire);
        if pending.saturating_add(data.len()) > PTY_WRITE_PENDING_BYTES_MAX {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                format!(
                    "pty write queue holds {pending} unwritten bytes \
                     (ceiling {PTY_WRITE_PENDING_BYTES_MAX}); child is not reading input"
                ),
            ));
        }
        self.pending_write_bytes
            .fetch_add(data.len(), Ordering::AcqRel);

        match self.write_tx.send(data.to_vec()) {
            Ok(()) => Ok(()),
            Err(mpsc::SendError(unsent)) => {
                self.pending_write_bytes
                    .fetch_sub(unsent.len(), Ordering::AcqRel);
                self.dead = true;
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "pty writer thread has exited",
                ))
            }
        }
    }

    /// Bytes queued for the child but not yet written. Test and diagnostic
    /// hook for the write-path ceiling.
    pub fn pending_write_bytes(&self) -> usize {
        self.pending_write_bytes.load(Ordering::Acquire)
    }

    /// Resize the PTY.
    pub fn resize(&self, cols: u16, rows: u16) {
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
    }
}

impl Drop for Pty {
    /// Kill the child process so neither the OS process nor the reader/writer
    /// threads outlive this `Pty`. The reader thread exits once the master
    /// read returns EOF after the child closes; the writer thread exits once
    /// `write_tx` is dropped along with the rest of `Self`.
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn launch_spec(shell: &str, cols: u16, rows: u16, cwd: Option<&str>) -> TerminalLaunchSpec {
    let profile = ShellProfile::interactive_default(shell, cwd);
    TerminalLaunchSpec::interactive_shell(&profile, cols, rows)
}

#[cfg(test)]
mod tests {
    use super::{Pty, PtyReadResult};
    use std::io;
    use std::time::{Duration, Instant};

    #[test]
    fn reports_process_identity_and_exit_once() {
        let mut pty = Pty::spawn_in("/bin/sh", 80, 24, None).unwrap();

        assert!(pty.process_id().is_some());
        pty.write(b"exit 7\n").unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        let exit_code = loop {
            match pty.try_read() {
                PtyReadResult::Data(_) | PtyReadResult::Empty => {}
                PtyReadResult::Disconnected(code) => break code,
            }
            assert!(Instant::now() < deadline, "timed out waiting for PTY exit");
            std::thread::sleep(Duration::from_millis(10));
        };

        assert_eq!(exit_code, Some(7));
        assert!(matches!(pty.try_read(), PtyReadResult::Empty));
    }

    /// The regression that motivated the backpressure rewrite: the old
    /// implementation used a 64-slot queue and `try_send`, so the 65th chunk
    /// written before the shell drained was discarded with only a log line.
    /// Every chunk must now be queued.
    #[test]
    fn burst_writes_past_old_queue_capacity_are_never_dropped() {
        let mut pty = Pty::spawn_in("/bin/sh", 80, 24, None).unwrap();

        // Comfortably past the old PTY_WRITE_QUEUE_CAPACITY of 64.
        for _ in 0..512 {
            pty.write(b"x").expect("write must not drop input");
        }

        // Drain to completion so the count returns to zero, proving the bytes
        // reached the child rather than being accounted and discarded.
        let deadline = Instant::now() + Duration::from_secs(5);
        while pty.pending_write_bytes() > 0 {
            assert!(
                Instant::now() < deadline,
                "timed out draining {} pending bytes",
                pty.pending_write_bytes()
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn write_to_dead_pty_reports_broken_pipe() {
        let mut pty = Pty::spawn_in("/bin/sh", 80, 24, None).unwrap();
        pty.kill().unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        while let PtyReadResult::Data(_) | PtyReadResult::Empty = pty.try_read() {
            assert!(Instant::now() < deadline, "timed out waiting for exit");
            std::thread::sleep(Duration::from_millis(10));
        }

        let err = pty
            .write(b"ignored")
            .expect_err("write after exit must fail");
        assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
    }

    /// A child that exits immediately after printing must not lose its final
    /// output: `try_read` has to keep reporting `Empty` until the reader
    /// thread signals it is finished, not the instant `try_wait` succeeds.
    #[test]
    fn final_output_survives_immediate_exit() {
        let mut pty = Pty::spawn_in("/bin/sh", 80, 24, None).unwrap();
        pty.write(b"printf 'SENTINEL_OUTPUT'; exit 0\n").unwrap();

        let mut collected = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match pty.try_read() {
                PtyReadResult::Data(bytes) => collected.extend_from_slice(&bytes),
                PtyReadResult::Empty => {}
                PtyReadResult::Disconnected(_) => break,
            }
            assert!(Instant::now() < deadline, "timed out waiting for PTY exit");
            std::thread::sleep(Duration::from_millis(5));
        }

        let text = String::from_utf8_lossy(&collected);
        assert!(
            text.contains("SENTINEL_OUTPUT"),
            "final output lost at exit; got: {text:?}"
        );
    }

    #[test]
    fn kill_reports_disconnect() {
        let mut pty = Pty::spawn_in("/bin/sh", 80, 24, None).unwrap();
        pty.kill().unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        while let PtyReadResult::Data(_) | PtyReadResult::Empty = pty.try_read() {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for killed PTY exit"
            );
            std::thread::sleep(Duration::from_millis(10));
        }

        assert!(matches!(pty.try_read(), PtyReadResult::Empty));
    }
}

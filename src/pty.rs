use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{self, Read, Write};
use std::sync::mpsc;
use std::sync::Arc;
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

/// Maximum number of pending write chunks before new writes are dropped.
/// 64 chunks × typical write granularity comfortably covers normal pastes
/// while bounding queued memory when the shell is unable to drain.
const PTY_WRITE_QUEUE_CAPACITY: usize = 64;

pub struct Pty {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    process_id: Option<u32>,
    write_tx: mpsc::SyncSender<Vec<u8>>,
    output_rx: mpsc::Receiver<Vec<u8>>,
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

        let (read_tx, read_rx) = mpsc::channel();
        let (write_tx, write_rx) = mpsc::sync_channel::<Vec<u8>>(PTY_WRITE_QUEUE_CAPACITY);

        let wakeup = Arc::new(Notify::new());
        let wakeup_reader = wakeup.clone();

        // Spawn a dedicated thread for reading PTY output. After every read
        // (data arrived, EOF, or error) the wakeup notifier is pulsed so the
        // GPUI render task can drain the channel without polling. The notify
        // is always called in the loop, even on the error/EOF paths, so the
        // task wakes up to observe the disconnect.
        std::thread::spawn(move || {
            let mut buf = [0u8; 65536];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        wakeup_reader.notify_one();
                        break;
                    }
                    Ok(n) => {
                        if read_tx.send(buf[..n].to_vec()).is_err() {
                            wakeup_reader.notify_one();
                            break;
                        }
                        wakeup_reader.notify_one();
                    }
                    Err(_) => {
                        wakeup_reader.notify_one();
                        break;
                    }
                }
            }
        });

        // Spawn a dedicated thread for writing PTY input.
        // This prevents large pastes from blocking the main/render thread
        // (macOS PTY buffers are ~4KB; write_all blocks when the buffer is full).
        std::thread::spawn(move || {
            while let Ok(data) = write_rx.recv() {
                if writer.write_all(&data).is_err() {
                    break;
                }
                let _ = writer.flush();
            }
        });

        Ok(Pty {
            master: pair.master,
            child,
            process_id,
            write_tx,
            output_rx: read_rx,
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
                if let Ok(Some(status)) = self.child.try_wait() {
                    self.dead = true;
                    PtyReadResult::Disconnected(Some(status.exit_code() as i32))
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

    /// Write input bytes to the PTY (non-blocking).
    /// Data is sent to a background write thread that handles the
    /// potentially-blocking PTY write, keeping the main thread free.
    /// The write queue is bounded; on overflow the chunk is dropped and a
    /// warning is logged rather than blocking the caller or growing memory.
    pub fn write(&mut self, data: &[u8]) {
        if self.dead {
            return;
        }
        match self.write_tx.try_send(data.to_vec()) {
            Ok(()) => {}
            Err(mpsc::TrySendError::Full(dropped)) => {
                log::warn!(
                    "PTY write queue full ({} chunks); dropping {} bytes",
                    PTY_WRITE_QUEUE_CAPACITY,
                    dropped.len()
                );
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                self.dead = true;
            }
        }
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
    use std::time::{Duration, Instant};

    #[test]
    fn reports_process_identity_and_exit_once() {
        let mut pty = Pty::spawn_in("/bin/sh", 80, 24, None).unwrap();

        assert!(pty.process_id().is_some());
        pty.write(b"exit 7\n");

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

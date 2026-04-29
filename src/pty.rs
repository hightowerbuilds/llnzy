use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{self, Read, Write};
use std::sync::mpsc;

/// Result of a non-blocking PTY read.
pub enum PtyReadResult {
    /// Data was available.
    Data(Vec<u8>),
    /// No data available right now, but the channel is still open.
    Empty,
    /// The PTY reader thread has exited (child process is gone).
    Disconnected(Option<i32>),
}

pub struct Pty {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    process_id: Option<u32>,
    write_tx: mpsc::Sender<Vec<u8>>,
    output_rx: mpsc::Receiver<Vec<u8>>,
    /// Set to true once the reader channel disconnects (child exited).
    dead: bool,
}

impl Pty {
    pub fn spawn(
        shell: &str,
        cols: u16,
        rows: u16,
        proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>,
    ) -> io::Result<Self> {
        Self::spawn_in(shell, cols, rows, proxy, None)
    }

    pub fn spawn_in(
        shell: &str,
        cols: u16,
        rows: u16,
        proxy: winit::event_loop::EventLoopProxy<crate::UserEvent>,
        cwd: Option<&str>,
    ) -> io::Result<Self> {
        Self::spawn_in_with_proxy(shell, cols, rows, Some(proxy), cwd)
    }

    #[cfg(test)]
    fn spawn_in_without_proxy(
        shell: &str,
        cols: u16,
        rows: u16,
        cwd: Option<&str>,
    ) -> io::Result<Self> {
        Self::spawn_in_with_proxy(shell, cols, rows, None, cwd)
    }

    fn spawn_in_with_proxy(
        shell: &str,
        cols: u16,
        rows: u16,
        proxy: Option<winit::event_loop::EventLoopProxy<crate::UserEvent>>,
        cwd: Option<&str>,
    ) -> io::Result<Self> {
        let pty_system = native_pty_system();
        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system.openpty(size).map_err(io::Error::other)?;

        let mut cmd = CommandBuilder::new(shell);
        cmd.arg("-l");
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }

        let child = pair.slave.spawn_command(cmd).map_err(io::Error::other)?;
        let process_id = child.process_id();

        // Close slave in parent process
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().map_err(io::Error::other)?;
        let mut writer = pair.master.take_writer().map_err(io::Error::other)?;

        let (read_tx, read_rx) = mpsc::channel();
        let (write_tx, write_rx) = mpsc::channel::<Vec<u8>>();

        // Spawn a dedicated thread for reading PTY output
        std::thread::spawn(move || {
            let mut buf = [0u8; 65536];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if read_tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                        // Wake the event loop so it processes the output
                        if let Some(proxy) = &proxy {
                            let _ = proxy.send_event(crate::UserEvent::PtyOutput);
                        }
                    }
                    Err(_) => break,
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
            dead: false,
        })
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
    pub fn write(&mut self, data: &[u8]) {
        if self.dead {
            return;
        }
        let _ = self.write_tx.send(data.to_vec());
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

#[cfg(test)]
mod tests {
    use super::{Pty, PtyReadResult};
    use std::time::{Duration, Instant};

    #[test]
    fn reports_process_identity_and_exit_once() {
        let mut pty = Pty::spawn_in_without_proxy("/bin/sh", 80, 24, None).unwrap();

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
        let mut pty = Pty::spawn_in_without_proxy("/bin/sh", 80, 24, None).unwrap();
        pty.kill().unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match pty.try_read() {
                PtyReadResult::Data(_) | PtyReadResult::Empty => {}
                PtyReadResult::Disconnected(_) => break,
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for killed PTY exit"
            );
            std::thread::sleep(Duration::from_millis(10));
        }

        assert!(matches!(pty.try_read(), PtyReadResult::Empty));
    }
}

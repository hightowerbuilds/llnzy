use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{self, Read, Write};
use std::sync::mpsc;

/// Result of a non-blocking PTY read.
pub enum PtyReadResult {
    /// Data was available.
    Data(Vec<u8>),
    /// No data available right now, but the channel is still open.
    Empty,
    /// The PTY reader thread has exited (child process is gone).
    Disconnected,
}

pub struct Pty {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
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

        let _child = pair.slave.spawn_command(cmd).map_err(io::Error::other)?;

        // Close slave in parent process
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().map_err(io::Error::other)?;
        let writer = pair.master.take_writer().map_err(io::Error::other)?;

        let (tx, rx) = mpsc::channel();

        // Spawn a dedicated thread for reading PTY output
        std::thread::spawn(move || {
            let mut buf = [0u8; 65536];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                        // Wake the event loop so it processes the output
                        let _ = proxy.send_event(crate::UserEvent::PtyOutput);
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Pty {
            master: pair.master,
            writer,
            output_rx: rx,
            dead: false,
        })
    }

    /// Non-blocking read of PTY output.
    /// Distinguishes between "no data yet" and "child process exited."
    pub fn try_read(&mut self) -> PtyReadResult {
        if self.dead {
            return PtyReadResult::Disconnected;
        }
        match self.output_rx.try_recv() {
            Ok(data) => PtyReadResult::Data(data),
            Err(mpsc::TryRecvError::Empty) => PtyReadResult::Empty,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.dead = true;
                PtyReadResult::Disconnected
            }
        }
    }

    /// Returns true if the reader channel has disconnected (child exited).
    pub fn is_dead(&self) -> bool {
        self.dead
    }

    /// Write input bytes to the PTY. Skips writes if the child is dead.
    pub fn write(&mut self, data: &[u8]) {
        if self.dead {
            return;
        }
        let _ = self.writer.write_all(data);
        let _ = self.writer.flush();
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

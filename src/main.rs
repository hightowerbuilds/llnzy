mod config;
mod input;
mod pty;
mod renderer;
mod selection;
mod terminal;

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

use config::Config;
use pty::Pty;
use renderer::Renderer;
use selection::Selection;
use terminal::Terminal;

#[derive(Debug)]
pub enum UserEvent {
    PtyOutput,
}

struct App {
    config: Config,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    terminal: Option<Terminal>,
    pty: Option<Pty>,
    proxy: winit::event_loop::EventLoopProxy<UserEvent>,
    modifiers: ModifiersState,
    selection: Selection,
    clipboard: Option<arboard::Clipboard>,
    cursor_pos: winit::dpi::PhysicalPosition<f64>,
    mouse_pressed: bool,
}

impl App {
    fn new(proxy: winit::event_loop::EventLoopProxy<UserEvent>) -> Self {
        let clipboard = arboard::Clipboard::new().ok();
        Self {
            config: Config::load(),
            window: None,
            renderer: None,
            terminal: None,
            pty: None,
            proxy,
            modifiers: ModifiersState::empty(),
            selection: Selection::new(),
            clipboard,
            cursor_pos: winit::dpi::PhysicalPosition::new(0.0, 0.0),
            mouse_pressed: false,
        }
    }

    fn process_pty_output(&mut self) {
        let mut changed = false;
        if let (Some(pty), Some(terminal)) = (&self.pty, &mut self.terminal) {
            while let Some(bytes) = pty.try_read() {
                terminal.process(&bytes);
                changed = true;
            }
        }
        if changed {
            self.request_redraw();
        }
    }

    fn pixel_to_grid(&self, pos: winit::dpi::PhysicalPosition<f64>) -> (usize, usize) {
        let (cw, ch) = self
            .renderer
            .as_ref()
            .map(|r| r.cell_dimensions())
            .unwrap_or((1.0, 1.0));
        let (cols, rows) = self
            .terminal
            .as_ref()
            .map(|t| t.size())
            .unwrap_or((80, 24));

        let col = ((pos.x as f32 / cw) as usize).min(cols.saturating_sub(1));
        let row = ((pos.y as f32 / ch) as usize).min(rows.saturating_sub(1));
        (row, col)
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Send bytes to the PTY, auto-scrolling to bottom first.
    fn write_to_pty(&mut self, bytes: &[u8]) {
        if let Some(terminal) = &mut self.terminal {
            terminal.scroll_to_bottom();
        }
        if let Some(pty) = &mut self.pty {
            pty.write(bytes);
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("llnzy")
            .with_inner_size(winit::dpi::LogicalSize::new(900.0, 600.0));

        let window = Arc::new(event_loop.create_window(attrs).unwrap());

        let renderer = pollster::block_on(Renderer::new(window.clone(), self.config.clone()));

        let (cell_w, cell_h) = renderer.cell_dimensions();
        let size = window.inner_size();
        let cols = (size.width as f32 / cell_w) as u16;
        let rows = (size.height as f32 / cell_h) as u16;

        let terminal = Terminal::new(cols.max(1), rows.max(1));

        let pty = match Pty::spawn(&self.config.shell, cols.max(1), rows.max(1), self.proxy.clone())
        {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to spawn PTY: {}", e);
                event_loop.exit();
                return;
            }
        };

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.terminal = Some(terminal);
        self.pty = Some(pty);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);

                    let (cw, ch) = renderer.cell_dimensions();
                    let cols = (size.width as f32 / cw) as u16;
                    let rows = (size.height as f32 / ch) as u16;

                    if cols > 0 && rows > 0 {
                        if let Some(terminal) = &mut self.terminal {
                            terminal.resize(cols, rows);
                        }
                        if let Some(pty) = &self.pty {
                            pty.resize(cols, rows);
                        }
                    }
                }
                self.selection.clear();
            }

            WindowEvent::RedrawRequested => {
                self.process_pty_output();
                if let (Some(renderer), Some(terminal)) = (&mut self.renderer, &self.terminal) {
                    let (cw, ch) = renderer.cell_dimensions();
                    let (cols, _) = terminal.size();
                    let sel_rects = self.selection.rects(cw, ch, cols);
                    renderer.render(terminal, &sel_rects);
                }
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }

            // --- Mouse wheel: scrollback ---

            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => (y * 3.0) as i32,
                    MouseScrollDelta::PixelDelta(pos) => {
                        let (_, ch) = self
                            .renderer
                            .as_ref()
                            .map(|r| r.cell_dimensions())
                            .unwrap_or((1.0, 1.0));
                        (pos.y / ch as f64) as i32
                    }
                };
                if lines != 0 {
                    if let Some(terminal) = &mut self.terminal {
                        terminal.scroll(lines);
                    }
                    self.request_redraw();
                }
            }

            // --- Mouse: selection ---

            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => match state {
                ElementState::Pressed => {
                    let (row, col) = self.pixel_to_grid(self.cursor_pos);
                    self.selection.start(row, col);
                    self.mouse_pressed = true;
                    self.request_redraw();
                }
                ElementState::Released => {
                    self.mouse_pressed = false;
                }
            },

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = position;
                if self.mouse_pressed {
                    let (row, col) = self.pixel_to_grid(position);
                    self.selection.update(row, col);
                    self.request_redraw();
                }
            }

            // --- Keyboard ---

            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                if key_event.state != ElementState::Pressed {
                    return;
                }

                // Cmd (Super) shortcuts
                if self.modifiers.super_key() {
                    match &key_event.logical_key {
                        Key::Character(c) if c.as_str() == "c" => {
                            if self.selection.is_active() {
                                if let Some(terminal) = &self.terminal {
                                    let text = self.selection.text(terminal);
                                    if let Some(cb) = &mut self.clipboard {
                                        let _ = cb.set_text(text);
                                    }
                                }
                                self.selection.clear();
                                self.request_redraw();
                            }
                            return;
                        }
                        Key::Character(c) if c.as_str() == "v" => {
                            if let Some(cb) = &mut self.clipboard {
                                if let Ok(text) = cb.get_text() {
                                    self.write_to_pty(text.as_bytes());
                                }
                            }
                            return;
                        }
                        Key::Character(c) if c.as_str() == "a" => {
                            if let Some(terminal) = &self.terminal {
                                let (cols, rows) = terminal.size();
                                self.selection.select_all(rows, cols);
                            }
                            self.request_redraw();
                            return;
                        }
                        _ => {}
                    }
                }

                // Shift+PageUp/Down for scrollback
                if self.modifiers.shift_key() {
                    match &key_event.logical_key {
                        Key::Named(NamedKey::PageUp) => {
                            if let Some(terminal) = &mut self.terminal {
                                terminal.scroll_page_up();
                            }
                            self.request_redraw();
                            return;
                        }
                        Key::Named(NamedKey::PageDown) => {
                            if let Some(terminal) = &mut self.terminal {
                                terminal.scroll_page_down();
                            }
                            self.request_redraw();
                            return;
                        }
                        _ => {}
                    }
                }

                // Any non-modifier keypress clears selection
                if self.selection.is_active() {
                    self.selection.clear();
                    self.request_redraw();
                }

                // Regular terminal input (scroll to bottom + send)
                if let Some(bytes) = input::encode_key(&key_event, self.modifiers) {
                    self.write_to_pty(&bytes);
                }
            }

            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::PtyOutput => {
                self.process_pty_output();
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.process_pty_output();
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy);

    event_loop.run_app(&mut app).unwrap();
}

use dpi::{LogicalPosition, LogicalSize};
use serde::Deserialize;
use winit::{event_loop::EventLoopProxy, window::Window};
use wry::{Rect, WebView, WebViewBuilder};

use crate::{stacker::input::StackerSelection, UserEvent};

#[derive(Clone, Debug, Deserialize)]
pub struct StackerWebViewMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(default)]
    pub text: String,
    #[serde(rename = "selectionStart", default)]
    pub selection_start: usize,
    #[serde(rename = "selectionEnd", default)]
    pub selection_end: usize,
}

pub struct StackerWebView {
    webview: WebView,
    visible: bool,
    last_text: String,
    last_selection: StackerSelection,
    last_rect: Option<[i32; 4]>,
    last_font_size: f32,
}

impl StackerWebView {
    pub fn new(window: &Window, proxy: EventLoopProxy<UserEvent>) -> Result<Self, String> {
        let webview = WebViewBuilder::new()
            .with_html(stacker_editor_html())
            .with_ipc_handler(move |request| {
                let _ = proxy.send_event(UserEvent::StackerWebViewMessage(request.body().clone()));
            })
            .build_as_child(window)
            .map_err(|err| format!("failed to create Stacker WebView editor: {err}"))?;

        let this = Self {
            webview,
            visible: true,
            last_text: String::new(),
            last_selection: StackerSelection::collapsed(0),
            last_rect: None,
            last_font_size: 16.0,
        };
        let _ = this.webview.set_visible(false);
        Ok(this)
    }

    pub fn set_visible(&mut self, visible: bool) -> bool {
        if self.visible == visible {
            return false;
        }
        self.visible = visible;
        let _ = self.webview.set_visible(visible);
        true
    }

    pub fn set_bounds(&mut self, rect: egui::Rect) {
        let left = rect.left().round() as i32;
        let top = rect.top().round() as i32;
        let width = rect.width().round().max(1.0) as i32;
        let height = rect.height().round().max(1.0) as i32;
        let next = [left, top, width, height];
        if self.last_rect == Some(next) {
            return;
        }
        self.last_rect = Some(next);
        let _ = self.webview.set_bounds(Rect {
            position: LogicalPosition::new(left, top).into(),
            size: LogicalSize::new(width, height).into(),
        });
    }

    pub fn set_document(&mut self, text: &str, selection: StackerSelection) {
        let selection = clamp_selection(text, selection);
        if self.last_text == text && self.last_selection == selection {
            return;
        }

        if self.last_text != text {
            self.last_text.clear();
            self.last_text.push_str(text);
        }
        self.last_selection = selection;
        let Ok(encoded) = serde_json::to_string(text) else {
            return;
        };
        let start = char_index_to_utf16_index(text, selection.start);
        let end = char_index_to_utf16_index(text, selection.end);
        let script = format!("window.__llnzySetDocument({encoded}, {start}, {end});");
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn note_webview_document(&mut self, text: &str, selection: StackerSelection) {
        self.last_text.clear();
        self.last_text.push_str(text);
        self.last_selection = clamp_selection(text, selection);
    }

    pub fn set_font_size(&mut self, font_size: f32) {
        if (self.last_font_size - font_size).abs() < 0.1 {
            return;
        }
        self.last_font_size = font_size;
        let script = format!("window.__llnzySetFontSize({font_size});");
        let _ = self.webview.evaluate_script(&script);
    }

    pub fn focus(&self) {
        let _ = self.webview.focus();
        let _ = self.webview.evaluate_script("window.__llnzyFocus();");
    }
}

pub fn utf16_index_to_char_index(text: &str, utf16_index: usize) -> usize {
    let mut units = 0;
    for (char_index, ch) in text.chars().enumerate() {
        if units >= utf16_index {
            return char_index;
        }
        units += ch.len_utf16();
        if units > utf16_index {
            return char_index + 1;
        }
    }
    text.chars().count()
}

pub fn char_index_to_utf16_index(text: &str, char_index: usize) -> usize {
    text.chars().take(char_index).map(|ch| ch.len_utf16()).sum()
}

fn clamp_selection(text: &str, selection: StackerSelection) -> StackerSelection {
    let char_count = text.chars().count();
    StackerSelection {
        start: selection.start.min(char_count),
        end: selection.end.min(char_count),
    }
}

fn stacker_editor_html() -> &'static str {
    r#"<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <style>
    :root {
      color-scheme: dark;
      background: #1c1c1c;
      font-family: "Atkinson Hyperlegible", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }
    html, body {
      width: 100%;
      height: 100%;
      margin: 0;
      overflow: hidden;
      background: #1c1c1c;
    }
    textarea {
      box-sizing: border-box;
      width: 100%;
      height: 100%;
      margin: 0;
      padding: 34px;
      border: 0;
      outline: 0;
      resize: none;
      background: #1c1c1c;
      color: #f0f8ff;
      caret-color: #6aff90;
      font: 16px/1.45 "Atkinson Hyperlegible", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      letter-spacing: 0;
      tab-size: 2;
      white-space: pre-wrap;
    }
    textarea::placeholder {
      color: #5a5c69;
    }
    ::selection {
      background: rgba(106, 255, 144, 0.28);
    }
  </style>
</head>
<body>
  <textarea id="stacker-editor" spellcheck="false" autocomplete="off" autocapitalize="sentences" placeholder="Write your prompt here..."></textarea>
  <script>
    const editor = document.getElementById('stacker-editor');
    let applyingHostUpdate = false;

    function post(type) {
      if (applyingHostUpdate || !window.ipc) return;
      window.ipc.postMessage(JSON.stringify({
        type,
        text: editor.value,
        selectionStart: editor.selectionStart || 0,
        selectionEnd: editor.selectionEnd || 0
      }));
    }

    window.__llnzySetDocument = function(text, selectionStart, selectionEnd) {
      applyingHostUpdate = true;
      if (editor.value !== text) {
        editor.value = text;
      }
      const start = Math.min(text.length, selectionStart || 0);
      const end = Math.min(text.length, selectionEnd || start);
      editor.setSelectionRange(start, end);
      applyingHostUpdate = false;
    };

    window.__llnzySetFontSize = function(size) {
      editor.style.fontSize = `${size}px`;
    };

    window.__llnzyFocus = function() {
      editor.focus({ preventScroll: true });
    };

    editor.addEventListener('input', () => post('textChanged'));
    editor.addEventListener('pointerdown', () => post('pointerDown'));
    editor.addEventListener('mousedown', () => post('pointerDown'));
    editor.addEventListener('select', () => post('selectionChanged'));
    editor.addEventListener('keyup', () => post('selectionChanged'));
    editor.addEventListener('mouseup', () => post('selectionChanged'));
    editor.addEventListener('focus', () => post('focus'));
    editor.focus({ preventScroll: true });
  </script>
</body>
</html>"#
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipboardPayload {
    PlainText(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipboardStatus {
    Available,
    Unavailable(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipboardError {
    Unavailable(String),
    ReadFailed(String),
    WriteFailed(String),
}

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClipboardError::Unavailable(message)
            | ClipboardError::ReadFailed(message)
            | ClipboardError::WriteFailed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ClipboardError {}

pub struct PlatformClipboard {
    inner: Option<arboard::Clipboard>,
    unavailable_reason: Option<String>,
}

impl PlatformClipboard {
    pub fn current() -> Self {
        match arboard::Clipboard::new() {
            Ok(clipboard) => Self {
                inner: Some(clipboard),
                unavailable_reason: None,
            },
            Err(error) => Self {
                inner: None,
                unavailable_reason: Some(error.to_string()),
            },
        }
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            inner: None,
            unavailable_reason: Some(reason.into()),
        }
    }

    pub fn status(&self) -> ClipboardStatus {
        match &self.inner {
            Some(_) => ClipboardStatus::Available,
            None => ClipboardStatus::Unavailable(
                self.unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "Clipboard is unavailable.".to_string()),
            ),
        }
    }

    pub fn get_text(&mut self) -> Result<String, ClipboardError> {
        let Some(clipboard) = &mut self.inner else {
            return Err(ClipboardError::Unavailable(self.unavailable_message()));
        };
        clipboard
            .get_text()
            .map_err(|error| ClipboardError::ReadFailed(error.to_string()))
    }

    pub fn set_text(&mut self, text: impl Into<String>) -> Result<(), ClipboardError> {
        let Some(clipboard) = &mut self.inner else {
            return Err(ClipboardError::Unavailable(self.unavailable_message()));
        };
        clipboard
            .set_text(text.into())
            .map_err(|error| ClipboardError::WriteFailed(error.to_string()))
    }

    pub fn read_payload(&mut self) -> Result<ClipboardPayload, ClipboardError> {
        self.get_text().map(ClipboardPayload::PlainText)
    }

    pub fn write_payload(&mut self, payload: ClipboardPayload) -> Result<(), ClipboardError> {
        match payload {
            ClipboardPayload::PlainText(text) => self.set_text(text),
        }
    }

    fn unavailable_message(&self) -> String {
        self.unavailable_reason
            .clone()
            .unwrap_or_else(|| "Clipboard is unavailable.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_clipboard_reports_status_and_read_error() {
        let mut clipboard = PlatformClipboard::unavailable("missing clipboard provider");

        assert_eq!(
            clipboard.status(),
            ClipboardStatus::Unavailable("missing clipboard provider".to_string())
        );
        assert_eq!(
            clipboard.get_text(),
            Err(ClipboardError::Unavailable(
                "missing clipboard provider".to_string()
            ))
        );
    }

    #[test]
    fn plain_text_payload_roundtrip_uses_text_variant() {
        let payload = ClipboardPayload::PlainText("copy me".to_string());

        let ClipboardPayload::PlainText(text) = payload;

        assert_eq!(text, "copy me");
    }
}

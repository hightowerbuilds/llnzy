#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipboardPayload {
    PlainText(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipboardStatus {
    Available,
    Unavailable(String),
}

use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalHostKind {
    UnixPty,
    ConPty,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalLaunchSpec {
    pub host: TerminalHostKind,
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalProcessState {
    pub process_id: Option<u32>,
    pub exited: Option<i32>,
    pub failure: Option<String>,
}

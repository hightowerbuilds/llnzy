use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShellKind {
    Unix,
    PowerShell,
    WindowsPowerShell,
    CommandPrompt,
    GitBash,
    Wsl,
    Custom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CwdPolicy {
    InheritActive,
    ProjectRoot,
    Explicit(PathBuf),
    Home,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellProfile {
    pub name: String,
    pub kind: ShellKind,
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd_policy: CwdPolicy,
    pub env: Vec<(String, String)>,
    pub default: bool,
    pub interactive: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskLaunchSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub shell_profile: Option<String>,
}

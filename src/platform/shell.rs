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

impl ShellProfile {
    pub fn interactive_default(program: impl Into<PathBuf>, cwd: Option<&str>) -> Self {
        let program = program.into();
        let name = program
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("shell")
            .to_string();
        Self {
            name,
            kind: current_shell_kind(),
            program,
            args: interactive_shell_args(),
            cwd_policy: cwd
                .map(|cwd| CwdPolicy::Explicit(PathBuf::from(cwd)))
                .unwrap_or(CwdPolicy::InheritActive),
            env: default_terminal_env(),
            default: true,
            interactive: true,
        }
    }
}

pub fn current_shell_kind() -> ShellKind {
    if cfg!(target_os = "windows") {
        ShellKind::PowerShell
    } else {
        ShellKind::Unix
    }
}

pub fn interactive_shell_args() -> Vec<String> {
    if cfg!(target_os = "windows") {
        Vec::new()
    } else {
        vec!["-l".to_string()]
    }
}

pub fn default_terminal_env() -> Vec<(String, String)> {
    vec![
        ("TERM".to_string(), "xterm-256color".to_string()),
        ("COLORTERM".to_string(), "truecolor".to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interactive_default_preserves_shell_program_and_cwd() {
        let profile = ShellProfile::interactive_default("/bin/sh", Some("/tmp"));

        assert_eq!(profile.program, PathBuf::from("/bin/sh"));
        assert_eq!(
            profile.cwd_policy,
            CwdPolicy::Explicit(PathBuf::from("/tmp"))
        );
        assert!(profile.default);
        assert!(profile.interactive);
        assert!(profile.env.iter().any(|(key, _)| key == "TERM"));
    }
}

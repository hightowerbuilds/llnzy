use std::path::PathBuf;

use super::shell::{CwdPolicy, ShellProfile};

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

impl TerminalLaunchSpec {
    pub fn interactive_shell(profile: &ShellProfile, cols: u16, rows: u16) -> Self {
        Self {
            host: current_terminal_host_kind(),
            program: profile.program.clone(),
            args: profile.args.clone(),
            cwd: cwd_from_policy(&profile.cwd_policy),
            env: profile.env.clone(),
            cols,
            rows,
        }
    }
}

pub fn current_terminal_host_kind() -> TerminalHostKind {
    if cfg!(target_os = "windows") {
        TerminalHostKind::ConPty
    } else {
        TerminalHostKind::UnixPty
    }
}

fn cwd_from_policy(policy: &CwdPolicy) -> Option<PathBuf> {
    match policy {
        CwdPolicy::Explicit(path) => Some(path.clone()),
        CwdPolicy::InheritActive | CwdPolicy::ProjectRoot | CwdPolicy::Home => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::shell::ShellProfile;

    #[test]
    fn interactive_shell_launch_spec_uses_profile_details() {
        let profile = ShellProfile::interactive_default("/bin/sh", Some("/tmp"));

        let spec = TerminalLaunchSpec::interactive_shell(&profile, 100, 30);

        assert_eq!(spec.program, PathBuf::from("/bin/sh"));
        assert_eq!(spec.cwd, Some(PathBuf::from("/tmp")));
        assert_eq!(spec.cols, 100);
        assert_eq!(spec.rows, 30);
        assert!(spec.env.iter().any(|(key, _)| key == "COLORTERM"));
    }
}

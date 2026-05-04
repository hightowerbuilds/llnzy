use std::path::PathBuf;
use std::process::Command;

use super::PlatformFamily;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenTarget {
    Url(String),
    File(PathBuf),
    Folder(PathBuf),
    Reveal(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenRequest {
    pub target: OpenTarget,
    pub fallback_to_parent_folder: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopOpenCommand {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenError {
    Unsupported(String),
    SpawnFailed(String),
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::Unsupported(message) | OpenError::SpawnFailed(message) => {
                f.write_str(message)
            }
        }
    }
}

impl std::error::Error for OpenError {}

pub fn open_url(url: impl Into<String>) -> Result<(), OpenError> {
    open_request(OpenRequest {
        target: OpenTarget::Url(url.into()),
        fallback_to_parent_folder: false,
    })
}

pub fn open_file(path: impl Into<PathBuf>) -> Result<(), OpenError> {
    open_request(OpenRequest {
        target: OpenTarget::File(path.into()),
        fallback_to_parent_folder: false,
    })
}

pub fn open_folder(path: impl Into<PathBuf>) -> Result<(), OpenError> {
    open_request(OpenRequest {
        target: OpenTarget::Folder(path.into()),
        fallback_to_parent_folder: false,
    })
}

pub fn reveal_path(path: impl Into<PathBuf>) -> Result<(), OpenError> {
    open_request(OpenRequest {
        target: OpenTarget::Reveal(path.into()),
        fallback_to_parent_folder: true,
    })
}

pub fn open_request(request: OpenRequest) -> Result<(), OpenError> {
    let command = command_for_request(&request)?;
    Command::new(&command.program)
        .args(&command.args)
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            OpenError::SpawnFailed(format!(
                "Failed to launch desktop opener '{}': {error}",
                command.program
            ))
        })
}

pub fn command_for_request(request: &OpenRequest) -> Result<DesktopOpenCommand, OpenError> {
    command_for_family(&crate::platform::current_family(), request)
}

pub fn command_for_family(
    family: &PlatformFamily,
    request: &OpenRequest,
) -> Result<DesktopOpenCommand, OpenError> {
    match family {
        PlatformFamily::Macos => macos_command(request),
        PlatformFamily::Windows => windows_command(request),
        PlatformFamily::Linux => linux_command(request),
        PlatformFamily::Other(name) => Err(OpenError::Unsupported(format!(
            "Desktop open is not configured for {name}."
        ))),
    }
}

fn macos_command(request: &OpenRequest) -> Result<DesktopOpenCommand, OpenError> {
    match &request.target {
        OpenTarget::Url(url) => Ok(command("open", [url.clone()])),
        OpenTarget::File(path) | OpenTarget::Folder(path) => {
            Ok(command("open", [path.display().to_string()]))
        }
        OpenTarget::Reveal(path) => Ok(command(
            "open",
            ["-R".to_string(), path.display().to_string()],
        )),
    }
}

fn windows_command(request: &OpenRequest) -> Result<DesktopOpenCommand, OpenError> {
    match &request.target {
        OpenTarget::Url(url) => Ok(command(
            "cmd",
            [
                "/C".to_string(),
                "start".to_string(),
                String::new(),
                url.clone(),
            ],
        )),
        OpenTarget::File(path) | OpenTarget::Folder(path) => {
            Ok(command("explorer", [path.display().to_string()]))
        }
        OpenTarget::Reveal(path) => {
            Ok(command("explorer", [format!("/select,{}", path.display())]))
        }
    }
}

fn linux_command(request: &OpenRequest) -> Result<DesktopOpenCommand, OpenError> {
    match &request.target {
        OpenTarget::Url(url) => Ok(command("xdg-open", [url.clone()])),
        OpenTarget::File(path) | OpenTarget::Folder(path) => {
            Ok(command("xdg-open", [path.display().to_string()]))
        }
        OpenTarget::Reveal(path) => {
            if request.fallback_to_parent_folder {
                let folder = path.parent().unwrap_or(path.as_path());
                Ok(command("xdg-open", [folder.display().to_string()]))
            } else {
                Err(OpenError::Unsupported(
                    "Reveal is not uniformly supported on Linux.".to_string(),
                ))
            }
        }
    }
}

fn command(
    program: impl Into<String>,
    args: impl IntoIterator<Item = String>,
) -> DesktopOpenCommand {
    DesktopOpenCommand {
        program: program.into(),
        args: args.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn macos_reveal_uses_finder_reveal_flag() {
        let request = OpenRequest {
            target: OpenTarget::Reveal(PathBuf::from("/tmp/example.txt")),
            fallback_to_parent_folder: true,
        };

        let command = command_for_family(&PlatformFamily::Macos, &request).unwrap();

        assert_eq!(command.program, "open");
        assert_eq!(command.args, vec!["-R", "/tmp/example.txt"]);
    }

    #[test]
    fn linux_reveal_falls_back_to_parent_folder_when_requested() {
        let request = OpenRequest {
            target: OpenTarget::Reveal(PathBuf::from("/tmp/example.txt")),
            fallback_to_parent_folder: true,
        };

        let command = command_for_family(&PlatformFamily::Linux, &request).unwrap();

        assert_eq!(command.program, "xdg-open");
        assert_eq!(command.args, vec!["/tmp"]);
    }

    #[test]
    fn windows_url_uses_shell_start_command() {
        let request = OpenRequest {
            target: OpenTarget::Url("https://example.com".to_string()),
            fallback_to_parent_folder: false,
        };

        let command = command_for_family(&PlatformFamily::Windows, &request).unwrap();

        assert_eq!(command.program, "cmd");
        assert_eq!(command.args, vec!["/C", "start", "", "https://example.com"]);
    }
}

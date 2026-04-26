/// Configuration for a language server.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub lang_id: &'static str,
    pub command: &'static str,
    pub args: &'static [&'static str],
}

/// All known language server configurations.
pub fn builtin_servers() -> Vec<ServerConfig> {
    vec![
        ServerConfig {
            lang_id: "rust",
            command: "rust-analyzer",
            args: &[],
        },
        ServerConfig {
            lang_id: "typescript",
            command: "typescript-language-server",
            args: &["--stdio"],
        },
        ServerConfig {
            lang_id: "javascript",
            command: "typescript-language-server",
            args: &["--stdio"],
        },
        ServerConfig {
            lang_id: "tsx",
            command: "typescript-language-server",
            args: &["--stdio"],
        },
        ServerConfig {
            lang_id: "python",
            command: "pyright-langserver",
            args: &["--stdio"],
        },
        ServerConfig {
            lang_id: "go",
            command: "gopls",
            args: &["serve"],
        },
        ServerConfig {
            lang_id: "c",
            command: "clangd",
            args: &[],
        },
        ServerConfig {
            lang_id: "bash",
            command: "bash-language-server",
            args: &["start"],
        },
        ServerConfig {
            lang_id: "html",
            command: "vscode-html-language-server",
            args: &["--stdio"],
        },
        ServerConfig {
            lang_id: "css",
            command: "vscode-css-language-server",
            args: &["--stdio"],
        },
        ServerConfig {
            lang_id: "json",
            command: "vscode-json-language-server",
            args: &["--stdio"],
        },
    ]
}

/// Check whether a server command is available on PATH.
pub fn is_available(command: &str) -> bool {
    std::process::Command::new("which")
        .arg(command)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Find the server config for a language, if the server is installed.
pub fn find_server(lang_id: &str) -> Option<ServerConfig> {
    builtin_servers()
        .into_iter()
        .find(|s| s.lang_id == lang_id && is_available(s.command))
}

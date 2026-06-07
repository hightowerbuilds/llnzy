use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, MutexGuard};

/// Configuration for a language server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerConfig {
    pub lang_id: &'static str,
    pub command: &'static str,
    pub args: &'static [&'static str],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerLookup {
    Available(ServerConfig),
    MissingCommand(ServerConfig),
    UnsupportedLanguage,
}

static COMMAND_AVAILABILITY_CACHE: LazyLock<Mutex<HashMap<String, bool>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn command_availability_cache() -> MutexGuard<'static, HashMap<String, bool>> {
    COMMAND_AVAILABILITY_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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
///
/// Cached: `which` shell-outs are not free (each spawn forks/execs and
/// reads PATH), and `LspManager::ensure_server_for_language` calls this
/// once per language the editor encounters. Caching is per-process and
/// per-command. A user installing a server mid-session would need to
/// restart the app to pick it up; an acceptable tradeoff since spawn
/// would just fail at exec time anyway if a cached "available" command
/// were later removed.
pub fn is_available(command: &str) -> bool {
    if let Some(&cached) = command_availability_cache().get(command) {
        return cached;
    }
    let result = std::process::Command::new("which")
        .arg(command)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    command_availability_cache().insert(command.to_string(), result);
    result
}

pub fn resolve_server(lang_id: &str) -> ServerLookup {
    resolve_server_with(lang_id, is_available)
}

pub fn resolve_server_with(
    lang_id: &str,
    is_command_available: impl Fn(&str) -> bool,
) -> ServerLookup {
    let Some(config) = builtin_servers().into_iter().find(|s| s.lang_id == lang_id) else {
        return ServerLookup::UnsupportedLanguage;
    };

    if is_command_available(config.command) {
        ServerLookup::Available(config)
    } else {
        ServerLookup::MissingCommand(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_server_reports_available_command() {
        let lookup = resolve_server_with("rust", |command| command == "rust-analyzer");

        assert_eq!(
            lookup,
            ServerLookup::Available(ServerConfig {
                lang_id: "rust",
                command: "rust-analyzer",
                args: &[],
            })
        );
    }

    #[test]
    fn resolve_server_reports_missing_command_for_known_language() {
        let lookup = resolve_server_with("python", |_| false);

        assert_eq!(
            lookup,
            ServerLookup::MissingCommand(ServerConfig {
                lang_id: "python",
                command: "pyright-langserver",
                args: &["--stdio"],
            })
        );
    }

    #[test]
    fn resolve_server_reports_unsupported_language() {
        assert_eq!(
            resolve_server_with("totally-unknown", |_| true),
            ServerLookup::UnsupportedLanguage
        );
    }

    #[test]
    fn command_cache_recovers_from_poisoned_lock() {
        let _ = std::panic::catch_unwind(|| {
            let _guard = COMMAND_AVAILABILITY_CACHE.lock().unwrap();
            panic!("poison command cache");
        });

        command_availability_cache().insert("cached-server".to_string(), true);

        assert!(is_available("cached-server"));
    }
}

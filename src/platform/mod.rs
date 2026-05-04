pub mod clipboard;
pub mod input;
pub mod menu;
pub mod open;
pub mod packaging;
pub mod paths;
pub mod shell;
pub mod terminal_host;

pub use packaging::{BuildMode, PackageFormat, PlatformPackagingMetadata};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlatformFamily {
    Macos,
    Windows,
    Linux,
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformServices {
    pub family: PlatformFamily,
    pub packaging: PlatformPackagingMetadata,
}

impl PlatformServices {
    pub fn current() -> Self {
        Self {
            family: current_family(),
            packaging: PlatformPackagingMetadata::development(),
        }
    }
}

pub fn current_family() -> PlatformFamily {
    if cfg!(target_os = "macos") {
        PlatformFamily::Macos
    } else if cfg!(target_os = "windows") {
        PlatformFamily::Windows
    } else if cfg!(target_os = "linux") {
        PlatformFamily::Linux
    } else {
        PlatformFamily::Other(std::env::consts::OS.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_services_include_development_packaging() {
        let services = PlatformServices::current();

        assert_eq!(services.packaging.build_mode, BuildMode::Development);
        assert_eq!(
            services.packaging.package_format,
            PackageFormat::Development
        );
    }
}

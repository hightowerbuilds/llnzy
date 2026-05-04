#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildMode {
    Development,
    Packaged,
    Sandboxed,
    Portable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageFormat {
    Development,
    MacAppBundle,
    MacDmg,
    WindowsInstaller,
    WindowsPortableZip,
    LinuxTarball,
    LinuxDeb,
    LinuxRpm,
    LinuxAppImage,
    Flatpak,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformPackagingMetadata {
    pub app_id: String,
    pub executable_name: String,
    pub display_name: String,
    pub version: String,
    pub build_channel: String,
    pub update_channel: Option<String>,
    pub icon_resource: Option<String>,
    pub file_associations: Vec<String>,
    pub protocol_handlers: Vec<String>,
    pub signing_identity: Option<String>,
    pub build_mode: BuildMode,
    pub package_format: PackageFormat,
}

const PACKAGING_ENV: &str = include_str!("../../assets/packaging.env");

impl PlatformPackagingMetadata {
    pub fn development() -> Self {
        Self {
            app_id: packaging_value("APP_ID", "com.hightowerbuilds.llnzy"),
            executable_name: packaging_value("EXECUTABLE_NAME", "llnzy"),
            display_name: packaging_value("DISPLAY_NAME", "LLNZY"),
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_channel: packaging_value("BUILD_CHANNEL", "development"),
            update_channel: None,
            icon_resource: Some(packaging_value("ICON_RESOURCE", "llnzy.icns")),
            file_associations: Vec::new(),
            protocol_handlers: Vec::new(),
            signing_identity: None,
            build_mode: BuildMode::Development,
            package_format: PackageFormat::Development,
        }
    }

    pub fn mac_app_bundle() -> Self {
        Self {
            build_mode: BuildMode::Packaged,
            package_format: PackageFormat::MacAppBundle,
            ..Self::development()
        }
    }
}

pub fn packaging_value(key: &str, fallback: &str) -> String {
    PACKAGING_ENV
        .lines()
        .filter_map(|line| line.split_once('='))
        .find_map(|(name, value)| (name.trim() == key).then(|| value.trim().to_string()))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_metadata_uses_shared_packaging_values() {
        let metadata = PlatformPackagingMetadata::development();

        assert_eq!(metadata.app_id, "com.hightowerbuilds.llnzy");
        assert_eq!(metadata.executable_name, "llnzy");
        assert_eq!(metadata.display_name, "LLNZY");
        assert_eq!(metadata.icon_resource.as_deref(), Some("llnzy.icns"));
    }

    #[test]
    fn mac_bundle_metadata_uses_packaged_bundle_format() {
        let metadata = PlatformPackagingMetadata::mac_app_bundle();

        assert_eq!(metadata.build_mode, BuildMode::Packaged);
        assert_eq!(metadata.package_format, PackageFormat::MacAppBundle);
    }
}

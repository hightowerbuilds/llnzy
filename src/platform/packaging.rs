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

impl PlatformPackagingMetadata {
    pub fn development() -> Self {
        Self {
            app_id: "com.hightowerbuilds.llnzy".to_string(),
            executable_name: "llnzy".to_string(),
            display_name: "LLNZY".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_channel: "development".to_string(),
            update_channel: None,
            icon_resource: None,
            file_associations: Vec::new(),
            protocol_handlers: Vec::new(),
            signing_identity: None,
            build_mode: BuildMode::Development,
            package_format: PackageFormat::Development,
        }
    }
}

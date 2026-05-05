use crate::performance::PowerSource;

pub fn current_power_source() -> PowerSource {
    current_power_source_for_platform()
}

#[cfg(target_os = "macos")]
fn current_power_source_for_platform() -> PowerSource {
    let Ok(output) = std::process::Command::new("pmset")
        .args(["-g", "batt"])
        .output()
    else {
        return PowerSource::Unknown;
    };
    let text = String::from_utf8_lossy(&output.stdout);
    parse_macos_pmset_power_source(&text)
}

#[cfg(target_os = "linux")]
fn current_power_source_for_platform() -> PowerSource {
    linux_power_source_from_path(std::path::Path::new("/sys/class/power_supply"))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn current_power_source_for_platform() -> PowerSource {
    PowerSource::Unknown
}

#[cfg(target_os = "macos")]
pub(crate) fn parse_macos_pmset_power_source(text: &str) -> PowerSource {
    if text.contains("Battery Power") {
        PowerSource::Battery
    } else if text.contains("AC Power") {
        PowerSource::External
    } else {
        PowerSource::Unknown
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn linux_power_source_from_path(path: &std::path::Path) -> PowerSource {
    let Ok(entries) = std::fs::read_dir(path) else {
        return PowerSource::Unknown;
    };

    let mut saw_battery = false;
    let mut saw_external_online = false;

    for entry in entries.flatten() {
        let supply_path = entry.path();
        let supply_type = read_trimmed(supply_path.join("type"));
        match supply_type.as_deref() {
            Some("Battery") => {
                saw_battery = true;
                if matches!(
                    read_trimmed(supply_path.join("status")).as_deref(),
                    Some("Discharging")
                ) {
                    return PowerSource::Battery;
                }
            }
            Some("Mains") | Some("USB") | Some("USB_C") | Some("USB_PD") => {
                if matches!(
                    read_trimmed(supply_path.join("online")).as_deref(),
                    Some("1")
                ) {
                    saw_external_online = true;
                }
            }
            _ => {}
        }
    }

    if saw_external_online {
        PowerSource::External
    } else if saw_battery {
        PowerSource::Battery
    } else {
        PowerSource::Unknown
    }
}

#[cfg(target_os = "linux")]
fn read_trimmed(path: impl AsRef<std::path::Path>) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn parses_macos_pmset_power_sources() {
        assert_eq!(
            parse_macos_pmset_power_source("Now drawing from 'Battery Power'\n"),
            PowerSource::Battery
        );
        assert_eq!(
            parse_macos_pmset_power_source("Now drawing from 'AC Power'\n"),
            PowerSource::External
        );
        assert_eq!(
            parse_macos_pmset_power_source("No battery information available\n"),
            PowerSource::Unknown
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_power_source_detects_discharging_battery() {
        let root = temp_power_root("battery");
        let bat = root.join("BAT0");
        std::fs::create_dir_all(&bat).unwrap();
        std::fs::write(bat.join("type"), "Battery\n").unwrap();
        std::fs::write(bat.join("status"), "Discharging\n").unwrap();

        assert_eq!(linux_power_source_from_path(&root), PowerSource::Battery);
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_power_source_prefers_online_external_power() {
        let root = temp_power_root("external");
        let bat = root.join("BAT0");
        let ac = root.join("AC");
        std::fs::create_dir_all(&bat).unwrap();
        std::fs::create_dir_all(&ac).unwrap();
        std::fs::write(bat.join("type"), "Battery\n").unwrap();
        std::fs::write(bat.join("status"), "Charging\n").unwrap();
        std::fs::write(ac.join("type"), "Mains\n").unwrap();
        std::fs::write(ac.join("online"), "1\n").unwrap();

        assert_eq!(linux_power_source_from_path(&root), PowerSource::External);
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(target_os = "linux")]
    fn temp_power_root(name: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("llnzy_power_test_{}_{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }
}

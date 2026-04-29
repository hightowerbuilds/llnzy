use std::path::PathBuf;

fn window_state_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("llnzy").join("window_state.toml"))
}

pub fn save_window_placement(width: u32, height: u32, position: Option<(i32, i32)>) {
    let Some(state_path) = window_state_path() else {
        return;
    };
    if let Some(parent) = state_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut content = format!("width = {}\nheight = {}\n", width, height);
    if let Some((x, y)) = position {
        content.push_str(&format!("x = {}\ny = {}\n", x, y));
    }
    let _ = std::fs::write(state_path, content);
}

pub fn load_window_size() -> Option<(u32, u32)> {
    let content = std::fs::read_to_string(window_state_path()?).ok()?;

    #[derive(serde::Deserialize)]
    struct WinState {
        width: Option<u32>,
        height: Option<u32>,
    }

    let state: WinState = toml::from_str(&content).ok()?;
    Some((state.width.unwrap_or(900), state.height.unwrap_or(600)))
}

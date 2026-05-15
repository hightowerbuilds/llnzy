use super::model::ColorScheme;

#[derive(Clone, Debug)]
pub struct ColorTransition {
    pub from: ColorScheme,
    pub to: ColorScheme,
    pub progress: f32,
    pub duration: f32,
    pub elapsed: f32,
}

impl ColorTransition {
    pub fn new(from: ColorScheme, to: ColorScheme, duration: f32) -> Self {
        Self {
            from,
            to,
            progress: 0.0,
            duration,
            elapsed: 0.0,
        }
    }

    pub fn advance(&mut self, dt: f32) -> bool {
        self.elapsed += dt;
        self.progress = (self.elapsed / self.duration).min(1.0);
        self.progress >= 1.0
    }

    pub fn current(&self) -> ColorScheme {
        let t = self.progress * self.progress * (3.0 - 2.0 * self.progress);
        ColorScheme {
            ansi: std::array::from_fn(|i| lerp_rgb(self.from.ansi[i], self.to.ansi[i], t)),
            foreground: lerp_rgb(self.from.foreground, self.to.foreground, t),
            background: lerp_rgb(self.from.background, self.to.background, t),
            cursor: lerp_rgb(self.from.cursor, self.to.cursor, t),
            selection: lerp_rgb(self.from.selection, self.to.selection, t),
            selection_alpha: self.from.selection_alpha
                + (self.to.selection_alpha - self.from.selection_alpha) * t,
        }
    }
}

fn lerp_rgb(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    [
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8,
    ]
}

pub fn apply_time_of_day(colors: &mut ColorScheme) {
    apply_time_of_day_at_hour(colors, local_hour());
}

pub(super) fn apply_time_of_day_at_hour(colors: &mut ColorScheme, hour: f32) {
    let warmth = if (6.0..=18.0).contains(&hour) {
        let day_progress = (hour - 6.0) / 12.0;
        let mid = 1.0 - (day_progress - 0.5).abs() * 2.0;
        -mid * 0.08
    } else {
        0.12
    };

    fn shift_warm(c: &mut [u8; 3], w: f32) {
        let r = (c[0] as f32 + w * 30.0).clamp(0.0, 255.0) as u8;
        let b = (c[2] as f32 - w * 20.0).clamp(0.0, 255.0) as u8;
        c[0] = r;
        c[2] = b;
    }

    shift_warm(&mut colors.foreground, warmth);
    shift_warm(&mut colors.background, warmth);
}

#[cfg(unix)]
fn local_hour() -> f32 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as libc::time_t;
    let mut local = std::mem::MaybeUninit::<libc::tm>::uninit();
    let local = unsafe {
        // SAFETY: `local` points to valid, writable storage for one `libc::tm`
        // and `now` lives for the duration of the `localtime_r` call. A null
        // return is handled before reading the initialized value.
        if libc::localtime_r(&now, local.as_mut_ptr()).is_null() {
            return utc_hour_from_unix_secs(now as u64);
        }
        local.assume_init()
    };
    local.tm_hour as f32
}

#[cfg(not(unix))]
fn local_hour() -> f32 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    utc_hour_from_unix_secs(secs)
}

fn utc_hour_from_unix_secs(secs: u64) -> f32 {
    ((secs % 86400) / 3600) as f32
}

pub(super) fn parse_hex(hex: &str) -> Option<[u8; 3]> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
}

pub fn indexed_color(idx: u8, scheme: &ColorScheme) -> [u8; 3] {
    match idx {
        0..=15 => scheme.ansi[idx as usize],
        16..=231 => {
            let idx = idx - 16;
            let r = idx / 36;
            let g = (idx % 36) / 6;
            let b = idx % 6;
            let to_val = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
            [to_val(r), to_val(g), to_val(b)]
        }
        232..=255 => {
            let v = 8 + 10 * (idx - 232);
            [v, v, v]
        }
    }
}

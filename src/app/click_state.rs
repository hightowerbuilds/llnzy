use std::time::{Duration, Instant};

pub struct ClickState {
    last_time: Instant,
    last_pos: (usize, usize),
    count: u8,
}

impl ClickState {
    pub fn new() -> Self {
        Self {
            last_time: Instant::now() - Duration::from_secs(10),
            last_pos: (usize::MAX, usize::MAX),
            count: 0,
        }
    }

    pub fn click(&mut self, row: usize, col: usize) -> u8 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_time);
        let same_pos = self.last_pos == (row, col);
        if same_pos && elapsed.as_millis() < 500 && self.count < 3 {
            self.count += 1;
        } else {
            self.count = 1;
        }
        self.last_time = now;
        self.last_pos = (row, col);
        self.count
    }

    pub fn count(&self) -> u8 {
        self.count
    }
}

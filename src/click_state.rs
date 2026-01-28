use tokio::time::{Duration, Instant};

pub struct ClickState {
    last_click_time: Option<Instant>,
    last_click_pos: Option<(u16, u16)>,
    double_click_threshold: Duration,
}

impl ClickState {
    pub fn new() -> Self {
        Self {
            last_click_time: None,
            last_click_pos: None,
            double_click_threshold: Duration::from_millis(500),
        }
    }

    pub fn is_double_click(&mut self, x: u16, y: u16) -> bool {
        let now = Instant::now();
        let is_double = if let Some(last_time) = self.last_click_time {
            if let Some((last_x, last_y)) = self.last_click_pos {
                now.duration_since(last_time) <= self.double_click_threshold && last_x == x && last_y == y
            } else {
                false
            }
        } else {
            false
        };

        self.last_click_time = Some(now);
        self.last_click_pos = Some((x, y));
        is_double
    }
}

use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timeline {
    pub position: Duration,
    pub duration: Option<Duration>,
    pub rate: f64,
    pub updated_at: Instant,
    pub is_playing: bool,
}

impl Timeline {
    pub fn new(
        position: Duration,
        duration: Option<Duration>,
        rate: f64,
        is_playing: bool,
    ) -> Self {
        Self {
            position,
            duration,
            rate: if rate <= 0.0 { 1.0 } else { rate },
            updated_at: Instant::now(),
            is_playing,
        }
    }

    pub fn current_position(&self) -> Duration {
        if !self.is_playing {
            return self.position;
        }

        let elapsed = self.updated_at.elapsed().mul_f64(self.rate);
        let current = self.position + elapsed;

        self.duration
            .map_or(current, |duration| current.min(duration))
    }

    pub fn remaining(&self) -> Option<Duration> {
        self.duration
            .map(|duration| duration.saturating_sub(self.current_position()))
    }
}

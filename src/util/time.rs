use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct GameInstant {
    time_unpaused: Duration,
}

impl GameInstant {
    pub fn from_game_start() -> Self {
        Self {
            time_unpaused: Duration::ZERO,
        }
    }

    pub fn add_unpaused_delta(&mut self, delta: Duration) {
        self.time_unpaused += delta;
    }
}

impl std::ops::Add<Duration> for GameInstant {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        Self {
            time_unpaused: self.time_unpaused + rhs,
        }
    }
}

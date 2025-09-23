use chrono::Duration;
use notify_rust::{Notification, Urgency};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Idle,
    Work,
    ShortBreak,
    LongBreak,
    Paused,
}

impl State {
    pub fn notify_when_start() -> Notification {
        Notification::new()
            .summary("It's time to focus!")
            .urgency(Urgency::Low)
            .appname("pomobar")
            .icon("pomobar")
            .clone()
    }

    pub fn notify_when_pause() -> Notification {
        Notification::new()
            .summary("Paused the pomodoro!")
            .urgency(Urgency::Low)
            .appname("pomobar")
            .icon("pomobar")
            .clone()
    }

    pub fn notify_when_take_break() -> Notification {
        Notification::new()
            .summary("It's time to take break!")
            .urgency(Urgency::Low)
            .appname("pomobar")
            .icon("pomobar")
            .clone()
    }

    pub fn notify_when_reset() -> Notification {
        Notification::new()
            .summary("Reset the pomodoro!")
            .urgency(Urgency::Low)
            .appname("pomobar")
            .icon("pomobar")
            .clone()
    }
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content = serde_json::to_string(&self).unwrap();
        write!(f, "{}", content)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Pomobar {
    pub id: String,
    pub state: State,
    pub last_state: State,
    pub pomodoro_count: usize,
    pub remaining_time: Duration,
}

impl Pomobar {
    pub fn with_state(&mut self, new_state: &State) -> &mut Self {
        self.state = new_state.clone();
        self
    }
}

impl Default for Pomobar {
    fn default() -> Self {
        let id = Uuid::new_v4().to_string();
        let state = State::Idle;
        let last_state = State::Idle;
        let pomodoro_count = 0;
        let remaining_time = Duration::minutes(25);

        Pomobar {
            id,
            state,
            last_state,
            pomodoro_count,
            remaining_time,
        }
    }
}

impl std::fmt::Display for Pomobar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content = serde_json::to_string(&self).unwrap();
        f.write_str(&content)
    }
}

impl std::str::FromStr for Pomobar {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let data: Pomobar = serde_json::from_str(s)?;
        Ok(data)
    }
}

impl Pomobar {
    pub fn status(&mut self) -> Self {
        match self.state {
            State::Paused | State::Idle => self.clone(),
            State::Work => {
                if !self.timeout() {
                    self.count_down()
                } else {
                    self.pomodoro_count += 1;
                    let result = self.take_break();
                    State::notify_when_take_break().show().unwrap();
                    result
                }
            }
            State::LongBreak | State::ShortBreak => {
                if !self.timeout() {
                    self.count_down()
                } else {
                    let result = self.work();
                    State::notify_when_start().show().unwrap();
                    result
                }
            }
        }
    }

    fn count_down(&mut self) -> Self {
        self.remaining_time -= Duration::seconds(1);
        self.clone()
    }

    fn timeout(&self) -> bool {
        self.remaining_time <= Duration::seconds(0)
    }

    fn work(&mut self) -> Self {
        self.last_state = self.state.clone();
        self.state = State::Work;
        self.remaining_time = Duration::minutes(25);

        if let State::LongBreak = self.last_state {
            self.pomodoro_count = 0;
        }

        self.clone()
    }

    pub fn toggle(&mut self) -> Self {
        if let State::Idle = self.state {
            State::notify_when_start().show().unwrap();
            return self.work();
        }

        if let State::Paused = self.state {
            self.state = self.last_state.clone();
            self.last_state = State::Paused;

            if let State::LongBreak | State::ShortBreak = self.last_state {
                State::notify_when_take_break().show().unwrap();
            } else {
                State::notify_when_start().show().unwrap();
            };
        } else {
            self.last_state = self.state.clone();
            self.state = State::Paused;

            State::notify_when_pause().show().unwrap();
        }
        self.clone()
    }

    fn take_break(&mut self) -> Self {
        if let State::Work = self.state {
            if self.pomodoro_count < 4 {
                self.last_state = self.state.clone();
                self.state = State::ShortBreak;
                self.remaining_time = Duration::minutes(5);
            } else if self.pomodoro_count == 4 {
                self.last_state = self.state.clone();
                self.state = State::LongBreak;
                self.remaining_time = Duration::minutes(15);
            }
        }
        self.clone()
    }

    pub fn reset(&mut self) -> Self {
        let resutl = Self::default();
        State::notify_when_reset().show().unwrap();
        resutl
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let pomobar = Pomobar::default();
        assert_eq!(pomobar.state, State::Idle);
        assert_eq!(pomobar.last_state, State::Idle);
        assert_eq!(pomobar.pomodoro_count, 0);
        assert_eq!(pomobar.remaining_time, Duration::minutes(25));
    }

    #[test]
    fn test_toggle() {
        let mut pomobar = Pomobar::default();
        pomobar = pomobar.toggle();
        assert_eq!(pomobar.state, State::Work);

        pomobar = pomobar.toggle();
        assert_eq!(pomobar.state, State::Paused);

        pomobar = pomobar.toggle();
        assert_eq!(pomobar.state, State::Work);
    }

    #[test]
    fn test_work_flow() {
        let mut pomobar = Pomobar::default();
        pomobar = pomobar.toggle();

        for i in 0..3 {
            pomobar.remaining_time = Duration::seconds(0);
            pomobar = pomobar.status();
            assert_eq!(pomobar.pomodoro_count, i + 1);
            assert_eq!(pomobar.state, State::ShortBreak);

            pomobar.remaining_time = Duration::seconds(0);
            pomobar = pomobar.status();
            assert_eq!(pomobar.pomodoro_count, i + 1);
            assert_eq!(pomobar.state, State::Work);
        }

        pomobar.remaining_time = Duration::seconds(0);
        pomobar = pomobar.status();
        assert_eq!(pomobar.pomodoro_count, 4);
        assert_eq!(pomobar.state, State::LongBreak);

        pomobar.remaining_time = Duration::seconds(0);
        pomobar = pomobar.status();
        assert_eq!(pomobar.pomodoro_count, 0);
        assert_eq!(pomobar.state, State::Work);
    }

    #[test]
    fn test_count_down() {
        let mut pomobar = Pomobar::default();
        let initial_time = pomobar.remaining_time;
        pomobar = pomobar.count_down();
        assert_eq!(pomobar.remaining_time, initial_time - Duration::seconds(1));
    }

    #[test]
    fn test_reset() {
        let mut pomobar = Pomobar::default();
        pomobar.with_state(&State::Work);
        pomobar.pomodoro_count = 2;
        pomobar = pomobar.reset();
        assert_eq!(pomobar.state, State::Idle);
        assert_eq!(pomobar.pomodoro_count, 0);
    }
}

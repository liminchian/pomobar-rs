use chrono::{Duration, Local, NaiveDateTime, TimeDelta};
use notify_rust::{Notification, Urgency};
use serde::{Deserialize, Serialize};

// --- Notifications ---

pub fn send_notification(summary: &str) {
    Notification::new()
        .summary(summary)
        .urgency(Urgency::Low)
        .appname("pomobar")
        .icon("pomobar")
        .show()
        .unwrap();
}

// --- State Marker Traits and Structs ---

/// A trait for states that have a defined duration.
pub trait TimedState {
    fn duration(&self) -> Duration;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Idle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Work {
    pub started_at: NaiveDateTime,
    pub cycles: u32,
}

impl TimedState for Work {
    fn duration(&self) -> Duration {
        Duration::minutes(25)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paused {
    pub remaining: Duration,
    pub cycles: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortBreak {
    pub started_at: NaiveDateTime,
    pub cycles: u32,
}

impl TimedState for ShortBreak {
    fn duration(&self) -> Duration {
        Duration::minutes(5)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongBreak {
    pub started_at: NaiveDateTime,
    pub cycles: u32,
}

impl TimedState for LongBreak {
    fn duration(&self) -> Duration {
        Duration::minutes(15)
    }
}

// --- Generic Pomodoro Timer ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pomobar<S> {
    pub state: S,
}

// --- Transitions ---

// Transitions from Idle
impl Pomobar<Idle> {
    pub fn new() -> Self {
        Pomobar { state: Idle }
    }

    pub fn start(self) -> Pomobar<Work> {
        send_notification("Time to focus!");
        Pomobar {
            state: Work {
                started_at: Local::now().naive_local(),
                cycles: 0,
            },
        }
    }
}

impl Default for Pomobar<Idle> {
    fn default() -> Self {
        Self::new()
    }
}

// Transitions from Working
impl Pomobar<Work> {
    pub fn pause(self) -> Pomobar<Paused> {
        send_notification("Pomodoro paused.");
        let elapsed = Local::now().naive_local() - self.state.started_at;
        let remaining = self.state.duration() - elapsed;
        Pomobar {
            state: Paused {
                remaining: if remaining > Duration::zero() {
                    remaining
                } else {
                    Duration::zero()
                },
                cycles: self.state.cycles,
            },
        }
    }

    pub fn finish(self) -> PomobarDispatcher {
        send_notification("Time for a break!");
        let new_cycles = self.state.cycles + 1;
        if new_cycles.is_multiple_of(4) {
            PomobarDispatcher::LongBreak(Pomobar {
                state: LongBreak {
                    started_at: Local::now().naive_local(),
                    cycles: new_cycles,
                },
            })
        } else {
            PomobarDispatcher::ShortBreak(Pomobar {
                state: ShortBreak {
                    started_at: Local::now().naive_local(),
                    cycles: new_cycles,
                },
            })
        }
    }
}

// Transitions from Paused
impl Pomobar<Paused> {
    pub fn resume(self) -> Pomobar<Work> {
        send_notification("Resuming pomodoro.");
        // To keep the original end time, we calculate a new start time.
        let new_started_at =
            Local::now().naive_local() - (Duration::minutes(25) - self.state.remaining);

        Pomobar {
            state: Work {
                started_at: new_started_at,
                cycles: self.state.cycles,
            },
        }
    }
}

// Transitions from Breaks
impl Pomobar<ShortBreak> {
    pub fn finish(self) -> Pomobar<Work> {
        send_notification("Break is over. Time to focus!");
        Pomobar {
            state: Work {
                started_at: Local::now().naive_local(),
                cycles: self.state.cycles,
            },
        }
    }
}

impl Pomobar<LongBreak> {
    pub fn finish(self) -> Pomobar<Work> {
        send_notification("Long break is over. Time to get back to it!");
        Pomobar {
            state: Work {
                started_at: Local::now().naive_local(),
                cycles: self.state.cycles,
            },
        }
    }
}

// --- Pomodoro Dispatcher Enum ---

/// An enum to hold any possible state of the Pomodoro timer.
/// This allows us to have a single variable hold the state machine
/// while still enforcing typed transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum PomobarDispatcher {
    Idle(Pomobar<Idle>),
    Work(Pomobar<Work>),
    Paused(Pomobar<Paused>),
    ShortBreak(Pomobar<ShortBreak>),
    LongBreak(Pomobar<LongBreak>),
}

#[derive(Serialize, Deserialize)]
struct StatusView<'view> {
    alt: &'view str,
    class: &'view str,
    text: &'view str,
    tooltip: &'view str,
}

impl PomobarDispatcher {
    pub fn get_remaining_time(&self) -> Duration {
        match self {
            PomobarDispatcher::Work(p) => {
                let elapsed = Local::now().naive_local() - p.state.started_at;
                let remaining = p.state.duration() - elapsed;
                if remaining < Duration::zero() {
                    Duration::zero()
                } else {
                    remaining
                }
            }
            PomobarDispatcher::Paused(p) => p.state.remaining,
            PomobarDispatcher::ShortBreak(p) => {
                let elapsed = Local::now().naive_local() - p.state.started_at;
                let remaining = p.state.duration() - elapsed;
                if remaining < Duration::zero() {
                    Duration::zero()
                } else {
                    remaining
                }
            }
            PomobarDispatcher::LongBreak(p) => {
                let elapsed = Local::now().naive_local() - p.state.started_at;
                let remaining = p.state.duration() - elapsed;
                if remaining < Duration::zero() {
                    Duration::zero()
                } else {
                    remaining
                }
            }
            PomobarDispatcher::Idle(_) => Duration::minutes(25),
        }
    }

    pub fn get_state_name(&self) -> &str {
        match self {
            PomobarDispatcher::Idle(_) => "idle",
            PomobarDispatcher::Work(_) => "work",
            PomobarDispatcher::Paused(_) => "paused",
            PomobarDispatcher::ShortBreak(_) => "short_break",
            PomobarDispatcher::LongBreak(_) => "long_break",
        }
    }

    pub fn get_cycles(&self) -> u32 {
        match self {
            PomobarDispatcher::Idle(_) => 0,
            PomobarDispatcher::Work(p) => p.state.cycles,
            PomobarDispatcher::Paused(p) => p.state.cycles,
            PomobarDispatcher::ShortBreak(p) => p.state.cycles,
            PomobarDispatcher::LongBreak(p) => p.state.cycles,
        }
    }

    pub fn to_view(&self) -> String {
        let mins = self.get_remaining_time().num_minutes();
        let secs = self
            .get_remaining_time()
            .checked_sub(&TimeDelta::minutes(mins))
            .unwrap()
            .num_seconds();

        let view = StatusView {
            alt: self.get_state_name(),
            class: self.get_state_name(),
            text: &format!("{mins:02}:{secs:02}"),
            tooltip: &format!("Completed {} pomodoros.", self.get_cycles()),
        };

        serde_json::to_string(&view).unwrap()
    }
}

use std::env;
use std::error::Error;
use std::io;
use std::time::{Duration, Instant};

const FPS_VARIABLE: &str = "AMBER_MAX_FPS";
const DEMO_VARIABLE: &str = "AMBER_TIMING_DEMO";
const COLLIDER_DEBUG_VARIABLE: &str = "AMBER_COLLIDER_DEBUG";

/// Дополнительное ограничение кадров для проверки; BFGE сам ограничивает их до 60.
pub struct FrameLimiter {
    interval: Option<Duration>,
    previous: Instant,
}

impl FrameLimiter {
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        let interval = match env::var(FPS_VARIABLE) {
            Ok(value) => {
                let fps = value
                    .parse::<u32>()
                    .ok()
                    .filter(|fps| (1..=60).contains(fps));
                let fps = fps.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "AMBER_MAX_FPS: ожидается целое число от 1 до 60",
                    )
                })?;
                Some(Duration::from_secs_f64(1.0 / f64::from(fps)))
            }
            Err(env::VarError::NotPresent) => None,
            Err(error) => return Err(error.into()),
        };
        Ok(Self {
            interval,
            previous: Instant::now(),
        })
    }

    pub fn wait(&mut self) {
        if let Some(interval) = self.interval {
            std::thread::sleep(interval.saturating_sub(self.previous.elapsed()));
        }
        self.previous = Instant::now();
    }
}

pub fn timing_demo_enabled() -> Result<bool, Box<dyn Error>> {
    flag_from_env(DEMO_VARIABLE)
}

pub fn collider_debug_enabled() -> Result<bool, Box<dyn Error>> {
    flag_from_env(COLLIDER_DEBUG_VARIABLE)
}

fn flag_from_env(name: &str) -> Result<bool, Box<dyn Error>> {
    match env::var(name) {
        Ok(value) if value == "1" => Ok(true),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name}: ожидается 1 либо отсутствие переменной"),
        )
        .into()),
        Err(env::VarError::NotPresent) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

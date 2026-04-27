use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct TelemetrySample {
    pub name: &'static str,
    pub duration: Duration,
}

pub fn measure<F, T>(name: &'static str, f: F) -> (T, TelemetrySample)
where
    F: FnOnce() -> T,
{
    let started = Instant::now();
    let value = f();
    (
        value,
        TelemetrySample {
            name,
            duration: started.elapsed(),
        },
    )
}

use std::time::Duration;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::Instant;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use wasm_bindgen::{JsCast, JsValue};

/// Clock available to repository tooling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Clock {
    /// Use the host monotonic clock when available.
    System,
    /// Disable timing samples.
    None,
}

/// One captured host clock reading.
#[derive(Debug, Clone, Copy)]
pub struct Moment {
    /// The native monotonic instant.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    instant: Instant,
    /// The browser monotonic timestamp in microseconds.
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    micros: u64,
}

impl Default for Clock {
    /// Return the default clock capability for this host platform.
    fn default() -> Self {
        default_clock()
    }
}

impl Clock {
    /// Return the current instant when timing is enabled and supported.
    pub fn now(self) -> Option<Moment> {
        match self {
            Self::System => system_moment(),
            Self::None => None,
        }
    }

    /// Return elapsed time since one instant when timing is enabled and supported.
    pub fn elapsed(self, moment: Moment) -> Duration {
        match self {
            Self::System => elapsed(moment),
            Self::None => Duration::ZERO,
        }
    }

    /// Return one offset from the provided epoch when timing is enabled and supported.
    pub fn duration_since(self, moment: Moment, epoch: Option<Moment>) -> Duration {
        match self {
            Self::System => duration_since(moment, epoch),
            Self::None => Duration::ZERO,
        }
    }
}

/// Return the default clock on hosts with standard time support.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn default_clock() -> Clock {
    Clock::System
}

/// Return the default clock on JavaScript WebAssembly.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
fn default_clock() -> Clock {
    Clock::System
}

/// Return the current host clock reading when the platform exposes one.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn system_moment() -> Option<Moment> {
    Some(Moment {
        instant: Instant::now(),
    })
}

/// Return the current JavaScript clock reading when the platform exposes one.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
fn system_moment() -> Option<Moment> {
    let global = js_sys::global();
    let performance = js_sys::Reflect::get(&global, &JsValue::from_str("performance")).ok()?;
    let now = js_sys::Reflect::get(&performance, &JsValue::from_str("now")).ok()?;
    let now = now.dyn_ref::<js_sys::Function>()?;
    let millis = now.call0(&performance).ok()?.as_f64()?;
    if !millis.is_finite() || millis < 0.0 {
        return None;
    }

    Some(Moment {
        micros: (millis * 1000.0) as u64,
    })
}

/// Return elapsed time since one instant when the platform exposes time.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn elapsed(moment: Moment) -> Duration {
    moment.instant.elapsed()
}

/// Return elapsed time since one JavaScript clock reading.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
fn elapsed(moment: Moment) -> Duration {
    system_moment()
        .map(|now| Duration::from_micros(now.micros.saturating_sub(moment.micros)))
        .unwrap_or(Duration::ZERO)
}

/// Return one instant offset when the platform exposes time.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn duration_since(moment: Moment, epoch: Option<Moment>) -> Duration {
    epoch
        .map(|epoch| moment.instant.duration_since(epoch.instant))
        .unwrap_or(Duration::ZERO)
}

/// Return one JavaScript clock offset when the platform exposes time.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
fn duration_since(moment: Moment, epoch: Option<Moment>) -> Duration {
    epoch
        .map(|epoch| Duration::from_micros(moment.micros.saturating_sub(epoch.micros)))
        .unwrap_or(Duration::ZERO)
}

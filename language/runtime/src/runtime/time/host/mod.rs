#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

mod clock;

pub(crate) use clock::{HostClock, HostClockSource};

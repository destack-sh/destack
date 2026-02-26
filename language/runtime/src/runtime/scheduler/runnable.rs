use super::{Microtask, Task, Timer};
use crate::host::HostEvent;
use crate::runtime::poller::PollerEvent;

/// Runnable item returned by the event loop.
#[derive(Debug)]
pub enum Runnable {
    /// A macrotask selected for execution.
    Task(Task),
    /// A microtask selected for execution.
    Microtask(Microtask),
    /// A timer ready to fire.
    Timer(Timer),
    /// An external platform event.
    PollerEvent(PollerEvent),
    /// A host semantic event.
    HostEvent(HostEvent),
}

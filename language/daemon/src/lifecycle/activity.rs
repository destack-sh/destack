use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// Connection activity used by daemon idle shutdown.
#[derive(Debug)]
pub struct DaemonActivity {
    /// Mutable activity state.
    state: Mutex<ActivityState>,
}

impl DaemonActivity {
    /// Create activity tracking with an optional idle timeout.
    pub fn new(idle_timeout: Option<Duration>) -> Self {
        Self {
            state: Mutex::new(ActivityState {
                active_connections: 0,
                idle_timeout,
                last_activity: Instant::now(),
            }),
        }
    }

    /// Register one accepted connection.
    pub fn connect(&self) {
        let mut state = self.state.lock();
        state.active_connections += 1;
        state.last_activity = Instant::now();
    }

    /// Release one terminated connection.
    pub fn disconnect(&self) {
        let mut state = self.state.lock();
        state.active_connections = state.active_connections.saturating_sub(1);
        state.last_activity = Instant::now();
    }

    /// Return whether the daemon has remained idle for its configured timeout.
    pub fn is_idle(&self) -> bool {
        let state = self.state.lock();
        let Some(timeout) = state.idle_timeout else {
            return false;
        };

        state.active_connections == 0 && state.last_activity.elapsed() >= timeout
    }
}

/// Mutable daemon activity values.
#[derive(Debug)]
struct ActivityState {
    /// Number of live RPC connections.
    active_connections: usize,
    /// Optional duration before idle shutdown.
    idle_timeout: Option<Duration>,
    /// Last connection transition.
    last_activity: Instant,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::DaemonActivity;

    /// Keep a daemon active while any connection remains.
    #[test]
    fn test_track_connections() {
        let activity = DaemonActivity::new(Some(Duration::ZERO));
        activity.connect();
        assert!(!activity.is_idle());

        activity.disconnect();
        assert!(activity.is_idle());
    }
}

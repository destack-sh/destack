use std::collections::{BTreeMap, HashMap};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};

use parking_lot::Mutex;

use super::identity::serial_descriptor_snapshot;
use super::state::UnixSerialDescriptorInfo;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::runtime::control::queue::BoundedQueue;
use crate::runtime::service::Service;
use crate::runtime::{ExecutionMode, ExecutionPolicy, WorkerLoop};

use super::super::core::SerialWatchEventState;

/// Maximum queued serial topology events per opened watch stream.
pub(super) const SERIAL_WATCH_QUEUE_CAPACITY: usize = 64;

/// One queued unix serial topology event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum UnixSerialWatchRecord {
    /// One attached serial endpoint.
    Instance(UnixSerialDescriptorInfo),
    /// One detached serial endpoint.
    Detached(UnixSerialDescriptorInfo),
}

/// One opened unix serial watch resource.
pub(super) struct UnixSerialWatchResource {
    /// Queued topology events for this watch stream.
    pub(super) event_queue: Arc<BoundedQueue<UnixSerialWatchRecord>>,
    /// Monotonic event sequencing for this watch stream.
    pub(super) event_state: Mutex<SerialWatchEventState>,
}

/// One registered unix serial topology watch.
struct UnixSerialWatchRegistration {
    /// Weak handle to the opened watch resource.
    resource: Weak<UnixSerialWatchResource>,
    /// Last published descriptor snapshot for this watch stream.
    known_ports: BTreeMap<String, UnixSerialDescriptorInfo>,
}

/// One process-global unix serial topology service.
pub(crate) struct UnixSerialWatchService {
    /// The next stable watch registration identifier.
    next_registration_id: AtomicU64,
    /// Registered watch streams keyed by registration identifier.
    registrations: Mutex<HashMap<u64, UnixSerialWatchRegistration>>,
    /// Shared native watcher when at least one watch is active.
    watch_runtime: Mutex<Option<UnixSerialWatchRuntime>>,
}

/// One live linux serial topology watcher.
struct UnixSerialWatchRuntime {
    /// Shared ingress loop runtime.
    _loop: WorkerLoop,
}

impl UnixSerialWatchService {
    /// Build one empty unix serial topology service.
    fn new() -> RuntimeResult<Self> {
        Ok(Self {
            next_registration_id: AtomicU64::new(1),
            registrations: Mutex::new(HashMap::new()),
            watch_runtime: Mutex::new(None),
        })
    }

    /// Register one opened serial watch stream.
    pub(super) fn register_watch(
        self: &Arc<Self>,
        resource: &Arc<UnixSerialWatchResource>,
        known_ports: BTreeMap<String, UnixSerialDescriptorInfo>,
    ) -> RuntimeResult<u64> {
        let registration_id = self.next_registration_id.fetch_add(1, Ordering::AcqRel);

        // retain the watch before arming the shared worker
        {
            let mut registrations = self.registrations.lock();
            registrations.insert(
                registration_id,
                UnixSerialWatchRegistration {
                    resource: Arc::downgrade(resource),
                    known_ports,
                },
            );
        }

        // ensure one shared topology watcher is active
        if let Err(error) = self.ensure_watch_runtime() {
            self.registrations.lock().remove(&registration_id);
            return Err(error);
        }

        Ok(registration_id)
    }

    /// Unregister one opened serial watch stream.
    pub(super) fn unregister_watch(&self, registration_id: u64) {
        let should_stop = {
            let mut registrations = self.registrations.lock();
            registrations.remove(&registration_id);
            registrations.is_empty()
        };

        // retire the shared watcher when no watches remain
        if should_stop {
            self.watch_runtime.lock().take();
        }
    }

    /// Ensure one shared native topology watcher is active.
    fn ensure_watch_runtime(self: &Arc<Self>) -> RuntimeResult<()> {
        let mut watch_runtime = self.watch_runtime.lock();
        if watch_runtime.is_some() {
            return Ok(());
        }

        let service = Arc::downgrade(self);

        // run one shared native monitor loop
        let loop_runtime = WorkerLoop::open(
            "destack-serial-udev-watch",
            "platform.service.spawn",
            Self::POLICY,
            move || {
                let monitor = udev::MonitorBuilder::new()
                    .map_err(|error| monitor_error("MonitorBuilder::new", &error))?
                    .match_subsystem("tty")
                    .map_err(|error| monitor_error("MonitorBuilder::match_subsystem", &error))?
                    .listen()
                    .map_err(|error| monitor_error("MonitorBuilder::listen", &error))?;
                let (shutdown_read, shutdown_write) =
                    shutdown_pipe("destack.device.serial.watchOpen")?;

                let shutdown = Box::new(move || {
                    let _ = unsafe {
                        libc::write(shutdown_write.as_raw_fd(), [1u8].as_ptr().cast(), 1)
                    };
                });
                let run = Box::new(move || watch_monitor_loop(service, monitor, shutdown_read));

                Ok((shutdown, run))
            },
        )?;

        *watch_runtime = Some(UnixSerialWatchRuntime {
            _loop: loop_runtime,
        });

        Ok(())
    }

    /// Refresh all registered watch streams from the current topology snapshot.
    fn refresh_watches(&self) {
        let snapshot = match serial_descriptor_snapshot("destack.device.serial.watchRead") {
            Ok(snapshot) => snapshot,
            Err(error) => {
                tracing::warn!("linux serial topology refresh failed: {error}");
                return;
            }
        };

        let should_stop = {
            let mut registrations = self.registrations.lock();

            // publish one delta pass to each live watch and discard stale registrations
            registrations.retain(|_, registration| {
                let Some(resource) = registration.resource.upgrade() else {
                    return false;
                };

                publish_watch_delta(&resource, &mut registration.known_ports, &snapshot);
                true
            });

            registrations.is_empty()
        };

        // retire the watcher once the last watch disappears
        if should_stop {
            self.watch_runtime.lock().take();
        }
    }
}

impl Service for UnixSerialWatchService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Loop);
}

/// Build one linux serial monitor error.
fn monitor_error(error_site: &'static str, error: &std::io::Error) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io(format!(
        "{error_site} failed while opening linux serial topology monitor: {error}",
    )))
    .boxed()
}

/// Build one shutdown pipe for the serial monitor thread.
fn shutdown_pipe(operation: &'static str) -> RuntimeResult<(OwnedFd, OwnedFd)> {
    let mut descriptors = [0; 2];

    // build one cloexec shutdown pipe for the watcher thread
    let status = unsafe { libc::pipe2(descriptors.as_mut_ptr(), libc::O_CLOEXEC) };
    if status < 0 {
        return Err(RuntimeError::from(PlatformError::io_with(
            None,
            None,
            Some(crate::platform::core::get_errno()),
            Some(operation.to_string()),
            Some(String::from("pipe2")),
            String::from("failed to build linux serial watcher shutdown pipe"),
        ))
        .boxed());
    }

    // take ownership of the shutdown descriptors
    let read_fd = unsafe { OwnedFd::from_raw_fd(descriptors[0]) };
    let write_fd = unsafe { OwnedFd::from_raw_fd(descriptors[1]) };

    Ok((read_fd, write_fd))
}

/// Run one linux serial topology monitor loop.
fn watch_monitor_loop(
    service: Weak<UnixSerialWatchService>,
    monitor: udev::MonitorSocket,
    shutdown_read: OwnedFd,
) -> RuntimeResult<()> {
    let monitor_fd = monitor.as_raw_fd();
    let shutdown_fd = shutdown_read.as_raw_fd();
    let mut monitor = monitor;

    loop {
        let mut poll_fds = [
            libc::pollfd {
                fd: monitor_fd,
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: shutdown_fd,
                events: libc::POLLIN,
                revents: 0,
            },
        ];

        // wait for either one udev event or one shutdown request
        let poll_status = unsafe { libc::poll(poll_fds.as_mut_ptr(), poll_fds.len() as _, -1) };

        // retry interrupted waits without changing the blocking contract
        if poll_status < 0 && crate::platform::core::get_errno() == libc::EINTR {
            continue;
        }

        // stop the watcher after one terminal poll failure
        if poll_status < 0 {
            let errno = crate::platform::core::get_errno();
            return Err(RuntimeError::from(PlatformError::io_with(
                None,
                None,
                Some(errno),
                Some(String::from("destack.device.serial.watchRead")),
                Some(String::from("poll")),
                format!("linux serial topology monitor poll failed: errno={errno}"),
            ))
            .boxed());
        }

        // stop immediately when teardown requests shutdown
        if poll_fds[1].revents & (libc::POLLIN | libc::POLLERR | libc::POLLHUP | libc::POLLNVAL)
            != 0
        {
            return Ok(());
        }

        // stop the watcher after one terminal monitor failure
        if poll_fds[0].revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
            return Err(RuntimeError::from(PlatformError::io_with(
                None,
                None,
                None,
                Some(String::from("destack.device.serial.watchRead")),
                Some(String::from("poll")),
                String::from("linux serial topology monitor reported one terminal poll state"),
            ))
            .boxed());
        }

        // drain all queued udev events before refreshing the shared snapshot
        if poll_fds[0].revents & libc::POLLIN != 0 {
            while monitor.iter().next().is_some() {}

            let Some(service) = service.upgrade() else {
                return Ok(());
            };

            service.refresh_watches();
        }
    }
}

/// Publish one snapshot delta into one watch queue.
fn publish_watch_delta(
    resource: &UnixSerialWatchResource,
    known_ports: &mut BTreeMap<String, UnixSerialDescriptorInfo>,
    snapshot: &BTreeMap<String, UnixSerialDescriptorInfo>,
) {
    // queue detach or replace transitions from the previous snapshot
    for (port_id, previous) in &*known_ports {
        match snapshot.get(port_id) {
            Some(current) if current == previous => {}
            Some(_) | None => {
                resource
                    .event_queue
                    .push_drop_oldest(UnixSerialWatchRecord::Detached(previous.clone()));
            }
        }
    }

    // queue attach or replace transitions from the current snapshot
    for (port_id, current) in snapshot {
        match known_ports.get(port_id) {
            Some(previous) if previous == current => {}
            Some(_) | None => {
                resource
                    .event_queue
                    .push_drop_oldest(UnixSerialWatchRecord::Instance(current.clone()));
            }
        }
    }

    *known_ports = snapshot.clone();
}

/// Resolve the shared unix serial topology service.
pub(crate) fn unix_serial_watch_service(
    operation: &'static str,
) -> RuntimeResult<Arc<UnixSerialWatchService>> {
    UnixSerialWatchService::global(UnixSerialWatchService::new).map_err(|error| {
        let platform_code = error.platform_error().map(|error| error.code);

        RuntimeError::from(PlatformError::io_with(
            platform_code,
            None,
            None,
            Some(operation.to_string()),
            Some(String::from("unixSerialWatchService")),
            format!("unix serial watch service unavailable: {error}"),
        ))
        .boxed()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build one deterministic unix serial descriptor fixture.
    fn descriptor(id: &str, name: &str) -> UnixSerialDescriptorInfo {
        UnixSerialDescriptorInfo {
            id: id.to_string(),
            transport: crate::platform::device::SerialPortTransport::Native,
            name: name.to_string(),
            manufacturer: None,
            product: None,
            serial_number: None,
            path_bytes: name.as_bytes().to_vec(),
            usb_vendor_id: None,
            usb_product_id: None,
            bluetooth_service_class_id: None,
        }
    }

    /// Publish attached and detached records when the topology snapshot changes.
    #[test]
    fn test_publish_watch_delta_emits_attached_and_detached_records() {
        let resource = UnixSerialWatchResource {
            event_queue: Arc::new(BoundedQueue::new(8)),
            event_state: Mutex::new(SerialWatchEventState {
                next_sequence: 1,
                reported_dropped_count: 0,
            }),
        };
        let previous = descriptor("serial-a", "/dev/ttyA");
        let current = descriptor("serial-b", "/dev/ttyB");
        let mut known_ports = BTreeMap::from([(previous.id.clone(), previous.clone())]);
        let snapshot = BTreeMap::from([(current.id.clone(), current.clone())]);

        // publish one topology transition
        publish_watch_delta(&resource, &mut known_ports, &snapshot);

        let first = resource.event_queue.try_pop().unwrap();
        let second = resource.event_queue.try_pop().unwrap();

        assert_eq!(first, UnixSerialWatchRecord::Detached(previous));
        assert_eq!(second, UnixSerialWatchRecord::Instance(current.clone()));
        assert_eq!(known_ports.get(&current.id), Some(&current));
    }

    /// Keep the watch queue quiet when the topology snapshot is unchanged.
    #[test]
    fn test_publish_watch_delta_skips_unchanged_snapshots() {
        let current = descriptor("serial-a", "/dev/ttyA");
        let resource = UnixSerialWatchResource {
            event_queue: Arc::new(BoundedQueue::new(8)),
            event_state: Mutex::new(SerialWatchEventState {
                next_sequence: 1,
                reported_dropped_count: 0,
            }),
        };
        let mut known_ports = BTreeMap::from([(current.id.clone(), current.clone())]);
        let snapshot = BTreeMap::from([(current.id.clone(), current.clone())]);

        // publish one identical snapshot
        publish_watch_delta(&resource, &mut known_ports, &snapshot);

        assert!(resource.event_queue.try_pop().is_none());
        assert_eq!(known_ports.get(&current.id), Some(&current));
    }
}

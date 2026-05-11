use std::collections::BTreeMap;
use std::mem::size_of;
use std::ptr::addr_of;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use parking_lot::Mutex;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Media::Audio::{
    HMIDIIN, HMIDIOUT, MIDIHDR, midiInClose, midiInReset, midiInStop, midiInUnprepareHeader,
    midiOutClose, midiOutReset,
};

use crate::platform::device::midi::core::{
    MidiEventValue, MidiInputRecordValue, MidiPortDescriptorValue,
    define_backend_midi_resource_inserters,
};
use crate::platform::device::{
    MidiBackend, MidiDataFormat, MidiEventOverflowPolicy, MidiEventSubscriptionFlags,
    MidiPortDirection, MidiPortDirectionFlags, MidiProtocol,
};
use crate::runtime::control::queue::BoundedQueue;
use crate::runtime::service::executor::periodic::PeriodicTaskHandle;

use super::super::service::WinMmService;
use super::native::release_input_callback_context;

/// Default number of queued WinMM SysEx buffers.
pub(crate) const DEFAULT_SYSEX_BUFFER_COUNT: usize = 4;

/// One cached WinMM endpoint row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WinMmEndpointInfo {
    /// Descriptor snapshot for this endpoint.
    pub(crate) descriptor: MidiPortDescriptorValue,
    /// Backend device id.
    pub(crate) device_id: u32,
}

/// One process-global snapshot of WinMM topology.
#[derive(Default)]
pub(crate) struct WinMmTopologyState {
    /// Current input endpoint rows.
    pub(crate) inputs: BTreeMap<String, WinMmEndpointInfo>,
    /// Current output endpoint rows.
    pub(crate) outputs: BTreeMap<String, WinMmEndpointInfo>,
}

/// One input SysEx buffer.
pub(crate) struct WinMmInputBuffer {
    /// Raw WinMM header.
    pub(crate) header: MIDIHDR,
    /// Owned payload storage.
    pub(crate) _data: Vec<u8>,
}

unsafe impl Send for WinMmInputBuffer {}
unsafe impl Sync for WinMmInputBuffer {}

/// One terminal WinMM input failure.
#[derive(Debug, Clone)]
pub(crate) enum WinMmInputTerminalError {
    /// The backend reported one invalid short message.
    ShortMessageError,
    /// The backend reported one invalid long message.
    LongMessageError,
    /// The backend could not queue the next SysEx input buffer.
    BufferRecycleFailed(u32),
}

impl WinMmInputTerminalError {
    /// Return one stable runtime error message for this terminal failure.
    pub(crate) fn message(&self) -> String {
        match self {
            Self::ShortMessageError => {
                "WinMM input session received one malformed short MIDI message".to_string()
            }
            Self::LongMessageError => {
                "WinMM input session received one malformed SysEx buffer".to_string()
            }
            Self::BufferRecycleFailed(status) => {
                format!("WinMM input session could not recycle one SysEx buffer: MMRESULT {status}")
            }
        }
    }
}

/// One input callback context retained by WinMM.
pub(crate) struct WinMmInputCallbackContext {
    /// Stable runtime source id for this session.
    pub(crate) source_id: Arc<str>,
    /// Monotonic epoch captured when the session starts.
    pub(crate) start_epoch_ns: AtomicU64,
    /// Shared input queue.
    pub(crate) queue: Arc<BoundedQueue<MidiInputRecordValue>>,
    /// Deferred terminal backend failure.
    pub(crate) terminal_error: Arc<Mutex<Option<WinMmInputTerminalError>>>,
    /// Track whether the current long-data stream is inside SysEx.
    pub(crate) is_inside_sysex: Arc<Mutex<bool>>,
    /// Track whether the session is closing.
    pub(crate) is_closing: Arc<AtomicBool>,
}

/// One opened WinMM input session.
pub(crate) struct WinMmInputRepository {
    /// Shared service owner.
    pub(crate) _service: Arc<WinMmService>,
    /// Current descriptor snapshot.
    pub(crate) descriptor: MidiPortDescriptorValue,
    /// Shared input queue.
    pub(crate) queue: Arc<BoundedQueue<MidiInputRecordValue>>,
    /// Deferred terminal backend failure.
    pub(crate) terminal_error: Arc<Mutex<Option<WinMmInputTerminalError>>>,
    /// WinMM input session resources.
    pub(crate) kind: Mutex<WinMmInputRepositoryKind>,
}

/// One opened WinMM input-session kind.
pub(crate) struct WinMmInputRepositoryKind {
    /// Raw WinMM input handle.
    pub(crate) handle: HMIDIIN,
    /// Prepared SysEx buffers.
    pub(crate) buffers: Vec<WinMmInputBuffer>,
    /// Callback state owned for the session lifetime.
    pub(crate) _context_owner: Arc<WinMmInputCallbackContext>,
    /// Retained callback token held by WinMM.
    pub(crate) callback_context_token: usize,
    /// Closing state shared with the callback.
    pub(crate) is_closing: Arc<AtomicBool>,
}

/// One opened WinMM output session.
pub(crate) struct WinMmOutputRepository {
    /// Shared service owner.
    pub(crate) _service: Arc<WinMmService>,
    /// Current descriptor snapshot.
    pub(crate) descriptor: MidiPortDescriptorValue,
    /// Selected data format for this session.
    pub(crate) data_format: MidiDataFormat,
    /// Selected protocol for this session.
    pub(crate) protocol: Option<MidiProtocol>,
    /// Raw WinMM output handle.
    pub(crate) handle: HMIDIOUT,
    /// Win32 event signaled by WinMM output completion callbacks.
    pub(crate) completion_event: HANDLE,
}

/// One opened WinMM event subscription.
#[derive(Clone)]
pub(crate) struct WinMmEventRepository {
    /// Selected backend for the subscription.
    pub(crate) backend: MidiBackend,
    /// Included directions.
    pub(crate) direction_mask: MidiPortDirectionFlags,
    /// Subscription flags.
    pub(crate) flags: MidiEventSubscriptionFlags,
    /// Overflow policy.
    pub(crate) overflow_policy: MidiEventOverflowPolicy,
    /// Registered synthetic poll task.
    pub(crate) poll_task: Option<Arc<PeriodicTaskHandle>>,
    /// Pending event queue.
    pub(crate) queue: Arc<BoundedQueue<MidiEventValue>>,
    /// Next sequence number.
    pub(crate) next_sequence: u64,
    /// Previous endpoint snapshot.
    pub(crate) snapshot: BTreeMap<String, (MidiPortDirection, MidiPortDescriptorValue)>,
}

/// One stable snapshot-key type for WinMM topology subscriptions.
pub(crate) type SnapshotKey = String;

/// Resource payload for one input session.
pub(crate) struct WinMmInputResource {
    /// Shared session state.
    pub(crate) session: Arc<WinMmInputRepository>,
}

/// Resource payload for one output session.
pub(crate) struct WinMmOutputResource {
    /// Shared session state.
    pub(crate) session: Arc<WinMmOutputRepository>,
}

/// Resource payload for one event subscription.
#[derive(Clone)]
pub(crate) struct WinMmEventResource {
    /// Shared session state.
    pub(crate) session: Arc<Mutex<WinMmEventRepository>>,
}

impl Drop for WinMmInputRepository {
    /// Release WinMM input resources on drop.
    fn drop(&mut self) {
        let mut kind = self.kind.lock();
        let handle = kind.handle;

        // closing state
        kind.is_closing.store(true, Ordering::Release);

        // stop and reset
        unsafe {
            let _ = midiInStop(handle);
            let _ = midiInReset(handle);
        }

        // unprepare all SysEx buffers
        for buffer in &mut kind.buffers {
            unsafe {
                let _ = midiInUnprepareHeader(
                    handle,
                    addr_of!(buffer.header).cast_mut(),
                    size_of::<MIDIHDR>() as u32,
                );
            }
        }

        // close the raw device handle
        unsafe {
            let _ = midiInClose(handle);
        }

        // callback token and queue
        release_input_callback_context(kind.callback_context_token);
        self.queue.close();
    }
}

impl Drop for WinMmOutputRepository {
    /// Release WinMM output resources on drop.
    fn drop(&mut self) {
        // stop backend delivery before releasing native resources
        unsafe {
            let _ = midiOutReset(self.handle);
            let _ = midiOutClose(self.handle);
        }

        // release the session completion event
        if self.completion_event != 0 {
            unsafe {
                let _ = CloseHandle(self.completion_event);
            }
        }
    }
}

define_backend_midi_resource_inserters!(
    vis = pub(crate),
    backend = MidiBackend::WinMM,
    input = (insert_input_resource, WinMmInputResource, WinMmInputRepository),
    output = (insert_output_resource, WinMmOutputResource, WinMmOutputRepository),
    event = (insert_event_resource, WinMmEventResource, WinMmEventRepository)
);

use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use block2::{Block, RcBlock};

use crate::platform::core::BoundedQueue;
use crate::platform::device::midi::core::{MidiInputRecordValue, MidiRecordBytes};
use crate::platform::device::{MidiDataFormat, MidiProtocol, MidiRecordFraming};

use super::abi::{
    K_MIDI_PROTOCOL_2_0, MIDIEventList, MIDIPacketList, midi_event_packet_next, midi_packet_next,
};
use super::core::core_midi_host_time_to_mono_ns;

/// One input callback context passed through CoreMIDI callbacks.
pub(super) struct LegacyInputCallbackContext {
    /// Queue that receives decoded input records.
    pub(super) queue: Arc<BoundedQueue<MidiInputRecordValue>>,
    /// Stable source id for opened source sessions.
    pub(super) source_id: Option<Arc<str>>,
    /// Whether the previous legacy packet ended inside one SysEx fragment chain.
    pub(super) is_inside_sysex: AtomicBool,
}

/// One heap block retained for one CoreMIDI receive callback.
pub(super) struct CoreMidiReceiveBlock {
    /// Raw retained block pointer.
    pub(super) raw: *mut Block<dyn Fn(*const MIDIEventList, *mut u8)>,
}

/// One owned legacy callback token.
pub(super) struct LegacyInputCallbackToken {
    /// Raw callback context pointer owned by this token.
    raw: *mut LegacyInputCallbackContext,
}

unsafe impl Send for CoreMidiReceiveBlock {}
unsafe impl Sync for CoreMidiReceiveBlock {}
unsafe impl Send for LegacyInputCallbackToken {}
unsafe impl Sync for LegacyInputCallbackToken {}

impl Drop for CoreMidiReceiveBlock {
    /// Release the retained callback block.
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }

        unsafe {
            drop(RcBlock::from_raw(self.raw));
        }
    }
}

impl LegacyInputCallbackToken {
    /// Create one owned legacy callback token.
    pub(super) fn new(context: Box<LegacyInputCallbackContext>) -> Self {
        Self {
            raw: Box::into_raw(context),
        }
    }

    /// Borrow the raw callback context pointer for one CoreMIDI call.
    pub(super) fn as_ptr(&self) -> *mut std::ffi::c_void {
        self.raw.cast()
    }
}

impl Drop for LegacyInputCallbackToken {
    /// Release the legacy callback context.
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }

        unsafe {
            drop(Box::from_raw(self.raw));
        }
    }
}

/// Decode one legacy packet list into inbound records.
unsafe fn decode_legacy_packet_list(
    packet_list: *const MIDIPacketList,
    source_id: Option<&Arc<str>>,
    is_inside_sysex: &AtomicBool,
    mut push: impl FnMut(MidiInputRecordValue),
) {
    if packet_list.is_null() {
        return;
    }

    let mut packet = unsafe { ptr::addr_of!((*packet_list).packet[0]) };
    let packet_count = unsafe { (*packet_list).num_packets as usize };

    for _ in 0..packet_count {
        let length = unsafe { (*packet).length as usize };
        let data = unsafe { std::slice::from_raw_parts(ptr::addr_of!((*packet).data[0]), length) };
        let (framing, next_is_inside_sysex) =
            legacy_record_framing(data, is_inside_sysex.load(Ordering::Relaxed));
        is_inside_sysex.store(next_is_inside_sysex, Ordering::Relaxed);

        push(MidiInputRecordValue {
            received_at_ns: core_midi_host_time_to_mono_ns(unsafe { (*packet).time_stamp }),
            source_id: source_id.cloned(),
            data_format: MidiDataFormat::Midi1Bytes,
            protocol: Some(MidiProtocol::Midi1),
            framing,
            data: MidiRecordBytes::from_slice(data),
        });

        packet = unsafe { midi_packet_next(packet) };
    }
}

/// Return one legacy record framing decision and the next SysEx carry state.
fn legacy_record_framing(data: &[u8], is_inside_sysex: bool) -> (MidiRecordFraming, bool) {
    let starts_sysex = data.first() == Some(&0xF0);
    let ends_sysex = data.last() == Some(&0xF7);

    // whole messages do not carry fragment state forward
    if starts_sysex && ends_sysex {
        return (MidiRecordFraming::Complete, false);
    }

    // an explicit sysex start enters fragment mode
    if starts_sysex {
        return (MidiRecordFraming::Start, true);
    }

    // a trailing terminator only closes an in-flight fragment chain
    if ends_sysex && is_inside_sysex {
        return (MidiRecordFraming::End, false);
    }

    // packets in the middle of a sysex chain inherit continuation framing
    if is_inside_sysex {
        return (MidiRecordFraming::Continue, true);
    }

    (MidiRecordFraming::Complete, false)
}

/// Decode one UMP event list into inbound records.
unsafe fn decode_modern_event_list(
    event_list: *const MIDIEventList,
    source_id: Option<&Arc<str>>,
    mut push: impl FnMut(MidiInputRecordValue),
) {
    if event_list.is_null() {
        return;
    }

    let mut packet = unsafe { ptr::addr_of!((*event_list).packet[0]) };
    let packet_count = unsafe { (*event_list).num_packets as usize };

    for _ in 0..packet_count {
        let word_count = unsafe { (*packet).word_count as usize };
        let words =
            unsafe { std::slice::from_raw_parts(ptr::addr_of!((*packet).words[0]), word_count) };
        let mut data = MidiRecordBytes::with_capacity(word_count * 4);

        for word in words {
            data.extend_from_slice(&word.to_be_bytes());
        }

        push(MidiInputRecordValue {
            received_at_ns: core_midi_host_time_to_mono_ns(unsafe { (*packet).time_stamp }),
            source_id: source_id.cloned(),
            data_format: MidiDataFormat::Ump,
            protocol: Some(
                if unsafe { (*event_list).protocol } == K_MIDI_PROTOCOL_2_0 {
                    MidiProtocol::Midi2
                } else {
                    MidiProtocol::Midi1
                },
            ),
            framing: MidiRecordFraming::Complete,
            data,
        });

        packet = unsafe { midi_event_packet_next(packet) };
    }
}

/// Legacy input callback entrypoint.
pub(super) unsafe extern "C" fn legacy_input_read_proc(
    packet_list: *const MIDIPacketList,
    read_proc_ref_con: *mut std::ffi::c_void,
    _src_conn_ref_con: *mut std::ffi::c_void,
) {
    if read_proc_ref_con.is_null() {
        return;
    }

    let context = unsafe { &*(read_proc_ref_con.cast::<LegacyInputCallbackContext>()) };
    unsafe {
        decode_legacy_packet_list(
            packet_list,
            context.source_id.as_ref(),
            &context.is_inside_sysex,
            |record| context.queue.push_drop_oldest(record),
        )
    };
}

/// Build one retained modern receive block and its raw context.
pub(super) fn modern_receive_block(
    queue: Arc<BoundedQueue<MidiInputRecordValue>>,
    source_id: Option<Arc<str>>,
) -> CoreMidiReceiveBlock {
    let block: RcBlock<dyn Fn(*const MIDIEventList, *mut u8)> = RcBlock::new(
        move |event_list: *const MIDIEventList, _conn_ref_con: *mut u8| {
            unsafe {
                decode_modern_event_list(event_list, source_id.as_ref(), |record| {
                    queue.push_drop_oldest(record)
                })
            };
        },
    );
    let raw = RcBlock::into_raw(block);

    CoreMidiReceiveBlock { raw }
}

#[cfg(test)]
mod tests {
    use super::legacy_record_framing;
    use crate::platform::device::MidiRecordFraming;

    /// Preserve SysEx fragment framing across legacy CoreMIDI packet boundaries.
    #[test]
    fn test_legacy_record_framing_tracks_sysex_fragment_state() {
        let (framing, is_inside_sysex) = legacy_record_framing(&[0xF0, 0x7D, 0x01], false);
        assert_eq!(framing, MidiRecordFraming::Start);
        assert!(is_inside_sysex);

        let (framing, is_inside_sysex) = legacy_record_framing(&[0x02, 0x03], is_inside_sysex);
        assert_eq!(framing, MidiRecordFraming::Continue);
        assert!(is_inside_sysex);

        let (framing, is_inside_sysex) = legacy_record_framing(&[0x04, 0xF7], is_inside_sysex);
        assert_eq!(framing, MidiRecordFraming::End);
        assert!(!is_inside_sysex);

        let (framing, is_inside_sysex) = legacy_record_framing(&[0x90, 0x3C, 0x40], false);
        assert_eq!(framing, MidiRecordFraming::Complete);
        assert!(!is_inside_sysex);
    }
}

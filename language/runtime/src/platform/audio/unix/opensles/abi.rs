#![allow(non_snake_case)]

use std::ffi::c_void;

#[link(name = "OpenSLES")]
unsafe extern "C" {
    /// Create one OpenSL ES engine object.
    pub(super) fn slCreateEngine(
        engine: *mut SLObjectItf,
        num_options: SLuint32,
        engine_options: *const SLEngineOption,
        num_interfaces: SLuint32,
        interface_ids: *const SLInterfaceID,
        interface_required: *const SLboolean,
    ) -> SLresult;

    /// OpenSL ES engine interface id.
    pub(super) static mut SL_IID_ENGINE: SLInterfaceID;
    /// OpenSL ES player interface id.
    pub(super) static mut SL_IID_PLAY: SLInterfaceID;
    /// OpenSL ES recorder interface id.
    pub(super) static mut SL_IID_RECORD: SLInterfaceID;
    /// OpenSL ES Android simple-buffer-queue interface id.
    pub(super) static mut SL_IID_ANDROIDSIMPLEBUFFERQUEUE: SLInterfaceID;
}

/// One OpenSL ES unsigned 32-bit type.
pub(super) type SLuint32 = u32;
/// One OpenSL ES boolean type.
pub(super) type SLboolean = SLuint32;
/// One OpenSL ES result type.
pub(super) type SLresult = SLuint32;
/// One OpenSL ES interface id pointer.
pub(super) type SLInterfaceID = *const SLInterfaceID_;
/// One OpenSL ES object interface pointer.
pub(super) type SLObjectItf = *const *const SLObjectItf_;
/// One OpenSL ES engine interface pointer.
pub(super) type SLEngineItf = *const *const SLEngineItf_;
/// One OpenSL ES play interface pointer.
pub(super) type SLPlayItf = *const *const SLPlayItf_;
/// One OpenSL ES record interface pointer.
pub(super) type SLRecordItf = *const *const SLRecordItf_;
/// One OpenSL ES Android simple-buffer-queue interface pointer.
pub(super) type SLAndroidSimpleBufferQueueItf = *const *const SLAndroidSimpleBufferQueueItf_;

/// One OpenSL ES Android simple-buffer-queue callback type.
pub(super) type SlAndroidSimpleBufferQueueCallback =
    Option<unsafe extern "C" fn(SLAndroidSimpleBufferQueueItf, *mut c_void)>;

/// One OpenSL ES interface id payload.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct SLInterfaceID_ {
    /// Low UUID field.
    pub time_low: SLuint32,
    /// Mid UUID field.
    pub time_mid: u16,
    /// Hi UUID field.
    pub time_hi_and_version: u16,
    /// Clock-sequence UUID field.
    pub clock_seq: u16,
    /// Node UUID field.
    pub node: [u8; 6],
}

/// One OpenSL ES object interface table.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct SLObjectItf_ {
    /// Realize method.
    pub Realize: Option<unsafe extern "C" fn(SLObjectItf, SLboolean) -> SLresult>,
    /// Resume method.
    pub Resume: Option<unsafe extern "C" fn(SLObjectItf, SLboolean) -> SLresult>,
    /// Get-state method.
    pub GetState: Option<unsafe extern "C" fn(SLObjectItf, *mut SLuint32) -> SLresult>,
    /// Get-interface method.
    pub GetInterface:
        Option<unsafe extern "C" fn(SLObjectItf, SLInterfaceID, *mut c_void) -> SLresult>,
    /// Register-callback method.
    pub RegisterCallback:
        Option<unsafe extern "C" fn(SLObjectItf, *mut c_void, *mut c_void) -> SLresult>,
    /// Abort-async method.
    pub AbortAsyncTask: Option<unsafe extern "C" fn(SLObjectItf)>,
    /// Destroy method.
    pub Destroy: Option<unsafe extern "C" fn(SLObjectItf)>,
}

/// One OpenSL ES engine interface table.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct SLEngineItf_ {
    /// Create LED device method.
    pub CreateLEDDevice: Option<unsafe extern "C" fn() -> SLresult>,
    /// Create vibra device method.
    pub CreateVibraDevice: Option<unsafe extern "C" fn() -> SLresult>,
    /// Create audio player method.
    pub CreateAudioPlayer: Option<
        unsafe extern "C" fn(
            SLEngineItf,
            *mut SLObjectItf,
            *mut SLDataSource,
            *mut SLDataSink,
            SLuint32,
            *const SLInterfaceID,
            *const SLboolean,
        ) -> SLresult,
    >,
    /// Create audio recorder method.
    pub CreateAudioRecorder: Option<
        unsafe extern "C" fn(
            SLEngineItf,
            *mut SLObjectItf,
            *mut SLDataSource,
            *mut SLDataSink,
            SLuint32,
            *const SLInterfaceID,
            *const SLboolean,
        ) -> SLresult,
    >,
    /// Create midi player method.
    pub CreateMidiPlayer: Option<unsafe extern "C" fn() -> SLresult>,
    /// Create listener method.
    pub CreateListener: Option<unsafe extern "C" fn() -> SLresult>,
    /// Create 3d group method.
    pub Create3DGroup: Option<unsafe extern "C" fn() -> SLresult>,
    /// Create output mix method.
    pub CreateOutputMix: Option<
        unsafe extern "C" fn(
            SLEngineItf,
            *mut SLObjectItf,
            SLuint32,
            *const SLInterfaceID,
            *const SLboolean,
        ) -> SLresult,
    >,
}

/// One OpenSL ES play interface table.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct SLPlayItf_ {
    /// Set-play-state method.
    pub SetPlayState: Option<unsafe extern "C" fn(SLPlayItf, SLuint32) -> SLresult>,
}

/// One OpenSL ES record interface table.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct SLRecordItf_ {
    /// Set-record-state method.
    pub SetRecordState: Option<unsafe extern "C" fn(SLRecordItf, SLuint32) -> SLresult>,
}

/// One OpenSL ES Android simple-buffer-queue state payload.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct SLAndroidSimpleBufferQueueState {
    /// Queue entry count.
    pub count: SLuint32,
    /// Queue ring index.
    pub index: SLuint32,
}

/// One OpenSL ES Android simple-buffer-queue interface table.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct SLAndroidSimpleBufferQueueItf_ {
    /// Enqueue method.
    pub Enqueue: Option<
        unsafe extern "C" fn(SLAndroidSimpleBufferQueueItf, *const c_void, SLuint32) -> SLresult,
    >,
    /// Clear method.
    pub Clear: Option<unsafe extern "C" fn(SLAndroidSimpleBufferQueueItf) -> SLresult>,
    /// Get-state method.
    pub GetState: Option<
        unsafe extern "C" fn(
            SLAndroidSimpleBufferQueueItf,
            *mut SLAndroidSimpleBufferQueueState,
        ) -> SLresult,
    >,
    /// Register-callback method.
    pub RegisterCallback: Option<
        unsafe extern "C" fn(
            SLAndroidSimpleBufferQueueItf,
            SlAndroidSimpleBufferQueueCallback,
            *mut c_void,
        ) -> SLresult,
    >,
}

/// One OpenSL ES data-source payload.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct SLDataSource {
    /// Locator pointer.
    pub pLocator: *mut c_void,
    /// Format pointer.
    pub pFormat: *mut c_void,
}

/// One OpenSL ES data-sink payload.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct SLDataSink {
    /// Locator pointer.
    pub pLocator: *mut c_void,
    /// Format pointer.
    pub pFormat: *mut c_void,
}

/// One OpenSL ES i/o device locator payload.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct SLDataLocator_IODevice {
    /// Locator tag.
    pub locatorType: SLuint32,
    /// Device-type tag.
    pub deviceType: SLuint32,
    /// Device identifier.
    pub deviceID: SLuint32,
    /// Optional object handle.
    pub device: SLObjectItf,
}

/// One OpenSL ES output-mix locator payload.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct SLDataLocator_OutputMix {
    /// Locator tag.
    pub locatorType: SLuint32,
    /// Output-mix object handle.
    pub outputMix: SLObjectItf,
}

/// One OpenSL ES Android simple-buffer-queue locator payload.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct SLDataLocator_AndroidSimpleBufferQueue {
    /// Locator tag.
    pub locatorType: SLuint32,
    /// Queue slot count.
    pub numBuffers: SLuint32,
}

/// One OpenSL ES PCM data-format payload.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct SLDataFormat_PCM {
    /// Format tag.
    pub formatType: SLuint32,
    /// Channel count.
    pub numChannels: SLuint32,
    /// Sample rate in milli-hertz.
    pub samplesPerSec: SLuint32,
    /// Bits per sample lane.
    pub bitsPerSample: SLuint32,
    /// Container size in bits.
    pub containerSize: SLuint32,
    /// Channel-mask selector.
    pub channelMask: SLuint32,
    /// Endianness selector.
    pub endianness: SLuint32,
}

/// One OpenSL ES engine-option payload.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct SLEngineOption {
    /// Option feature id.
    pub feature: SLuint32,
    /// Option feature payload.
    pub data: SLuint32,
}

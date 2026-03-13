use windows::Foundation::TypedEventHandler;
use windows::core::IInspectable;

use super::sdk::{
    IMidiMessageReceivedEventSource, MidiEndpointConnection,
    MidiEndpointDeviceInformationAddedEventArgs, MidiEndpointDeviceInformationRemovedEventArgs,
    MidiEndpointDeviceInformationUpdatedEventArgs, MidiEndpointDeviceWatcher,
    MidiMessageReceivedEventArgs,
};

#[repr(C)]
struct MidiEndpointDeviceWatcherVtblHack {
    base__: windows::core::IInspectable_Vtbl,
    start: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows::core::HRESULT,
    stop: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows::core::HRESULT,
    enumerated_endpoint_devices: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows::core::HRESULT,
    status: usize,
    added: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut i64,
    ) -> windows::core::HRESULT,
    remove_added: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows::core::HRESULT,
    removed: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut i64,
    ) -> windows::core::HRESULT,
    remove_removed:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows::core::HRESULT,
    updated: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut i64,
    ) -> windows::core::HRESULT,
    remove_updated:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows::core::HRESULT,
    enumeration_completed: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut i64,
    ) -> windows::core::HRESULT,
    remove_enumeration_completed:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows::core::HRESULT,
    stopped: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut i64,
    ) -> windows::core::HRESULT,
    remove_stopped:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows::core::HRESULT,
}

#[repr(C)]
struct MidiMessageReceivedEventSourceVtblHack {
    base__: windows::core::IInspectable_Vtbl,
    message_received: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut i64,
    ) -> windows::core::HRESULT,
    remove_message_received:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows::core::HRESULT,
    get_endpoint_connection_source: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows::core::HRESULT,
}

/// Register one added handler on one MIDI endpoint watcher.
pub(super) fn watcher_add_added<P0>(
    watcher: &MidiEndpointDeviceWatcher,
    handler: P0,
) -> windows::core::Result<i64>
where
    P0: windows::core::Param<
            TypedEventHandler<
                MidiEndpointDeviceWatcher,
                MidiEndpointDeviceInformationAddedEventArgs,
            >,
        >,
{
    unsafe {
        let mut token = core::mem::zeroed();
        let vtable = windows::core::Interface::vtable(watcher) as *const _
            as *const MidiEndpointDeviceWatcherVtblHack;

        ((*vtable).added)(
            windows::core::Interface::as_raw(watcher),
            handler.param().abi(),
            &mut token,
        )
        .map(|| token)
    }
}

/// Register one removed handler on one MIDI endpoint watcher.
pub(super) fn watcher_add_removed<P0>(
    watcher: &MidiEndpointDeviceWatcher,
    handler: P0,
) -> windows::core::Result<i64>
where
    P0: windows::core::Param<
            TypedEventHandler<
                MidiEndpointDeviceWatcher,
                MidiEndpointDeviceInformationRemovedEventArgs,
            >,
        >,
{
    unsafe {
        let mut token = core::mem::zeroed();
        let vtable = windows::core::Interface::vtable(watcher) as *const _
            as *const MidiEndpointDeviceWatcherVtblHack;

        ((*vtable).removed)(
            windows::core::Interface::as_raw(watcher),
            handler.param().abi(),
            &mut token,
        )
        .map(|| token)
    }
}

/// Register one updated handler on one MIDI endpoint watcher.
pub(super) fn watcher_add_updated<P0>(
    watcher: &MidiEndpointDeviceWatcher,
    handler: P0,
) -> windows::core::Result<i64>
where
    P0: windows::core::Param<
            TypedEventHandler<
                MidiEndpointDeviceWatcher,
                MidiEndpointDeviceInformationUpdatedEventArgs,
            >,
        >,
{
    unsafe {
        let mut token = core::mem::zeroed();
        let vtable = windows::core::Interface::vtable(watcher) as *const _
            as *const MidiEndpointDeviceWatcherVtblHack;

        ((*vtable).updated)(
            windows::core::Interface::as_raw(watcher),
            handler.param().abi(),
            &mut token,
        )
        .map(|| token)
    }
}

/// Register one enumeration-completed handler on one MIDI endpoint watcher.
pub(super) fn watcher_add_enumeration_completed<P0>(
    watcher: &MidiEndpointDeviceWatcher,
    handler: P0,
) -> windows::core::Result<i64>
where
    P0: windows::core::Param<TypedEventHandler<MidiEndpointDeviceWatcher, IInspectable>>,
{
    unsafe {
        let mut token = core::mem::zeroed();
        let vtable = windows::core::Interface::vtable(watcher) as *const _
            as *const MidiEndpointDeviceWatcherVtblHack;

        ((*vtable).enumeration_completed)(
            windows::core::Interface::as_raw(watcher),
            handler.param().abi(),
            &mut token,
        )
        .map(|| token)
    }
}

/// Register one stopped handler on one MIDI endpoint watcher.
pub(super) fn watcher_add_stopped<P0>(
    watcher: &MidiEndpointDeviceWatcher,
    handler: P0,
) -> windows::core::Result<i64>
where
    P0: windows::core::Param<TypedEventHandler<MidiEndpointDeviceWatcher, IInspectable>>,
{
    unsafe {
        let mut token = core::mem::zeroed();
        let vtable = windows::core::Interface::vtable(watcher) as *const _
            as *const MidiEndpointDeviceWatcherVtblHack;

        ((*vtable).stopped)(
            windows::core::Interface::as_raw(watcher),
            handler.param().abi(),
            &mut token,
        )
        .map(|| token)
    }
}

/// Register one message-received handler on one MIDI endpoint connection.
pub(super) fn connection_add_message_received<P0>(
    connection: &MidiEndpointConnection,
    handler: P0,
) -> windows::core::Result<i64>
where
    P0: windows::core::Param<
            TypedEventHandler<MidiEndpointConnection, MidiMessageReceivedEventArgs>,
        >,
{
    let source = &windows::core::Interface::cast::<IMidiMessageReceivedEventSource>(connection)?;

    unsafe {
        let mut token = core::mem::zeroed();
        let vtable = windows::core::Interface::vtable(source) as *const _
            as *const MidiMessageReceivedEventSourceVtblHack;

        ((*vtable).message_received)(
            windows::core::Interface::as_raw(source),
            handler.param().abi(),
            &mut token,
        )
        .map(|| token)
    }
}

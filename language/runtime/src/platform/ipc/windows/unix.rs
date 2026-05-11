use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::ipc::UnixReceiveAncillary;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Unix ancillary receive operation.
const UNIX_RECEIVE_OPERATION: &str = "destack.ipc.unix.receive";
/// Unix ancillary send operation.
const UNIX_SEND_OPERATION: &str = "destack.ipc.unix.send";

/// Receive payload and transferred handles.
pub(crate) unsafe fn destack_ipc_unix_receive(
    _binding: &BindingCallContext,
    out: *mut UnixReceiveAncillary,
    socket: resource::SocketHandle,
    maxhandles: u32,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    let _ = (socket, maxhandles);

    Err(core_platform::not_supported(UNIX_RECEIVE_OPERATION))
}

/// Send payload and transferred handles.
pub(crate) unsafe fn destack_ipc_unix_send(
    _binding: &BindingCallContext,
    out: *mut u64,
    socket: resource::SocketHandle,
    argument_payload: NativeSlice<u8>,
    handles: NativeSlice<resource::TransferredHandle>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    let _ = (socket, argument_payload, handles);

    Err(core_platform::not_supported(UNIX_SEND_OPERATION))
}

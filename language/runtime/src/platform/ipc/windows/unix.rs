use crate::diagnostic::RuntimeResult;
use crate::platform::ipc::UnixReceiveAncillary;
use crate::platform::{NativeSlice, resource};
use crate::runtime::BindingCallContext;

use super::core::{ensure_out, not_supported};

/// Unix ancillary receive operation.
const UNIX_RECEIVE_OPERATION: &str = "destack.ipc.unix.receive";
/// Unix ancillary send operation.
const UNIX_SEND_OPERATION: &str = "destack.ipc.unix.send";

/// Receive payload and transferred handles.
pub(crate) unsafe fn destack_ipc_unix_receive(
    _context: &BindingCallContext,
    out: *mut UnixReceiveAncillary,
    socket: resource::SocketHandle,
    maxhandles: u32,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    let _ = (socket, maxhandles);

    Err(not_supported(UNIX_RECEIVE_OPERATION))
}

/// Send payload and transferred handles.
pub(crate) unsafe fn destack_ipc_unix_send(
    _context: &BindingCallContext,
    out: *mut u64,
    socket: resource::SocketHandle,
    argument_payload: NativeSlice<u8>,
    handles: NativeSlice<resource::TransferredHandle>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    let _ = (socket, argument_payload, handles);

    Err(not_supported(UNIX_SEND_OPERATION))
}

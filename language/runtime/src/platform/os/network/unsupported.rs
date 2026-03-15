use crate::diagnostic::RuntimeResult;
use crate::platform::core::not_supported;
use crate::platform::os::NetworkState;
use crate::runtime::BindingCallContext;

/// Read one network state snapshot on unsupported hosts.
pub(crate) fn read_network_state(_binding: &BindingCallContext) -> RuntimeResult<NetworkState> {
    Err(not_supported("destack.os.network.state"))
}

use crate::platform::host::HostContext;

/// Return the process arguments from the host context.
pub fn process_args(host: &HostContext) -> &[String] {
    host.args()
}

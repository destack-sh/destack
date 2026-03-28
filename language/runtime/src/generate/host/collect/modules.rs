use crate::host::model::HostModule;

/// Collect the authored host modules.
pub(crate) fn collect_host_modules() -> Vec<HostModule> {
    destack_runtime::host::abi::host_abi_modules()
        .into_iter()
        .map(HostModule::new)
        .collect()
}

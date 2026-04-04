use super::{HostOperation, decode};

use crate::host::HostRequest;
use crate::platform::os::abi_generated::{
    BackgroundStatusValue, BackgroundTaskDescriptorValue, BackgroundTaskOptionsValue,
    BackgroundTaskResultValue,
};

/// Build one background status operation.
pub(crate) fn status() -> HostOperation<BackgroundStatusValue> {
    HostOperation::new(HostRequest::OsBackgroundStatus, decode::background_status)
}

/// Build one background task list operation.
pub(crate) fn list() -> HostOperation<Vec<BackgroundTaskDescriptorValue>> {
    HostOperation::new(
        HostRequest::OsBackgroundList,
        decode::background_task_descriptors,
    )
}

/// Build one background registration operation.
pub(crate) fn register(options: BackgroundTaskOptionsValue) -> HostOperation<()> {
    HostOperation::new(HostRequest::OsBackgroundRegister { options }, decode::none)
}

/// Build one background unregister operation.
pub(crate) fn unregister(identifier: String) -> HostOperation<()> {
    HostOperation::new(
        HostRequest::OsBackgroundUnregister { identifier },
        decode::none,
    )
}

/// Build one background trigger-test operation.
pub(crate) fn trigger_test(identifier: String) -> HostOperation<bool> {
    HostOperation::new(
        HostRequest::OsBackgroundTriggerTest { identifier },
        decode::bool_value,
    )
}

/// Build one background completion operation.
pub(crate) fn complete(
    execution_id: String,
    result: BackgroundTaskResultValue,
) -> HostOperation<()> {
    HostOperation::new(
        HostRequest::OsBackgroundComplete {
            execution_id,
            result,
        },
        decode::none,
    )
}

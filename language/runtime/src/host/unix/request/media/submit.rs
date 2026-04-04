use crate::diagnostic::RuntimeResult;
use crate::host::{HostRequest, HostRequestOutcome, HostRequestResult, RequestContext};
use crate::platform::os::media::{
    delete_media_assets, import_media_asset, list_media_assets, read_media_asset,
};

/// Submit one Unix media operation.
pub(crate) fn submit_media_request(
    context: &RequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsMediaList { query } => {
            let page = list_media_assets(context.platform, query)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::MediaPage(page),
            )))
        }
        HostRequest::OsMediaRead { id } => {
            let descriptor = read_media_asset(context.platform, id)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::MediaAssetDescriptor(descriptor),
            )))
        }
        HostRequest::OsMediaImportPath { path, kind } => {
            let imported_id = import_media_asset(context.platform, *kind, *path)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::String(imported_id),
            )))
        }
        HostRequest::OsMediaDelete { ids } => {
            let deleted_count = delete_media_assets(context.platform, ids)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::U32(
                deleted_count,
            ))))
        }
        _ => Ok(None),
    }
}

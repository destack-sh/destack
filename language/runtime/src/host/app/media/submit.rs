use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult};

use super::catalog::{describe_media_asset, list_media_assets};
use super::storage::{delete_media_assets, import_media_asset};
use super::watch::{close_media_watch, open_media_watch};

/// Submit one media operation when the current host provides one media-library bridge.
pub(crate) fn submit_media_request(
    context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsMediaList { query } => {
            let page = list_media_assets(context.platform, query)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::MediaPage(page),
            )))
        }
        HostRequest::OsMediaDescribe { id } => {
            let descriptor = describe_media_asset(context.platform, id)?;

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
        HostRequest::OsMediaWatchOpen { watch_id, options } => {
            open_media_watch(context, watch_id, options)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsMediaWatchClose { watch_id } => {
            close_media_watch(context.host_runtime_id, watch_id)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

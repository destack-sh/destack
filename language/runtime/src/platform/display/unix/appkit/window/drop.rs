use std::sync::Arc;

use objc2::ClassType;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSDragOperation, NSDraggingInfo, NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::{NSArray, NSPoint, NSURL};

use crate::platform::ResourceTable;
use crate::platform::display::core::{
    DisplayDragSession, DisplayDragSessionItem, DisplayDragSessionItemValue,
    display_drag_operation_mask, open_external_drag_session,
};
use crate::platform::display::unix::appkit::core::{self as appkit_core, AppKitWindowHost};
use crate::platform::display::unix::appkit::event;
use crate::platform::display::{
    DisplayDragItemDescriptorValue, DisplayDragItemKind, DisplayDragOperation,
    DisplayDragOperationMask, DisplayDragPosition,
};
use crate::platform::resource::{DisplayDragSessionHandle, ResourceKind, WindowHandle};

const APPKIT_DRAG_TEXT_ITEM_TYPE: &str = "text/plain";
const APPKIT_DRAG_FILE_URL_ITEM_TYPE: &str = "public.file-url";

/// Parsed drag payload for one AppKit pasteboard snapshot.
enum AppKitDropPayload {
    /// File-path payloads decoded from the drag pasteboard.
    Files(Vec<String>),
    /// Text payload decoded from the drag pasteboard.
    Text(String),
}

/// Convert one AppKit point into one runtime drag position.
fn position_from_point(point: NSPoint) -> DisplayDragPosition {
    DisplayDragPosition {
        x: point.x.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32,
        y: point.y.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32,
    }
}

/// Resolve one preferred drag operation from one source operation mask.
fn preferred_drag_operation(mask: NSDragOperation) -> Option<DisplayDragOperation> {
    // prefer copy over move over link
    if mask.contains(NSDragOperation::Copy) {
        return Some(DisplayDragOperation::Copy);
    }

    // prefer move operations when the source offers them
    if mask.contains(NSDragOperation::Move) {
        return Some(DisplayDragOperation::Move);
    }

    // otherwise prefer link operations
    if mask.contains(NSDragOperation::Link) {
        return Some(DisplayDragOperation::Link);
    }

    None
}

/// Convert one runtime drag operation into one AppKit drag operation.
fn appkit_drag_operation(operation: DisplayDragOperation) -> NSDragOperation {
    match operation {
        DisplayDragOperation::None => NSDragOperation::None,
        DisplayDragOperation::Copy => NSDragOperation::Copy,
        DisplayDragOperation::Move => NSDragOperation::Move,
        DisplayDragOperation::Link => NSDragOperation::Link,
    }
}

/// Decode one file-path payload list from one drag pasteboard.
fn file_paths_from_pasteboard(pasteboard: &NSPasteboard) -> Option<Vec<String>> {
    let classes = NSArray::from_slice(&[NSURL::class()]);
    let property_list = unsafe { pasteboard.readObjectsForClasses_options(&classes, None) }?;
    let property_list = property_list.downcast::<NSArray>().ok()?;
    let mut paths = Vec::new();

    // decode each file URL from the pasteboard object array
    for value in property_list.iter() {
        let path = value.downcast_ref::<NSURL>()?;
        if !path.isFileURL() {
            return None;
        }
        let path = path.standardizedURL()?;
        let path = path.path()?;
        paths.push(path.to_string());
    }
    if paths.is_empty() {
        return None;
    }

    Some(paths)
}

/// Decode one text payload from one drag pasteboard.
fn text_from_pasteboard(pasteboard: &NSPasteboard) -> Option<String> {
    pasteboard
        .stringForType(unsafe { NSPasteboardTypeString })
        .map(|value| value.to_string())
}

/// Decode the highest-priority payload from one drag pasteboard.
fn drop_payload_from_pasteboard(pasteboard: &NSPasteboard) -> Option<AppKitDropPayload> {
    // prefer file payloads over text payloads
    if let Some(paths) = file_paths_from_pasteboard(pasteboard) {
        return Some(AppKitDropPayload::Files(paths));
    }

    text_from_pasteboard(pasteboard).map(AppKitDropPayload::Text)
}

/// Resolve one drag payload and current position from one dragging info object.
fn drag_payload_from_info(
    sender: &ProtocolObject<dyn NSDraggingInfo>,
) -> Option<(Vec<DisplayDragSessionItem>, DisplayDragPosition)> {
    let pasteboard = sender.draggingPasteboard();
    let payload = drop_payload_from_pasteboard(&pasteboard)?;
    let position = position_from_point(sender.draggingLocation());
    let items = drag_items_from_payload(payload);

    Some((items, position))
}

/// Convert one parsed AppKit payload into canonical drag-session items.
fn drag_items_from_payload(payload: AppKitDropPayload) -> Vec<DisplayDragSessionItem> {
    match payload {
        AppKitDropPayload::Files(paths) => paths.into_iter().map(appkit_path_drag_item).collect(),
        AppKitDropPayload::Text(text) => vec![appkit_text_drag_item(text)],
    }
}

/// Build one text drag item from one AppKit pasteboard payload.
fn appkit_text_drag_item(text: String) -> DisplayDragSessionItem {
    let byte_length = u64::try_from(text.len()).ok();

    DisplayDragSessionItem {
        descriptor: DisplayDragItemDescriptorValue {
            kind: DisplayDragItemKind::String,
            item_type: Some(APPKIT_DRAG_TEXT_ITEM_TYPE.to_string()),
            name: None,
            is_directory: false,
            byte_length,
        },
        value: DisplayDragSessionItemValue::Text(text),
    }
}

/// Build one file drag item from one AppKit file-url payload.
fn appkit_path_drag_item(path: String) -> DisplayDragSessionItem {
    let metadata = std::fs::metadata(&path).ok();
    let is_directory = metadata.as_ref().is_some_and(|metadata| metadata.is_dir());
    let byte_length = metadata
        .as_ref()
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len());
    let name = std::path::Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string);

    DisplayDragSessionItem {
        descriptor: DisplayDragItemDescriptorValue {
            kind: DisplayDragItemKind::File,
            item_type: Some(APPKIT_DRAG_FILE_URL_ITEM_TYPE.to_string()),
            name,
            is_directory,
            byte_length,
        },
        value: DisplayDragSessionItemValue::Path(path),
    }
}

/// Ensure one active drag session exists and update it with the latest payload.
fn ensure_drag_session(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    host: &AppKitWindowHost,
    allowed_operation: DisplayDragOperation,
    position: DisplayDragPosition,
    items: Vec<DisplayDragSessionItem>,
) -> DisplayDragSessionHandle {
    // derive the canonical session payload from the current host snapshot
    let mut drop_session = host.drop_session.borrow_mut();
    let allowed_operations = display_drag_operation_mask(allowed_operation);
    let proposed_operation = Some(allowed_operation);

    // update the active session in place when one already exists
    if let Some(session) = drop_session.session
        && replace_drag_session(
            runtime_state.resource_table(),
            session,
            allowed_operations,
            proposed_operation,
            position,
            items.clone(),
        )
    {
        return session;
    }

    // otherwise open a new external drag session for this window
    let session = open_external_drag_session(
        runtime_state.resource_table(),
        allowed_operations,
        proposed_operation,
        Some(position),
        items,
    );

    drop_session.session = Some(session);
    session
}

/// Replace one active drag-session payload in place.
fn replace_drag_session(
    resource_table: &ResourceTable,
    session: DisplayDragSessionHandle,
    allowed_operations: DisplayDragOperationMask,
    proposed_operation: Option<DisplayDragOperation>,
    position: DisplayDragPosition,
    items: Vec<DisplayDragSessionItem>,
) -> bool {
    // update only display drag-session resources
    resource_table.with_entry_mut(session.0, |entry| {
        if entry.kind != ResourceKind::DisplayDragSession {
            return false;
        }

        let Some(payload) = entry.payload_mut::<DisplayDragSession>() else {
            return false;
        };

        payload.allowed_operations = allowed_operations;
        payload.proposed_operation = proposed_operation;
        payload.position = Some(position);
        payload.items = items;
        true
    }) == Some(true)
}

/// Take the current active drag session for one window.
fn take_drag_session(host: &AppKitWindowHost) -> Option<DisplayDragSessionHandle> {
    host.drop_session.borrow_mut().session.take()
}

/// Handle one `draggingEntered:` callback.
pub(crate) fn handle_dragging_entered(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: WindowHandle,
    sender: &ProtocolObject<dyn NSDraggingInfo>,
) -> NSDragOperation {
    let Some(allowed_operation) = preferred_drag_operation(sender.draggingSourceOperationMask())
    else {
        return NSDragOperation::None;
    };
    let Some((items, position)) = drag_payload_from_info(sender) else {
        return NSDragOperation::None;
    };

    let publish_result = appkit_core::with_window_host(
        runtime_state,
        window_handle,
        "destack.display.window.draggingEntered",
        |host| {
            let session =
                ensure_drag_session(runtime_state, host, allowed_operation, position, items);
            event::publish_window_drag_entered(runtime_state, window_handle, session);
            Ok(())
        },
    );

    // log callback failures without aborting the host callback
    if let Err(error) = publish_result {
        appkit_core::warn_callback_error(
            runtime_state.as_ref(),
            "destack.display.window.draggingEntered",
            error.as_ref(),
        );
        return NSDragOperation::None;
    }

    appkit_drag_operation(allowed_operation)
}

/// Handle one `draggingUpdated:` callback.
pub(crate) fn handle_dragging_updated(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: WindowHandle,
    sender: &ProtocolObject<dyn NSDraggingInfo>,
) -> NSDragOperation {
    let Some(allowed_operation) = preferred_drag_operation(sender.draggingSourceOperationMask())
    else {
        return NSDragOperation::None;
    };
    let Some((items, position)) = drag_payload_from_info(sender) else {
        return NSDragOperation::None;
    };

    let publish_result = appkit_core::with_window_host(
        runtime_state,
        window_handle,
        "destack.display.window.draggingUpdated",
        |host| {
            let session =
                ensure_drag_session(runtime_state, host, allowed_operation, position, items);
            event::publish_window_drag_updated(runtime_state, window_handle, session);
            Ok(())
        },
    );

    // log callback failures without aborting the host callback
    if let Err(error) = publish_result {
        appkit_core::warn_callback_error(
            runtime_state.as_ref(),
            "destack.display.window.draggingUpdated",
            error.as_ref(),
        );
        return NSDragOperation::None;
    }

    appkit_drag_operation(allowed_operation)
}

/// Handle one `draggingExited:` callback.
pub(crate) fn handle_dragging_exited(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: WindowHandle,
) {
    if let Err(error) = appkit_core::with_window_host(
        runtime_state,
        window_handle,
        "destack.display.window.draggingExited",
        |host| {
            if let Some(session) = take_drag_session(host) {
                event::publish_window_drag_exited(runtime_state, window_handle, session);
            }
            Ok(())
        },
    ) {
        appkit_core::warn_callback_error(
            runtime_state.as_ref(),
            "destack.display.window.draggingExited",
            error.as_ref(),
        );
    }
}

/// Handle one `prepareForDragOperation:` callback.
pub(crate) fn handle_prepare_for_drag_operation(
    sender: &ProtocolObject<dyn NSDraggingInfo>,
) -> bool {
    drag_payload_from_info(sender).is_some()
}

/// Handle one `performDragOperation:` callback.
pub(crate) fn handle_perform_drag_operation(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: WindowHandle,
    sender: &ProtocolObject<dyn NSDraggingInfo>,
) -> bool {
    let Some(allowed_operation) = preferred_drag_operation(sender.draggingSourceOperationMask())
    else {
        handle_dragging_exited(runtime_state, window_handle);
        return false;
    };
    let Some((items, position)) = drag_payload_from_info(sender) else {
        handle_dragging_exited(runtime_state, window_handle);
        return false;
    };

    let publish_result = appkit_core::with_window_host(
        runtime_state,
        window_handle,
        "destack.display.window.performDragOperation",
        |host| {
            let session =
                ensure_drag_session(runtime_state, host, allowed_operation, position, items);
            event::publish_window_dropped(runtime_state, window_handle, session);
            let _ = take_drag_session(host);
            Ok(())
        },
    );

    if let Err(error) = publish_result {
        appkit_core::warn_callback_error(
            runtime_state.as_ref(),
            "destack.display.window.performDragOperation",
            error.as_ref(),
        );
        return false;
    }

    true
}

/// Clear one active drag session while closing one host window.
pub(crate) fn cancel_active_drop_session(
    _runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    _window_handle: WindowHandle,
    host: &AppKitWindowHost,
) {
    let _ = take_drag_session(host);
}

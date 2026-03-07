use std::sync::Arc;

use objc2::ClassType;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSDragOperation, NSDraggingInfo, NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::{NSArray, NSPoint, NSURL};

use crate::platform::display::WindowPosition;
use crate::platform::resource::WindowHandle;

use crate::platform::display::unix::appkit::core::{self as appkit_core, AppKitWindowHost};
use crate::platform::display::unix::appkit::event;

/// Parsed drag payload for one AppKit pasteboard snapshot.
enum AppKitDropPayload {
    /// File-path payloads decoded from the drag pasteboard.
    Files(Vec<String>),
    /// Text payload decoded from the drag pasteboard.
    Text(String),
}

/// Convert one AppKit point into one runtime window position.
fn position_from_point(point: NSPoint) -> WindowPosition {
    WindowPosition {
        x: point.x.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32,
        y: point.y.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32,
    }
}

/// Resolve one effective drag operation from one source operation mask.
fn accepted_drag_operation(mask: NSDragOperation) -> NSDragOperation {
    // prefer copy over move over link
    if mask.contains(NSDragOperation::Copy) {
        return NSDragOperation::Copy;
    }

    // prefer move operations when the source offers them
    if mask.contains(NSDragOperation::Move) {
        return NSDragOperation::Move;
    }

    // otherwise prefer link operations
    if mask.contains(NSDragOperation::Link) {
        return NSDragOperation::Link;
    }

    NSDragOperation::None
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
fn drop_payload_from_info(
    sender: &ProtocolObject<dyn NSDraggingInfo>,
) -> Option<(AppKitDropPayload, WindowPosition)> {
    let pasteboard = sender.draggingPasteboard();
    let payload = drop_payload_from_pasteboard(&pasteboard)?;
    let position = position_from_point(sender.draggingLocation());
    Some((payload, position))
}

/// Publish one hover transition and update one active drop session.
fn publish_hover_transition(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: WindowHandle,
    host: &AppKitWindowHost,
    path: Option<String>,
    position: WindowPosition,
) {
    let mut session = host.drop_session.borrow_mut();

    // publish the session start once for the first accepted drag payload
    if !session.started {
        session.started = true;
        session.completed = false;
        event::publish_window_drop_started(runtime_state, window_handle);
    }

    // publish one hover-leave transition before changing the hovered file path
    if session.last_hovered_path != path && session.last_hovered_path.is_some() {
        event::publish_window_file_hover_left(
            runtime_state,
            window_handle,
            session.last_hovered_path.clone(),
            Some(position),
        );
    }

    session.last_hovered_path = path.clone();
    session.position = Some(position);

    event::publish_window_file_hovered(runtime_state, window_handle, path, Some(position));
}

/// Publish one drop completion or cancellation event and clear session state.
fn finalize_drop_session(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: WindowHandle,
    host: &AppKitWindowHost,
    accepted: bool,
) {
    let mut session = host.drop_session.borrow_mut();

    // skip when this window has no active drop session
    if !session.started {
        return;
    }

    // publish one final hover-leave event before closing the session
    if let Some(previous_path) = session.last_hovered_path.take() {
        event::publish_window_file_hover_left(
            runtime_state,
            window_handle,
            Some(previous_path),
            session.position,
        );
    }

    if accepted {
        event::publish_window_drop_completed(runtime_state, window_handle);
    } else {
        event::publish_window_drop_cancelled(runtime_state, window_handle);
    }

    session.started = false;
    session.completed = accepted;
    session.position = None;
}

/// Handle one `draggingEntered:` callback.
pub(crate) fn handle_dragging_entered(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: WindowHandle,
    sender: &ProtocolObject<dyn NSDraggingInfo>,
) -> NSDragOperation {
    let accepted_operation = accepted_drag_operation(sender.draggingSourceOperationMask());
    // reject drags that do not advertise a supported operation
    if accepted_operation == NSDragOperation::None {
        return NSDragOperation::None;
    }

    let Some((payload, position)) = drop_payload_from_info(sender) else {
        return NSDragOperation::None;
    };
    let hover_path = match payload {
        AppKitDropPayload::Files(paths) => paths.first().cloned(),
        AppKitDropPayload::Text(_) => None,
    };

    let publish_result = appkit_core::with_window_host(
        runtime_state,
        window_handle,
        "destack.display.window.draggingEntered",
        |host| {
            publish_hover_transition(runtime_state, window_handle, host, hover_path, position);
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

    accepted_operation
}

/// Handle one `draggingUpdated:` callback.
pub(crate) fn handle_dragging_updated(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: WindowHandle,
    sender: &ProtocolObject<dyn NSDraggingInfo>,
) -> NSDragOperation {
    let accepted_operation = accepted_drag_operation(sender.draggingSourceOperationMask());
    // reject drags that do not advertise a supported operation
    if accepted_operation == NSDragOperation::None {
        return NSDragOperation::None;
    }

    let Some((payload, position)) = drop_payload_from_info(sender) else {
        return NSDragOperation::None;
    };
    let hover_path = match payload {
        AppKitDropPayload::Files(paths) => paths.first().cloned(),
        AppKitDropPayload::Text(_) => None,
    };

    let publish_result = appkit_core::with_window_host(
        runtime_state,
        window_handle,
        "destack.display.window.draggingUpdated",
        |host| {
            publish_hover_transition(runtime_state, window_handle, host, hover_path, position);
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

    accepted_operation
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
            finalize_drop_session(runtime_state, window_handle, host, false);
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
    drop_payload_from_info(sender).is_some()
}

/// Handle one `performDragOperation:` callback.
pub(crate) fn handle_perform_drag_operation(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: WindowHandle,
    sender: &ProtocolObject<dyn NSDraggingInfo>,
) -> bool {
    let Some((payload, position)) = drop_payload_from_info(sender) else {
        handle_dragging_exited(runtime_state, window_handle);
        return false;
    };

    let publish_result = appkit_core::with_window_host(
        runtime_state,
        window_handle,
        "destack.display.window.performDragOperation",
        |host| {
            match payload {
                AppKitDropPayload::Files(paths) => {
                    for path in paths {
                        event::publish_window_file_dropped(
                            runtime_state,
                            window_handle,
                            Some(path),
                            Some(position),
                        );
                    }
                }
                AppKitDropPayload::Text(text) => {
                    event::publish_window_text_dropped(
                        runtime_state,
                        window_handle,
                        text,
                        Some(position),
                    );
                }
            }

            finalize_drop_session(runtime_state, window_handle, host, true);
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

/// Cancel one active drop session while closing one host window.
pub(crate) fn cancel_active_drop_session(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window_handle: WindowHandle,
    host: &AppKitWindowHost,
) {
    finalize_drop_session(runtime_state, window_handle, host, false);
}

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::{
    DISPLAY_DRAG_OPERATION_COPY, DISPLAY_DRAG_OPERATION_LINK, DISPLAY_DRAG_OPERATION_MOVE,
    DISPLAY_DRAG_OPERATION_NONE, DisplayDragItemDescriptor, DisplayDragItemDescriptorValue,
    DisplayDragOperation, DisplayDragOperationMask, DisplayDragPosition, DisplayDragTransfer,
};
use crate::platform::fs::{OsPath, core as fs_core};
use crate::platform::resource::resolve::resolve_payload;
use crate::platform::resource::{DisplayDragSessionHandle, ResourceEntry, ResourceKind};
use crate::platform::{NativeAbiCodec, PlatformError, ResourceTable};
use crate::runtime::BindingCallContext;

const DISPLAY_DRAG_SESSION_READ_BYTES_OPERATION: &str = "destack.display.drag.sessionReadBytes";
const DISPLAY_DRAG_SESSION_READ_PATH_OPERATION: &str = "destack.display.drag.sessionReadPath";
const DISPLAY_DRAG_SESSION_READ_TEXT_OPERATION: &str = "destack.display.drag.sessionReadText";

/// Stored payload for one drag-transfer item.
#[derive(Debug, Clone)]
pub(crate) enum DisplayDragSessionItemValue {
    /// One string item payload.
    Text(String),
    /// One file-path item payload.
    Path(String),
    /// One binary item payload.
    #[allow(dead_code)]
    Bytes(Vec<u8>),
}

/// Stored item record for one drag session.
#[derive(Debug, Clone)]
pub(crate) struct DisplayDragSessionItem {
    /// Public item descriptor.
    pub(crate) descriptor: DisplayDragItemDescriptorValue,
    /// Backing item payload.
    pub(crate) value: DisplayDragSessionItemValue,
}

/// Stored mutable payload for one drag session resource.
#[derive(Debug, Clone)]
pub(crate) struct DisplayDragSession {
    /// Whether the drag session originated outside the current runtime.
    pub(crate) is_external: bool,
    /// Operations offered by the drag source.
    pub(crate) allowed_operations: DisplayDragOperationMask,
    /// Host-proposed operation for the current gesture state.
    pub(crate) proposed_operation: Option<DisplayDragOperation>,
    /// Current drag position when available.
    pub(crate) position: Option<DisplayDragPosition>,
    /// Offered items in source order.
    pub(crate) items: Vec<DisplayDragSessionItem>,
}

/// Build one display drag-operation mask from one concrete operation.
pub(crate) fn display_drag_operation_mask(
    operation: DisplayDragOperation,
) -> DisplayDragOperationMask {
    match operation {
        DisplayDragOperation::None => DISPLAY_DRAG_OPERATION_NONE,
        DisplayDragOperation::Copy => DISPLAY_DRAG_OPERATION_COPY,
        DisplayDragOperation::Move => DISPLAY_DRAG_OPERATION_MOVE,
        DisplayDragOperation::Link => DISPLAY_DRAG_OPERATION_LINK,
    }
}

/// Open one external drag session resource.
pub(crate) fn open_external_drag_session(
    resource_table: &ResourceTable,
    allowed_operations: DisplayDragOperationMask,
    proposed_operation: Option<DisplayDragOperation>,
    position: Option<DisplayDragPosition>,
    items: Vec<DisplayDragSessionItem>,
) -> DisplayDragSessionHandle {
    // store the external drag snapshot as one display resource
    let entry = ResourceEntry::new(ResourceKind::DisplayDragSession)
        .with_label(ResourceKind::DisplayDragSession.label())
        .with_payload(DisplayDragSession {
            is_external: true,
            allowed_operations,
            proposed_operation,
            position,
            items,
        });
    let resource_id = resource_table.insert_untracked(entry);

    DisplayDragSessionHandle(resource_id)
}

/// Close one drag session resource.
pub(crate) fn close_drag_session(
    binding: &BindingCallContext,
    session: DisplayDragSessionHandle,
) -> RuntimeResult<()> {
    // remove the resource and validate the handle kind
    let removed =
        binding
            .worker()
            .resources
            .remove(&binding.world(), session.0, Some(binding.engine()));

    if let Some(entry) = removed
        && entry.kind == ResourceKind::DisplayDragSession
    {
        return Ok(());
    }

    Err(RuntimeError::from(PlatformError::invalid_argument(
        "unknown display drag session handle",
    ))
    .boxed())
}

/// Set the preferred operation for one drag session.
pub(crate) fn set_drag_session_operation(
    binding: &BindingCallContext,
    session: DisplayDragSessionHandle,
    operation: DisplayDragOperation,
) -> RuntimeResult<()> {
    // update the current proposed operation in place
    let updated = binding
        .worker()
        .resources
        .with_entry_mut(session.0, |entry| {
            if entry.kind != ResourceKind::DisplayDragSession {
                return false;
            }

            let Some(payload) = entry.payload_mut::<DisplayDragSession>() else {
                return false;
            };

            payload.proposed_operation = Some(operation);
            true
        });

    if updated == Some(true) {
        return Ok(());
    }

    Err(RuntimeError::from(PlatformError::invalid_argument(
        "unknown display drag session handle",
    ))
    .boxed())
}

/// Read one drag-transfer snapshot from one session handle.
pub(crate) fn drag_transfer(
    binding: &BindingCallContext,
    session: DisplayDragSessionHandle,
) -> RuntimeResult<DisplayDragTransfer> {
    // resolve the stored drag-session payload
    let payload = resolve_payload::<DisplayDragSession>(
        binding,
        session.0,
        ResourceKind::DisplayDragSession,
        Some(ResourceKind::DisplayDragSession.label()),
        DISPLAY_DRAG_SESSION_READ_TEXT_OPERATION,
    )?
    .ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument(
            "unknown display drag session handle",
        ))
        .boxed()
    })?;

    // expose descriptors through the public ABI
    let items = payload
        .items
        .iter()
        .map(|item| DisplayDragItemDescriptor::from_value(binding, item.descriptor.clone()))
        .collect();

    Ok(DisplayDragTransfer {
        session,
        is_external: payload.is_external,
        allowed_operations: payload.allowed_operations,
        proposed_operation: payload.proposed_operation,
        position: payload.position,
        items: binding.store_slice(items),
    })
}

/// Read one byte payload from one drag session item.
pub(crate) fn read_drag_session_item_bytes(
    binding: &BindingCallContext,
    session: DisplayDragSessionHandle,
    item_index: u32,
) -> RuntimeResult<Vec<u8>> {
    let item = session_item(
        binding,
        session,
        item_index,
        DISPLAY_DRAG_SESSION_READ_BYTES_OPERATION,
    )?;

    match &item.value {
        DisplayDragSessionItemValue::Bytes(bytes) => Ok(bytes.clone()),
        DisplayDragSessionItemValue::Text(_) | DisplayDragSessionItemValue::Path(_) => {
            Err(RuntimeError::from(PlatformError::invalid_argument(
                "drag session item is not binary",
            ))
            .boxed())
        }
    }
}

/// Read one filesystem payload from one drag session item.
pub(crate) fn read_drag_session_item_path(
    binding: &BindingCallContext,
    session: DisplayDragSessionHandle,
    item_index: u32,
) -> RuntimeResult<OsPath> {
    let item = session_item(
        binding,
        session,
        item_index,
        DISPLAY_DRAG_SESSION_READ_PATH_OPERATION,
    )?;

    match &item.value {
        DisplayDragSessionItemValue::Path(path) => {
            Ok(fs_core::os_path_from_utf8_string(binding, path.clone()))
        }
        DisplayDragSessionItemValue::Text(_) | DisplayDragSessionItemValue::Bytes(_) => {
            Err(RuntimeError::from(PlatformError::invalid_argument(
                "drag session item is not a path",
            ))
            .boxed())
        }
    }
}

/// Read one text payload from one drag session item.
pub(crate) fn read_drag_session_item_text(
    binding: &BindingCallContext,
    session: DisplayDragSessionHandle,
    item_index: u32,
) -> RuntimeResult<String> {
    let item = session_item(
        binding,
        session,
        item_index,
        DISPLAY_DRAG_SESSION_READ_TEXT_OPERATION,
    )?;

    match &item.value {
        DisplayDragSessionItemValue::Text(text) => Ok(text.clone()),
        DisplayDragSessionItemValue::Path(_) | DisplayDragSessionItemValue::Bytes(_) => {
            Err(RuntimeError::from(PlatformError::invalid_argument(
                "drag session item is not text",
            ))
            .boxed())
        }
    }
}

/// Read one drag-session item descriptor and payload.
fn session_item(
    binding: &BindingCallContext,
    session: DisplayDragSessionHandle,
    item_index: u32,
    operation: &'static str,
) -> RuntimeResult<DisplayDragSessionItem> {
    // resolve the stored drag-session payload
    let payload = resolve_payload::<DisplayDragSession>(
        binding,
        session.0,
        ResourceKind::DisplayDragSession,
        Some(ResourceKind::DisplayDragSession.label()),
        operation,
    )?
    .ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument(
            "unknown display drag session handle",
        ))
        .boxed()
    })?;

    // read the requested item by stable source order
    let Some(item) = payload.items.get(item_index as usize) else {
        return Err(RuntimeError::from(PlatformError::invalid_argument(
            "drag session item index out of range",
        ))
        .boxed());
    };

    Ok(item.clone())
}

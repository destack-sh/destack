use std::collections::HashMap;

use destack_mir as mir;

use crate::module::{ElementAccess, FieldAccess, Layout, TypedAccess, ValueKind, repr_type};

/// Build one field access from one compiled layout.
pub(super) fn field_access_for_pointee(
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    pointee_type: mir::LocalNodeId<mir::Type>,
    index: u32,
) -> Option<FieldAccess> {
    // resolve the pointee field layout first
    let layout = layouts.get(&pointee_type)?;
    let field = layout.field(index)?;

    // cache whether the loaded field can stay scalar in lowered memory ops
    let is_scalar = layout_is_scalar(layouts, field.ty);

    Some(FieldAccess {
        value_type: field.ty,
        byte_offset: field.offset,
        byte_len: field.byte_len,
        is_scalar,
    })
}

/// Build one element access from one compiled layout.
pub(super) fn element_access_for_pointee(
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    pointee_type: mir::LocalNodeId<mir::Type>,
) -> Option<ElementAccess> {
    // resolve the pointee element layout first
    let layout = layouts.get(&pointee_type)?;
    let element = layout.element()?;

    // cache whether the loaded element can stay scalar in lowered memory ops
    let is_scalar = layout_is_scalar(layouts, element.ty);

    Some(ElementAccess {
        value_type: element.ty,
        byte_stride: element.stride,
        byte_len: element.byte_len,
        is_scalar,
    })
}

/// Build one typed pointee access from one compiled layout.
pub(super) fn typed_access_for_pointee(
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    pointee_type: mir::LocalNodeId<mir::Type>,
) -> Option<TypedAccess> {
    // resolve the pointee layout directly
    let layout = layouts.get(&pointee_type)?;
    let is_scalar = layout.is_scalar();

    Some(TypedAccess {
        value_type: pointee_type,
        byte_len: layout.byte_len,
        is_scalar,
    })
}

/// Build one tensor element access from one compiled element layout.
pub(super) fn tensor_element_access(
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    element_type: mir::LocalNodeId<mir::Type>,
) -> Option<ElementAccess> {
    // resolve the lowered tensor element layout directly
    let layout = layouts.get(&element_type)?;
    let is_scalar = layout.is_scalar();

    Some(ElementAccess {
        value_type: element_type,
        byte_stride: layout.stride(),
        byte_len: layout.byte_len,
        is_scalar,
    })
}

/// Report whether one compiled layout stays scalar in lowered memory ops.
fn layout_is_scalar(
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ty: mir::LocalNodeId<mir::Type>,
) -> bool {
    layouts.get(&ty).is_some_and(Layout::is_scalar)
}

/// Resolve the tensor element type for one tensor value or reference type.
pub(super) fn tensor_element_type_for_view_type(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Option<mir::LocalNodeId<mir::Type>> {
    let ty = repr_type(tree, ty);

    match tree.get(ty) {
        mir::Type::Tensor { element, .. } | mir::Type::TensorView { element, .. } => element.ty(),
        _ => None,
    }
}

/// Resolve the field count for a struct or tuple kind.
pub(super) fn field_count_from_kind(tree: &mir::NodeTree, kind: ValueKind) -> Option<u32> {
    match kind {
        ValueKind::Composite { ty } => match tree.get(ty) {
            mir::Type::Struct { fields, copy: _ } => u32::try_from(fields.len()).ok(),
            mir::Type::Tuple { elements, copy: _ } => u32::try_from(elements.len()).ok(),
            _ => None,
        },
        ValueKind::Pointer { pointee, .. } => match tree.get(pointee) {
            mir::Type::Struct { fields, copy: _ } => u32::try_from(fields.len()).ok(),
            mir::Type::Tuple { elements, copy: _ } => u32::try_from(elements.len()).ok(),
            _ => None,
        },
        _ => None,
    }
}

/// Resolve the element length for an array kind.
pub(super) fn array_length_from_kind(tree: &mir::NodeTree, kind: ValueKind) -> Option<u64> {
    match kind {
        ValueKind::Array { length, .. } => Some(length),
        ValueKind::Composite { ty } => match tree.get(ty) {
            mir::Type::Array { length, .. } => Some(*length),
            _ => None,
        },
        ValueKind::Pointer { pointee, .. } => match tree.get(pointee) {
            mir::Type::Array { length, .. } => Some(*length),
            _ => None,
        },
        _ => None,
    }
}

use std::collections::HashMap;

use destack_mir as mir;

use crate::program::{
    ElementAccess, FieldAccess, Layout, PointeeAccess, PointerClass, ValueRepr,
    pointer_class_from_reference, repr_type,
};

const VIRTUAL_TABLE_FIELD: u32 = 0;
const INTERFACE_TABLE_FIELD: u32 = 1;

/// Build one field access from one compiled layout.
pub(super) fn field_access_for_pointee(
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    pointee_type: mir::LocalNodeId<mir::Type>,
    pointer_class: PointerClass,
    index: u32,
) -> Option<FieldAccess> {
    // resolve the pointee field layout first
    let layout = layouts.get(&pointee_type)?;
    let field = layout.field(index)?;

    // cache whether the loaded field can stay scalar in lowered memory ops
    let is_scalar = layout_is_scalar(layouts, field.ty);

    Some(FieldAccess {
        pointer_class,
        value_type: field.ty,
        byte_offset: field.offset,
        byte_len: field.byte_len,
        is_scalar,
    })
}

/// Build the vtable field for one virtual receiver.
pub(super) fn virtual_table_field_for_receiver(
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    receiver_type: Option<mir::LocalNodeId<mir::Type>>,
    pointer_class: PointerClass,
) -> Option<FieldAccess> {
    let receiver_type = receiver_type?;

    field_access_for_pointee(layouts, receiver_type, pointer_class, VIRTUAL_TABLE_FIELD)
}

/// Build the itab field for one interface receiver.
pub(super) fn interface_table_field_for_receiver(
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    receiver_type: Option<mir::LocalNodeId<mir::Type>>,
    pointer_class: PointerClass,
) -> Option<FieldAccess> {
    let receiver_type = receiver_type?;

    field_access_for_pointee(layouts, receiver_type, pointer_class, INTERFACE_TABLE_FIELD)
}

/// Build one element access from one compiled layout.
pub(super) fn element_access_for_pointee(
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    pointee_type: mir::LocalNodeId<mir::Type>,
    pointer_class: PointerClass,
) -> Option<ElementAccess> {
    // resolve the pointee element layout first
    let layout = layouts.get(&pointee_type)?;
    let element = layout.element()?;

    // cache whether the loaded element can stay scalar in lowered memory ops
    let is_scalar = layout_is_scalar(layouts, element.ty);

    Some(ElementAccess {
        pointer_class,
        value_type: element.ty,
        byte_stride: element.stride,
        byte_len: element.byte_len,
        is_scalar,
    })
}

/// Build one pointee access from one compiled layout.
pub(super) fn pointee_access_for_type(
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    pointee_type: mir::LocalNodeId<mir::Type>,
    pointer_class: PointerClass,
) -> Option<PointeeAccess> {
    // resolve the pointee layout directly
    let layout = layouts.get(&pointee_type)?;
    let is_scalar = layout.is_scalar();

    Some(PointeeAccess {
        pointer_class,
        value_type: pointee_type,
        byte_offset: 0,
        byte_len: layout.byte_len,
        is_scalar,
    })
}

/// Build one tensor element access from one compiled element layout.
pub(super) fn tensor_element_access(
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    element_type: mir::LocalNodeId<mir::Type>,
    pointer_class: PointerClass,
) -> Option<ElementAccess> {
    // resolve the lowered tensor element layout directly
    let layout = layouts.get(&element_type)?;
    let is_scalar = layout.is_scalar();

    Some(ElementAccess {
        pointer_class,
        value_type: element_type,
        byte_stride: layout.stride(),
        byte_len: layout.byte_len,
        is_scalar,
    })
}

/// Resolve the pointer class for one tensor value type.
pub(super) fn tensor_view_pointer_class(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Option<PointerClass> {
    match tree.get(repr_type(tree, ty)) {
        mir::Type::Tensor { .. } => Some(PointerClass::Heap),
        mir::Type::TensorView {
            kind,
            address_space,
            ..
        } => Some(pointer_class_from_reference(address_space.clone(), *kind)),
        _ => None,
    }
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
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Option<mir::LocalNodeId<mir::Type>> {
    let ty = repr_type(tree, ty);

    match tree.get(ty) {
        mir::Type::Tensor { element, .. } | mir::Type::TensorView { element, .. } => element.ty(),
        _ => None,
    }
}

/// Resolve the field count for a struct or tuple representation.
pub(super) fn field_count_from_repr(tree: &mir::Tree, repr: ValueRepr) -> Option<u32> {
    match repr {
        ValueRepr::FrameBytes { ty } => match tree.get(ty) {
            mir::Type::Struct { fields, copy: _ } => u32::try_from(fields.len()).ok(),
            mir::Type::Tuple { elements, copy: _ } => u32::try_from(elements.len()).ok(),
            _ => None,
        },
        ValueRepr::Pointer { pointee, .. } => match tree.get(pointee) {
            mir::Type::Struct { fields, copy: _ } => u32::try_from(fields.len()).ok(),
            mir::Type::Tuple { elements, copy: _ } => u32::try_from(elements.len()).ok(),
            _ => None,
        },
        _ => None,
    }
}

/// Resolve the element length for an array representation.
pub(super) fn array_length_from_repr(tree: &mir::Tree, repr: ValueRepr) -> Option<u64> {
    match repr {
        ValueRepr::Array { length, .. } => Some(length),
        ValueRepr::FrameBytes { ty } => match tree.get(ty) {
            mir::Type::Array { length, .. } => Some(*length),
            _ => None,
        },
        ValueRepr::Pointer { pointee, .. } => match tree.get(pointee) {
            mir::Type::Array { length, .. } => Some(*length),
            _ => None,
        },
        _ => None,
    }
}

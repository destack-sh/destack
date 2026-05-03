use std::collections::HashMap;

use destack_mir as mir;

use crate::program::{
    ElementAccess, FieldAccess, Layout, PointeeAccess, PointerClass, SliceElementAccess,
    ValueLayout, WordLayout, pointer_class_from_reference, repr_type, word_layout_from_type,
};

const VTABLE_FIELD_INDEX: u32 = 0;
const ITABLE_FIELD_INDEX: u32 = 1;

/// Build one field access from one compiled layout.
pub(super) fn field_access(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    pointee_type: mir::LocalNodeId<mir::Type>,
    pointer_class: PointerClass,
    index: u32,
) -> Option<FieldAccess> {
    // resolve the pointee field layout first
    let layout = layouts.get(&pointee_type)?;
    let field = layout.field(index)?;

    // cache scalar layout for lowered memory ops
    let word_layout = access_word_layout(tree, layouts, field.ty);

    Some(FieldAccess {
        pointer_class,
        value_type: field.ty,
        byte_offset: field.offset,
        byte_len: field.byte_len,
        word_layout,
    })
}

/// Build the vtable field for one virtual receiver.
pub(super) fn virtual_table_field(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    receiver_type: Option<mir::LocalNodeId<mir::Type>>,
    pointer_class: PointerClass,
) -> Option<FieldAccess> {
    let receiver_type = receiver_type?;

    field_access(
        tree,
        layouts,
        receiver_type,
        pointer_class,
        VTABLE_FIELD_INDEX,
    )
}

/// Build the itab field for one interface receiver.
pub(super) fn interface_table_field(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    receiver_type: Option<mir::LocalNodeId<mir::Type>>,
    pointer_class: PointerClass,
) -> Option<FieldAccess> {
    let receiver_type = receiver_type?;

    field_access(
        tree,
        layouts,
        receiver_type,
        pointer_class,
        ITABLE_FIELD_INDEX,
    )
}

/// Build one element access from one compiled layout.
pub(super) fn element_access(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    pointee_type: mir::LocalNodeId<mir::Type>,
    pointer_class: PointerClass,
) -> Option<ElementAccess> {
    // resolve the pointee element layout first
    let layout = layouts.get(&pointee_type)?;
    let element = layout.element()?;

    // cache scalar layout for lowered memory ops
    let word_layout = access_word_layout(tree, layouts, element.ty);

    Some(ElementAccess {
        pointer_class,
        value_type: element.ty,
        byte_stride: element.stride,
        byte_len: element.byte_len,
        word_layout,
    })
}

/// Build one slice element access from one slice descriptor type.
pub(super) fn slice_element_access(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    slice_type: mir::LocalNodeId<mir::Type>,
    descriptor_class: PointerClass,
) -> Option<SliceElementAccess> {
    let mir::Type::Slice {
        kind,
        element,
        address_space,
        ..
    } = tree.get(repr_type(tree, slice_type))
    else {
        return None;
    };
    let element_type = element.ty()?;
    let layout = layouts.get(&slice_type)?;
    let slice = layout.slice()?;
    let element_layout = layouts.get(&element_type)?;
    let element_class = pointer_class_from_reference(address_space.clone(), *kind);
    let element_word_layout = access_word_layout(tree, layouts, element_type);

    Some(SliceElementAccess {
        data: FieldAccess {
            pointer_class: descriptor_class,
            value_type: slice.data.ty,
            byte_offset: slice.data.offset,
            byte_len: slice.data.byte_len,
            word_layout: word_layout_from_type(tree, slice.data.ty),
        },
        length: FieldAccess {
            pointer_class: descriptor_class,
            value_type: slice.length.ty,
            byte_offset: slice.length.offset,
            byte_len: slice.length.byte_len,
            word_layout: word_layout_from_type(tree, slice.length.ty),
        },
        element: ElementAccess {
            pointer_class: element_class,
            value_type: element_type,
            byte_stride: element_layout.stride(),
            byte_len: element_layout.byte_len,
            word_layout: element_word_layout,
        },
    })
}

/// Build one pointee access from one compiled layout.
pub(super) fn pointee_access(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    pointee_type: mir::LocalNodeId<mir::Type>,
    pointer_class: PointerClass,
) -> Option<PointeeAccess> {
    // resolve the pointee layout directly
    let layout = layouts.get(&pointee_type)?;
    let word_layout = access_word_layout(tree, layouts, pointee_type);

    Some(PointeeAccess {
        pointer_class,
        value_type: pointee_type,
        byte_offset: 0,
        byte_len: layout.byte_len,
        word_layout,
    })
}

/// Build one tensor element access from one compiled element layout.
pub(super) fn tensor_element_access(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    element_type: mir::LocalNodeId<mir::Type>,
    pointer_class: PointerClass,
) -> Option<ElementAccess> {
    // resolve the lowered tensor element layout directly
    let layout = layouts.get(&element_type)?;
    let word_layout = access_word_layout(tree, layouts, element_type);

    Some(ElementAccess {
        pointer_class,
        value_type: element_type,
        byte_stride: layout.stride(),
        byte_len: layout.byte_len,
        word_layout,
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
fn access_word_layout(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ty: mir::LocalNodeId<mir::Type>,
) -> Option<WordLayout> {
    if !layouts.get(&ty).is_some_and(Layout::is_word) {
        return None;
    }

    word_layout_from_type(tree, ty)
}

/// Resolve the tensor element type for one tensor value or reference type.
pub(super) fn tensor_element_type(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Option<mir::LocalNodeId<mir::Type>> {
    let ty = repr_type(tree, ty);

    match tree.get(ty) {
        mir::Type::Tensor { element, .. } | mir::Type::TensorView { element, .. } => element.ty(),
        _ => None,
    }
}

/// Resolve the field count for a struct or tuple layout.
pub(super) fn field_count_for_layout(tree: &mir::Tree, layout: ValueLayout) -> Option<u32> {
    match layout {
        ValueLayout::FrameBytes { ty } => match tree.get(ty) {
            mir::Type::Struct { fields, copy: _ } => u32::try_from(fields.len()).ok(),
            mir::Type::Tuple { elements, copy: _ } => u32::try_from(elements.len()).ok(),
            _ => None,
        },
        ValueLayout::Pointer { pointee, .. } => match tree.get(pointee) {
            mir::Type::Struct { fields, copy: _ } => u32::try_from(fields.len()).ok(),
            mir::Type::Tuple { elements, copy: _ } => u32::try_from(elements.len()).ok(),
            _ => None,
        },
        _ => None,
    }
}

/// Resolve the element count for an array layout.
pub(super) fn array_element_count(tree: &mir::Tree, layout: ValueLayout) -> Option<u64> {
    match layout {
        ValueLayout::Array { length, .. } => Some(length),
        ValueLayout::FrameBytes { ty } => match tree.get(ty) {
            mir::Type::Array { length, .. } => Some(*length),
            _ => None,
        },
        ValueLayout::Pointer { pointee, .. } => match tree.get(pointee) {
            mir::Type::Array { length, .. } => Some(*length),
            _ => None,
        },
        _ => None,
    }
}

use std::collections::HashMap;

use destack_mir as mir;

use crate::program::{
    Layout, PointerClass, Projection, SliceProjection, ValueLayout, WordLayout,
    pointer_class_from_reference, repr_type, word_layout_from_type,
};

const VTABLE_FIELD_INDEX: u32 = 0;
const ITABLE_FIELD_INDEX: u32 = 1;

/// Build one field projection from one compiled layout.
pub(super) fn field_projection(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    pointee_type: mir::LocalNodeId<mir::Type>,
    index: u32,
) -> Option<Projection> {
    // resolve the pointee field layout first
    let layout = layouts.get(&pointee_type)?;
    let field = layout.field(index)?;

    // cache scalar layout for lowered memory ops
    let word_layout = access_word_layout(tree, layouts, field.ty);

    Some(Projection::fixed(
        field.ty,
        field.offset,
        field.byte_len,
        word_layout,
    ))
}

/// Build the vtable field for one virtual receiver.
pub(super) fn virtual_table_field(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    receiver_type: Option<mir::LocalNodeId<mir::Type>>,
) -> Option<Projection> {
    let receiver_type = receiver_type?;

    field_projection(tree, layouts, receiver_type, VTABLE_FIELD_INDEX)
}

/// Build the itab field for one interface receiver.
pub(super) fn interface_table_field(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    receiver_type: Option<mir::LocalNodeId<mir::Type>>,
) -> Option<Projection> {
    let receiver_type = receiver_type?;

    field_projection(tree, layouts, receiver_type, ITABLE_FIELD_INDEX)
}

/// Build one element projection from one compiled layout.
pub(super) fn element_projection(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    pointee_type: mir::LocalNodeId<mir::Type>,
) -> Option<Projection> {
    // resolve the pointee element layout first
    let layout = layouts.get(&pointee_type)?;
    let element = layout.element()?;
    let length = layout.element_count()? as u64;

    // cache scalar layout for lowered memory ops
    let word_layout = access_word_layout(tree, layouts, element.ty);

    Some(Projection::indexed(
        element.ty,
        length,
        element.stride,
        element.byte_len,
        word_layout,
    ))
}

/// Build one slice projection from one slice descriptor type.
pub(super) fn slice_projection(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    slice_type: mir::LocalNodeId<mir::Type>,
) -> Option<SliceProjection> {
    let mir::Type::Slice {
        element,
        kind: _,
        address_space: _,
        mutability: _,
        ..
    } = tree.get(repr_type(tree, slice_type))
    else {
        return None;
    };
    let element_type = element.ty()?;
    let layout = layouts.get(&slice_type)?;
    let slice = layout.slice()?;
    let element_layout = layouts.get(&element_type)?;
    let element_word_layout = access_word_layout(tree, layouts, element_type);

    Some(SliceProjection {
        data: Projection::fixed(
            slice.data.ty,
            slice.data.offset,
            slice.data.byte_len,
            word_layout_from_type(tree, slice.data.ty),
        ),
        length: Projection::fixed(
            slice.length.ty,
            slice.length.offset,
            slice.length.byte_len,
            word_layout_from_type(tree, slice.length.ty),
        ),
        element: Projection::indexed(
            element_type,
            0,
            element_layout.stride(),
            element_layout.byte_len,
            element_word_layout,
        ),
    })
}

/// Resolve the backing pointer class for one slice descriptor type.
pub(super) fn slice_element_pointer_class(
    tree: &mir::Tree,
    slice_type: mir::LocalNodeId<mir::Type>,
) -> Option<PointerClass> {
    let mir::Type::Slice {
        kind,
        address_space,
        ..
    } = tree.get(repr_type(tree, slice_type))
    else {
        return None;
    };

    Some(pointer_class_from_reference(address_space.clone(), *kind))
}

/// Build one pointee projection from one compiled layout.
pub(super) fn pointee_projection(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    pointee_type: mir::LocalNodeId<mir::Type>,
) -> Option<Projection> {
    // resolve the pointee layout directly
    let layout = layouts.get(&pointee_type)?;
    let word_layout = access_word_layout(tree, layouts, pointee_type);

    Some(Projection::fixed(
        pointee_type,
        0,
        layout.byte_len,
        word_layout,
    ))
}

/// Build one tensor element projection from one compiled element layout.
pub(super) fn tensor_element_projection(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    element_type: mir::LocalNodeId<mir::Type>,
) -> Option<Projection> {
    // resolve the lowered tensor element layout directly
    let layout = layouts.get(&element_type)?;
    let word_layout = access_word_layout(tree, layouts, element_type);

    Some(Projection::indexed(
        element_type,
        0,
        layout.stride(),
        layout.byte_len,
        word_layout,
    ))
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

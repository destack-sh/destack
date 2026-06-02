use std::collections::HashMap;

use destack_mir as mir;

use crate::program::{
    AddressSpace, CellLayout, Layout, Projection, SliceProjection, SlotProjection, ValueShape,
    address_space_from_reference, cell_layout_from_address_space, cell_layout_from_type, repr_type,
};

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
    let cell_layout = access_cell_layout(tree, layouts, field.ty);

    Some(Projection::fixed(
        field.ty,
        field.offset,
        field.byte_len,
        cell_layout,
    ))
}

/// Build the vtable projection for one class receiver.
pub(super) fn virtual_table_projection(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    receiver_type: Option<mir::LocalNodeId<mir::Type>>,
) -> Option<Projection> {
    let receiver_type = receiver_type?;
    let raw_layout = tree.type_layout(repr_type(tree, receiver_type))?;
    let mir::LayoutShape::Object(_) = &raw_layout.shape else {
        return None;
    };

    field_projection_at_offset(tree, layouts, receiver_type, 0)
}

/// Build the dispatch-table projection for one dynamic receiver.
pub(super) fn dynamic_table_projection(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    receiver_type: Option<mir::LocalNodeId<mir::Type>>,
) -> Option<Projection> {
    let receiver_type = receiver_type?;
    let raw_layout = tree.type_layout(repr_type(tree, receiver_type))?;
    let dispatch_offset = raw_layout.dynamic_dispatch_offset()?;

    field_projection_at_offset(tree, layouts, receiver_type, dispatch_offset as usize)
}

/// Build one field projection from a lowered byte offset.
fn field_projection_at_offset(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    pointee_type: mir::LocalNodeId<mir::Type>,
    byte_offset: usize,
) -> Option<Projection> {
    let layout = layouts.get(&pointee_type)?;
    let field = (0..layout.field_count()?)
        .filter_map(|index| layout.field(index as u32))
        .find(|field| field.offset == byte_offset)?;

    let cell_layout = access_cell_layout(tree, layouts, field.ty);

    Some(Projection::fixed(
        field.ty,
        field.offset,
        field.byte_len,
        cell_layout,
    ))
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
    let cell_layout = access_cell_layout(tree, layouts, element.ty);

    Some(Projection::indexed(
        element.ty,
        length,
        element.stride,
        element.byte_len,
        cell_layout,
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
        kind,
        space,
        ..
    } = tree.get(repr_type(tree, slice_type))
    else {
        return None;
    };
    let element_type = element.ty()?;
    let layout = layouts.get(&slice_type)?;
    if !layout.is_slice() {
        return None;
    }

    let element_layout = layouts.get(&element_type)?;
    let element_cell_layout = access_cell_layout(tree, layouts, element_type);
    let data_address_space = address_space_from_reference(space.clone(), *kind);
    let data_cell_layout = cell_layout_from_address_space(data_address_space)?;
    let pointer_bytes = tree.pointer_bytes() as usize;
    let data_byte_len = data_cell_layout.byte_len(pointer_bytes);
    let length_offset = data_byte_len.next_multiple_of(pointer_bytes);
    let length_cell_layout = CellLayout::Uint {
        width: usize::BITS as u8,
    };

    Some(SliceProjection {
        data: SlotProjection::fixed(0, data_byte_len, data_cell_layout),
        length: SlotProjection::fixed(length_offset, pointer_bytes, length_cell_layout),
        element: Projection::indexed(
            element_type,
            0,
            element_layout.stride(),
            element_layout.byte_len,
            element_cell_layout,
        ),
    })
}

/// Resolve the backing address space for one slice descriptor type.
pub(super) fn slice_element_address_space(
    tree: &mir::Tree,
    slice_type: mir::LocalNodeId<mir::Type>,
) -> Option<AddressSpace> {
    let mir::Type::Slice { kind, space, .. } = tree.get(repr_type(tree, slice_type)) else {
        return None;
    };

    Some(address_space_from_reference(space.clone(), *kind))
}

/// Build one pointee projection from one compiled layout.
pub(super) fn pointee_projection(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    pointee_type: mir::LocalNodeId<mir::Type>,
) -> Option<Projection> {
    // resolve the pointee layout directly
    let layout = layouts.get(&pointee_type)?;
    let cell_layout = access_cell_layout(tree, layouts, pointee_type);

    Some(Projection::fixed(
        pointee_type,
        0,
        layout.byte_len,
        cell_layout,
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
    let cell_layout = access_cell_layout(tree, layouts, element_type);

    Some(Projection::indexed(
        element_type,
        0,
        layout.stride(),
        layout.byte_len,
        cell_layout,
    ))
}

/// Resolve the address space for one tensor value type.
pub(super) fn tensor_view_address_space(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Option<AddressSpace> {
    match tree.get(repr_type(tree, ty)) {
        mir::Type::Tensor { .. } => Some(AddressSpace::Local),
        mir::Type::TensorView { kind, space, .. } => {
            Some(address_space_from_reference(space.clone(), *kind))
        }
        _ => None,
    }
}

/// Report whether one compiled layout stays scalar in lowered memory ops.
fn access_cell_layout(
    tree: &mir::Tree,
    layouts: &HashMap<mir::LocalNodeId<mir::Type>, Layout>,
    ty: mir::LocalNodeId<mir::Type>,
) -> Option<CellLayout> {
    if !layouts.get(&ty).is_some_and(Layout::is_cell) {
        return None;
    }

    cell_layout_from_type(tree, ty)
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
pub(super) fn field_count_for_layout(tree: &mir::Tree, layout: ValueShape) -> Option<u32> {
    match layout {
        ValueShape::FrameBytes { ty } => match tree.get(ty) {
            mir::Type::Struct { fields, copy: _ } => u32::try_from(fields.len()).ok(),
            mir::Type::Tuple { elements, copy: _ } => u32::try_from(elements.len()).ok(),
            _ => None,
        },
        ValueShape::Pointer { pointee, .. } => match tree.get(pointee) {
            mir::Type::Struct { fields, copy: _ } => u32::try_from(fields.len()).ok(),
            mir::Type::Tuple { elements, copy: _ } => u32::try_from(elements.len()).ok(),
            _ => None,
        },
        _ => None,
    }
}

/// Resolve the element count for an array layout.
pub(super) fn array_element_count(tree: &mir::Tree, layout: ValueShape) -> Option<u64> {
    match layout {
        ValueShape::Array { length, .. } => Some(length),
        ValueShape::FrameBytes { ty } => match tree.get(ty) {
            mir::Type::Array { length, .. } => Some(*length),
            _ => None,
        },
        ValueShape::Pointer { pointee, .. } => match tree.get(pointee) {
            mir::Type::Array { length, .. } => Some(*length),
            _ => None,
        },
        _ => None,
    }
}

#![cfg(test)]

use std::sync::Arc;

use destack_mir::{Layout, LayoutId, LayoutKind, LayoutTable, ReferenceMap};

/// Insert one test layout and return its id.
pub(crate) fn insert_test_layout(
    layouts: &mut LayoutTable,
    byte_len: usize,
    reference_map: ReferenceMap,
) -> LayoutId {
    layouts.insert(Layout {
        kind: LayoutKind::Struct,
        size: byte_len as u32,
        alignment: 1,
        reference_map,
        fields: Vec::new(),
    })
}

/// Create one shared layout table and the inserted layout ids.
pub(crate) fn test_layouts(layouts: &[(usize, ReferenceMap)]) -> (Arc<LayoutTable>, Vec<LayoutId>) {
    let mut table = LayoutTable::new();
    let mut layout_ids = Vec::with_capacity(layouts.len());

    for (byte_len, reference_map) in layouts {
        let layout_id = insert_test_layout(&mut table, *byte_len, reference_map.clone());
        layout_ids.push(layout_id);
    }

    (Arc::new(table), layout_ids)
}

/// Create one shared layout table and the inserted layout id.
pub(crate) fn test_layout(
    byte_len: usize,
    reference_map: ReferenceMap,
) -> (Arc<LayoutTable>, LayoutId) {
    let (layouts, layout_ids) = test_layouts(&[(byte_len, reference_map)]);

    (layouts, layout_ids[0])
}

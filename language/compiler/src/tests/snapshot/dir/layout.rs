use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::LayoutSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (type_id, layout_id) in self.type_layouts() {
            let layout = self.get_layout(layout_id);

            add_type_layout_rows(builder, self, type_id, layout);
        }

        let layout_count = self.layout_count();
        let type_count = self.type_layout_count();
        if layout_count == 0 && type_count == 0 {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "layout", "summary")
            .count_field("layouts", layout_count)
            .count_field("types", type_count);
        builder.push(row);
    }
}

/// Add rows for one type-bound layout.
fn add_type_layout_rows(
    builder: &mut DirSnapshotBuilder<'_>,
    layouts: &dir::LayoutSegment,
    type_id: dir::LocalTypeId,
    layout: &dir::Layout,
) {
    let anchor = builder.anchor_type(type_id);
    let ty = builder.type_label(type_id);
    let row = SnapshotRow::new(anchor, "layout", "type")
        .type_field("type", ty.clone())
        .field(
            "shape",
            DirSnapshotBuilder::layout_shape_label(&layout.shape),
        )
        .optional_field("size", DirSnapshotBuilder::optional_u32_label(layout.size))
        .optional_field(
            "align",
            DirSnapshotBuilder::optional_u32_label(layout.alignment),
        )
        .optional_field("backing", newtype_backing_label(layouts, &layout.shape));
    builder.push(row);

    match &layout.shape {
        dir::LayoutShape::Struct(layout) => {
            for field in &layout.fields {
                add_layout_field_row(builder, anchor, "field", &ty, field);
            }
        }
        dir::LayoutShape::Tuple(layout) => {
            for field in &layout.elements {
                add_layout_field_row(builder, anchor, "element", &ty, field);
            }
        }
        dir::LayoutShape::Variant(layout) => {
            for variant in &layout.variants {
                add_variant_case_row(builder, layouts, anchor, &ty, variant);
            }
        }
        dir::LayoutShape::None
        | dir::LayoutShape::Scalar
        | dir::LayoutShape::Any
        | dir::LayoutShape::Newtype(_)
        | dir::LayoutShape::Function => {}
    }
}

/// Return the backing layout label for a newtype layout.
fn newtype_backing_label(layouts: &dir::LayoutSegment, shape: &dir::LayoutShape) -> Option<String> {
    let dir::LayoutShape::Newtype(layout) = shape else {
        return None;
    };

    let backing = layouts.get_layout(layout.backing);
    let shape = DirSnapshotBuilder::layout_shape_label(&backing.shape);
    let size = backing
        .size
        .map(|size| size.to_string())
        .unwrap_or_else(|| "?".to_string());
    let align = backing
        .alignment
        .map(|align| align.to_string())
        .unwrap_or_else(|| "?".to_string());

    Some(format!("{shape}({size}/{align})"))
}

/// Add one field-like layout row.
fn add_layout_field_row(
    builder: &mut DirSnapshotBuilder<'_>,
    anchor: SnapshotAnchor,
    entry: &'static str,
    parent: &str,
    field: &dir::LayoutField,
) {
    let row = SnapshotRow::new(anchor, "layout", entry)
        .field("parent", parent)
        .optional_field("key", field.key.map(|key| builder.static_key(key)))
        .type_field("type", builder.type_label(field.ty))
        .optional_field(
            "offset",
            DirSnapshotBuilder::optional_u32_label(field.offset),
        )
        .optional_field("size", DirSnapshotBuilder::optional_u32_label(field.size))
        .optional_field(
            "align",
            DirSnapshotBuilder::optional_u32_label(field.alignment),
        );

    builder.push(row);
}

/// Add one variant case layout row.
fn add_variant_case_row(
    builder: &mut DirSnapshotBuilder<'_>,
    layouts: &dir::LayoutSegment,
    anchor: SnapshotAnchor,
    parent: &str,
    variant: &dir::VariantCaseLayout,
) {
    let layout = layouts.get_layout(variant.layout);
    let row = SnapshotRow::new(anchor, "layout", "variant")
        .field("parent", parent)
        .type_field("type", builder.type_label(variant.ty))
        .optional_field("size", DirSnapshotBuilder::optional_u32_label(layout.size))
        .optional_field(
            "align",
            DirSnapshotBuilder::optional_u32_label(layout.alignment),
        );

    builder.push(row);
}

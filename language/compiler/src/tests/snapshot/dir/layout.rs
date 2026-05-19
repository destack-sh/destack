use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, value};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::LayoutSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (layout_id, layout) in self.iter_layouts() {
            add_layout_entry_rows(builder, layout_id, layout);
        }

        for (type_id, layout_id) in self.type_layouts() {
            let row = SnapshotRow::new(SnapshotAnchor::End, "layout", "type")
                .field("type", value::type_label(type_id))
                .field("layout", value::layout_label(layout_id));

            builder.push(row);
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "layout", "summary")
            .field("layouts", self.layout_count().to_string())
            .field("types", self.type_layout_count().to_string());
        builder.push(row);
    }
}

/// Add rows for one concrete layout.
fn add_layout_entry_rows(
    builder: &mut DirSnapshotBuilder<'_>,
    layout_id: dir::LocalLayoutId,
    layout: &dir::Layout,
) {
    let row = SnapshotRow::new(SnapshotAnchor::End, "layout", "entry")
        .field("layout", value::layout_label(layout_id))
        .field("shape", value::layout_shape_label(&layout.shape))
        .optional_field("size", value::optional_u32_label(layout.size))
        .optional_field("align", value::optional_u32_label(layout.alignment));
    builder.push(row);

    match &layout.shape {
        dir::LayoutShape::Struct(layout) => {
            for field in &layout.fields {
                add_layout_field_row(builder, layout_id, "field", field);
            }
        }
        dir::LayoutShape::Tuple(layout) => {
            for field in &layout.elements {
                add_layout_field_row(builder, layout_id, "element", field);
            }
        }
        dir::LayoutShape::Variant(layout) => {
            for variant in &layout.variants {
                add_variant_case_row(builder, layout_id, variant);
            }
        }
        dir::LayoutShape::Newtype(layout) => {
            let row = SnapshotRow::new(SnapshotAnchor::End, "layout", "newtype")
                .field("layout", value::layout_label(layout_id))
                .field("backing", value::layout_label(layout.backing));

            builder.push(row);
        }
        dir::LayoutShape::None
        | dir::LayoutShape::Scalar
        | dir::LayoutShape::Any
        | dir::LayoutShape::Function => {}
    }
}

/// Add one field-like layout row.
fn add_layout_field_row(
    builder: &mut DirSnapshotBuilder<'_>,
    layout_id: dir::LocalLayoutId,
    entry: &'static str,
    field: &dir::LayoutField,
) {
    let row = SnapshotRow::new(SnapshotAnchor::End, "layout", entry)
        .field("layout", value::layout_label(layout_id))
        .optional_field("key", field.key.map(|key| builder.static_key(key)))
        .field("type", value::type_label(field.ty))
        .field("field_layout", value::layout_label(field.layout))
        .optional_field("offset", value::optional_u32_label(field.offset))
        .optional_field("size", value::optional_u32_label(field.size))
        .optional_field("align", value::optional_u32_label(field.alignment));

    builder.push(row);
}

/// Add one variant case layout row.
fn add_variant_case_row(
    builder: &mut DirSnapshotBuilder<'_>,
    layout_id: dir::LocalLayoutId,
    variant: &dir::VariantCaseLayout,
) {
    let row = SnapshotRow::new(SnapshotAnchor::End, "layout", "variant")
        .field("layout", value::layout_label(layout_id))
        .field("type", value::type_label(variant.ty))
        .field("case_layout", value::layout_label(variant.layout));

    builder.push(row);
}

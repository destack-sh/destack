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
    type_id: dir::GlobalTypeId,
    layout: &dir::Layout,
) {
    let anchor = builder.anchor_global_type(type_id);
    let ty = builder.global_type_label(type_id);
    let row = SnapshotRow::new(anchor, "layout", "type")
        .type_field("type", ty.clone())
        .field(
            "shape",
            DirSnapshotBuilder::layout_shape_label(&layout.shape),
        )
        .field("size", layout.size.to_string())
        .field("align", layout.alignment.to_string())
        .optional_field("tag_size", variant_tag_size_label(&layout.shape))
        .optional_field(
            "payload_offset",
            variant_payload_offset_label(&layout.shape),
        )
        .optional_field("element", element_label(builder, &layout.shape))
        .optional_field("stride", element_stride_label(&layout.shape))
        .optional_field("count", element_count_label(&layout.shape))
        .optional_field("rank", tensor_rank_label(&layout.shape))
        .optional_field("format", tensor_format_label(&layout.shape))
        .optional_field("sharding", tensor_sharding_label(&layout.shape))
        .optional_field("backing", backing_label(builder, layouts, &layout.shape));
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
        dir::LayoutShape::Object(layout) => {
            for field in &layout.fields {
                add_layout_field_row(builder, anchor, "field", &ty, field);
            }
        }
        dir::LayoutShape::Variant(layout) => {
            for variant in &layout.variants {
                add_variant_case_row(builder, layouts, anchor, &ty, variant);
            }
        }
        dir::LayoutShape::None
        | dir::LayoutShape::Scalar
        | dir::LayoutShape::Slice
        | dir::LayoutShape::Array(_)
        | dir::LayoutShape::Vector(_)
        | dir::LayoutShape::Tensor(_)
        | dir::LayoutShape::TensorView(_)
        | dir::LayoutShape::Enum(_)
        | dir::LayoutShape::Dynamic
        | dir::LayoutShape::Function
        | dir::LayoutShape::Newtype(_)
        | dir::LayoutShape::Pointer(_) => {}
    }
}

/// Return the variant tag size label.
fn variant_tag_size_label(shape: &dir::LayoutShape) -> Option<String> {
    let dir::LayoutShape::Variant(layout) = shape else {
        return None;
    };

    Some(layout.tag.size.to_string())
}

/// Return the variant payload offset label.
fn variant_payload_offset_label(shape: &dir::LayoutShape) -> Option<String> {
    let dir::LayoutShape::Variant(layout) = shape else {
        return None;
    };

    DirSnapshotBuilder::optional_u32_label(layout.payload_offset)
}

/// Return the element type label for element-shaped layouts.
fn element_label(builder: &DirSnapshotBuilder<'_>, shape: &dir::LayoutShape) -> Option<String> {
    match shape {
        dir::LayoutShape::Array(layout) | dir::LayoutShape::Vector(layout) => {
            Some(builder.global_type_label(layout.element))
        }
        dir::LayoutShape::Tensor(layout) => Some(builder.global_type_label(layout.element)),
        dir::LayoutShape::TensorView(layout) => Some(builder.global_type_label(layout.element)),
        _ => None,
    }
}

/// Return the element stride label for inline element layouts.
fn element_stride_label(shape: &dir::LayoutShape) -> Option<String> {
    match shape {
        dir::LayoutShape::Array(layout) | dir::LayoutShape::Vector(layout) => {
            Some(layout.stride.to_string())
        }
        _ => None,
    }
}

/// Return the element count label for inline element layouts.
fn element_count_label(shape: &dir::LayoutShape) -> Option<String> {
    match shape {
        dir::LayoutShape::Array(layout) | dir::LayoutShape::Vector(layout) => {
            Some(layout.count.to_string())
        }
        _ => None,
    }
}

/// Return the tensor rank label for tensor layouts.
fn tensor_rank_label(shape: &dir::LayoutShape) -> Option<String> {
    match shape {
        dir::LayoutShape::Tensor(layout) => Some(layout.rank.to_string()),
        dir::LayoutShape::TensorView(layout) => Some(layout.rank.to_string()),
        _ => None,
    }
}

/// Return the tensor format label for tensor layouts.
fn tensor_format_label(shape: &dir::LayoutShape) -> Option<String> {
    match shape {
        dir::LayoutShape::Tensor(layout) => Some(tensor_format_name(layout.format).to_string()),
        dir::LayoutShape::TensorView(layout) => {
            Some(tensor_view_format_name(layout.format).to_string())
        }
        _ => None,
    }
}

/// Return the tensor sharding label for tensor layouts.
fn tensor_sharding_label(shape: &dir::LayoutShape) -> Option<String> {
    match shape {
        dir::LayoutShape::Tensor(layout) => Some(tensor_sharding_name(&layout.sharding)),
        dir::LayoutShape::TensorView(layout) => Some(tensor_sharding_name(&layout.sharding)),
        _ => None,
    }
}

/// Return the snapshot name for a tensor sharding descriptor.
fn tensor_sharding_name(sharding: &dir::TensorSharding) -> String {
    match sharding {
        dir::TensorSharding::Unsharded => "unsharded".to_string(),
        dir::TensorSharding::Sharding { axes } => {
            let axes = axes
                .iter()
                .map(tensor_sharding_axis_name)
                .collect::<Vec<_>>()
                .join(", ");

            format!("sharding({axes})")
        }
    }
}

/// Return the snapshot name for one tensor sharding axis.
fn tensor_sharding_axis_name(axis: &dir::TensorShardingAxis) -> String {
    match axis {
        dir::TensorShardingAxis::Shard { axis } => format!("shard({axis})"),
        dir::TensorShardingAxis::Replicate => "replicate".to_string(),
        dir::TensorShardingAxis::Partial { reduction } => {
            let reduction = tensor_reduction_name(*reduction);

            format!("partial({reduction})")
        }
    }
}

/// Return the snapshot name for one tensor partial reduction.
fn tensor_reduction_name(reduction: dir::TensorReduction) -> &'static str {
    match reduction {
        dir::TensorReduction::Add => "add",
        dir::TensorReduction::Multiply => "multiply",
        dir::TensorReduction::Minimum => "minimum",
        dir::TensorReduction::Maximum => "maximum",
        dir::TensorReduction::And => "and",
        dir::TensorReduction::Or => "or",
    }
}

/// Return the snapshot name for an owning tensor format.
fn tensor_format_name(format: dir::TensorFormat) -> &'static str {
    match format {
        dir::TensorFormat::Dense {
            order: dir::TensorDimensionOrder::RowMajor,
        } => "dense(rowMajor)",
        dir::TensorFormat::Dense {
            order: dir::TensorDimensionOrder::ColumnMajor,
        } => "dense(columnMajor)",
    }
}

/// Return the snapshot name for a tensor view format.
fn tensor_view_format_name(format: dir::TensorViewFormat) -> &'static str {
    match format {
        dir::TensorViewFormat::Dense { order } => {
            tensor_format_name(dir::TensorFormat::Dense { order })
        }
        dir::TensorViewFormat::Strided => "strided",
    }
}

/// Return the backing layout label for a nominal layout.
fn backing_label(
    builder: &DirSnapshotBuilder<'_>,
    layouts: &dir::LayoutSegment,
    shape: &dir::LayoutShape,
) -> Option<String> {
    let backing_layout = match shape {
        dir::LayoutShape::Enum(layout) => {
            return Some(builder.global_type_label(layout.backing_type));
        }
        dir::LayoutShape::Newtype(layout) => layout.backing_layout,
        _ => return None,
    };

    let backing = layouts.get_layout(backing_layout);
    let shape = DirSnapshotBuilder::layout_shape_label(&backing.shape);
    let size = backing.size;
    let align = backing.alignment;

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
        .type_field("type", builder.global_type_label(field.ty))
        .field("offset", field.offset.to_string())
        .field("size", field.size.to_string())
        .field("align", field.alignment.to_string());

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
        .type_field("type", builder.global_type_label(variant.ty))
        .field("size", layout.size.to_string())
        .field("align", layout.alignment.to_string());

    builder.push(row);
}

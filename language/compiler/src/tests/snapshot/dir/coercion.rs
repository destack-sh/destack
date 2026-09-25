use tspp_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::CoercionSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (node_id, coercion) in self.coercions() {
            let row = SnapshotRow::new(builder.anchor_node(node_id), "coercion", "node")
                .optional_field("source", builder.node_source(node_id))
                .type_field("from", builder.global_type_label(coercion.source))
                .verbatim_field(
                    "adjustments",
                    coercion_adjustments_label(builder, &coercion.adjustments),
                )
                .field("origin", cast_origin_label(coercion.origin));

            builder.push(row);
        }

        let count = self.coercions().count();
        if count == 0 {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "coercion", "summary")
            .count_field("nodes", count);
        builder.push(row);
    }
}

/// Return one coercion adjustment list label.
fn coercion_adjustments_label(
    builder: &DirSnapshotBuilder<'_>,
    adjustments: &[dir::CoercionAdjustment],
) -> String {
    let adjustments = adjustments
        .iter()
        .map(|adjustment| coercion_adjustment_label(builder, adjustment));
    let adjustments = adjustments.collect::<Vec<_>>().join(", ");

    format!("[{adjustments}]")
}

/// Return one coercion adjustment label.
fn coercion_adjustment_label(
    builder: &DirSnapshotBuilder<'_>,
    adjustment: &dir::CoercionAdjustment,
) -> String {
    let kind = adjustment.as_str();
    let target = builder.global_type_label(adjustment.target());
    let dir::CoercionAdjustment::Union { cases, .. } = adjustment else {
        return format!("{{ kind: {kind}, target: {target} }}");
    };

    let cases = cases
        .iter()
        .map(|case| coercion_case_label(builder, case))
        .collect::<Vec<_>>()
        .join(", ");

    format!("{{ kind: {kind}, target: {target}, cases: ({cases}) }}")
}

/// Return one union coercion case label.
fn coercion_case_label(builder: &DirSnapshotBuilder<'_>, case: &dir::CoercionCase) -> String {
    let source = builder.global_type_label(case.source);
    let target = builder.global_type_label(case.target);
    if case.adjustments.is_empty() {
        return format!("{{ source: {source}, target: {target} }}");
    }
    let adjustments = coercion_adjustments_label(builder, &case.adjustments);

    format!("{{ source: {source}, target: {target}, adjustments: {adjustments} }}")
}

/// Return the snapshot label for one cast origin.
fn cast_origin_label(origin: dir::CastOrigin) -> &'static str {
    match origin {
        dir::CastOrigin::Explicit => "explicit",
        dir::CastOrigin::Implicit => "implicit",
    }
}

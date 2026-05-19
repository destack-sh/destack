use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, value};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::ResolutionSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (node_id, resolution) in self.name_entries() {
            add_name_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.label_entries() {
            add_label_resolution_row(builder, node_id, *resolution);
        }

        for (node_id, resolution) in self.member_entries() {
            add_member_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.call_entries() {
            add_call_resolution_row(builder, node_id, resolution);
        }

        let rows = self.name_entries().count()
            + self.label_entries().count()
            + self.member_entries().count()
            + self.call_entries().count();
        let row = SnapshotRow::new(SnapshotAnchor::End, "resolution", "summary")
            .field("rows", rows.to_string());
        builder.push(row);
    }
}

/// Add one name resolution row.
fn add_name_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::NameResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "name")
        .field("node", builder.node_label(node_id))
        .field("target", value::symbol_label(builder, resolution.symbol));

    builder.push(row);
}

/// Add one label resolution row.
fn add_label_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: dir::LabelResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "label")
        .field("node", builder.node_label(node_id));

    let row = match resolution {
        dir::LabelResolution::Symbol(symbol_id) => row
            .field("kind", "symbol")
            .field("target", value::symbol_label(builder, symbol_id)),
        dir::LabelResolution::Loop => row.field("kind", "loop"),
        dir::LabelResolution::Function => row.field("kind", "function"),
    };

    builder.push(row);
}

/// Add one member resolution row.
fn add_member_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::MemberResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "member")
        .field("node", builder.node_label(node_id))
        .optional_field("receiver", value::optional_type_label(resolution.receiver));

    let row = match &resolution.target {
        dir::MemberTarget::Intrinsic => row.field("kind", "intrinsic"),
        dir::MemberTarget::Direct(candidate) => {
            add_member_candidate_fields(builder, row.field("kind", "direct"), candidate)
        }
        dir::MemberTarget::Select(candidates) => row.field("kind", "select").list_field(
            "targets",
            candidates
                .iter()
                .map(|candidate| value::member_candidate_label(builder, candidate)),
        ),
    };

    builder.push(row);
}

/// Add one call resolution row.
fn add_call_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::CallResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "call")
        .field("node", builder.node_label(node_id))
        .list_field(
            "parameters",
            resolution
                .parameters
                .iter()
                .map(|type_id| value::type_label(*type_id)),
        )
        .optional_field("return", value::optional_type_label(resolution.return_type));

    let row = match &resolution.target {
        dir::CallTarget::Intrinsic { receiver } => row
            .field("kind", "intrinsic")
            .optional_field("receiver", value::optional_type_label(*receiver)),
        dir::CallTarget::Direct(candidate) => {
            add_call_candidate_fields(builder, row.field("kind", "direct"), candidate)
        }
        dir::CallTarget::Select(candidates) => row.field("kind", "select").list_field(
            "targets",
            candidates
                .iter()
                .map(|candidate| value::call_candidate_label(builder, candidate)),
        ),
    };

    builder.push(row);
}

/// Add direct member candidate fields.
fn add_member_candidate_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    candidate: &dir::MemberCandidate,
) -> SnapshotRow {
    row.field("target", value::member_candidate_label(builder, candidate))
        .optional_field("receiver", value::optional_type_label(candidate.receiver))
        .optional_field(
            "instance",
            candidate.instantiation.map(value::instantiation_label),
        )
}

/// Add direct call candidate fields.
fn add_call_candidate_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    candidate: &dir::CallCandidate,
) -> SnapshotRow {
    row.field("target", value::call_candidate_label(builder, candidate))
        .optional_field("receiver", value::optional_type_label(candidate.receiver))
        .optional_field(
            "instance",
            candidate.instantiation.map(value::instantiation_label),
        )
}

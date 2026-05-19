use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, label};
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

        let row = SnapshotRow::new(SnapshotAnchor::End, "resolution", "summary")
            .field("names", self.name_entries().count().to_string())
            .field("labels", self.label_entries().count().to_string())
            .field("members", self.member_entries().count().to_string())
            .field("calls", self.call_entries().count().to_string());
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
        .optional_field("source", builder.node_source(node_id))
        .field("target", builder.symbol_label(resolution.symbol));

    builder.push(row);
}

/// Add one label resolution row.
fn add_label_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: dir::LabelResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "label")
        .optional_field("source", builder.node_source(node_id));

    let row = match resolution {
        dir::LabelResolution::Symbol(symbol_id) => row
            .field("kind", "symbol")
            .field("target", builder.symbol_label(symbol_id)),
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
        .optional_field("source", builder.node_source(node_id))
        .optional_field(
            "receiver",
            resolution.receiver.map(|ty| builder.type_label(ty)),
        );

    let row = match &resolution.target {
        dir::MemberTarget::Intrinsic => row.field("kind", "intrinsic"),
        dir::MemberTarget::Direct(candidate) => {
            add_member_candidate_fields(builder, row.field("kind", "direct"), candidate)
        }
        dir::MemberTarget::Select(candidates) => row.field("kind", "select").list_field(
            "targets",
            candidates
                .iter()
                .map(|candidate| label::member_candidate_label(builder, candidate)),
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
        .optional_field("source", builder.node_source(node_id))
        .list_field(
            "parameters",
            resolution
                .parameters
                .iter()
                .map(|type_id| builder.type_label(*type_id)),
        )
        .optional_field(
            "return",
            resolution.return_type.map(|ty| builder.type_label(ty)),
        );

    let row = match &resolution.target {
        dir::CallTarget::Intrinsic { receiver } => row
            .field("kind", "intrinsic")
            .optional_field("receiver", receiver.map(|ty| builder.type_label(ty))),
        dir::CallTarget::Direct(candidate) => {
            add_call_candidate_fields(builder, row.field("kind", "direct"), candidate)
        }
        dir::CallTarget::Select(candidates) => row.field("kind", "select").list_field(
            "targets",
            candidates
                .iter()
                .map(|candidate| label::call_candidate_label(builder, candidate)),
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
    row.field("target", label::member_candidate_label(builder, candidate))
        .optional_field(
            "receiver",
            candidate.receiver.map(|ty| builder.type_label(ty)),
        )
        .optional_field("instance", candidate.instance.map(label::instance_label))
}

/// Add direct call candidate fields.
fn add_call_candidate_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    candidate: &dir::CallCandidate,
) -> SnapshotRow {
    row.field("target", label::call_candidate_label(builder, candidate))
        .optional_field(
            "receiver",
            candidate.receiver.map(|ty| builder.type_label(ty)),
        )
        .optional_field("instance", candidate.instance.map(label::instance_label))
}

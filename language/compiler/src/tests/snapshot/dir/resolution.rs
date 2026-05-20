use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
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

        let name_count = self.name_entries().count();
        let label_count = self.label_entries().count();
        let member_count = self.member_entries().count();
        let call_count = self.call_entries().count();
        if name_count == 0 && label_count == 0 && member_count == 0 && call_count == 0 {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "resolution", "summary")
            .count_field("names", name_count)
            .count_field("labels", label_count)
            .count_field("members", member_count)
            .count_field("calls", call_count);
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
                .map(|candidate| builder.member_candidate_label(candidate)),
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
                .map(|candidate| builder.call_candidate_label(candidate)),
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
    row.field("target", builder.member_candidate_label(candidate))
        .optional_field(
            "receiver",
            candidate.receiver.map(|ty| builder.type_label(ty)),
        )
        .optional_field(
            "instance",
            candidate.instance.map(|id| builder.instance_label(id)),
        )
}

/// Add direct call candidate fields.
fn add_call_candidate_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    candidate: &dir::CallCandidate,
) -> SnapshotRow {
    row.field("target", builder.call_candidate_label(candidate))
        .optional_field(
            "receiver",
            candidate.receiver.map(|ty| builder.type_label(ty)),
        )
        .optional_field(
            "instance",
            candidate.instance.map(|id| builder.instance_label(id)),
        )
}

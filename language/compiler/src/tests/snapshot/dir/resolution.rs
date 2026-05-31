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

        for (node_id, resolution) in self.receiver_entries() {
            add_receiver_resolution_row(builder, node_id, *resolution);
        }

        for (node_id, resolution) in self.member_entries() {
            add_member_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.call_entries() {
            add_call_resolution_row(builder, node_id, resolution);
        }

        let name_count = self.name_entries().count();
        let label_count = self.label_entries().count();
        let receiver_count = self.receiver_entries().count();
        let member_count = self.member_entries().count();
        let call_count = self.call_entries().count();
        if name_count == 0
            && label_count == 0
            && receiver_count == 0
            && member_count == 0
            && call_count == 0
        {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "resolution", "summary")
            .count_field("names", name_count)
            .count_field("labels", label_count)
            .count_field("receivers", receiver_count)
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
    let row = SnapshotRow::new(
        builder.name_resolution_anchor(node_id),
        "resolution",
        "name",
    )
    .optional_field("source", builder.name_resolution_source(node_id));
    let row = if resolution.symbols.len() == 1 {
        row.field("target", builder.symbol_path_label(resolution.symbols[0]))
    } else {
        row.list_field(
            "target",
            resolution
                .symbols
                .iter()
                .map(|symbol| builder.symbol_path_label(*symbol)),
        )
    };

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

/// Add one receiver resolution row.
fn add_receiver_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: dir::ReceiverResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "receiver")
        .optional_field("source", builder.node_source(node_id))
        .field("kind", receiver_kind_label(resolution.kind))
        .optional_field(
            "owner",
            resolution
                .owner
                .map(|symbol| builder.symbol_path_label(symbol)),
        )
        .optional_type_field("type", resolution.ty.map(|ty| builder.type_label(ty)));

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
        .optional_type_field(
            "receiver",
            resolution.receiver.map(|ty| builder.type_label(ty)),
        );

    let row = match &resolution.target {
        dir::MemberTarget::Builtin(builtin) => row
            .field("kind", "builtin")
            .field("builtin", builtin_member_label(*builtin)),
        dir::MemberTarget::Field(key) => row
            .field("kind", "field")
            .field("key", builder.static_key(*key)),
        dir::MemberTarget::Symbol(candidate) => row
            .field("kind", "symbol")
            .field("target", builder.member_candidate_label(candidate))
            .optional_field(
                "application",
                candidate
                    .application
                    .map(|id| builder.generic_application_label(id)),
            ),
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
        .type_tuple_field(
            "parameters",
            resolution
                .parameters
                .iter()
                .map(|type_id| builder.type_label(*type_id)),
        )
        .optional_type_field(
            "return",
            resolution.return_type.map(|ty| builder.type_label(ty)),
        );

    let row = match &resolution.target {
        dir::CallTarget::Builtin(builtin) => row
            .field("kind", "builtin")
            .field("builtin", builtin_call_label(*builtin)),
        dir::CallTarget::Value => row.field("kind", "value"),
        dir::CallTarget::Construct(candidate) => {
            add_call_candidate_fields(builder, row.field("kind", "construct"), candidate)
        }
        dir::CallTarget::Symbol(candidate) => {
            add_call_candidate_fields(builder, row.field("kind", "symbol"), candidate)
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

/// Return one receiver kind label.
fn receiver_kind_label(kind: dir::ReceiverKind) -> &'static str {
    match kind {
        dir::ReceiverKind::This => "this",
        dir::ReceiverKind::Super => "super",
    }
}

/// Return one builtin member label.
fn builtin_member_label(builtin: dir::BuiltinMember) -> &'static str {
    match builtin {
        dir::BuiltinMember::Index => "subscript.index",
        dir::BuiltinMember::Slice => "subscript.slice",
    }
}

/// Return one builtin call label.
fn builtin_call_label(builtin: dir::BuiltinCall) -> String {
    match builtin {
        dir::BuiltinCall::UnaryOperator { operator } => {
            format!("unary.{}", DirSnapshotBuilder::variant_label(operator))
        }
        dir::BuiltinCall::BinaryOperator { operator } => {
            format!("binary.{}", DirSnapshotBuilder::variant_label(operator))
        }
    }
}

/// Add direct call candidate fields.
fn add_call_candidate_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    candidate: &dir::CallCandidate,
) -> SnapshotRow {
    row.field("target", builder.call_candidate_label(candidate))
        .optional_type_field(
            "receiver",
            candidate.receiver.map(|ty| builder.type_label(ty)),
        )
        .optional_field(
            "application",
            candidate
                .application
                .map(|id| builder.generic_application_label(id)),
        )
}

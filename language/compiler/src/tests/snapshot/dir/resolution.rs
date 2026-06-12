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

        for (node_id, resolution) in self.construct_entries() {
            add_construct_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.pattern_entries() {
            add_pattern_resolution_row(builder, node_id, resolution);
        }

        let name_count = self.name_entries().count();
        let label_count = self.label_entries().count();
        let receiver_count = self.receiver_entries().count();
        let member_count = self.member_entries().count();
        let call_count = self.call_entries().count();
        let construct_count = self.construct_entries().count();
        let pattern_count = self.pattern_entries().count();
        if name_count == 0
            && label_count == 0
            && receiver_count == 0
            && member_count == 0
            && call_count == 0
            && construct_count == 0
            && pattern_count == 0
        {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "resolution", "summary")
            .count_field("names", name_count)
            .count_field("labels", label_count)
            .count_field("receivers", receiver_count)
            .count_field("members", member_count)
            .count_field("calls", call_count)
            .count_field("constructs", construct_count)
            .count_field("patterns", pattern_count);
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
    let symbols = resolution.symbols();
    let row = if symbols.len() == 1 {
        row.field("target", builder.symbol_path_label(symbols[0]))
    } else {
        row.list_field(
            "target",
            symbols
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
        .field("owner", builder.symbol_path_label(resolution.owner))
        .type_field("type", builder.global_type_label(resolution.ty));

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
        .type_field("receiver", builder.global_type_label(resolution.receiver));

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
            .optional_field("arguments", arguments_label(builder, &candidate.arguments)),
        dir::MemberTarget::Union(candidates) => row.field("kind", "union").list_field(
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
                .map(|type_id| builder.global_type_label(*type_id)),
        )
        .type_field("return", builder.global_type_label(resolution.return_type));

    let row = match &resolution.target {
        dir::CallTarget::Builtin(builtin) => row
            .field("kind", "builtin")
            .field("builtin", builtin_call_label(*builtin)),
        dir::CallTarget::Expression { arguments } => row
            .field("kind", "expression")
            .optional_field("arguments", arguments_label(builder, arguments)),
        dir::CallTarget::Symbol(candidate) => {
            add_call_candidate_fields(builder, row.field("kind", "symbol"), candidate)
        }
        dir::CallTarget::Union(candidates) => row.field("kind", "union").list_field(
            "targets",
            candidates
                .iter()
                .map(|candidate| builder.call_candidate_label(candidate)),
        ),
    };

    builder.push(row);
}

/// Add one construct resolution row.
fn add_construct_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::ConstructResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "construct")
        .optional_field("source", builder.node_source(node_id))
        .type_tuple_field(
            "parameters",
            resolution
                .parameters
                .iter()
                .map(|type_id| builder.global_type_label(*type_id)),
        )
        .type_field("return", builder.global_type_label(resolution.return_type));

    let row = match &resolution.target {
        dir::ConstructTarget::Class(candidate) => {
            add_class_construct_candidate_fields(builder, row.field("kind", "class"), candidate)
        }
        dir::ConstructTarget::Newtype(candidate) => {
            add_construct_candidate_fields(builder, row.field("kind", "newtype"), candidate)
        }
    };

    builder.push(row);
}

/// Add one pattern resolution row.
fn add_pattern_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::PatternResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "pattern")
        .optional_field("source", builder.node_source(node_id))
        .field("kind", pattern_resolution_label(resolution));

    let row = match resolution {
        dir::PatternResolution::Wildcard => row,
        dir::PatternResolution::Binding(binding) => row
            .optional_field(
                "target",
                binding
                    .symbol
                    .map(|symbol| builder.symbol_path_label(symbol)),
            )
            .optional_field(
                "pattern",
                binding.pattern.map(|node| builder.node_label(node)),
            ),
        dir::PatternResolution::Literal(literal) => {
            row.field("value", builder.scalar_literal_label(&literal.value))
        }
        dir::PatternResolution::Range(range) => row
            .type_field("domain", builder.global_type_label(range.domain))
            .optional_field(
                "start",
                range
                    .start
                    .map(|value| builder.scalar_literal_label(&value)),
            )
            .optional_field(
                "end",
                range.end.map(|value| builder.scalar_literal_label(&value)),
            )
            .field("bound", DirSnapshotBuilder::variant_label(range.end_bound)),
        dir::PatternResolution::Tuple(tuple) => {
            row.list_field("fields", pattern_field_labels(builder, &tuple.fields))
        }
        dir::PatternResolution::Sequence(sequence) => {
            add_pattern_sequence_fields(builder, row, sequence)
        }
        dir::PatternResolution::Shape(shape) => {
            row.list_field("fields", pattern_field_labels(builder, &shape.fields))
        }
        dir::PatternResolution::Nominal(nominal) => row
            .field("target", builder.symbol_path_label(nominal.symbol))
            .optional_field("arguments", arguments_label(builder, &nominal.arguments))
            .list_field("fields", pattern_field_labels(builder, &nominal.fields)),
        dir::PatternResolution::Newtype(newtype) => row
            .field("target", builder.symbol_path_label(newtype.symbol))
            .optional_field("arguments", arguments_label(builder, &newtype.arguments))
            .optional_field("value", newtype.value.map(|node| builder.node_label(node))),
        dir::PatternResolution::Variant(variant) => row
            .field("target", builder.symbol_path_label(variant.symbol))
            .optional_field("arguments", arguments_label(builder, &variant.arguments))
            .optional_field(
                "discriminant",
                variant
                    .discriminant
                    .map(|value| builder.scalar_literal_label(&value)),
            )
            .list_field("fields", pattern_field_labels(builder, &variant.fields)),
        dir::PatternResolution::Union(union) => row.list_field(
            "alternatives",
            union
                .alternatives
                .iter()
                .map(|node| builder.node_label(*node)),
        ),
        dir::PatternResolution::Borrow(borrow) => row
            .optional_field(
                "access",
                borrow.access.map(DirSnapshotBuilder::variant_label),
            )
            .field("pattern", builder.node_label(borrow.pattern)),
        dir::PatternResolution::Move(move_) => row
            .optional_field(
                "access",
                move_.access.map(DirSnapshotBuilder::variant_label),
            )
            .field("pattern", builder.node_label(move_.pattern)),
        dir::PatternResolution::Dereference(dereference) => {
            row.field("pattern", builder.node_label(dereference.pattern))
        }
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

/// Return one pattern resolution label.
fn pattern_resolution_label(resolution: &dir::PatternResolution) -> &'static str {
    match resolution {
        dir::PatternResolution::Wildcard => "wildcard",
        dir::PatternResolution::Binding(_) => "binding",
        dir::PatternResolution::Literal(_) => "literal",
        dir::PatternResolution::Range(_) => "range",
        dir::PatternResolution::Tuple(_) => "tuple",
        dir::PatternResolution::Sequence(_) => "sequence",
        dir::PatternResolution::Shape(_) => "shape",
        dir::PatternResolution::Nominal(_) => "nominal",
        dir::PatternResolution::Newtype(_) => "newtype",
        dir::PatternResolution::Variant(_) => "variant",
        dir::PatternResolution::Union(_) => "union",
        dir::PatternResolution::Borrow(_) => "borrow",
        dir::PatternResolution::Move(_) => "move",
        dir::PatternResolution::Dereference(_) => "dereference",
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
            candidate.receiver.map(|ty| builder.global_type_label(ty)),
        )
        .optional_field("arguments", arguments_label(builder, &candidate.arguments))
}

/// Add direct class construct candidate fields.
fn add_class_construct_candidate_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    candidate: &dir::ClassConstructCandidate,
) -> SnapshotRow {
    row.field("target", builder.symbol_path_label(candidate.symbol))
        .optional_field(
            "constructor",
            candidate
                .constructor
                .map(|symbol| builder.symbol_path_label(symbol)),
        )
        .optional_field("arguments", arguments_label(builder, &candidate.arguments))
}

/// Add direct newtype construct candidate fields.
fn add_construct_candidate_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    candidate: &dir::NewtypeConstructCandidate,
) -> SnapshotRow {
    row.field("target", builder.symbol_path_label(candidate.symbol))
        .optional_field("arguments", arguments_label(builder, &candidate.arguments))
}

/// Add ordered pattern sequence fields.
fn add_pattern_sequence_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    sequence: &dir::PatternSequenceResolution,
) -> SnapshotRow {
    match sequence {
        dir::PatternSequenceResolution::Array { fields, rest } => row
            .field("sequence", "array")
            .list_field("fields", pattern_field_labels(builder, fields))
            .optional_field(
                "rest",
                rest.as_ref().map(|rest| pattern_rest_label(builder, rest)),
            ),
        dir::PatternSequenceResolution::Slice { fields, rest } => row
            .field("sequence", "slice")
            .list_field("fields", pattern_field_labels(builder, fields))
            .optional_field(
                "rest",
                rest.as_ref().map(|rest| pattern_rest_label(builder, rest)),
            ),
        dir::PatternSequenceResolution::FixedArray { fields, length } => row
            .field("sequence", "fixed_array")
            .field("length", builder.global_type_label(*length))
            .list_field("fields", pattern_field_labels(builder, fields)),
    }
}

/// Return pattern field labels.
fn pattern_field_labels<'a>(
    builder: &'a DirSnapshotBuilder<'_>,
    fields: &'a [dir::PatternFieldResolution],
) -> impl Iterator<Item = String> + 'a {
    fields
        .iter()
        .map(|field| pattern_field_label(builder, field))
}

/// Return one pattern field label.
fn pattern_field_label(
    builder: &DirSnapshotBuilder<'_>,
    field: &dir::PatternFieldResolution,
) -> String {
    let target = match field.target {
        dir::PatternFieldTarget::Key(key) => builder.static_key(key),
        dir::PatternFieldTarget::Index(index) => format!("#{index}"),
    };

    let Some(pattern) = field.pattern else {
        return target;
    };

    let pattern = builder.node_label(pattern);

    format!("{target}: {pattern}")
}

/// Return one pattern rest label.
fn pattern_rest_label(
    builder: &DirSnapshotBuilder<'_>,
    rest: &dir::PatternRestResolution,
) -> String {
    let Some(pattern) = rest.pattern else {
        return "...".to_string();
    };

    let pattern = builder.node_label(pattern);

    format!("...{pattern}")
}

/// Render one applied generic argument list label.
fn arguments_label(
    builder: &DirSnapshotBuilder<'_>,
    arguments: &[dir::GlobalTypeId],
) -> Option<String> {
    if arguments.is_empty() {
        return None;
    }

    Some(
        arguments
            .iter()
            .map(|argument| builder.global_type_label(*argument))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

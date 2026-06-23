use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::ResolutionSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (node_id, resolution) in self.name_entries() {
            add_name_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.instantiation_entries() {
            add_instantiation_resolution_row(builder, node_id, resolution);
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

        for (node_id, resolution) in self.read_write_entries() {
            add_read_write_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.predicate_entries() {
            add_predicate_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.construct_entries() {
            add_construct_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.pattern_entries() {
            add_pattern_resolution_row(builder, self, node_id, resolution);
        }

        for (node_id, resolution) in self.assign_pattern_entries() {
            add_assign_pattern_resolution_row(builder, self, node_id, resolution);
        }

        let name_count = self.name_entries().count();
        let instantiation_count = self.instantiation_entries().count();
        let label_count = self.label_entries().count();
        let receiver_count = self.receiver_entries().count();
        let member_count = self.member_entries().count();
        let call_count = self.call_entries().count();
        let read_write_count = self.read_write_entries().count();
        let predicate_count = self.predicate_entries().count();
        let construct_count = self.construct_entries().count();
        let pattern_count = self.pattern_entries().count();
        let assign_pattern_count = self.assign_pattern_entries().count();
        if name_count == 0
            && instantiation_count == 0
            && label_count == 0
            && receiver_count == 0
            && member_count == 0
            && call_count == 0
            && read_write_count == 0
            && predicate_count == 0
            && construct_count == 0
            && pattern_count == 0
            && assign_pattern_count == 0
        {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "resolution", "summary")
            .count_field("names", name_count)
            .count_field("instantiations", instantiation_count)
            .count_field("labels", label_count)
            .count_field("receivers", receiver_count)
            .count_field("members", member_count)
            .count_field("calls", call_count)
            .count_field("predicates", predicate_count)
            .count_field("constructs", construct_count)
            .count_field("patterns", pattern_count)
            .count_field("assign_patterns", assign_pattern_count);
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

/// Add one explicit instantiation resolution row.
fn add_instantiation_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::InstantiationResolution,
) {
    let anchor = builder.anchor_node(node_id);
    let source = builder.node_source(node_id);
    let instance = builder.generic_instance_label(resolution.symbol, &resolution.arguments);
    let row = SnapshotRow::new(anchor, "resolution", "instantiation")
        .optional_field("source", source.clone())
        .field("target", builder.symbol_path_label(resolution.symbol))
        .field("instance", instance);

    builder.push(row);
    builder.add_generic_instance(anchor, source, resolution.symbol, &resolution.arguments);
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
        dir::MemberTarget::Field(key) => row
            .field("kind", "field")
            .field("key", builder.static_key(*key)),
        dir::MemberTarget::Element(index) => row
            .field("kind", "element")
            .field("index", index.to_string()),
        dir::MemberTarget::Index(key) => row
            .field("kind", "index")
            .type_field("key", builder.global_type_label(*key)),
        dir::MemberTarget::Symbol(candidate) => row
            .field("kind", "symbol")
            .field("target", builder.member_candidate_label(candidate))
            .optional_field(
                "instance",
                generic_instance_label(builder, candidate.symbol, &candidate.arguments),
            ),
        dir::MemberTarget::Existential(candidates) => row.field("kind", "existential").list_field(
            "targets",
            candidates
                .iter()
                .map(|candidate| builder.member_candidate_label(candidate)),
        ),
        dir::MemberTarget::Universal(candidates) => row.field("kind", "universal").list_field(
            "targets",
            candidates
                .iter()
                .map(|candidate| builder.member_candidate_label(candidate)),
        ),
    };

    builder.push(row);
    add_member_target_generic_instances(builder, node_id, &resolution.target);
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
        dir::CallTarget::Universal(candidates) => row.field("kind", "universal").list_field(
            "targets",
            candidates
                .iter()
                .map(|candidate| builder.call_candidate_label(candidate)),
        ),
    };

    builder.push(row);
    add_call_target_generic_instances(builder, node_id, &resolution.target);
}

/// Add one paired read-write resolution row.
fn add_read_write_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::ReadWriteResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "readwrite")
        .optional_field("source", builder.node_source(node_id))
        .type_field(
            "element",
            builder.global_type_label(resolution.read.return_type),
        );

    // both halves resolve symbol-backed accessor methods
    let row = match (&resolution.read.target, &resolution.write.target) {
        (dir::CallTarget::Symbol(read), dir::CallTarget::Symbol(write)) => row
            .field("read", builder.call_candidate_label(read))
            .field("write", builder.call_candidate_label(write)),
        _ => row,
    };

    builder.push(row);
}

/// Add one predicate resolution row.
fn add_predicate_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::PredicateResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "predicate")
        .optional_field("source", builder.node_source(node_id));

    let row = match resolution {
        dir::PredicateResolution::Is(predicate) => row
            .field("kind", "is")
            .type_field("value", builder.global_type_label(predicate.value_type))
            .type_field("target", builder.global_type_label(predicate.target_type)),
        dir::PredicateResolution::InstanceOf(predicate) => row
            .field("kind", "instanceof")
            .type_field("value", builder.global_type_label(predicate.value_type))
            .field("target", builder.symbol_path_label(predicate.target))
            .type_field(
                "target_type",
                builder.global_type_label(predicate.target_type),
            ),
        dir::PredicateResolution::In(predicate) => row
            .field("kind", "in")
            .type_field("key_type", builder.global_type_label(predicate.key_type))
            .type_field(
                "receiver",
                builder.global_type_label(predicate.receiver_type),
            )
            .optional_field("key", predicate.key.map(|key| builder.static_key(key))),
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
        dir::ConstructTarget::Variant(candidate) => row
            .field("kind", "variant")
            .field("owner", builder.symbol_path_label(candidate.owner))
            .field("variant", builder.symbol_path_label(candidate.variant))
            .optional_field(
                "instance",
                generic_instance_label(builder, candidate.owner, &candidate.arguments),
            )
            .field(
                "discriminant",
                builder.scalar_literal_label(&candidate.discriminant),
            ),
    };

    builder.push(row);
    add_construct_target_generic_instances(builder, node_id, &resolution.target);
}

/// Add one pattern resolution row.
fn add_pattern_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
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
        dir::PatternResolution::Tuple(tuple) => row.tuple_field(
            "fields",
            pattern_positional_field_labels(builder, segment, &tuple.fields),
        ),
        dir::PatternResolution::Sequence(sequence) => {
            add_pattern_sequence_fields(builder, segment, row, sequence)
        }
        dir::PatternResolution::Shape(shape) => row.object_field(
            "fields",
            pattern_keyed_fields_label(builder, segment, &shape.fields),
        ),
        dir::PatternResolution::Nominal(nominal) => row
            .field("target", builder.symbol_path_label(nominal.symbol))
            .optional_field(
                "instance",
                generic_instance_label(builder, nominal.symbol, &nominal.arguments),
            )
            .object_field(
                "fields",
                pattern_keyed_fields_label(builder, segment, &nominal.fields),
            ),
        dir::PatternResolution::Newtype(newtype) => row
            .field("target", builder.symbol_path_label(newtype.symbol))
            .optional_field(
                "instance",
                generic_instance_label(builder, newtype.symbol, &newtype.arguments),
            )
            .optional_field("value", newtype.value.map(|node| builder.node_label(node))),
        dir::PatternResolution::Variant(variant) => {
            let row = row
                .field("owner", builder.symbol_path_label(variant.owner))
                .field("variant", builder.symbol_path_label(variant.variant))
                .optional_field(
                    "instance",
                    generic_instance_label(builder, variant.owner, &variant.arguments),
                )
                .field(
                    "discriminant",
                    builder.scalar_literal_label(&variant.discriminant),
                )
                .optional_field(
                    "payload",
                    pattern_variant_payload_label(&variant.fields).map(str::to_string),
                );

            add_pattern_variant_fields(builder, segment, row, &variant.fields)
        }
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
    add_pattern_generic_instances(builder, node_id, resolution);
}

/// Return one receiver kind label.
fn receiver_kind_label(kind: dir::ReceiverKind) -> &'static str {
    match kind {
        dir::ReceiverKind::This => "this",
        dir::ReceiverKind::Super => "super",
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
        dir::PatternResolution::Shape(_) => "object",
        dir::PatternResolution::Nominal(_) => "nominal_object",
        dir::PatternResolution::Newtype(_) => "newtype",
        dir::PatternResolution::Variant(_) => "variant",
        dir::PatternResolution::Union(_) => "union",
        dir::PatternResolution::Borrow(_) => "borrow",
        dir::PatternResolution::Move(_) => "move",
        dir::PatternResolution::Dereference(_) => "dereference",
    }
}

/// Add one assignment pattern resolution row.
fn add_assign_pattern_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::AssignPatternResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "pattern.assign")
        .optional_field("source", builder.node_source(node_id))
        .field("kind", assign_pattern_resolution_label(resolution));

    let row = match resolution {
        dir::AssignPatternResolution::Place(place) => {
            row.field("target", builder.node_label(place.target))
        }
        dir::AssignPatternResolution::Default(default) => row
            .field(
                "pattern",
                assign_pattern_child_label(builder, segment, default.pattern),
            )
            .field("value", builder.node_label(default.value)),
        dir::AssignPatternResolution::Sequence(sequence) => row
            .tuple_field(
                "fields",
                assign_pattern_positional_field_labels(builder, segment, &sequence.fields),
            )
            .optional_field(
                "rest",
                sequence
                    .rest
                    .as_ref()
                    .map(|rest| assign_pattern_rest_label(builder, segment, rest)),
            ),
        dir::AssignPatternResolution::Object(object) => row
            .object_field(
                "fields",
                assign_pattern_keyed_fields_label(builder, segment, &object.fields),
            )
            .optional_field(
                "rest",
                object
                    .rest
                    .as_ref()
                    .map(|rest| assign_pattern_rest_label(builder, segment, rest)),
            ),
    };

    builder.push(row);
}

/// Return one assignment pattern resolution label.
fn assign_pattern_resolution_label(resolution: &dir::AssignPatternResolution) -> &'static str {
    match resolution {
        dir::AssignPatternResolution::Place(_) => "place",
        dir::AssignPatternResolution::Default(_) => "default",
        dir::AssignPatternResolution::Sequence(_) => "sequence",
        dir::AssignPatternResolution::Object(_) => "object",
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
        .optional_field(
            "instance",
            generic_instance_label(builder, candidate.symbol, &candidate.arguments),
        )
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
        .optional_field(
            "instance",
            generic_instance_label(builder, candidate.symbol, &candidate.arguments),
        )
}

/// Add direct newtype construct candidate fields.
fn add_construct_candidate_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    candidate: &dir::NewtypeConstructCandidate,
) -> SnapshotRow {
    row.field("target", builder.symbol_path_label(candidate.symbol))
        .optional_field(
            "instance",
            generic_instance_label(builder, candidate.symbol, &candidate.arguments),
        )
}

/// Add generic instance rows from one member target.
fn add_member_target_generic_instances(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    target: &dir::MemberTarget,
) {
    let anchor = builder.anchor_node(node_id);
    let source = builder.node_source(node_id);

    match target {
        dir::MemberTarget::Symbol(candidate) => {
            builder.add_generic_instance(anchor, source, candidate.symbol, &candidate.arguments);
        }
        dir::MemberTarget::Existential(candidates) | dir::MemberTarget::Universal(candidates) => {
            for candidate in candidates {
                builder.add_generic_instance(
                    anchor,
                    source.clone(),
                    candidate.symbol,
                    &candidate.arguments,
                );
            }
        }
        dir::MemberTarget::Field(_)
        | dir::MemberTarget::Element(_)
        | dir::MemberTarget::Index(_) => {}
    }
}

/// Add generic instance rows from one call target.
fn add_call_target_generic_instances(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    target: &dir::CallTarget,
) {
    let anchor = builder.anchor_node(node_id);
    let source = builder.node_source(node_id);

    match target {
        dir::CallTarget::Symbol(candidate) => {
            builder.add_generic_instance(anchor, source, candidate.symbol, &candidate.arguments);
        }
        dir::CallTarget::Universal(candidates) => {
            for candidate in candidates {
                builder.add_generic_instance(
                    anchor,
                    source.clone(),
                    candidate.symbol,
                    &candidate.arguments,
                );
            }
        }
        dir::CallTarget::Builtin(_) | dir::CallTarget::Expression { .. } => {}
    }
}

/// Add generic instance rows from one construct target.
fn add_construct_target_generic_instances(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    target: &dir::ConstructTarget,
) {
    let anchor = builder.anchor_node(node_id);
    let source = builder.node_source(node_id);

    match target {
        dir::ConstructTarget::Class(candidate) => {
            builder.add_generic_instance(anchor, source, candidate.symbol, &candidate.arguments);
        }
        dir::ConstructTarget::Newtype(candidate) => {
            builder.add_generic_instance(anchor, source, candidate.symbol, &candidate.arguments);
        }
        dir::ConstructTarget::Variant(candidate) => {
            builder.add_generic_instance(anchor, source, candidate.owner, &candidate.arguments);
        }
    }
}

/// Add generic instance rows from one pattern resolution.
fn add_pattern_generic_instances(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::PatternResolution,
) {
    let anchor = builder.anchor_node(node_id);
    let source = builder.node_source(node_id);

    match resolution {
        dir::PatternResolution::Nominal(nominal) => {
            builder.add_generic_instance(anchor, source, nominal.symbol, &nominal.arguments);
        }
        dir::PatternResolution::Newtype(newtype) => {
            builder.add_generic_instance(anchor, source, newtype.symbol, &newtype.arguments);
        }
        dir::PatternResolution::Variant(variant) => {
            builder.add_generic_instance(anchor, source, variant.owner, &variant.arguments);
        }
        dir::PatternResolution::Wildcard
        | dir::PatternResolution::Binding(_)
        | dir::PatternResolution::Literal(_)
        | dir::PatternResolution::Range(_)
        | dir::PatternResolution::Tuple(_)
        | dir::PatternResolution::Sequence(_)
        | dir::PatternResolution::Shape(_)
        | dir::PatternResolution::Union(_)
        | dir::PatternResolution::Borrow(_)
        | dir::PatternResolution::Move(_)
        | dir::PatternResolution::Dereference(_) => {}
    }
}

/// Render one applied generic declaration label.
fn generic_instance_label(
    builder: &DirSnapshotBuilder<'_>,
    symbol: dir::GlobalSymbolId,
    arguments: &[dir::GlobalTypeId],
) -> Option<String> {
    if arguments.is_empty() {
        return None;
    }

    Some(builder.generic_instance_label(symbol, arguments))
}

/// Add ordered pattern sequence fields.
fn add_pattern_sequence_fields(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    row: SnapshotRow,
    sequence: &dir::PatternSequenceResolution,
) -> SnapshotRow {
    match sequence {
        dir::PatternSequenceResolution::Array { fields, rest } => row
            .field("sequence", "array")
            .tuple_field(
                "fields",
                pattern_positional_field_labels(builder, segment, fields),
            )
            .optional_field(
                "rest",
                rest.as_ref()
                    .map(|rest| pattern_rest_label(builder, segment, rest)),
            ),
        dir::PatternSequenceResolution::Slice { fields, rest } => row
            .field("sequence", "slice")
            .tuple_field(
                "fields",
                pattern_positional_field_labels(builder, segment, fields),
            )
            .optional_field(
                "rest",
                rest.as_ref()
                    .map(|rest| pattern_rest_label(builder, segment, rest)),
            ),
        dir::PatternSequenceResolution::FixedArray { fields, length } => row
            .field("sequence", "fixed_array")
            .field("length", builder.global_type_label(*length))
            .tuple_field(
                "fields",
                pattern_positional_field_labels(builder, segment, fields),
            ),
    }
}

/// Add one variant payload field list.
fn add_pattern_variant_fields(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    row: SnapshotRow,
    fields: &[dir::PatternFieldResolution],
) -> SnapshotRow {
    if pattern_fields_are_positional(fields) {
        row.tuple_field(
            "fields",
            pattern_positional_field_labels(builder, segment, fields),
        )
    } else {
        row.object_field(
            "fields",
            pattern_keyed_fields_label(builder, segment, fields),
        )
    }
}

/// Return keyed pattern field labels.
fn pattern_keyed_field_labels<'a>(
    builder: &'a DirSnapshotBuilder<'_>,
    segment: &'a dir::ResolutionSegment,
    fields: &'a [dir::PatternFieldResolution],
) -> impl Iterator<Item = String> + 'a {
    fields
        .iter()
        .map(|field| pattern_keyed_field_label(builder, segment, field))
}

/// Return positional pattern field labels.
fn pattern_positional_field_labels<'a>(
    builder: &'a DirSnapshotBuilder<'_>,
    segment: &'a dir::ResolutionSegment,
    fields: &'a [dir::PatternFieldResolution],
) -> impl Iterator<Item = String> + 'a {
    fields
        .iter()
        .map(|field| pattern_positional_field_label(builder, segment, field))
}

/// Return one keyed pattern field group label.
fn pattern_keyed_fields_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    fields: &[dir::PatternFieldResolution],
) -> String {
    let fields = pattern_keyed_field_labels(builder, segment, fields)
        .collect::<Vec<_>>()
        .join(", ");
    if fields.is_empty() {
        return "{}".to_string();
    }

    format!("{{ {fields} }}")
}

/// Return one keyed pattern field label.
fn pattern_keyed_field_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    field: &dir::PatternFieldResolution,
) -> String {
    let target = match field.target {
        dir::PatternFieldTarget::Key(key) => builder.static_key(key),
        dir::PatternFieldTarget::Index(index) => index.to_string(),
    };

    let Some(pattern) = field.pattern else {
        return target;
    };

    let pattern = pattern_child_label(builder, segment, pattern);
    if target == pattern {
        return pattern;
    }

    format!("{target}: {pattern}")
}

/// Return one positional pattern field label.
fn pattern_positional_field_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    field: &dir::PatternFieldResolution,
) -> String {
    let Some(pattern) = field.pattern else {
        return match field.target {
            dir::PatternFieldTarget::Key(key) => builder.static_key(key),
            dir::PatternFieldTarget::Index(index) => index.to_string(),
        };
    };

    pattern_child_label(builder, segment, pattern)
}

/// Return one variant payload label.
fn pattern_variant_payload_label(fields: &[dir::PatternFieldResolution]) -> Option<&'static str> {
    if fields.is_empty() {
        return None;
    }

    if pattern_fields_are_positional(fields) {
        Some("tuple")
    } else {
        Some("object")
    }
}

/// Return whether all fields are positional.
fn pattern_fields_are_positional(fields: &[dir::PatternFieldResolution]) -> bool {
    fields
        .iter()
        .all(|field| matches!(field.target, dir::PatternFieldTarget::Index(_)))
}

/// Return one compact child pattern label.
fn pattern_child_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    pattern: dir::GlobalNodeIdAny,
) -> String {
    match segment.pattern_resolution(pattern) {
        Some(dir::PatternResolution::Wildcard) => "_".to_string(),
        Some(dir::PatternResolution::Binding(binding)) => binding
            .symbol
            .map(|symbol| builder.symbol_path_label(symbol))
            .unwrap_or_else(|| builder.node_label(pattern)),
        Some(dir::PatternResolution::Literal(literal)) => {
            builder.scalar_literal_label(&literal.value)
        }
        _ => builder.node_label(pattern),
    }
}

/// Return keyed assignment pattern field labels.
fn assign_pattern_keyed_field_labels<'a>(
    builder: &'a DirSnapshotBuilder<'_>,
    segment: &'a dir::ResolutionSegment,
    fields: &'a [dir::AssignPatternFieldResolution],
) -> impl Iterator<Item = String> + 'a {
    fields
        .iter()
        .map(|field| assign_pattern_keyed_field_label(builder, segment, field))
}

/// Return one keyed assignment pattern field group label.
fn assign_pattern_keyed_fields_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    fields: &[dir::AssignPatternFieldResolution],
) -> String {
    let fields = assign_pattern_keyed_field_labels(builder, segment, fields)
        .collect::<Vec<_>>()
        .join(", ");
    if fields.is_empty() {
        return "{}".to_string();
    }

    format!("{{ {fields} }}")
}

/// Return positional assignment pattern field labels.
fn assign_pattern_positional_field_labels<'a>(
    builder: &'a DirSnapshotBuilder<'_>,
    segment: &'a dir::ResolutionSegment,
    fields: &'a [dir::AssignPatternFieldResolution],
) -> impl Iterator<Item = String> + 'a {
    fields
        .iter()
        .map(|field| assign_pattern_positional_field_label(builder, segment, field))
}

/// Return one keyed assignment pattern field label.
fn assign_pattern_keyed_field_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    field: &dir::AssignPatternFieldResolution,
) -> String {
    let target = match field.target {
        dir::PatternFieldTarget::Key(key) => builder.static_key(key),
        dir::PatternFieldTarget::Index(index) => index.to_string(),
    };

    let Some(pattern) = field.pattern else {
        return target;
    };

    let pattern = assign_pattern_child_label(builder, segment, pattern);
    if target == pattern {
        return pattern;
    }

    format!("{target}: {pattern}")
}

/// Return one positional assignment pattern field label.
fn assign_pattern_positional_field_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    field: &dir::AssignPatternFieldResolution,
) -> String {
    let Some(pattern) = field.pattern else {
        return match field.target {
            dir::PatternFieldTarget::Key(key) => builder.static_key(key),
            dir::PatternFieldTarget::Index(index) => index.to_string(),
        };
    };

    assign_pattern_child_label(builder, segment, pattern)
}

/// Return one compact child assignment pattern label.
fn assign_pattern_child_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    pattern: dir::GlobalNodeIdAny,
) -> String {
    match segment.assign_pattern_resolution(pattern) {
        Some(dir::AssignPatternResolution::Place(place)) => {
            assign_pattern_place_label(builder, segment, place.target)
        }
        Some(dir::AssignPatternResolution::Default(default)) => {
            assign_pattern_child_label(builder, segment, default.pattern)
        }
        _ => builder.node_label(pattern),
    }
}

/// Return one compact assignment place label.
fn assign_pattern_place_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    target: dir::GlobalNodeIdAny,
) -> String {
    let Some(resolution) = segment.name_resolution(target) else {
        return builder.node_label(target);
    };

    let symbols = resolution.symbols();
    if symbols.len() != 1 {
        return builder.node_label(target);
    }

    builder.symbol_path_label(symbols[0])
}

/// Return one pattern rest label.
fn pattern_rest_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    rest: &dir::PatternRestResolution,
) -> String {
    let Some(pattern) = rest.pattern else {
        return "...".to_string();
    };

    let pattern = pattern_child_label(builder, segment, pattern);

    format!("...{pattern}")
}

/// Return one assignment pattern rest label.
fn assign_pattern_rest_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    rest: &dir::AssignPatternRestResolution,
) -> String {
    let Some(pattern) = rest.pattern else {
        return "...".to_string();
    };

    let pattern = assign_pattern_child_label(builder, segment, pattern);

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

    let arguments = arguments
        .iter()
        .map(|argument| builder.global_type_label(*argument))
        .collect::<Vec<_>>()
        .join(", ");

    Some(format!("({arguments})"))
}

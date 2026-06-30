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

        for (node_id, resolution) in self.place_entries() {
            add_place_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.guard_entries() {
            add_guard_resolution_row(builder, node_id, resolution);
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
        let place_count = self.place_entries().count();
        let guard_count = self.guard_entries().count();
        let construct_count = self.construct_entries().count();
        let pattern_count = self.pattern_entries().count();
        let assign_pattern_count = self.assign_pattern_entries().count();
        if name_count == 0
            && instantiation_count == 0
            && label_count == 0
            && receiver_count == 0
            && member_count == 0
            && call_count == 0
            && place_count == 0
            && guard_count == 0
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
            .count_field("places", place_count)
            .count_field("guards", guard_count)
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
    let arguments = generic_argument_values(&resolution.generic_arguments);
    let instance = builder.generic_instance_label(resolution.symbol, &arguments);
    let row = SnapshotRow::new(anchor, "resolution", "instantiation")
        .optional_field("source", source.clone())
        .field("target", builder.symbol_path_label(resolution.symbol))
        .field("instance", instance);

    builder.push(row);
    add_generic_instance(
        builder,
        anchor,
        source,
        resolution.symbol,
        &resolution.generic_arguments,
    );
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
        .field(
            "declaration",
            builder.symbol_path_label(resolution.declaration),
        )
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
                generic_instance_label(builder, candidate.symbol, &candidate.generic_arguments),
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
        .optional_field(
            "arguments",
            argument_bindings_label(builder, &resolution.arguments),
        )
        .type_field("return", builder.global_type_label(resolution.return_type));

    let row = match &resolution.target {
        dir::CallTarget::Builtin(builtin) => row
            .field("kind", "builtin")
            .field("builtin", builtin_call_label(*builtin)),
        dir::CallTarget::Expression { generic_arguments } => {
            row.field("kind", "expression").optional_field(
                "generic_arguments",
                generic_arguments_label(builder, generic_arguments),
            )
        }
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

/// Add one place resolution row.
fn add_place_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::PlaceResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "place")
        .optional_field("source", builder.node_source(node_id))
        .field("place", storage_label(builder, &resolution.storage))
        .type_field("type", builder.global_type_label(resolution.ty));

    builder.push(row);
}

/// Add one guard resolution row.
fn add_guard_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::GuardResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "guard")
        .optional_field("source", builder.node_source(node_id));

    let predicate = match resolution {
        dir::GuardResolution::Is(predicate) => &predicate.predicate,
        dir::GuardResolution::InstanceOf(predicate) => &predicate.predicate,
        dir::GuardResolution::In(predicate) => &predicate.predicate,
    };

    let row = match resolution {
        dir::GuardResolution::Is(predicate) => {
            let row = row
                .field("kind", "is")
                .type_field("value", builder.global_type_label(predicate.value_type))
                .type_field("target", builder.global_type_label(predicate.target_type));

            add_predicate_fields(builder, row, &predicate.predicate)
        }
        dir::GuardResolution::InstanceOf(predicate) => {
            let row = row
                .field("kind", "instanceof")
                .type_field("value", builder.global_type_label(predicate.value_type))
                .field("target", builder.symbol_path_label(predicate.target))
                .type_field(
                    "target_type",
                    builder.global_type_label(predicate.target_type),
                );

            add_predicate_fields(builder, row, &predicate.predicate)
        }
        dir::GuardResolution::In(predicate) => {
            let row = row
                .field("kind", "in")
                .type_field("key_type", builder.global_type_label(predicate.key_type))
                .type_field(
                    "receiver",
                    builder.global_type_label(predicate.receiver_type),
                );

            add_predicate_fields(builder, row, &predicate.predicate)
        }
    };

    builder.push(row);
    add_predicate_generic_instances(builder, node_id, predicate);
}

/// Add common predicate fields to one row.
fn add_predicate_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    predicate: &dir::Predicate,
) -> SnapshotRow {
    row.field("predicate", predicate_label(builder, predicate))
        .optional_type_field(
            "narrowed",
            predicate.narrowed.map(|ty| builder.global_type_label(ty)),
        )
        .optional_field(
            "projection",
            predicate
                .projection
                .as_ref()
                .map(|projection| projection_label(builder, projection)),
        )
}

/// Return one projection snapshot label.
fn projection_label(builder: &DirSnapshotBuilder<'_>, projection: &dir::Projection) -> String {
    match projection {
        dir::Projection::FieldGet { field, ty } => format!(
            "field.get({}, {})",
            projection_field_label(builder, field),
            builder.global_type_label(*ty)
        ),
        dir::Projection::PropertyGet { read, ty } => format!(
            "property.get({}, {})",
            getter_label(builder, read),
            builder.global_type_label(*ty)
        ),
        dir::Projection::SubscriptGet { index, read, ty } => format!(
            "subscript.get({}, {}, {})",
            builder
                .node_source(*index)
                .unwrap_or_else(|| builder.node_label(*index)),
            subscript_operation_label(builder, read),
            builder.global_type_label(*ty)
        ),
        dir::Projection::Call { call, ty } => format!(
            "call({}, {})",
            call_target_label(builder, &call.target),
            builder.global_type_label(*ty)
        ),
        dir::Projection::ObjectRest { fields, ty } => {
            let fields = fields
                .iter()
                .map(|field| {
                    format!(
                        "{}: {}",
                        builder.static_key(field.key),
                        projection_label(builder, &field.projection)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");

            format!(
                "object.rest({{ {fields} }}, {})",
                builder.global_type_label(*ty)
            )
        }
        dir::Projection::SliceLength { ty } => {
            format!("slice.length({})", builder.global_type_label(*ty))
        }
        dir::Projection::DynamicPayload { ty } => {
            format!("dynamic.payload({})", builder.global_type_label(*ty))
        }
        dir::Projection::DynamicType { ty } => {
            format!("dynamic.type({})", builder.global_type_label(*ty))
        }
        dir::Projection::VariantTag { ty } => {
            format!("variant.tag({})", builder.global_type_label(*ty))
        }
        dir::Projection::VariantPayload {
            case,
            generic_arguments,
            ty,
            ..
        } => {
            let arguments = projection_generic_arguments_label(builder, generic_arguments);

            format!(
                "variant.payload({}.{}{arguments}, {})",
                builder.symbol_path_label(case.owner),
                builder.static_key(case.key),
                builder.global_type_label(*ty)
            )
        }
        dir::Projection::NewtypePayload {
            symbol,
            generic_arguments,
            ty,
        } => {
            let arguments = projection_generic_arguments_label(builder, generic_arguments);

            format!(
                "newtype.payload({}{}, {})",
                builder.symbol_path_label(*symbol),
                arguments,
                builder.global_type_label(*ty)
            )
        }
        dir::Projection::Borrow { access, ty } => {
            let access = access
                .map(|access| dir::MemoryLiteral::Access(access).text().to_string())
                .unwrap_or_else(|| "inferred".to_string());

            format!("borrow({access}, {})", builder.global_type_label(*ty))
        }
        dir::Projection::Move { access, ty } => {
            let access = access
                .map(|access| dir::MemoryLiteral::Access(access).text().to_string())
                .unwrap_or_else(|| "inferred".to_string());

            format!("move({access}, {})", builder.global_type_label(*ty))
        }
        dir::Projection::Dereference { read, ty } => {
            format!(
                "dereference({}, {})",
                dereference_operation_label(builder, read),
                builder.global_type_label(*ty)
            )
        }
    }
}

/// Return one subscript operation snapshot label.
fn subscript_operation_label(
    builder: &DirSnapshotBuilder<'_>,
    operation: &dir::SubscriptOperation,
) -> String {
    match operation {
        dir::SubscriptOperation::Member(member) => {
            format!("member({})", member_target_label(builder, &member.target))
        }
        dir::SubscriptOperation::Call(call) => call_target_label(builder, &call.target),
    }
}

/// Return one storage snapshot label.
fn storage_label(builder: &DirSnapshotBuilder<'_>, storage: &dir::Storage) -> String {
    match storage {
        dir::Storage::Binding { symbol } => {
            format!("binding({})", builder.symbol_path_label(*symbol))
        }
        dir::Storage::Field { field, .. } => {
            format!("field({})", projection_field_label(builder, field))
        }
        dir::Storage::Property { read, write } => {
            let write = setter_label(builder, write);
            match read {
                Some(read) => format!("property({}, {write})", getter_label(builder, read)),
                None => format!("property({write})"),
            }
        }
        dir::Storage::Subscript { read, write, .. } => {
            let write = subscript_operation_label(builder, write);
            match read {
                Some(read) => format!(
                    "subscript({}, {write})",
                    subscript_operation_label(builder, read)
                ),
                None => format!("subscript({write})"),
            }
        }
        dir::Storage::Dereference { read, write } => {
            let write = dereference_operation_label(builder, write);
            match read {
                Some(read) => {
                    format!(
                        "dereference({}, {write})",
                        dereference_operation_label(builder, read)
                    )
                }
                None => format!("dereference({write})"),
            }
        }
    }
}

/// Return one getter snapshot label.
fn getter_label(builder: &DirSnapshotBuilder<'_>, member: &dir::MemberResolution) -> String {
    format!("getter({})", member_target_label(builder, &member.target))
}

/// Return one setter snapshot label.
fn setter_label(builder: &DirSnapshotBuilder<'_>, member: &dir::MemberResolution) -> String {
    format!("setter({})", member_target_label(builder, &member.target))
}

/// Return one dereference operation snapshot label.
fn dereference_operation_label(
    builder: &DirSnapshotBuilder<'_>,
    operation: &dir::DereferenceOperation,
) -> String {
    match operation {
        dir::DereferenceOperation::Direct => "direct".to_string(),
        dir::DereferenceOperation::Call(call) => call_target_label(builder, &call.target),
    }
}

/// Return one member target snapshot label.
fn member_target_label(builder: &DirSnapshotBuilder<'_>, target: &dir::MemberTarget) -> String {
    match target {
        dir::MemberTarget::Field(key) => format!("field({})", builder.static_key(*key)),
        dir::MemberTarget::Element(index) => format!("element({index})"),
        dir::MemberTarget::Index(key) => {
            format!("index({})", builder.global_type_label(*key))
        }
        dir::MemberTarget::Symbol(candidate) => builder.member_candidate_label(candidate),
        dir::MemberTarget::Existential(candidates) => candidates
            .iter()
            .map(|candidate| builder.member_candidate_label(candidate))
            .collect::<Vec<_>>()
            .join(" | "),
        dir::MemberTarget::Universal(candidates) => candidates
            .iter()
            .map(|candidate| builder.member_candidate_label(candidate))
            .collect::<Vec<_>>()
            .join(" & "),
    }
}

/// Return one projected field snapshot label.
fn projection_field_label(
    builder: &DirSnapshotBuilder<'_>,
    field: &dir::ProjectionField,
) -> String {
    match field {
        dir::ProjectionField::Key(key) => builder.static_key(*key),
        dir::ProjectionField::Member(symbol) => builder.symbol_path_label(*symbol),
    }
}

/// Return generic arguments for one projection label.
fn projection_generic_arguments_label(
    builder: &DirSnapshotBuilder<'_>,
    arguments: &[dir::GenericArgumentBinding],
) -> String {
    if arguments.is_empty() {
        String::new()
    } else {
        let arguments = arguments
            .iter()
            .map(|argument| builder.global_type_label(argument.argument))
            .collect::<Vec<_>>()
            .join(", ");

        format!("<{arguments}>")
    }
}

/// Return one predicate snapshot label.
fn predicate_label(builder: &DirSnapshotBuilder<'_>, predicate: &dir::Predicate) -> String {
    match &predicate.test {
        dir::PredicateTest::Unary(test) => format!(
            "{} is {}",
            predicate_operand_label(builder, &test.input),
            predicate_condition_label(builder, &test.condition)
        ),
        dir::PredicateTest::Has(test) => format!(
            "has({}, {})",
            predicate_operand_label(builder, &test.receiver),
            predicate_key_label(builder, &test.key)
        ),
        dir::PredicateTest::Call(resolution) => {
            format!("call({})", call_target_label(builder, &resolution.target))
        }
        dir::PredicateTest::Any(alternatives) => alternatives
            .iter()
            .map(|predicate| predicate_label(builder, predicate))
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

/// Return one predicate condition snapshot label.
fn predicate_condition_label(
    builder: &DirSnapshotBuilder<'_>,
    condition: &dir::PredicateCondition,
) -> String {
    match condition {
        dir::PredicateCondition::Always => "always".to_string(),
        dir::PredicateCondition::Never => "never".to_string(),
        dir::PredicateCondition::Literal(value) => builder.scalar_literal_label(value),
        dir::PredicateCondition::Range(range) => range_label(builder, range),
        dir::PredicateCondition::Primitive(primitive) => primitive_label(*primitive),
        dir::PredicateCondition::Type(ty) => format!("type({})", builder.global_type_label(*ty)),
        dir::PredicateCondition::Subtype(ty) => {
            format!("subtype({})", builder.global_type_label(*ty))
        }
    }
}

/// Return one predicate key snapshot label.
fn predicate_key_label(builder: &DirSnapshotBuilder<'_>, key: &dir::PredicateKey) -> String {
    match key {
        dir::PredicateKey::Static(key) => builder.static_key(*key),
        dir::PredicateKey::Dynamic(operand) => predicate_operand_label(builder, operand),
    }
}

/// Return one predicate operand snapshot label.
fn predicate_operand_label(
    builder: &DirSnapshotBuilder<'_>,
    operand: &dir::PredicateOperand,
) -> String {
    match &operand.projection {
        Some(projection) => projection_label(builder, projection),
        None => builder.global_type_label(operand.ty),
    }
}

/// Return one call target snapshot label.
fn call_target_label(builder: &DirSnapshotBuilder<'_>, target: &dir::CallTarget) -> String {
    match target {
        dir::CallTarget::Builtin(builtin) => builtin_call_label(*builtin),
        dir::CallTarget::Expression { .. } => "expression".to_string(),
        dir::CallTarget::Symbol(candidate) => builder.call_candidate_label(candidate),
        dir::CallTarget::Universal(candidates) => candidates
            .iter()
            .map(|candidate| builder.call_candidate_label(candidate))
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

/// Return one range condition snapshot label.
fn range_label(builder: &DirSnapshotBuilder<'_>, range: &dir::PredicateRange) -> String {
    let start = range
        .start
        .map(|value| builder.scalar_literal_label(&value))
        .unwrap_or_default();
    let end = range
        .end
        .map(|value| builder.scalar_literal_label(&value))
        .unwrap_or_default();
    let operator = match range.end_bound {
        dir::RangeEnd::Inclusive => "..=",
        dir::RangeEnd::Open => "..",
    };

    format!("{start}{operator}{end}")
}

/// Return the canonical label for one primitive predicate.
fn primitive_label(primitive: dir::PrimitiveType) -> String {
    match primitive {
        dir::PrimitiveType::Boolean => "boolean".to_string(),
        dir::PrimitiveType::Character => "char".to_string(),
        dir::PrimitiveType::String => "string".to_string(),
        dir::PrimitiveType::Bigint => "bigint".to_string(),
        dir::PrimitiveType::Integer(integer) => integer_label(integer),
        dir::PrimitiveType::Float(float) => float_label(float),
        dir::PrimitiveType::Symbol => "symbol".to_string(),
        dir::PrimitiveType::UniqueSymbol => "unique symbol".to_string(),
    }
}

/// Return the canonical label for one integer predicate.
fn integer_label(integer: dir::IntegerType) -> String {
    match integer {
        dir::IntegerType::Integer { is_signed: true } => "int".to_string(),
        dir::IntegerType::Integer { is_signed: false } => "uint".to_string(),
        dir::IntegerType::Fixed {
            width,
            is_signed: true,
        } => format!("int{width}"),
        dir::IntegerType::Fixed {
            width,
            is_signed: false,
        } => format!("uint{width}"),
        dir::IntegerType::Pointer { is_signed: true } => "isize".to_string(),
        dir::IntegerType::Pointer { is_signed: false } => "usize".to_string(),
    }
}

/// Return the canonical label for one float predicate.
fn float_label(float: dir::FloatType) -> String {
    match float {
        dir::FloatType::Float => "float".to_string(),
        dir::FloatType::Float16 => "float16".to_string(),
        dir::FloatType::Bfloat16 => "bfloat16".to_string(),
        dir::FloatType::Float32 => "float32".to_string(),
        dir::FloatType::Float64 => "float64".to_string(),
    }
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
        .optional_field(
            "arguments",
            argument_bindings_label(builder, &resolution.arguments),
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
            .field("owner", builder.symbol_path_label(candidate.case.owner))
            .field("variant", builder.static_key(candidate.case.key))
            .optional_field(
                "instance",
                generic_instance_label(builder, candidate.case.owner, &candidate.generic_arguments),
            )
            .field(
                "discriminant",
                builder.scalar_literal_value_label(&candidate.discriminant),
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
        dir::PatternResolution::Ignore => row,
        dir::PatternResolution::Bind(binding) => row
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
        dir::PatternResolution::Must(pattern) => {
            row.field("pattern", builder.node_label(pattern.pattern))
        }
        dir::PatternResolution::Default(default) => row
            .field("pattern", builder.node_label(default.pattern))
            .field("value", builder.node_label(default.value)),
        dir::PatternResolution::Test(test) => {
            add_pattern_test_fields(builder, row, &test.predicate)
        }
        dir::PatternResolution::Project(project) => row
            .field("projection", projection_label(builder, &project.projection))
            .optional_field(
                "pattern",
                project.pattern.map(|node| builder.node_label(node)),
            ),
        dir::PatternResolution::Destructure(destructure) => {
            add_pattern_destructure_fields(builder, segment, row, destructure)
        }
        dir::PatternResolution::Or(pattern) => row.list_field(
            "patterns",
            pattern
                .patterns
                .iter()
                .map(|node| builder.node_label(*node)),
        ),
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
        dir::PatternResolution::Ignore => "wildcard",
        dir::PatternResolution::Bind(_) => "binding",
        dir::PatternResolution::Must(_) => "must",
        dir::PatternResolution::Default(_) => "default",
        dir::PatternResolution::Test(test) => pattern_predicate_label(&test.predicate),
        dir::PatternResolution::Project(project) => pattern_projection_label(&project.projection),
        dir::PatternResolution::Destructure(destructure) => pattern_destructure_label(destructure),
        dir::PatternResolution::Or(_) => "union",
    }
}

/// Return one pattern predicate label.
fn pattern_predicate_label(predicate: &dir::Predicate) -> &'static str {
    match predicate_condition(predicate) {
        Some(dir::PredicateCondition::Literal(_)) => "literal",
        Some(dir::PredicateCondition::Range(_)) => "range",
        _ => "test",
    }
}

/// Return one pattern projection label.
fn pattern_projection_label(projection: &dir::Projection) -> &'static str {
    match projection {
        dir::Projection::Borrow { .. } => "borrow",
        dir::Projection::Move { .. } => "move",
        dir::Projection::Dereference { .. } => "dereference",
        dir::Projection::NewtypePayload { .. } => "newtype",
        _ => "project",
    }
}

/// Return one pattern destructure label.
fn pattern_destructure_label(destructure: &dir::PatternDestructureResolution) -> &'static str {
    match destructure {
        dir::PatternDestructureResolution::Tuple(_) => "tuple",
        dir::PatternDestructureResolution::Object(_) => "object",
        dir::PatternDestructureResolution::Nominal(_) => "nominal_object",
        dir::PatternDestructureResolution::Sequence(_) => "sequence",
        dir::PatternDestructureResolution::Variant(_) => "variant",
    }
}

/// Add fields for one pattern predicate.
fn add_pattern_test_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    predicate: &dir::Predicate,
) -> SnapshotRow {
    match predicate_condition(predicate) {
        Some(dir::PredicateCondition::Literal(value)) => {
            row.verbatim_field("value", builder.scalar_literal_label(value))
        }
        Some(dir::PredicateCondition::Range(range)) => row
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
        _ => row.field("predicate", predicate_label(builder, predicate)),
    }
}

/// Return the unary condition for one predicate.
fn predicate_condition(predicate: &dir::Predicate) -> Option<&dir::PredicateCondition> {
    match &predicate.test {
        dir::PredicateTest::Unary(test) => Some(&test.condition),
        dir::PredicateTest::Has(_) | dir::PredicateTest::Call(_) | dir::PredicateTest::Any(_) => {
            None
        }
    }
}

/// Add fields for one pattern destructure.
fn add_pattern_destructure_fields(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    row: SnapshotRow,
    destructure: &dir::PatternDestructureResolution,
) -> SnapshotRow {
    match destructure {
        dir::PatternDestructureResolution::Tuple(tuple) => row.tuple_field(
            "fields",
            pattern_positional_field_labels(builder, segment, &tuple.fields),
        ),
        dir::PatternDestructureResolution::Object(object) => row
            .object_field(
                "fields",
                pattern_keyed_fields_label(builder, segment, &object.fields),
            )
            .optional_field(
                "rest",
                object
                    .rest
                    .as_ref()
                    .map(|rest| pattern_rest_label(builder, segment, rest)),
            ),
        dir::PatternDestructureResolution::Nominal(nominal) => row
            .field("target", builder.symbol_path_label(nominal.symbol))
            .optional_field(
                "instance",
                generic_instance_label(builder, nominal.symbol, &nominal.generic_arguments),
            )
            .object_field(
                "fields",
                pattern_keyed_fields_label(builder, segment, &nominal.fields),
            )
            .optional_field(
                "rest",
                nominal
                    .rest
                    .as_ref()
                    .map(|rest| pattern_rest_label(builder, segment, rest)),
            ),
        dir::PatternDestructureResolution::Sequence(sequence) => add_pattern_sequence_fields(
            builder,
            segment,
            add_pattern_sequence_arity_field(
                row.optional_field(
                    "element",
                    sequence
                        .fields
                        .first()
                        .map(|field| builder.global_type_label(field.projection.ty())),
                ),
                &sequence.arity,
            ),
            &sequence.fields,
            sequence.rest.as_ref(),
        ),
        dir::PatternDestructureResolution::Variant(variant) => {
            let row = row
                .field("predicate", predicate_label(builder, &variant.predicate))
                .field("projection", projection_label(builder, &variant.projection))
                .optional_field(
                    "payload",
                    pattern_variant_payload_label(&variant.fields).map(str::to_string),
                );

            add_pattern_variant_fields(builder, segment, row, &variant.fields)
        }
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
        dir::AssignPatternResolution::Place(place) => row
            .field("place", storage_label(builder, &place.storage))
            .type_field("type", builder.global_type_label(place.ty)),
        dir::AssignPatternResolution::Default(default) => row
            .field(
                "pattern",
                assign_pattern_child_label(builder, segment, default.pattern),
            )
            .field("value", builder.node_label(default.value)),
        dir::AssignPatternResolution::Sequence(sequence) => add_assign_pattern_sequence_fields(
            builder,
            segment,
            add_pattern_sequence_arity_field(
                row.optional_field(
                    "element",
                    sequence
                        .fields
                        .first()
                        .map(|field| builder.global_type_label(field.projection.ty())),
                ),
                &sequence.arity,
            ),
            &sequence.fields,
            sequence.rest.as_ref(),
        ),
        dir::AssignPatternResolution::Tuple(tuple) => row.tuple_field(
            "fields",
            assign_pattern_positional_field_labels(builder, segment, &tuple.fields),
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
                    .map(|rest| assign_pattern_object_rest_label(builder, segment, rest)),
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
        dir::AssignPatternResolution::Tuple(_) => "tuple",
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
            generic_instance_label(builder, candidate.symbol, &candidate.generic_arguments),
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
            class_constructor_label(builder, &candidate.constructor),
        )
        .optional_field(
            "instance",
            generic_instance_label(builder, candidate.symbol, &candidate.generic_arguments),
        )
}

/// Return the selected class constructor label.
fn class_constructor_label(
    builder: &DirSnapshotBuilder<'_>,
    constructor: &dir::ClassConstructor,
) -> Option<String> {
    match constructor {
        dir::ClassConstructor::Declared { symbol } => Some(builder.symbol_path_label(*symbol)),
        dir::ClassConstructor::Default => Some("default".to_string()),
        dir::ClassConstructor::ForwardedDeclared { symbol, .. } => {
            Some(format!("forwarded:{}", builder.symbol_path_label(*symbol)))
        }
        dir::ClassConstructor::ForwardedDefault { base } => Some(format!(
            "forwarded:{}.default",
            builder.symbol_path_label(*base)
        )),
    }
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
            generic_instance_label(builder, candidate.symbol, &candidate.generic_arguments),
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
            add_generic_instance(
                builder,
                anchor,
                source,
                candidate.symbol,
                &candidate.generic_arguments,
            );
        }
        dir::MemberTarget::Existential(candidates) | dir::MemberTarget::Universal(candidates) => {
            for candidate in candidates {
                add_generic_instance(
                    builder,
                    anchor,
                    source.clone(),
                    candidate.symbol,
                    &candidate.generic_arguments,
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
            add_generic_instance(
                builder,
                anchor,
                source,
                candidate.symbol,
                &candidate.generic_arguments,
            );
        }
        dir::CallTarget::Universal(candidates) => {
            for candidate in candidates {
                add_generic_instance(
                    builder,
                    anchor,
                    source.clone(),
                    candidate.symbol,
                    &candidate.generic_arguments,
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
            add_generic_instance(
                builder,
                anchor,
                source,
                candidate.symbol,
                &candidate.generic_arguments,
            );
        }
        dir::ConstructTarget::Newtype(candidate) => {
            add_generic_instance(
                builder,
                anchor,
                source,
                candidate.symbol,
                &candidate.generic_arguments,
            );
        }
        dir::ConstructTarget::Variant(candidate) => {
            add_generic_instance(
                builder,
                anchor,
                source,
                candidate.case.owner,
                &candidate.generic_arguments,
            );
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
        dir::PatternResolution::Project(project) => {
            add_projection_generic_instance(builder, anchor, source, &project.projection);
        }
        dir::PatternResolution::Test(test) => {
            add_predicate_generic_instances(builder, node_id, &test.predicate);
        }
        dir::PatternResolution::Destructure(destructure) => {
            add_destructure_generic_instance(builder, anchor, source, destructure);
        }
        dir::PatternResolution::Ignore
        | dir::PatternResolution::Bind(_)
        | dir::PatternResolution::Must(_)
        | dir::PatternResolution::Default(_)
        | dir::PatternResolution::Or(_) => {}
    }
}

/// Add generic instance rows from one predicate.
fn add_predicate_generic_instances(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    predicate: &dir::Predicate,
) {
    let anchor = builder.anchor_node(node_id);
    let source = builder.node_source(node_id);

    match &predicate.test {
        dir::PredicateTest::Unary(test) => {
            add_predicate_operand_generic_instance(builder, anchor, source.clone(), &test.input);
        }
        dir::PredicateTest::Has(test) => {
            add_predicate_operand_generic_instance(builder, anchor, source.clone(), &test.receiver);
            if let dir::PredicateKey::Dynamic(operand) = &test.key {
                add_predicate_operand_generic_instance(builder, anchor, source.clone(), operand);
            }
        }
        dir::PredicateTest::Call(resolution) => {
            add_call_target_generic_instances(builder, node_id, &resolution.target);
        }
        dir::PredicateTest::Any(alternatives) => {
            for alternative in alternatives {
                add_predicate_generic_instances(builder, node_id, alternative);
            }
        }
    }

    if let Some(projection) = &predicate.projection {
        add_projection_generic_instance(builder, anchor, source, projection);
    }
}

/// Add generic instance rows from one predicate operand.
fn add_predicate_operand_generic_instance(
    builder: &mut DirSnapshotBuilder<'_>,
    anchor: SnapshotAnchor,
    source: Option<String>,
    operand: &dir::PredicateOperand,
) {
    if let Some(projection) = &operand.projection {
        add_projection_generic_instance(builder, anchor, source, projection);
    }
}

/// Add generic instance rows from one projection.
fn add_projection_generic_instance(
    builder: &mut DirSnapshotBuilder<'_>,
    anchor: SnapshotAnchor,
    source: Option<String>,
    projection: &dir::Projection,
) {
    match projection {
        dir::Projection::NewtypePayload {
            symbol,
            generic_arguments,
            ..
        } => {
            add_generic_instance(builder, anchor, source, *symbol, generic_arguments);
        }
        dir::Projection::VariantPayload {
            case,
            generic_arguments,
            ..
        } => {
            add_generic_instance(builder, anchor, source, case.owner, generic_arguments);
        }
        _ => {}
    }
}

/// Add generic instance rows from one destructure resolution.
fn add_destructure_generic_instance(
    builder: &mut DirSnapshotBuilder<'_>,
    anchor: SnapshotAnchor,
    source: Option<String>,
    destructure: &dir::PatternDestructureResolution,
) {
    match destructure {
        dir::PatternDestructureResolution::Nominal(nominal) => {
            add_generic_instance(
                builder,
                anchor,
                source,
                nominal.symbol,
                &nominal.generic_arguments,
            );
        }
        dir::PatternDestructureResolution::Variant(variant) => {
            add_projection_generic_instance(builder, anchor, source, &variant.projection);
        }
        dir::PatternDestructureResolution::Tuple(_)
        | dir::PatternDestructureResolution::Object(_)
        | dir::PatternDestructureResolution::Sequence(_) => {}
    }
}

/// Render one applied generic declaration label.
fn generic_instance_label(
    builder: &DirSnapshotBuilder<'_>,
    symbol: dir::GlobalSymbolId,
    arguments: &[dir::GenericArgumentBinding],
) -> Option<String> {
    let arguments = generic_instance_arguments(builder, symbol, arguments);
    if arguments.is_empty() {
        return None;
    }

    Some(builder.generic_instance_label(symbol, &arguments))
}

/// Add one generic instance row from selected argument bindings.
fn add_generic_instance(
    builder: &mut DirSnapshotBuilder<'_>,
    anchor: SnapshotAnchor,
    source: Option<String>,
    symbol: dir::GlobalSymbolId,
    arguments: &[dir::GenericArgumentBinding],
) {
    let arguments = generic_instance_arguments(builder, symbol, arguments);

    builder.add_generic_instance(anchor, source, symbol, &arguments);
}

/// Add ordered pattern sequence fields.
fn add_pattern_sequence_fields(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    row: SnapshotRow,
    fields: &[dir::PatternFieldResolution],
    rest: Option<&dir::PatternFieldResolution>,
) -> SnapshotRow {
    row.tuple_field(
        "fields",
        pattern_positional_field_labels(builder, segment, fields),
    )
    .optional_field(
        "rest",
        rest.map(|rest| pattern_rest_label(builder, segment, rest)),
    )
}

/// Add one sequence pattern arity requirement.
fn add_pattern_sequence_arity_field(
    row: SnapshotRow,
    arity: &dir::PatternSequenceArity,
) -> SnapshotRow {
    let label = match arity.maximum {
        Some(maximum) if maximum == arity.minimum => arity.minimum.to_string(),
        Some(maximum) => format!("{}..{}", arity.minimum, maximum),
        None => format!("{}..", arity.minimum),
    };

    row.field("arity", label)
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
    let target = pattern_field_target_label(builder, &field.projection);

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
        return pattern_field_target_label(builder, &field.projection);
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
    fields.iter().all(|field| {
        matches!(
            field.projection,
            dir::Projection::FieldGet {
                field: dir::ProjectionField::Key(dir::StaticKey::Index(_)),
                ..
            }
        )
    })
}

/// Return the source-facing target label for one pattern field projection.
fn pattern_field_target_label(
    builder: &DirSnapshotBuilder<'_>,
    projection: &dir::Projection,
) -> String {
    match projection {
        dir::Projection::FieldGet { field, .. } => projection_field_label(builder, field),
        dir::Projection::SubscriptGet { index, .. } => builder
            .node_source(*index)
            .unwrap_or_else(|| builder.node_label(*index)),
        _ => projection_label(builder, projection),
    }
}

/// Return one child pattern snapshot label.
fn pattern_child_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    pattern: dir::GlobalNodeIdAny,
) -> String {
    match segment.pattern_resolution(pattern) {
        Some(dir::PatternResolution::Ignore) => "_".to_string(),
        Some(dir::PatternResolution::Bind(binding)) => binding
            .symbol
            .map(|symbol| builder.symbol_path_label(symbol))
            .unwrap_or_else(|| builder.node_label(pattern)),
        Some(dir::PatternResolution::Must(must)) => {
            pattern_child_label(builder, segment, must.pattern)
        }
        Some(dir::PatternResolution::Default(default)) => {
            pattern_child_label(builder, segment, default.pattern)
        }
        Some(dir::PatternResolution::Test(test)) => match predicate_condition(&test.predicate) {
            Some(dir::PredicateCondition::Literal(value)) => builder.scalar_literal_label(value),
            _ => builder.node_label(pattern),
        },
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

/// Return one keyed assignment pattern field label.
fn assign_pattern_keyed_field_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    field: &dir::AssignPatternFieldResolution,
) -> String {
    let target = pattern_field_target_label(builder, &field.projection);

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
        return "_".to_string();
    };

    assign_pattern_child_label(builder, segment, pattern)
}

/// Return one child assignment pattern snapshot label.
fn assign_pattern_child_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    pattern: dir::GlobalNodeIdAny,
) -> String {
    match segment.assign_pattern_resolution(pattern) {
        Some(dir::AssignPatternResolution::Place(place)) => {
            place_source_label(builder, segment, place)
        }
        Some(dir::AssignPatternResolution::Default(default)) => {
            assign_pattern_child_label(builder, segment, default.pattern)
        }
        _ => builder.node_label(pattern),
    }
}

/// Return one place source snapshot label.
fn place_source_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    place: &dir::PlaceResolution,
) -> String {
    builder
        .node_source(place.source)
        .unwrap_or_else(|| assign_pattern_place_label(builder, segment, place.source))
}

/// Return one assignment place snapshot label.
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
    rest: &dir::PatternFieldResolution,
) -> String {
    let Some(pattern) = rest.pattern else {
        return "...".to_string();
    };

    let pattern = pattern_child_label(builder, segment, pattern);

    format!("...{pattern}")
}

/// Add ordered assignment pattern sequence fields.
fn add_assign_pattern_sequence_fields(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    row: SnapshotRow,
    fields: &[dir::AssignPatternFieldResolution],
    rest: Option<&dir::AssignPatternFieldResolution>,
) -> SnapshotRow {
    row.tuple_field(
        "fields",
        assign_pattern_positional_field_labels(builder, segment, fields),
    )
    .optional_field(
        "rest",
        rest.map(|rest| assign_pattern_rest_label(builder, segment, rest)),
    )
}

/// Return one assignment pattern rest label.
fn assign_pattern_rest_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionSegment,
    rest: &dir::AssignPatternFieldResolution,
) -> String {
    let Some(pattern) = rest.pattern else {
        return "...".to_string();
    };

    let pattern = assign_pattern_child_label(builder, segment, pattern);

    format!("...{pattern}")
}

/// Return one object assignment pattern rest label.
fn assign_pattern_object_rest_label(
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
fn generic_arguments_label(
    builder: &DirSnapshotBuilder<'_>,
    arguments: &[dir::GenericArgumentBinding],
) -> Option<String> {
    if arguments.is_empty() {
        return None;
    }

    let arguments = arguments
        .iter()
        .map(|argument| builder.global_type_label(argument.argument))
        .collect::<Vec<_>>()
        .join(", ");

    Some(format!("({arguments})"))
}

/// Render one runtime argument binding list label.
fn argument_bindings_label(
    builder: &DirSnapshotBuilder<'_>,
    arguments: &[dir::ArgumentBinding],
) -> Option<String> {
    if arguments.is_empty() {
        return None;
    }

    let arguments = arguments
        .iter()
        .map(|argument| argument_binding_label(builder, argument))
        .collect::<Vec<_>>()
        .join(", ");

    Some(format!("({arguments})"))
}

/// Render one runtime argument binding label.
fn argument_binding_label(
    builder: &DirSnapshotBuilder<'_>,
    binding: &dir::ArgumentBinding,
) -> String {
    let source = match &binding.argument {
        dir::ArgumentSource::Provided(node) => {
            let source = builder
                .node_source(*node)
                .unwrap_or_else(|| builder.node_label(*node));

            format!("provided({source})")
        }
        dir::ArgumentSource::Static(ty) => {
            format!("static({})", builder.global_type_label(*ty))
        }
        dir::ArgumentSource::Omitted => "omitted".to_string(),
        dir::ArgumentSource::Rest(nodes) => {
            let sources = nodes
                .iter()
                .map(|node| {
                    builder
                        .node_source(*node)
                        .unwrap_or_else(|| builder.node_label(*node))
                })
                .collect::<Vec<_>>()
                .join(", ");

            format!("rest({sources})")
        }
    };

    format!("{} as {}", source, builder.global_type_label(binding.ty))
}

/// Return selected generic argument values in binding order.
fn generic_argument_values(arguments: &[dir::GenericArgumentBinding]) -> Vec<dir::GlobalTypeId> {
    dir::GenericArgumentBinding::values(arguments).collect()
}

/// Return selected arguments owned by one generic symbol template.
fn generic_instance_arguments(
    builder: &DirSnapshotBuilder<'_>,
    symbol: dir::GlobalSymbolId,
    arguments: &[dir::GenericArgumentBinding],
) -> Vec<dir::GlobalTypeId> {
    let Some(generics) = generic_table(builder, symbol.module_id) else {
        return generic_argument_values(arguments);
    };
    let Some((_, template)) = generics
        .iter_templates()
        .find(|(_, template)| template.symbol == Some(symbol))
    else {
        return Vec::new();
    };

    let mut selected = Vec::new();
    for parameter in &template.parameters {
        let parameter = dir::GlobalGenericParameterId::new(symbol.module_id, *parameter);
        let Some(argument) = arguments
            .iter()
            .find(|argument| argument.parameter == parameter)
            .map(|argument| argument.argument)
        else {
            continue;
        };
        selected.push(argument);
    }

    selected
}

/// Return the generic table for one module, when loaded.
fn generic_table<'a>(
    builder: &'a DirSnapshotBuilder<'_>,
    module: destack_source::ModuleId,
) -> Option<&'a dir::GenericTable<'static>> {
    if module == builder.tree.module_id {
        builder.generics.as_ref()
    } else {
        builder.foreign_generics.get(&module)
    }
}

use std::slice;

use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::ResolutionSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        let stacked = dir::ResolutionTable::from_segments(Vec::new());
        stacked.with_tail(self).add_snapshot_rows(builder);
    }
}

impl SnapshotTable for dir::ResolutionTable<'_> {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (node_id, resolution) in self.name_entries() {
            add_name_resolution_row(builder, node_id, resolution);
        }

        for (node_id, segment, resolution) in self.path_entries() {
            add_path_resolution_row(builder, node_id, segment, resolution);
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

        for (node_id, resolution) in self.access_entries() {
            add_access_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.member_entries() {
            add_member_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.operator_entries() {
            add_operator_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.call_entries() {
            add_call_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.subscript_entries() {
            add_subscript_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.place_entries() {
            add_place_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.assignment_entries() {
            add_assignment_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.guard_entries() {
            add_guard_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.construct_entries() {
            add_construct_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.tree_entries() {
            add_tree_resolution_row(builder, node_id, resolution);
        }

        for (node_id, resolution) in self.pattern_entries() {
            add_pattern_resolution_row(builder, self, node_id, resolution);
        }

        for (node_id, resolution) in self.assign_pattern_entries() {
            add_assign_pattern_resolution_row(builder, self, node_id, resolution);
        }

        for (node_id, path) in self.unresolved_entries() {
            add_unresolved_reference_row(builder, node_id, path);
        }

        let name_count = self.name_entries().count();
        let path_count = self.path_entries().count();
        let instantiation_count = self.instantiation_entries().count();
        let label_count = self.label_entries().count();
        let receiver_count = self.receiver_entries().count();
        let access_count = self.access_entries().count();
        let member_count = self.member_entries().count();
        let operator_count = self.operator_entries().count();
        let call_count = self.call_entries().count();
        let subscript_count = self.subscript_entries().count();
        let place_count = self.place_entries().count();
        let assignment_count = self.assignment_entries().count();
        let guard_count = self.guard_entries().count();
        let construct_count = self.construct_entries().count();
        let tree_count = self.tree_entries().count();
        let pattern_count = self.pattern_entries().count();
        let assign_pattern_count = self.assign_pattern_entries().count();
        if name_count == 0
            && path_count == 0
            && instantiation_count == 0
            && label_count == 0
            && receiver_count == 0
            && access_count == 0
            && member_count == 0
            && operator_count == 0
            && call_count == 0
            && subscript_count == 0
            && place_count == 0
            && assignment_count == 0
            && guard_count == 0
            && construct_count == 0
            && tree_count == 0
            && pattern_count == 0
            && assign_pattern_count == 0
        {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "resolution", "summary")
            .count_field("names", name_count)
            .count_field("paths", path_count)
            .count_field("instantiations", instantiation_count)
            .count_field("labels", label_count)
            .count_field("receivers", receiver_count)
            .count_field("accesses", access_count)
            .count_field("members", member_count)
            .count_field("operators", operator_count)
            .count_field("calls", call_count)
            .count_field("subscripts", subscript_count)
            .count_field("places", place_count)
            .count_field("guards", guard_count)
            .count_field("constructs", construct_count)
            .count_field("trees", tree_count)
            .count_field("patterns", pattern_count)
            .count_field("assigns", assign_pattern_count);
        builder.push(row);
    }
}

/// Add one operator resolution row.
fn add_operator_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::OperatorResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "operator")
        .optional_field("source", builder.node_source(node_id))
        .type_field("type", builder.global_type_label(resolution.ty()));
    let row = match resolution {
        dir::OperationResolution::One(dir::OperatorApplication::Unary {
            operator, target, ..
        }) => {
            let row = row.verbatim_field("operator", format!("{:?}", operator.text()));
            match target {
                dir::OperatorTarget::Builtin(operand) => {
                    add_builtin_operator_fields(builder, row, slice::from_ref(operand))
                }
                dir::OperatorTarget::Call(call) => add_operator_call_fields(builder, row, call),
            }
        }
        dir::OperationResolution::One(dir::OperatorApplication::Binary {
            operator,
            target,
            ..
        }) => {
            let row = row.verbatim_field("operator", format!("{:?}", operator.text()));
            match target {
                dir::OperatorTarget::Builtin(operands) => {
                    add_builtin_operator_fields(builder, row, operands)
                }
                dir::OperatorTarget::Call(call) => add_operator_call_fields(builder, row, call),
            }
        }
        dir::OperationResolution::Union { arms, .. } => row.field("kind", "union").list_field(
            "arms",
            arms.iter()
                .map(|application| operator_application_label(builder, application)),
        ),
    };

    builder.push(row);
    add_operator_resolution_generic_instances(builder, node_id, resolution);
}

/// Return one singular operator application snapshot label.
fn operator_application_label(
    builder: &DirSnapshotBuilder<'_>,
    application: &dir::OperatorApplication,
) -> String {
    match application {
        dir::OperatorApplication::Unary {
            operator,
            target,
            ty,
        } => {
            let target = match target {
                dir::OperatorTarget::Builtin(operand) => {
                    format!("builtin({})", builtin_operand_label(builder, operand))
                }
                dir::OperatorTarget::Call(call) => call_label(builder, call),
            };

            format!(
                "{} {target} -> {}",
                operator.text(),
                builder.global_type_label(*ty)
            )
        }
        dir::OperatorApplication::Binary {
            operator,
            target,
            ty,
        } => {
            let target = match target {
                dir::OperatorTarget::Builtin(operands) => format!(
                    "builtin({})",
                    operands
                        .iter()
                        .map(|operand| builtin_operand_label(builder, operand))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                dir::OperatorTarget::Call(call) => call_label(builder, call),
            };

            format!(
                "{} {target} -> {}",
                operator.text(),
                builder.global_type_label(*ty)
            )
        }
    }
}

/// Add checked builtin operator fields.
fn add_builtin_operator_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    operands: &[dir::BuiltinOperand],
) -> SnapshotRow {
    row.field("kind", "builtin").list_field(
        "operands",
        operands
            .iter()
            .map(|operand| builtin_operand_label(builder, operand)),
    )
}

/// Add one protocol operator call.
fn add_operator_call_fields(
    builder: &mut DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    call: &dir::Call,
) -> SnapshotRow {
    let row = add_call_fields(builder, row.field("kind", "call"), call);

    add_call_target_fields(builder, row, &call.target)
}

/// Render one checked builtin operand.
fn builtin_operand_label(
    builder: &DirSnapshotBuilder<'_>,
    operand: &dir::BuiltinOperand,
) -> String {
    let source = builder
        .node_source(operand.source)
        .unwrap_or_else(|| builder.node_label(operand.source));
    let ty = builder.global_type_label(operand.ty);
    let Some(families) = &operand.scalar_families else {
        return format!("{source} as {ty}");
    };

    let families = families
        .iter()
        .map(|family| match family {
            dir::ScalarFamily::Domain(domain) => format!("{domain:?}").to_lowercase(),
            dir::ScalarFamily::Enum(symbol) => builder.symbol_path_label(*symbol),
        })
        .collect::<Vec<_>>()
        .join(" | ");

    format!("{source} as {ty} families=({families})")
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

/// Add one path segment resolution row.
fn add_path_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    segment: u16,
    resolution: &dir::NameResolution,
) {
    let row = SnapshotRow::new(
        builder.name_resolution_anchor(node_id),
        "resolution",
        "path",
    )
    .optional_field("source", builder.name_resolution_source(node_id))
    .field("index", segment.to_string())
    .field("target", builder.symbol_path_label(resolution.symbol()));

    builder.push(row);
}

/// Add one retained unresolved reference row.
fn add_unresolved_reference_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    path: &dir::Path,
) {
    let row = SnapshotRow::new(
        builder.name_resolution_anchor(node_id),
        "resolution",
        "unresolved",
    )
    .optional_field("source", builder.name_resolution_source(node_id))
    .field(
        "path",
        path.segments
            .iter()
            .map(|segment| builder.strings.get(*segment))
            .collect::<Vec<_>>()
            .join("."),
    );

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
    symbol: dir::GlobalSymbolId,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "label")
        .optional_field("source", builder.node_source(node_id))
        .field("target", builder.symbol_label(symbol));

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

/// Add one repeatable access resolution row.
fn add_access_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::AccessResolution,
) {
    let path = resolution.path();
    let root = match path.root() {
        dir::AccessRoot::Symbol(symbol) => builder.symbol_path_label(symbol),
        dir::AccessRoot::Receiver(receiver) => receiver_kind_label(receiver).to_string(),
    };
    let keys = (!path.keys().is_empty()).then(|| {
        let keys = path
            .keys()
            .iter()
            .map(|key| builder.static_key(*key))
            .collect::<Vec<_>>()
            .join(", ");

        format!("[{keys}]")
    });
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "access")
        .optional_field("source", builder.node_source(node_id))
        .field("root", root)
        .optional_field("keys", keys);

    builder.push(row);
}

/// Add one member resolution row.
fn add_member_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::MemberResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "member")
        .optional_field("source", builder.node_source(node_id));

    let row = match resolution {
        dir::OperationResolution::One(access) => {
            let row = row
                .type_field("receiver", builder.global_type_label(access.receiver))
                .type_field("type", builder.global_type_label(access.ty));

            add_member_access_fields(builder, row, access)
        }
        dir::OperationResolution::Union { arms, ty } => row
            .type_field("type", builder.global_type_label(*ty))
            .field("kind", "union")
            .list_field(
                "arms",
                arms.iter()
                    .map(|access| member_access_label(builder, access)),
            ),
    };

    builder.push(row);
    add_member_resolution_generic_instances(builder, node_id, resolution);
}

/// Add fields for one singular member access.
fn add_member_access_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    access: &dir::MemberAccess,
) -> SnapshotRow {
    match &access.target {
        dir::MemberTarget::Projection { projection, .. } => row
            .field("kind", "projection")
            .field("target", projection_label(builder, projection)),
        dir::MemberTarget::Field(field) => {
            let row =
                add_member_receiver_fields(builder, row.field("kind", "field"), &field.receiver);
            let target = match field.target {
                dir::FieldTarget::Structural { .. } => None,
                dir::FieldTarget::Member { symbol, .. } => Some(builder.symbol_path_label(symbol)),
            };

            row.field("key", builder.static_key(field.target.key()))
                .optional_field("target", target)
                .type_field("target_type", builder.global_type_label(field.ty))
        }
        dir::MemberTarget::Call(call) => row
            .field("kind", "call")
            .field("target", call_label(builder, call)),
        dir::MemberTarget::Index(index) => {
            let row =
                add_member_receiver_fields(builder, row.field("kind", "index"), &index.receiver)
                    .type_field("key", builder.global_type_label(index.key_type));
            match &index.target {
                dir::IndexTarget::Signature(position) => {
                    row.field("target", format!("signature({position})"))
                }
                dir::IndexTarget::Fields(keys) => {
                    row.list_field("target", keys.iter().map(|key| builder.static_key(*key)))
                }
            }
        }
        dir::MemberTarget::Symbol(candidate) => {
            let row = add_member_receiver_fields(
                builder,
                row.field("kind", "symbol"),
                &candidate.receiver,
            );

            row.field("target", builder.member_candidate_label(candidate))
                .optional_field(
                    "instance",
                    generic_instance_label(builder, candidate.symbol, &candidate.generic_arguments),
                )
        }
        dir::MemberTarget::Existential(candidates) => row.field("kind", "existential").list_field(
            "targets",
            candidates
                .iter()
                .map(|target| member_target_label(builder, target)),
        ),
        dir::MemberTarget::Intersection(targets) => row.field("kind", "intersection").list_field(
            "targets",
            targets
                .iter()
                .map(|target| member_target_label(builder, target)),
        ),
    }
}

/// Add one call resolution row.
fn add_call_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::CallResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "call")
        .optional_field("source", builder.node_source(node_id));
    let row = add_call_resolution_fields(builder, row, resolution);
    let row = match resolution {
        dir::OperationResolution::One(call) => add_call_target_fields(builder, row, &call.target),
        dir::OperationResolution::Union { arms, .. } => row
            .field("kind", "union")
            .list_field("arms", arms.iter().map(|call| call_label(builder, call))),
    };

    builder.push(row);
    add_call_resolution_generic_instances(builder, node_id, resolution);
}

/// Add one subscript resolution row.
fn add_subscript_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::SubscriptResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "subscript")
        .optional_field("source", builder.node_source(node_id))
        .type_field("type", builder.global_type_label(resolution.ty()));
    let row = match resolution {
        dir::OperationResolution::One(subscript) => add_subscript_fields(builder, row, subscript),
        dir::OperationResolution::Union { arms, .. } => row.field("kind", "union").list_field(
            "arms",
            arms.iter()
                .map(|subscript| subscript_label(builder, subscript)),
        ),
    };

    builder.push(row);
    add_subscript_resolution_generic_instances(builder, node_id, resolution);
}

/// Add fields shared by call and operator resolutions.
fn add_call_resolution_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    resolution: &dir::CallResolution,
) -> SnapshotRow {
    match resolution {
        dir::OperationResolution::One(call) => add_call_fields(builder, row, call),
        dir::OperationResolution::Union { ty, .. } => {
            row.type_field("return", builder.global_type_label(*ty))
        }
    }
}

/// Add fields for one singular call.
fn add_call_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    call: &dir::Call,
) -> SnapshotRow {
    row.type_tuple_field(
        "parameters",
        call.arguments
            .iter()
            .map(|argument| builder.global_type_label(argument.parameter_type)),
    )
    .optional_field(
        "arguments",
        builder.argument_bindings_label(&call.arguments),
    )
    .type_field("return", builder.global_type_label(call.return_type))
}

/// Add fields for one selected call target.
fn add_call_target_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    target: &dir::CallTarget,
) -> SnapshotRow {
    match target {
        dir::CallTarget::Expression { generic_arguments } => row
            .field("kind", "expression")
            .field("target", "expression")
            .optional_field(
                "generic_arguments",
                builder.generic_arguments_label(generic_arguments),
            ),
        dir::CallTarget::Symbol { function, dispatch } => {
            let row = add_function_target_fields(builder, row.field("kind", "symbol"), function);
            let dispatch = match dispatch {
                dir::FunctionDispatch::Direct => None,
                dir::FunctionDispatch::Virtual { class } => {
                    Some(format!("virtual({})", builder.global_type_label(*class)))
                }
            };

            row.optional_field("dispatch", dispatch)
        }
        dir::CallTarget::Dynamic {
            dispatch,
            function,
            generic_arguments,
        } => row
            .field("kind", "dynamic")
            .field("target", dynamic_function_label(builder, function))
            .type_field(
                "receiver",
                builder.global_type_label(dispatch.receiver.source),
            )
            .type_field("constraint", builder.global_type_label(dispatch.constraint))
            .optional_field(
                "adjustments",
                receiver_adjustments_label(builder, &dispatch.receiver.adjustments),
            )
            .optional_field(
                "generic_arguments",
                builder.generic_arguments_label(generic_arguments),
            ),
    }
}

/// Add one place resolution row.
fn add_place_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::PlaceResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "place")
        .optional_field("source", builder.node_source(node_id))
        .type_field("placement", builder.global_type_label(resolution.placement))
        .type_field("lifetime", builder.global_type_label(resolution.lifetime))
        .type_field("access", builder.global_type_label(resolution.access));

    builder.push(row);
}

/// Add one assignment resolution row.
fn add_assignment_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::AssignmentResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "assignment")
        .optional_field("source", builder.node_source(node_id))
        .optional_field(
            "read",
            resolution
                .read
                .as_ref()
                .map(|read| read_resolution_label(builder, read)),
        )
        .field("write", write_resolution_label(builder, &resolution.write))
        .type_field("type", builder.global_type_label(resolution.write.ty()));

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
        dir::Projection::Absent { ty } => {
            format!("absent({})", builder.global_type_label(*ty))
        }
        dir::Projection::Field(field) => {
            format!("field.get({})", field_resolution_label(builder, field))
        }
        dir::Projection::Subscript(read) => format!(
            "subscript({}, {})",
            subscript_label(builder, read),
            builder.global_type_label(read.ty)
        ),
        dir::Projection::Call(call) => format!(
            "call({}, {})",
            call_label(builder, call),
            builder.global_type_label(call.return_type)
        ),
        dir::Projection::Member(access) => {
            format!("member({})", member_access_label(builder, access))
        }
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
        dir::Projection::VariantTag {
            carrier,
            discriminator,
            ty,
        } => {
            format!(
                "variant.tag({}, {}, {})",
                builder.global_type_label(*carrier),
                builder.static_key(*discriminator),
                builder.global_type_label(*ty)
            )
        }
        dir::Projection::VariantPayload {
            case,
            backing,
            discriminator,
            discriminant,
            ty,
        } => {
            format!(
                "variant.payload({}.{}, backing={}, discriminator={}, value={discriminant:?}, type={})",
                builder.symbol_path_label(case.owner),
                builder.static_key(case.key),
                builder.global_type_label(*backing),
                builder.static_key(*discriminator),
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
        dir::Projection::Dereference(resolution) => {
            format!("dereference({})", dereference_label(builder, resolution))
        }
        dir::Projection::Copy { ty } => {
            format!("copy({})", builder.global_type_label(*ty))
        }
    }
}

/// Return one projection resolution snapshot label.
fn projection_resolution_label(
    builder: &DirSnapshotBuilder<'_>,
    resolution: &dir::ProjectionResolution,
) -> String {
    match resolution {
        dir::OperationResolution::One(projection) => projection_label(builder, projection),
        dir::OperationResolution::Union { arms, ty } => {
            let arms = arms
                .iter()
                .map(|projection| projection_label(builder, projection))
                .collect::<Vec<_>>();

            format!(
                "union(({}), {})",
                arms.join(", "),
                builder.global_type_label(*ty)
            )
        }
    }
}

/// Return one subscript resolution snapshot label.
fn subscript_resolution_label(
    builder: &DirSnapshotBuilder<'_>,
    resolution: &dir::SubscriptResolution,
) -> String {
    match resolution {
        dir::OperationResolution::One(subscript) => subscript_label(builder, subscript),
        dir::OperationResolution::Union { arms, .. } => {
            let arms = arms
                .iter()
                .map(|subscript| subscript_label(builder, subscript))
                .collect::<Vec<_>>();

            format!("union({})", arms.join(", "))
        }
    }
}

/// Return one singular subscript snapshot label.
fn subscript_label(builder: &DirSnapshotBuilder<'_>, subscript: &dir::Subscript) -> String {
    match &subscript.target {
        dir::SubscriptTarget::Member(member) => {
            format!("member({})", member_access_label(builder, member))
        }
        dir::SubscriptTarget::Call(call) => call_label(builder, call),
        dir::SubscriptTarget::Index(read) => call_label(builder, &read.call),
    }
}

/// Add fields for one singular subscript.
fn add_subscript_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    subscript: &dir::Subscript,
) -> SnapshotRow {
    match &subscript.target {
        dir::SubscriptTarget::Member(member) => row
            .field("kind", "member")
            .field("target", member_access_label(builder, member)),
        dir::SubscriptTarget::Call(call) => row
            .field("kind", "call")
            .field("target", call_label(builder, call)),
        dir::SubscriptTarget::Index(read) => row
            .field("kind", "call")
            .field("target", call_label(builder, &read.call)),
    }
}

/// Return one place read snapshot label.
fn read_resolution_label(
    builder: &DirSnapshotBuilder<'_>,
    resolution: &dir::ReadResolution,
) -> String {
    match resolution {
        dir::ReadResolution::Binding { symbol, .. } => {
            format!("binding({})", builder.symbol_path_label(*symbol))
        }
        dir::ReadResolution::Member(member) => member_resolution_label(builder, member),
        dir::ReadResolution::Subscript(subscript) => subscript_resolution_label(builder, subscript),
        dir::ReadResolution::Dereference(dereference) => {
            dereference_resolution_label(builder, dereference)
        }
    }
}

/// Return one place write snapshot label.
fn write_resolution_label(
    builder: &DirSnapshotBuilder<'_>,
    resolution: &dir::WriteResolution,
) -> String {
    match resolution {
        dir::WriteResolution::Binding { symbol, .. } => {
            format!("binding({})", builder.symbol_path_label(*symbol))
        }
        dir::WriteResolution::Member(member) => member_resolution_label(builder, member),
        dir::WriteResolution::Subscript(subscript) => {
            subscript_resolution_label(builder, subscript)
        }
        dir::WriteResolution::Dereference(dereference) => {
            dereference_resolution_label(builder, dereference)
        }
    }
}

/// Return one dereference resolution snapshot label.
fn dereference_resolution_label(
    builder: &DirSnapshotBuilder<'_>,
    resolution: &dir::DereferenceResolution,
) -> String {
    match resolution {
        dir::OperationResolution::One(dereference) => dereference_label(builder, dereference),
        dir::OperationResolution::Union { arms, .. } => {
            let arms = arms
                .iter()
                .map(|dereference| dereference_label(builder, dereference))
                .collect::<Vec<_>>();

            format!("union({})", arms.join(", "))
        }
    }
}

/// Return one singular dereference snapshot label.
fn dereference_label(builder: &DirSnapshotBuilder<'_>, dereference: &dir::Dereference) -> String {
    let target = match &dereference.target {
        dir::DereferenceTarget::Direct => "direct".to_string(),
        dir::DereferenceTarget::Call(call) => call_label(builder, call),
    };

    format!(
        "{} => {target} -> {}",
        builder.global_type_label(dereference.receiver),
        builder.global_type_label(dereference.ty),
    )
}

/// Return one member target snapshot label.
fn member_target_label(builder: &DirSnapshotBuilder<'_>, target: &dir::MemberTarget) -> String {
    match target {
        dir::MemberTarget::Projection { projection, .. } => projection_label(builder, projection),
        dir::MemberTarget::Field(field) => field_resolution_label(builder, field),
        dir::MemberTarget::Call(call) => call_label(builder, call),
        dir::MemberTarget::Index(index) => {
            format!("index({})", builder.global_type_label(index.key_type))
        }
        dir::MemberTarget::Symbol(candidate) => builder.member_candidate_label(candidate),
        dir::MemberTarget::Existential(candidates) => candidates
            .iter()
            .map(|target| member_target_label(builder, target))
            .collect::<Vec<_>>()
            .join(" | "),
        dir::MemberTarget::Intersection(targets) => targets
            .iter()
            .map(|target| member_target_label(builder, target))
            .collect::<Vec<_>>()
            .join(" & "),
    }
}

/// Return one member resolution snapshot label.
fn member_resolution_label(
    builder: &DirSnapshotBuilder<'_>,
    resolution: &dir::MemberResolution,
) -> String {
    match resolution {
        dir::OperationResolution::One(access) => member_access_label(builder, access),
        dir::OperationResolution::Union { arms, .. } => {
            let arms = arms
                .iter()
                .map(|access| member_access_label(builder, access))
                .collect::<Vec<_>>();

            format!("union({})", arms.join(", "))
        }
    }
}

/// Return one singular member access snapshot label.
fn member_access_label(builder: &DirSnapshotBuilder<'_>, access: &dir::MemberAccess) -> String {
    let receiver = builder.global_type_label(access.receiver);
    let target = member_target_label(builder, &access.target);
    let ty = builder.global_type_label(access.ty);

    format!("receiver={receiver}, target={target}, type={ty}")
}

/// Return one projected field snapshot label.
fn field_target_label(builder: &DirSnapshotBuilder<'_>, target: &dir::FieldTarget) -> String {
    match target {
        dir::FieldTarget::Structural { key, .. } => builder.static_key(*key),
        dir::FieldTarget::Member { symbol, .. } => builder.symbol_path_label(*symbol),
    }
}

/// Return one field resolution snapshot label.
fn field_resolution_label(
    builder: &DirSnapshotBuilder<'_>,
    resolution: &dir::FieldResolution,
) -> String {
    let receiver = member_receiver_label(builder, &resolution.receiver);
    let target = field_target_label(builder, &resolution.target);
    let ty = builder.global_type_label(resolution.ty);

    format!("field(receiver={receiver}, target={target}, type={ty})")
}

/// Return one member receiver snapshot label.
fn member_receiver_label(
    builder: &DirSnapshotBuilder<'_>,
    receiver: &dir::MemberReceiver,
) -> String {
    match receiver {
        dir::MemberReceiver::Direct(receiver) => adjusted_receiver_label(builder, receiver),
        dir::MemberReceiver::Dynamic(dispatch) => format!(
            "dynamic({}, constraint={})",
            adjusted_receiver_label(builder, &dispatch.receiver),
            builder.global_type_label(dispatch.constraint)
        ),
    }
}

/// Return one adjusted receiver snapshot label.
fn adjusted_receiver_label(
    builder: &DirSnapshotBuilder<'_>,
    receiver: &dir::AdjustedReceiver,
) -> String {
    let source = builder.global_type_label(receiver.source);
    let Some(adjustments) = receiver_adjustments_label(builder, &receiver.adjustments) else {
        return source;
    };

    format!("{source} adjustments={adjustments}")
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
        dir::PredicateTest::Membership(test) => format!(
            "membership({}, {})",
            predicate_operand_label(builder, &test.receiver),
            predicate_key_label(builder, &test.key)
        ),
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
    match operand {
        dir::PredicateOperand::Direct(ty) => builder.global_type_label(*ty),
        dir::PredicateOperand::Projected(projection) => projection_label(builder, projection),
    }
}

/// Return one singular call snapshot label.
fn call_label(builder: &DirSnapshotBuilder<'_>, call: &dir::Call) -> String {
    direct_call_label(
        builder,
        call_target_label(builder, &call.target),
        &call.arguments,
        call.return_type,
    )
}

/// Return one call target snapshot label.
fn call_target_label(builder: &DirSnapshotBuilder<'_>, target: &dir::CallTarget) -> String {
    match target {
        dir::CallTarget::Expression { .. } => "expression".to_string(),
        dir::CallTarget::Symbol { function, dispatch } => match dispatch {
            dir::FunctionDispatch::Direct => builder.function_target_label(function),
            dir::FunctionDispatch::Virtual { class } => format!(
                "virtual({}.{})",
                builder.global_type_label(*class),
                builder.function_target_label(function)
            ),
        },
        dir::CallTarget::Dynamic {
            dispatch, function, ..
        } => format!(
            "dynamic({} as {}, {})",
            builder.global_type_label(dispatch.receiver.source),
            builder.global_type_label(dispatch.constraint),
            dynamic_function_label(builder, function)
        ),
    }
}

/// Return one direct call snapshot label.
fn direct_call_label(
    builder: &DirSnapshotBuilder<'_>,
    target: String,
    arguments: &[dir::ArgumentBinding],
    return_type: dir::GlobalTypeId,
) -> String {
    let parameters = arguments
        .iter()
        .map(|argument| builder.global_type_label(argument.parameter_type))
        .collect::<Vec<_>>()
        .join(", ");
    let arguments = builder
        .argument_bindings_label(arguments)
        .unwrap_or_else(|| "()".to_string());
    let return_type = builder.global_type_label(return_type);

    format!("{target}(parameters=({parameters}), arguments={arguments}, return={return_type})")
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
        dir::PrimitiveType::Integer(integer) => integer.as_str(),
        dir::PrimitiveType::Float(float) => float_label(float),
        dir::PrimitiveType::Symbol => "symbol".to_string(),
        dir::PrimitiveType::UniqueSymbol => "unique symbol".to_string(),
    }
}

/// Return the canonical label for one float predicate.
fn float_label(float: dir::FloatType) -> String {
    match float {
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
                .arguments
                .iter()
                .map(|argument| builder.global_type_label(argument.parameter_type)),
        )
        .optional_field(
            "arguments",
            builder.argument_bindings_label(&resolution.arguments),
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
            .type_field("backing", builder.global_type_label(candidate.backing))
            .optional_field(
                "argument",
                candidate
                    .argument
                    .map(|argument| builder.global_type_label(argument)),
            )
            .field(
                "discriminant",
                builder.scalar_literal_value_label(&candidate.discriminant),
            ),
        dir::ConstructTarget::Dynamic { dispatch, function } => row
            .field("kind", "dynamic")
            .field("target", dynamic_function_label(builder, function))
            .type_field(
                "receiver",
                builder.global_type_label(dispatch.receiver.source),
            )
            .type_field("constraint", builder.global_type_label(dispatch.constraint)),
    };

    builder.push(row);
    add_construct_target_generic_instances(builder, node_id, &resolution.target);
}

/// Add one tree resolution row.
fn add_tree_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::TreeResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "tree")
        .optional_field("source", builder.node_source(node_id))
        .type_field("builder", builder.global_type_label(resolution.builder));
    let row = match &resolution.target {
        dir::TreeTarget::Element { tag, call } => row
            .field("form", "element")
            .field("tag", builder.strings.get(*tag).to_string())
            .optional_field("call", tree_call_label(builder, call)),
        dir::TreeTarget::Fragment { call } => row
            .field("form", "fragment")
            .optional_field("call", tree_call_label(builder, call)),
        dir::TreeTarget::Component { callee, invocation } => {
            let row = row
                .field("form", "component")
                .optional_field("callee", builder.node_source(*callee));
            match invocation {
                dir::TreeInvocation::Call(call) => {
                    row.optional_field("call", tree_call_label(builder, call))
                }
                dir::TreeInvocation::Construct(construct) => row.field(
                    "construct",
                    construct_target_label(builder, &construct.target),
                ),
                dir::TreeInvocation::Struct { ty } => {
                    row.type_field("struct", builder.global_type_label(*ty))
                }
            }
        }
    };
    let row = row
        .optional_field(
            "attributes",
            tree_attributes_label(builder, &resolution.attributes),
        )
        .type_tuple_field(
            "children",
            resolution.children.iter().map(|child| match child {
                dir::TreeChildBinding::Text { ty, .. }
                | dir::TreeChildBinding::Expression { ty, .. }
                | dir::TreeChildBinding::Spread { ty, .. } => builder.global_type_label(*ty),
            }),
        )
        .type_field("type", builder.global_type_label(resolution.ty));

    builder.push(row);
}

/// Return the label of one tree literal's selected call.
fn tree_call_label(builder: &DirSnapshotBuilder<'_>, call: &dir::CallResolution) -> Option<String> {
    let dir::OperationResolution::One(call) = call else {
        return None;
    };
    let target = match &call.target {
        dir::CallTarget::Symbol { function, .. } => builder.function_target_label(function),
        dir::CallTarget::Expression { .. } => "expression".to_string(),
        dir::CallTarget::Dynamic { .. } => "dynamic".to_string(),
    };

    Some(target)
}

/// Return the label of one component construct target.
fn construct_target_label(
    builder: &DirSnapshotBuilder<'_>,
    target: &dir::ConstructTarget,
) -> String {
    match target {
        dir::ConstructTarget::Class(candidate) => builder.symbol_path_label(candidate.symbol),
        dir::ConstructTarget::Newtype(candidate) => builder.symbol_path_label(candidate.symbol),
        dir::ConstructTarget::Variant(candidate) => builder.symbol_path_label(candidate.case.owner),
        dir::ConstructTarget::Dynamic { function, .. } => dynamic_function_label(builder, function),
    }
}

/// Return the label of one tree literal's attribute bindings.
fn tree_attributes_label(
    builder: &DirSnapshotBuilder<'_>,
    attributes: &[dir::TreeAttributeBinding],
) -> Option<String> {
    if attributes.is_empty() {
        return None;
    }
    let bindings = attributes
        .iter()
        .map(|attribute| {
            let key = builder.strings.get(attribute.key);
            let ty = builder.global_type_label(attribute.ty);

            format!("{key}: {ty}")
        })
        .collect::<Vec<_>>()
        .join(", ");

    Some(format!("({bindings})"))
}

/// Add one pattern resolution row.
fn add_pattern_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionTable<'_>,
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
        dir::PatternResolution::Variant(variant) => {
            add_pattern_variant_resolution_fields(builder, segment, row, variant)
        }
        dir::PatternResolution::Project(project) => row
            .field(
                "projection",
                projection_resolution_label(builder, &project.projection),
            )
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

/// Return one pattern resolution label.
fn pattern_resolution_label(resolution: &dir::PatternResolution) -> &'static str {
    match resolution {
        dir::PatternResolution::Ignore => "wildcard",
        dir::PatternResolution::Bind(_) => "binding",
        dir::PatternResolution::Must(_) => "must",
        dir::PatternResolution::Default(_) => "default",
        dir::PatternResolution::Test(test) => pattern_predicate_label(&test.predicate),
        dir::PatternResolution::Variant(_) => "variant",
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
fn pattern_projection_label(projection: &dir::ProjectionResolution) -> &'static str {
    match projection {
        dir::OperationResolution::One(dir::Projection::Borrow { .. }) => "borrow",
        dir::OperationResolution::One(dir::Projection::Move { .. }) => "move",
        dir::OperationResolution::One(dir::Projection::Dereference { .. }) => "dereference",
        dir::OperationResolution::One(dir::Projection::NewtypePayload { .. }) => "newtype",
        dir::OperationResolution::One(_) => "project",
        dir::OperationResolution::Union { .. } => "union",
    }
}

/// Return one pattern destructure label.
fn pattern_destructure_label(destructure: &dir::PatternDestructureResolution) -> &'static str {
    match destructure {
        dir::PatternDestructureResolution::Tuple(_) => "tuple",
        dir::PatternDestructureResolution::Object(_) => "object",
        dir::PatternDestructureResolution::Nominal(_) => "nominal_object",
        dir::PatternDestructureResolution::Sequence(_) => "sequence",
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
        dir::PredicateTest::Membership(_) | dir::PredicateTest::Any(_) => None,
    }
}

/// Add fields for one pattern destructure.
fn add_pattern_destructure_fields(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionTable<'_>,
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
            sequence.rest.as_deref(),
        ),
    }
}

/// Add fields for one selected variant pattern.
fn add_pattern_variant_resolution_fields(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionTable<'_>,
    row: SnapshotRow,
    variant: &dir::PatternVariantResolution,
) -> SnapshotRow {
    let row = row
        .field("predicate", predicate_label(builder, &variant.predicate))
        .optional_field(
            "projection",
            variant
                .predicate
                .projection
                .as_ref()
                .map(|projection| projection_label(builder, projection)),
        )
        .optional_field(
            "payload",
            match variant.payload {
                Some(_) => Some("pattern".to_string()),
                None => pattern_variant_payload_label(builder, &variant.fields).map(str::to_string),
            },
        );

    add_pattern_variant_fields(builder, segment, row, &variant.fields)
}

/// Add one assignment pattern resolution row.
fn add_assign_pattern_resolution_row(
    builder: &mut DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionTable<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::AssignPatternResolution,
) {
    let row = SnapshotRow::new(builder.anchor_node(node_id), "resolution", "pattern.assign")
        .optional_field("source", builder.node_source(node_id))
        .field("kind", assign_pattern_resolution_label(resolution));

    let row = match resolution {
        dir::AssignPatternResolution::Place => row,
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
            sequence.rest.as_deref(),
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
        dir::AssignPatternResolution::Place => "place",
        dir::AssignPatternResolution::Default(_) => "default",
        dir::AssignPatternResolution::Sequence(_) => "sequence",
        dir::AssignPatternResolution::Tuple(_) => "tuple",
        dir::AssignPatternResolution::Object(_) => "object",
    }
}

/// Add direct function target fields.
fn add_function_target_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    function: &dir::FunctionTarget,
) -> SnapshotRow {
    row.field("target", builder.function_target_label(function))
        .optional_type_field(
            "receiver",
            function
                .receiver
                .as_ref()
                .map(|receiver| builder.global_type_label(receiver.source)),
        )
        .optional_field(
            "adjustments",
            function
                .receiver
                .as_ref()
                .and_then(|receiver| receiver_adjustments_label(builder, &receiver.adjustments)),
        )
        .optional_field(
            "instance",
            function_target_instance_label(builder, function),
        )
}

/// Add one selected member receiver.
fn add_member_receiver_fields(
    builder: &DirSnapshotBuilder<'_>,
    row: SnapshotRow,
    receiver: &dir::MemberReceiver,
) -> SnapshotRow {
    let row = row
        .type_field(
            "target_receiver",
            builder.global_type_label(receiver.source()),
        )
        .optional_field(
            "adjustments",
            receiver_adjustments_label(builder, &receiver.adjusted().adjustments),
        );

    match receiver {
        dir::MemberReceiver::Direct(_) => row,
        dir::MemberReceiver::Dynamic(dispatch) => row
            .field("dispatch", "dynamic")
            .type_field("constraint", builder.global_type_label(dispatch.constraint)),
    }
}

/// Render one receiver adjustment list, or none when empty.
fn receiver_adjustments_label(
    builder: &DirSnapshotBuilder<'_>,
    adjustments: &[dir::ReceiverAdjustment],
) -> Option<String> {
    if adjustments.is_empty() {
        return None;
    }

    let labels = adjustments
        .iter()
        .map(|adjustment| receiver_adjustment_label(builder, adjustment))
        .collect::<Vec<_>>();

    Some(format!("({})", labels.join(", ")))
}

/// Render one receiver adjustment.
fn receiver_adjustment_label(
    builder: &DirSnapshotBuilder<'_>,
    adjustment: &dir::ReceiverAdjustment,
) -> String {
    match adjustment {
        dir::ReceiverAdjustment::Borrow { ty } => {
            format!("borrow({})", builder.global_type_label(*ty))
        }
        dir::ReceiverAdjustment::Dereference(resolution) => dereference_label(builder, resolution),
        dir::ReceiverAdjustment::NewtypePayload { symbol, ty, .. } => format!(
            "newtype.payload({}, {})",
            builder.symbol_path_label(*symbol),
            builder.global_type_label(*ty)
        ),
        dir::ReceiverAdjustment::VariantPayload { case, ty, .. } => format!(
            "variant.payload({}, {})",
            builder.symbol_path_label(case.variant),
            builder.global_type_label(*ty)
        ),
    }
}

/// Render one dynamic function target.
fn dynamic_function_label(
    builder: &DirSnapshotBuilder<'_>,
    function: &dir::DynamicFunction,
) -> String {
    let (operation, source) = match function {
        dir::DynamicFunction::Symbol(symbol) => return builder.symbol_path_label(*symbol),
        dir::DynamicFunction::CallSignature(source) => ("call", source),
        dir::DynamicFunction::ConstructSignature(source) => ("construct", source),
        dir::DynamicFunction::IndexRead(source) => ("index.read", source),
        dir::DynamicFunction::IndexWrite(source) => ("index.write", source),
    };
    let source = builder
        .node_source(*source)
        .unwrap_or_else(|| builder.node_label(*source));

    format!("{operation}({source})")
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
    candidate: &dir::NewtypeSelection,
) -> SnapshotRow {
    row.field("target", builder.symbol_path_label(candidate.symbol))
        .type_field("backing", builder.global_type_label(candidate.backing))
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
        dir::MemberTarget::Existential(targets) | dir::MemberTarget::Intersection(targets) => {
            for target in targets {
                add_member_target_generic_instances(builder, node_id, target);
            }
        }
        dir::MemberTarget::Call(call) => {
            add_call_generic_instances(builder, node_id, call);
        }
        dir::MemberTarget::Projection { .. }
        | dir::MemberTarget::Field(_)
        | dir::MemberTarget::Index(_) => {}
    }
}

/// Add generic instance rows from one member resolution.
fn add_member_resolution_generic_instances(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::MemberResolution,
) {
    match resolution {
        dir::OperationResolution::One(access) => {
            add_member_target_generic_instances(builder, node_id, &access.target);
        }
        dir::OperationResolution::Union { arms, .. } => {
            for access in arms {
                add_member_target_generic_instances(builder, node_id, &access.target);
            }
        }
    }
}

/// Add generic instance rows from one operator resolution.
fn add_operator_resolution_generic_instances(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::OperatorResolution,
) {
    match resolution {
        dir::OperationResolution::One(application) => {
            add_operator_application_generic_instances(builder, node_id, application);
        }
        dir::OperationResolution::Union { arms, .. } => {
            for application in arms {
                add_operator_application_generic_instances(builder, node_id, application);
            }
        }
    }
}

/// Add generic instance rows from one operator application.
fn add_operator_application_generic_instances(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    application: &dir::OperatorApplication,
) {
    match application {
        dir::OperatorApplication::Unary {
            target: dir::OperatorTarget::Call(call),
            ..
        }
        | dir::OperatorApplication::Binary {
            target: dir::OperatorTarget::Call(call),
            ..
        } => add_call_generic_instances(builder, node_id, call),
        dir::OperatorApplication::Unary {
            target: dir::OperatorTarget::Builtin(_),
            ..
        }
        | dir::OperatorApplication::Binary {
            target: dir::OperatorTarget::Builtin(_),
            ..
        } => {}
    }
}

/// Add generic instance rows from one call resolution.
fn add_call_resolution_generic_instances(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::CallResolution,
) {
    match resolution {
        dir::OperationResolution::One(call) => add_call_generic_instances(builder, node_id, call),
        dir::OperationResolution::Union { arms, .. } => {
            for call in arms {
                add_call_generic_instances(builder, node_id, call);
            }
        }
    }
}

/// Add generic instance rows from one singular call.
fn add_call_generic_instances(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    call: &dir::Call,
) {
    let anchor = builder.anchor_node(node_id);
    let source = builder.node_source(node_id);

    match &call.target {
        dir::CallTarget::Symbol { function, .. } => {
            add_function_target_generic_instance(builder, anchor, source, function);
        }
        dir::CallTarget::Dynamic {
            function: dir::DynamicFunction::Symbol(symbol),
            generic_arguments,
            ..
        } => add_generic_instance(builder, anchor, source, *symbol, generic_arguments),
        dir::CallTarget::Expression { .. }
        | dir::CallTarget::Dynamic {
            function:
                dir::DynamicFunction::CallSignature(_)
                | dir::DynamicFunction::IndexRead(_)
                | dir::DynamicFunction::IndexWrite(_)
                | dir::DynamicFunction::ConstructSignature(_),
            ..
        } => {}
    }
}

/// Add generic instance rows from one subscript resolution.
fn add_subscript_resolution_generic_instances(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    resolution: &dir::SubscriptResolution,
) {
    match resolution {
        dir::OperationResolution::One(subscript) => {
            add_subscript_generic_instances(builder, node_id, subscript);
        }
        dir::OperationResolution::Union { arms, .. } => {
            for subscript in arms {
                add_subscript_generic_instances(builder, node_id, subscript);
            }
        }
    }
}

/// Add generic instance rows from one singular subscript.
fn add_subscript_generic_instances(
    builder: &mut DirSnapshotBuilder<'_>,
    node_id: dir::GlobalNodeIdAny,
    subscript: &dir::Subscript,
) {
    match &subscript.target {
        dir::SubscriptTarget::Member(member) => {
            add_member_target_generic_instances(builder, node_id, &member.target);
        }
        dir::SubscriptTarget::Call(call) => add_call_generic_instances(builder, node_id, call),
        dir::SubscriptTarget::Index(read) => {
            add_call_generic_instances(builder, node_id, &read.call)
        }
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
        dir::ConstructTarget::Dynamic { .. } => {}
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
            add_projection_resolution_generic_instances(
                builder,
                anchor,
                source,
                &project.projection,
            );
        }
        dir::PatternResolution::Test(test) => {
            add_predicate_generic_instances(builder, node_id, &test.predicate);
        }
        dir::PatternResolution::Variant(variant) => {
            add_predicate_generic_instances(builder, node_id, &variant.predicate);
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

/// Add generic instance rows from one projection resolution.
fn add_projection_resolution_generic_instances(
    builder: &mut DirSnapshotBuilder<'_>,
    anchor: SnapshotAnchor,
    source: Option<String>,
    resolution: &dir::ProjectionResolution,
) {
    match resolution {
        dir::OperationResolution::One(projection) => {
            add_projection_generic_instance(builder, anchor, source, projection);
        }
        dir::OperationResolution::Union { arms, .. } => {
            for projection in arms {
                add_projection_generic_instance(builder, anchor, source.clone(), projection);
            }
        }
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
        dir::PredicateTest::Membership(test) => {
            add_predicate_operand_generic_instance(builder, anchor, source.clone(), &test.receiver);
            if let dir::PredicateKey::Dynamic(operand) = &test.key {
                add_predicate_operand_generic_instance(builder, anchor, source.clone(), operand);
            }
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
    if let dir::PredicateOperand::Projected(projection) = operand {
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
        dir::Projection::Absent { .. }
        | dir::Projection::Field(_)
        | dir::Projection::Subscript(_)
        | dir::Projection::Call(_)
        | dir::Projection::Member(_)
        | dir::Projection::ObjectRest { .. }
        | dir::Projection::SliceLength { .. }
        | dir::Projection::DynamicPayload { .. }
        | dir::Projection::DynamicType { .. }
        | dir::Projection::VariantTag { .. }
        | dir::Projection::VariantPayload { .. }
        | dir::Projection::Borrow { .. }
        | dir::Projection::Move { .. }
        | dir::Projection::Dereference(_)
        | dir::Projection::Copy { .. } => {}
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

/// Render one selected callable instance.
fn function_target_instance_label(
    builder: &DirSnapshotBuilder<'_>,
    function: &dir::FunctionTarget,
) -> Option<String> {
    let Some(owner) = function.generic_scope else {
        return generic_instance_label(builder, function.symbol, &function.generic_arguments);
    };

    let owner_arguments = generic_instance_arguments(builder, owner, &function.generic_arguments);
    let member_arguments =
        generic_instance_arguments(builder, function.symbol, &function.generic_arguments);
    if owner_arguments.is_empty() && member_arguments.is_empty() {
        return None;
    }

    let owner_label = function_target_owner_label(
        builder,
        owner,
        &owner_arguments,
        &function.generic_arguments,
    );
    let member_label = function_target_member_label(builder, owner, function.symbol);
    let member_label = match member_arguments.as_slice() {
        [] => member_label,
        _ => format!(
            "{}<{}>",
            member_label,
            member_arguments
                .iter()
                .map(|argument| builder.global_type_label(*argument))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };

    Some(format!("{owner_label}.{member_label}"))
}

/// Render the generic scope selected by one call.
fn function_target_owner_label(
    builder: &DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    arguments: &[dir::GlobalTypeId],
    bindings: &[dir::GenericArgumentBinding],
) -> String {
    let Some(dir::Definition::Extension(extension)) = builder.definition(owner) else {
        return match arguments {
            [] => builder.symbol_path_label(owner),
            _ => builder.generic_instance_label(owner, arguments),
        };
    };

    if builder.is_anonymous_symbol(owner) {
        let target = type_label_with_bindings(builder, extension.target.r#type(), bindings);
        let extension = builder.anonymous_extension_label(owner);

        format!("{target}.{extension}")
    } else {
        match arguments {
            [] => builder.symbol_path_label(owner),
            _ => builder.generic_instance_label(owner, arguments),
        }
    }
}

/// Render one type while applying selected generic bindings.
fn type_label_with_bindings(
    builder: &DirSnapshotBuilder<'_>,
    type_id: dir::GlobalTypeId,
    bindings: &[dir::GenericArgumentBinding],
) -> String {
    let Some(ty) = type_value(builder, type_id) else {
        return builder.global_type_label(type_id);
    };

    match ty {
        dir::Type::Parameter(parameter) => bindings
            .iter()
            .find(|binding| binding.parameter == parameter)
            .map(|binding| builder.global_type_label(binding.argument))
            .unwrap_or_else(|| builder.global_type_label(type_id)),
        dir::Type::Reference(reference) => builder.reference_symbol_label(reference.symbol),
        dir::Type::Application(instance) => {
            generic_instance_type_label(builder, type_id.module_id, &instance, bindings)
        }
        _ => builder.global_type_label(type_id),
    }
}

/// Render one applied declaration while applying selected generic bindings.
fn generic_instance_type_label(
    builder: &DirSnapshotBuilder<'_>,
    module: destack_source::ModuleId,
    instance: &dir::GenericApplication,
    bindings: &[dir::GenericArgumentBinding],
) -> String {
    let Some(types) = type_table(builder, module) else {
        return builder.symbol_path_label(instance.symbol);
    };

    let arguments = types
        .type_ids(instance.arguments)
        .iter()
        .map(|argument| type_label_with_bindings(builder, *argument, bindings))
        .collect::<Vec<_>>();
    if arguments.is_empty() {
        return builder.reference_symbol_label(instance.symbol);
    }

    let symbol = builder.reference_symbol_label(instance.symbol);
    let arguments = arguments.join(", ");

    format!("{symbol}<{arguments}>")
}

/// Return the stored type value for one visible type id.
fn type_value(builder: &DirSnapshotBuilder<'_>, type_id: dir::GlobalTypeId) -> Option<dir::Type> {
    let types = type_table(builder, type_id.module_id)?;

    Some(types.get_type(type_id.local_id))
}

/// Return the visible type table for one module.
fn type_table<'a>(
    builder: &'a DirSnapshotBuilder<'_>,
    module: destack_source::ModuleId,
) -> Option<&'a dir::TypeTable<'static>> {
    if module == builder.tree.module_id {
        builder.types.as_ref()
    } else {
        builder.foreign_types.get(&module)
    }
}

/// Add one selected callable instance row.
fn add_function_target_generic_instance(
    builder: &mut DirSnapshotBuilder<'_>,
    anchor: SnapshotAnchor,
    source: Option<String>,
    function: &dir::FunctionTarget,
) {
    let Some(label) = function_target_instance_label(builder, function) else {
        return;
    };
    let arguments = generic_argument_values(&function.generic_arguments);
    let arguments = builder.generic_instance_arguments_label(&arguments);
    let template = builder.symbol_path_label(function.symbol);

    builder.add_generic_instance_row(
        anchor,
        source.as_deref(),
        &label,
        Some(&template),
        &arguments,
    );
}

/// Render one member name relative to its owner.
fn function_target_member_label(
    builder: &DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    symbol: dir::GlobalSymbolId,
) -> String {
    let owner = builder.symbol_path_label(owner);
    let symbol = builder.symbol_path_label(symbol);
    let Some(member) = symbol.strip_prefix(&format!("{owner}.")) else {
        return symbol
            .rsplit_once('.')
            .map(|(_, member)| member.to_string())
            .unwrap_or(symbol);
    };

    member.to_string()
}

/// Add ordered pattern sequence fields.
fn add_pattern_sequence_fields(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionTable<'_>,
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
    segment: &dir::ResolutionTable<'_>,
    row: SnapshotRow,
    fields: &[dir::PatternFieldResolution],
) -> SnapshotRow {
    if fields.is_empty() {
        return row;
    }

    if is_positional_variant_payload(builder, fields) {
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
    segment: &'a dir::ResolutionTable<'_>,
    fields: &'a [dir::PatternFieldResolution],
) -> impl Iterator<Item = String> + 'a {
    fields
        .iter()
        .map(|field| pattern_keyed_field_label(builder, segment, field))
}

/// Return positional pattern field labels.
fn pattern_positional_field_labels<'a>(
    builder: &'a DirSnapshotBuilder<'_>,
    segment: &'a dir::ResolutionTable<'_>,
    fields: &'a [dir::PatternFieldResolution],
) -> impl Iterator<Item = String> + 'a {
    fields
        .iter()
        .map(|field| pattern_positional_field_label(builder, segment, field))
}

/// Return one keyed pattern field group label.
fn pattern_keyed_fields_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionTable<'_>,
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
    segment: &dir::ResolutionTable<'_>,
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
    segment: &dir::ResolutionTable<'_>,
    field: &dir::PatternFieldResolution,
) -> String {
    let Some(pattern) = field.pattern else {
        return pattern_field_target_label(builder, &field.projection);
    };

    pattern_child_label(builder, segment, pattern)
}

/// Return one variant payload label.
fn pattern_variant_payload_label(
    builder: &DirSnapshotBuilder<'_>,
    fields: &[dir::PatternFieldResolution],
) -> Option<&'static str> {
    if fields.is_empty() {
        return None;
    }

    if is_positional_variant_payload(builder, fields) {
        Some("tuple")
    } else {
        Some("object")
    }
}

/// Return whether one variant payload uses positional fields.
fn is_positional_variant_payload(
    builder: &DirSnapshotBuilder<'_>,
    fields: &[dir::PatternFieldResolution],
) -> bool {
    fields.iter().all(|field| {
        let field = field.source.local_id.into_typed::<dir::PatternField>();

        matches!(
            builder.tree.get(field),
            dir::PatternField::Positional { .. }
        )
    })
}

/// Return the source-facing target label for one pattern field projection.
fn pattern_field_target_label(
    builder: &DirSnapshotBuilder<'_>,
    projection: &dir::ProjectionResolution,
) -> String {
    match projection {
        dir::OperationResolution::One(dir::Projection::Field(field)) => {
            field_target_label(builder, &field.target)
        }
        _ => projection_resolution_label(builder, projection),
    }
}

/// Return one child pattern snapshot label.
fn pattern_child_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionTable<'_>,
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
    segment: &'a dir::ResolutionTable<'_>,
    fields: &'a [dir::AssignPatternFieldResolution],
) -> impl Iterator<Item = String> + 'a {
    fields
        .iter()
        .map(|field| assign_pattern_keyed_field_label(builder, segment, field))
}

/// Return positional assignment pattern field labels.
fn assign_pattern_positional_field_labels<'a>(
    builder: &'a DirSnapshotBuilder<'_>,
    segment: &'a dir::ResolutionTable<'_>,
    fields: &'a [dir::AssignPatternFieldResolution],
) -> impl Iterator<Item = String> + 'a {
    fields
        .iter()
        .map(|field| assign_pattern_positional_field_label(builder, segment, field))
}

/// Return one keyed assignment pattern field group label.
fn assign_pattern_keyed_fields_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionTable<'_>,
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
    segment: &dir::ResolutionTable<'_>,
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
    segment: &dir::ResolutionTable<'_>,
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
    segment: &dir::ResolutionTable<'_>,
    pattern: dir::GlobalNodeIdAny,
) -> String {
    match segment.assign_pattern_resolution(pattern) {
        Some(dir::AssignPatternResolution::Place) => {
            let pattern = pattern.local_id.into_typed::<dir::AssignPattern>();
            let dir::AssignPattern::Place { expression } = builder.tree.get(pattern) else {
                panic!("place resolution belongs to a non-place assignment pattern");
            };
            let expression = expression.into_global_any(builder.tree.module_id);
            let assignment = segment
                .assignment_resolution(expression)
                .unwrap_or_else(|| {
                    panic!("assignment target is missing its assignment resolution")
                });

            assignment_source_label(builder, segment, assignment)
        }
        Some(dir::AssignPatternResolution::Default(default)) => {
            assign_pattern_child_label(builder, segment, default.pattern)
        }
        _ => builder.node_label(pattern),
    }
}

/// Return one assignment source snapshot label.
fn assignment_source_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionTable<'_>,
    assignment: &dir::AssignmentResolution,
) -> String {
    builder
        .node_source(assignment.target)
        .unwrap_or_else(|| assign_pattern_place_label(builder, segment, assignment.target))
}

/// Return one assignment place snapshot label.
fn assign_pattern_place_label(
    builder: &DirSnapshotBuilder<'_>,
    segment: &dir::ResolutionTable<'_>,
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
    segment: &dir::ResolutionTable<'_>,
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
    segment: &dir::ResolutionTable<'_>,
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
    segment: &dir::ResolutionTable<'_>,
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
    segment: &dir::ResolutionTable<'_>,
    rest: &dir::AssignPatternRestResolution,
) -> String {
    let Some(pattern) = rest.pattern else {
        return "...".to_string();
    };

    let pattern = assign_pattern_child_label(builder, segment, pattern);

    format!("...{pattern}")
}

impl DirSnapshotBuilder<'_> {
    /// Render one applied generic argument list label.
    pub(super) fn generic_arguments_label(
        &self,
        arguments: &[dir::GenericArgumentBinding],
    ) -> Option<String> {
        if arguments.is_empty() {
            return None;
        }

        let arguments = arguments
            .iter()
            .map(|argument| self.global_type_label(argument.argument))
            .collect::<Vec<_>>()
            .join(", ");

        Some(format!("({arguments})"))
    }

    /// Render one runtime argument binding list label.
    pub(super) fn argument_bindings_label(
        &self,
        arguments: &[dir::ArgumentBinding],
    ) -> Option<String> {
        if arguments.is_empty() {
            return None;
        }

        let arguments = arguments
            .iter()
            .map(|argument| argument_binding_label(self, argument))
            .collect::<Vec<_>>()
            .join(", ");

        Some(format!("({arguments})"))
    }
}

/// Render one runtime argument binding label.
fn argument_binding_label(
    builder: &DirSnapshotBuilder<'_>,
    binding: &dir::ArgumentBinding,
) -> String {
    let source = match &binding.source {
        dir::ArgumentSource::Provided(node) => {
            let source = builder
                .node_source(*node)
                .unwrap_or_else(|| builder.node_label(*node));

            format!("provided({source})")
        }
        dir::ArgumentSource::Static(ty) => {
            format!("static({})", builder.global_type_label(*ty))
        }
        dir::ArgumentSource::Write => "write".to_string(),
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

    format!(
        "{} as {}",
        source,
        builder.global_type_label(binding.argument_type)
    )
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

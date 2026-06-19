use std::collections::HashSet;

use destack_mir as mir;

use crate::verify::VerifyState;

impl VerifyState<'_> {
    /// Generate missing drop glue functions.
    pub(in crate::verify) fn generate_drop_glue(&mut self) {
        let mut roots = Vec::new();
        let mut building = HashSet::new();
        self.collect_drop_glue_roots(&mut roots);

        // generate glue reachable from actual module surfaces
        for ty in roots {
            self.generate_reachable_drop_glue(ty, &mut building);
        }
    }

    /// Collect types that may need drop glue from real module surfaces.
    fn collect_drop_glue_roots(&self, roots: &mut Vec<mir::LocalNodeId<mir::Type>>) {
        // collect global storage types
        for (_, global) in self.tree.iter_nodes::<mir::Global>() {
            roots.push(global.ty);
        }

        // collect function interface and value types
        for (_, function) in self.tree.iter_nodes::<mir::Function>() {
            for parameter in &function.parameters {
                roots.push(parameter.ty);
            }

            roots.push(function.return_type);

            if let Some(environment) = function.environment {
                roots.push(environment);
            }

            for local in &function.locals {
                let local = self.tree.get(*local);
                roots.push(local.ty);
            }

            for ty in function.value_types.iter().flatten() {
                roots.push(*ty);
            }
        }
    }

    /// Generate drop glue reachable through one value type.
    fn generate_reachable_drop_glue(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        building: &mut HashSet<mir::LocalNodeId<mir::Type>>,
    ) {
        match self.tree.get(ty) {
            mir::Type::Reference {
                kind: mir::ReferenceKind::Unique,
                pointee,
                ..
            } => {
                self.drop_glue_function(*pointee, building);
            }
            _ => {
                self.drop_glue_function(ty, building);
            }
        }
    }

    /// Return existing or generated drop glue for one type.
    fn drop_glue_function(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        building: &mut HashSet<mir::LocalNodeId<mir::Type>>,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        // reuse already generated glue
        if let Some(glue) = self.tree.metadata.drop.drop_glue(ty) {
            return glue.function();
        }

        // skip copy and scalar shapes
        if !self.type_needs_function_glue(ty) {
            return None;
        }

        // declare before recursion so cycles can refer to the symbol
        let function = self.declare_drop_glue(ty)?;
        self.tree
            .metadata
            .drop
            .set_drop_glue(ty, mir::DropGlue::Generated { function });

        // build the body once per active recursion chain
        if building.insert(ty) {
            self.generate_child_drop_glue(ty, building);
            self.build_drop_glue_body(ty, function);
            building.remove(&ty);
        }

        Some(function)
    }

    /// Return whether a type needs a generated drop function.
    fn type_needs_function_glue(&self, ty: mir::LocalNodeId<mir::Type>) -> bool {
        // copy values never need drop glue
        if self.tree.get(ty).copy().is_yes() {
            return false;
        }
        if self.tree.metadata.drop.drop_hook(ty).is_some() {
            return true;
        }

        self.type_children_emit_drop(ty, &mut HashSet::new())
    }

    /// Return whether any child value of this type emits drop code.
    fn type_children_emit_drop(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        seen: &mut HashSet<mir::LocalNodeId<mir::Type>>,
    ) -> bool {
        match self.tree.get(ty) {
            mir::Type::Struct { fields, .. } => fields.iter().any(|field| {
                let field = self.tree.get(*field);

                self.type_emits_drop(field.ty, seen)
            }),
            mir::Type::Tuple { elements, .. } => elements
                .iter()
                .any(|element| self.type_emits_drop(*element, seen)),
            mir::Type::Newtype { inner, .. } => self.type_emits_drop(*inner, seen),
            mir::Type::FixedArray {
                element, length, ..
            } => *length > 0 && self.type_emits_drop(*element, seen),
            mir::Type::Variant { cases, .. } => {
                cases.iter().any(|case| self.type_emits_drop(case.ty, seen))
            }
            mir::Type::Slice {
                kind: mir::ReferenceKind::Unique,
                element,
                ..
            } => self.type_emits_drop(*element, seen),
            _ => false,
        }
    }

    /// Return whether dropping this type emits code.
    fn type_emits_drop(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        seen: &mut HashSet<mir::LocalNodeId<mir::Type>>,
    ) -> bool {
        if type_emits_drop_code(&self.tree, ty) {
            return true;
        }
        if self.tree.get(ty).copy().is_yes() || !seen.insert(ty) {
            return false;
        }

        let emits_drop = self.type_children_emit_drop(ty, seen);
        seen.remove(&ty);

        emits_drop
    }

    /// Declare the drop glue function for one type.
    fn declare_drop_glue(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        let name = self.drop_name(ty)?;
        let parameters = vec![mir::FunctionParameter::new(mir::Value::new(0), ty)];
        let void = self.tree.void_type();

        // register a bodyless function first so recursive glue can call it
        let function = mir::Function::declare(name, Vec::new(), parameters, void);
        let function = self.tree.insert(function);

        Some(function)
    }

    /// Return the generated drop function name for one type.
    fn drop_name(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<destack_core::StringId> {
        let stem = self.drop_glue_name_stem(ty)?;
        let drop_name = format!("{stem}.drop");

        Some(self.strings.intern(&drop_name))
    }

    /// Return a stable name stem for generated drop glue.
    fn drop_glue_name_stem(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<String> {
        match self.tree.get(ty) {
            mir::Type::Reference {
                kind: mir::ReferenceKind::Unique,
                pointee,
                ..
            } => {
                let pointee = self.drop_name_stem(*pointee)?;

                Some(format!("ref.{pointee}"))
            }
            mir::Type::Slice {
                kind: mir::ReferenceKind::Unique,
                element,
                ..
            } => {
                let element = self.drop_name_stem(*element)?;

                Some(format!("slice.{element}"))
            }
            _ => self.drop_name_stem(ty),
        }
    }

    /// Return a stable name stem for generated drop glue.
    fn drop_name_stem(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<String> {
        // prefer source names when the type has one
        if let Some(display_name) = self.tree.metadata.types.display_name(ty) {
            return Some(self.strings.get(display_name).to_string());
        }

        self.structural_drop_name_stem(ty)
    }

    /// Return a stable name stem for unnamed structural types.
    fn structural_drop_name_stem(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<String> {
        let name = match self.tree.get(ty) {
            mir::Type::Void => "void".to_string(),
            mir::Type::Boolean => "boolean".to_string(),
            mir::Type::Int { width, is_signed } => {
                if *is_signed {
                    format!("int{width}")
                } else {
                    format!("uint{width}")
                }
            }
            mir::Type::Isize => "isize".to_string(),
            mir::Type::Usize => "usize".to_string(),
            mir::Type::Dynamic { constraint } => {
                let constraint = self.drop_name_stem(*constraint)?;
                format!("dynamic.{constraint}")
            }
            mir::Type::Reference { kind, pointee, .. } => {
                let pointee = self.drop_name_stem(*pointee)?;
                format!("ref.{pointee}.{}", kind.name())
            }
            mir::Type::Slice { kind, element, .. } => {
                let element = self.drop_name_stem(*element)?;
                format!("slice.{element}.{}", kind.name())
            }
            mir::Type::FixedArray {
                element, length, ..
            } => {
                let element = self.drop_name_stem(*element)?;
                format!("array.{element}.length{length}")
            }
            mir::Type::Tuple { elements, .. } => {
                let elements = elements
                    .iter()
                    .map(|element| self.drop_name_stem(*element))
                    .collect::<Option<Vec<_>>>()?;
                format!("tuple.{}", elements.join("."))
            }
            mir::Type::Variant { tag, cases, .. } => {
                let tag = self.drop_name_stem(*tag)?;
                let cases = cases
                    .iter()
                    .map(|case| self.drop_name_stem(case.ty))
                    .collect::<Option<Vec<_>>>()?;
                format!("variant.{tag}.{}", cases.join("."))
            }
            _ => return None,
        };

        Some(name)
    }

    /// Generate nested aggregate glue before building this body.
    fn generate_child_drop_glue(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        building: &mut HashSet<mir::LocalNodeId<mir::Type>>,
    ) {
        match self.tree.get(ty) {
            mir::Type::Struct { fields, .. } => {
                let field_count = fields.len();

                // generate field glue without materializing child lists
                for index in 0..field_count {
                    let field_ty = match self.tree.get(ty) {
                        mir::Type::Struct { fields, .. } => self.tree.get(fields[index]).ty,
                        _ => unreachable!("type changed during drop glue generation"),
                    };
                    self.generate_value_drop_glue(field_ty, building);
                }
            }
            mir::Type::Tuple { elements, .. } => {
                let element_count = elements.len();

                // generate element glue without materializing child lists
                for index in 0..element_count {
                    let element = match self.tree.get(ty) {
                        mir::Type::Tuple { elements, .. } => elements[index],
                        _ => unreachable!("type changed during drop glue generation"),
                    };
                    self.generate_value_drop_glue(element, building);
                }
            }
            mir::Type::Newtype { inner, .. } => {
                let inner = *inner;
                self.generate_value_drop_glue(inner, building);
            }
            mir::Type::FixedArray {
                element, length, ..
            } => {
                let element = *element;
                let length = *length;
                if length > 0 {
                    self.generate_value_drop_glue(element, building);
                }
            }
            mir::Type::Slice {
                kind: mir::ReferenceKind::Unique,
                element,
                ..
            } => {
                let element = *element;
                self.generate_value_drop_glue(element, building);
            }
            mir::Type::Variant { cases, .. } => {
                let case_count = cases.len();

                // generate case glue without materializing child lists
                for index in 0..case_count {
                    let case_ty = match self.tree.get(ty) {
                        mir::Type::Variant { cases, .. } => cases[index].ty,
                        _ => unreachable!("type changed during drop glue generation"),
                    };
                    self.generate_value_drop_glue(case_ty, building);
                }
            }
            _ => {}
        }
    }

    /// Generate glue needed to drop one value type.
    fn generate_value_drop_glue(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        building: &mut HashSet<mir::LocalNodeId<mir::Type>>,
    ) {
        // unique storage drops the pointee, not the reference handle
        if let mir::Type::Reference {
            kind: mir::ReferenceKind::Unique,
            pointee,
            ..
        } = self.tree.get(ty)
        {
            self.generate_value_drop_glue(*pointee, building);
        } else {
            self.drop_glue_function(ty, building);
        }
    }

    /// Build the drop function body.
    fn build_drop_glue_body(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        function: mir::LocalNodeId<mir::Function>,
    ) {
        // turn the declaration into a real body
        let mut builder =
            mir::FunctionBuilder::from_declared(&mut self.tree, self.strings, function)
                .unwrap_or_else(|error| unreachable!("{error}"));

        let entry = builder.block();
        builder.switch_to_block(entry);
        let value = builder.function_parameter(0);

        // call the user hook before structural field drops
        if let Some(hook) = builder.tree().metadata.drop.drop_hook(ty).cloned() {
            let void = builder.tree_mut().void_type();
            let signature = builder
                .tree_mut()
                .insert_type(mir::Type::FunctionSignature {
                    lifetimes: Vec::new(),
                    parameters: vec![mir::SignatureParameter::new(ty)],
                    result: void,
                });
            builder.call_void(hook.function, signature, vec![value]);
        }

        // emit drop then return void
        emit_inline_drop(&mut builder, ty, value);
        builder.return_(None);
        builder
            .finish()
            .unwrap_or_else(|error| unreachable!("{error}"));
    }
}

/// Emit drop for a value whose own glue body is being generated.
fn emit_inline_drop(
    builder: &mut mir::FunctionBuilder<'_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: mir::Value,
) {
    match builder.tree().get(ty).clone() {
        mir::Type::Struct { fields, .. } => {
            // drop each stored field
            for (index, field) in fields.iter().enumerate() {
                let field_ty = builder.tree().get(*field).ty;
                let field_value = builder.field_get(value, index as u32);

                emit_drop(builder, field_ty, field_value);
            }
        }
        mir::Type::Tuple { elements, .. } => {
            // drop each tuple element
            for (index, element) in elements.iter().enumerate() {
                let element_value = builder.field_get(value, index as u32);

                emit_drop(builder, *element, element_value);
            }
        }
        mir::Type::Newtype { inner, .. } => {
            // drop the wrapped value
            let inner_value = builder.field_get(value, 0);

            emit_drop(builder, inner, inner_value);
        }
        mir::Type::FixedArray {
            element, length, ..
        } => {
            // drop each fixed array element
            for index in 0..length {
                let element_value = builder.field_get(value, index as u32);

                emit_drop(builder, element, element_value);
            }
        }
        mir::Type::Slice {
            kind: mir::ReferenceKind::Unique,
            element,
            space,
            access,
            ..
        } => {
            emit_unique_slice_drop(builder, element, space, access, value);
        }
        mir::Type::Variant { cases, .. } => {
            emit_variant_drop(builder, value, cases);
        }
        _ => {}
    }
}

/// Emit drop for an owning slice descriptor.
fn emit_unique_slice_drop(
    builder: &mut mir::FunctionBuilder<'_>,
    element: mir::LocalNodeId<mir::Type>,
    space: mir::Space,
    access: mir::Access,
    value: mir::Value,
) {
    // empty slice contents need no drop body
    if !type_emits_drop_code(builder.tree(), element) {
        return;
    }

    // build loop state
    let usize_type = builder.ensure_usize_type();
    let element_pointer = builder.reference_type(
        mir::ReferenceKind::Unique,
        element,
        access,
        space,
        mir::Nullability::None,
    );
    let index = builder.variable(usize_type);
    let header = builder.block();
    let body = builder.block();
    let done = builder.block();
    let length = builder.slice_length(value);
    let zero = builder.usize_const(0);

    // initialize index
    builder.define_variable(index, zero);
    builder.jump(header);

    // branch on index bounds
    builder.switch_to_block(header);
    let current = builder.use_variable(index);
    let is_in_bounds = builder.binary_op(mir::BinaryOperator::UnsignedLessThan, current, length);
    builder.branch(is_in_bounds, body, done);

    // drop current element
    builder.switch_to_block(body);
    let pointer = builder.element_addr(value, current, element_pointer);
    let element_value = builder.load(pointer, element);
    emit_drop(builder, element, element_value);

    // advance loop
    let one = builder.usize_const(1);
    let next = builder.iadd(current, one);
    builder.define_variable(index, next);
    builder.jump(header);

    builder.switch_to_block(done);
}

/// Emit drop for a physical tagged sum value.
fn emit_variant_drop(
    builder: &mut mir::FunctionBuilder<'_>,
    value: mir::Value,
    cases: Vec<mir::VariantCase>,
) {
    // keep cases with payload storage to drop
    let cases = cases
        .into_iter()
        .filter(|case| type_emits_drop_code(builder.tree(), case.ty))
        .collect::<Vec<_>>();
    if cases.is_empty() {
        return;
    }

    // build one payload block per nontrivial case
    let case_blocks = cases.iter().map(|_| builder.block()).collect::<Vec<_>>();
    let done = builder.block();
    let check_blocks = cases
        .iter()
        .skip(1)
        .map(|_| builder.block())
        .collect::<Vec<_>>();
    let mut check = builder.current_block();
    let tag_value = builder.variant_tag(value);
    let case_count = cases.len();

    // dispatch on the active tag
    for (index, case) in cases.into_iter().enumerate() {
        let case_block = case_blocks[index];
        let failure = if index + 1 == case_count {
            done
        } else {
            check_blocks[index]
        };

        builder.switch_to_block(check);
        builder.check(
            mir::CheckConstraint::Variant {
                value: tag_value,
                expected: case.tag.clone(),
            },
            case_block,
            failure,
        );

        // drop matching payload
        builder.switch_to_block(case_block);
        let payload = builder.variant_payload(value, case.tag);
        emit_drop(builder, case.ty, payload);
        builder.jump(done);

        check = failure;
    }

    builder.switch_to_block(done);
}

/// Emit drop for one value.
fn emit_drop(
    builder: &mut mir::FunctionBuilder<'_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: mir::Value,
) {
    emit_drop_contents(builder, ty, value);

    // release owned storage after its contents are destroyed
    if builder.tree().get(ty).is_unique_storage() {
        builder.free(value);
    }
}

/// Emit drop for one value's contents.
fn emit_drop_contents(
    builder: &mut mir::FunctionBuilder<'_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: mir::Value,
) {
    if let Some(glue) = builder.tree().metadata.drop.drop_glue(ty).cloned() {
        emit_drop_glue(builder, ty, value, glue);
        return;
    }

    let mir::Type::Reference {
        kind: mir::ReferenceKind::Unique,
        pointee,
        ..
    } = builder.tree().get(ty)
    else {
        return;
    };

    emit_unique_reference_contents(builder, value, *pointee);
}

/// Emit one explicit drop glue call.
fn emit_drop_glue(
    builder: &mut mir::FunctionBuilder<'_>,
    ty: mir::LocalNodeId<mir::Type>,
    value: mir::Value,
    glue: mir::DropGlue,
) {
    // build the drop call signature
    let void = builder.tree_mut().void_type();
    let signature = builder
        .tree_mut()
        .insert_type(mir::Type::FunctionSignature {
            lifetimes: Vec::new(),
            parameters: vec![mir::SignatureParameter::new(ty)],
            result: void,
        });

    // call concrete or dynamic glue
    match glue {
        mir::DropGlue::Generated { function } => {
            builder.call_void(function, signature, vec![value]);
        }
        mir::DropGlue::Dynamic { slot } => {
            let mir::Type::Dynamic { constraint } = builder.tree().get(ty) else {
                unreachable!("dynamic drop glue requires a dynamic value type");
            };
            builder.call_dynamic_void(value, *constraint, slot, signature, Vec::new());
        }
        mir::DropGlue::None => {}
    }
}

/// Emit inline drop for one unique reference's pointee.
fn emit_unique_reference_contents(
    builder: &mut mir::FunctionBuilder<'_>,
    value: mir::Value,
    pointee: mir::LocalNodeId<mir::Type>,
) {
    // drop pointee contents before caller releases the allocation
    if type_emits_drop_code(builder.tree(), pointee) {
        let loaded = builder.load(value, pointee);
        emit_drop(builder, pointee, loaded);
    }
}

/// Return whether dropping a value of this type emits MIR.
fn type_emits_drop_code(tree: &mir::Tree, ty: mir::LocalNodeId<mir::Type>) -> bool {
    if tree
        .metadata
        .drop
        .drop_glue(ty)
        .is_some_and(|glue| !matches!(glue, mir::DropGlue::None))
    {
        return true;
    }
    if tree.get(ty).is_unique_storage() {
        return true;
    }

    match tree.get(ty) {
        mir::Type::Reference {
            kind: mir::ReferenceKind::Unique,
            pointee,
            ..
        } => type_emits_drop_code(tree, *pointee),
        _ => false,
    }
}

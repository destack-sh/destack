use std::collections::HashSet;

use destack_core::StringId;
use destack_mir as mir;

use crate::elaborate::ElaborateState;

impl ElaborateState<'_> {
    /// Generate missing destructors.
    pub(in crate::elaborate) fn generate_destructors(&mut self) {
        let mut roots = Vec::new();
        let mut building = HashSet::new();
        self.collect_frame_drop_roots(&mut roots);
        self.collect_allocation_drop_roots(&mut roots);
        roots.sort_unstable();
        roots.dedup();

        // generate destructors for every reachable value and allocation type
        for (ty, storage) in roots {
            self.generate_reachable_destructor(ty, storage, &mut building);
        }
    }

    /// Collect managed heap allocation types with their exact storage.
    fn collect_allocation_drop_roots(&self, roots: &mut Vec<(mir::TypeId, mir::Storage)>) {
        for (_, function) in self.tree.iter_nodes::<mir::Function>() {
            for block in function.blocks() {
                let block = self.tree.get(*block);

                // collect infallible allocation types
                for instruction in &block.instructions {
                    match self.tree.get(*instruction) {
                        mir::Instruction::NewZeroed {
                            storage_type,
                            result_type,
                            ..
                        }
                        | mir::Instruction::NewUninit {
                            storage_type,
                            result_type,
                            ..
                        } if let Some(storage) = self.managed_storage(*result_type) => {
                            roots.push((*storage_type, storage));
                        }
                        mir::Instruction::NewSliceZeroed {
                            element,
                            result_type,
                            ..
                        }
                        | mir::Instruction::NewSliceUninit {
                            element,
                            result_type,
                            ..
                        } if let Some(storage) = self.managed_storage(*result_type) => {
                            roots.push((*element, storage));
                        }
                        _ => {}
                    }
                }

                // collect fallible allocation types from their success result
                let terminator = self.tree.get(block.terminator);
                match terminator {
                    mir::Terminator::NewZeroedTry {
                        storage_type,
                        success,
                        ..
                    }
                    | mir::Terminator::NewUninitTry {
                        storage_type,
                        success,
                        ..
                    } if let Some(storage) = self.target_managed_storage(success) => {
                        roots.push((*storage_type, storage));
                    }
                    mir::Terminator::NewSliceZeroedTry {
                        element, success, ..
                    }
                    | mir::Terminator::NewSliceUninitTry {
                        element, success, ..
                    } if let Some(storage) = self.target_managed_storage(success) => {
                        roots.push((*element, storage));
                    }
                    _ => {}
                }
            }
        }
    }

    /// Return the managed heap storage received by one block target.
    fn target_managed_storage(&self, target: &mir::BlockTarget) -> Option<mir::Storage> {
        self.tree
            .get(target.block)
            .parameters
            .first()
            .and_then(|parameter| self.managed_storage(parameter.ty))
    }

    /// Return the heap storage carried by one managed allocation result.
    fn managed_storage(&self, ty: mir::TypeId) -> Option<mir::Storage> {
        let mut ty = ty;

        // peel transparent and initialization wrappers to the allocation carrier
        loop {
            ty = self.tree.repr_type(ty);
            let mir::Type::Uninit { value } = self.tree.get(ty) else {
                break;
            };

            ty = *value;
        }

        match self.tree.get(ty) {
            mir::Type::Reference {
                kind: mir::ReferenceKind::Managed,
                storage,
                ..
            }
            | mir::Type::Slice {
                kind: mir::ReferenceKind::Managed,
                storage,
                ..
            } if storage.heap_space().is_some() => Some(*storage),
            _ => None,
        }
    }

    /// Collect types stored directly in function frames.
    fn collect_frame_drop_roots(&self, roots: &mut Vec<(mir::TypeId, mir::Storage)>) {
        for (_, function) in self.tree.iter_nodes::<mir::Function>() {
            for parameter in &function.parameters {
                roots.push((parameter.ty, mir::Storage::Frame));
            }

            roots.push((function.return_type, mir::Storage::Frame));

            if let Some(environment) = function.environment {
                roots.push((environment, mir::Storage::Frame));
            }

            for local in function.locals() {
                let local = self.tree.get(*local);
                roots.push((local.ty, mir::Storage::Frame));
            }

            for ty in function.value_types().iter().flatten() {
                roots.push((*ty, mir::Storage::Frame));
            }
        }
    }

    /// Generate destructors reachable through one value type.
    fn generate_reachable_destructor(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        storage: mir::Storage,
        building: &mut HashSet<(mir::LocalNodeId<mir::Type>, mir::Storage)>,
    ) {
        match self.tree.get(ty) {
            mir::Type::Reference {
                kind: mir::ReferenceKind::Unique,
                pointee,
                storage,
                ..
            } => {
                self.destructor(*pointee, *storage, building);
            }
            _ => {
                self.destructor(ty, storage, building);
            }
        }
    }

    /// Return the existing or generated destructor for one type.
    fn destructor(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        storage: mir::Storage,
        building: &mut HashSet<(mir::LocalNodeId<mir::Type>, mir::Storage)>,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        // reuse an existing destructor
        if let Some(destructor) = self.drops.destructor(ty, storage) {
            return Some(destructor);
        }

        // skip copy and scalar shapes
        if !self.type_needs_destructor(ty, storage) {
            return None;
        }

        // declare before recursion so cycles can refer to the symbol
        let function = self.declare_destructor(ty, storage)?;
        self.drops.set_destructor(ty, storage, function);

        // build the body once per active recursion chain
        let key = (ty, storage);
        if building.insert(key) {
            self.generate_child_destructors(ty, storage, building);
            self.build_destructor(ty, storage, function);
            building.remove(&key);
        }

        Some(function)
    }

    /// Return whether a type needs a generated destructor.
    fn type_needs_destructor(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        storage: mir::Storage,
    ) -> bool {
        // copy values never need destructors
        if self.tree.get(ty).copy(&self.tree).is_yes() {
            return false;
        }
        if self.drops.hook(ty, storage).is_some() {
            return true;
        }

        self.type_children_emit_drop(ty, storage, &mut HashSet::new())
    }

    /// Return whether any child value of this type emits drop code.
    fn type_children_emit_drop(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        storage: mir::Storage,
        seen: &mut HashSet<mir::LocalNodeId<mir::Type>>,
    ) -> bool {
        match self.tree.get(ty) {
            mir::Type::Struct { fields, .. } => fields.iter().any(|field| {
                let field = self.tree.get(*field);

                self.type_emits_drop(field.ty, storage, seen)
            }),
            mir::Type::Tuple { elements, .. } => elements
                .iter()
                .any(|element| self.type_emits_drop(*element, storage, seen)),
            mir::Type::Newtype { inner, .. } => self.type_emits_drop(*inner, storage, seen),
            mir::Type::FixedArray {
                element, length, ..
            } => *length > 0 && self.type_emits_drop(*element, storage, seen),
            mir::Type::Variant { cases, .. } => cases
                .iter()
                .any(|case| self.type_emits_drop(case.ty, storage, seen)),
            mir::Type::Slice {
                kind: mir::ReferenceKind::Unique,
                element,
                storage,
                ..
            } => self.type_emits_drop(*element, *storage, seen),
            _ => false,
        }
    }

    /// Return whether dropping this type emits code.
    fn type_emits_drop(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        storage: mir::Storage,
        seen: &mut HashSet<mir::LocalNodeId<mir::Type>>,
    ) -> bool {
        if DestructorEmitter::type_emits(&self.drops, &self.tree, ty, storage) {
            return true;
        }
        if self.tree.get(ty).copy(&self.tree).is_yes() || !seen.insert(ty) {
            return false;
        }

        let emits_drop = self.type_children_emit_drop(ty, storage, seen);
        seen.remove(&ty);

        emits_drop
    }

    /// Declare the destructor for one type.
    fn declare_destructor(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        storage: mir::Storage,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        let name = self.drop_name(ty, storage)?;
        let arguments = self.destructor_arguments(ty);
        let pointer_type = mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::empty(),
            storage,
            access: mir::Access::Exclusive,
            pointee: ty,
            nullability: mir::Nullability::None,
        };
        let pointer = self.tree.intern_type(pointer_type);
        let parameters = vec![mir::FunctionParameter::new(mir::Value::new(0), pointer)];
        let void = self.tree.void_type();

        // register a bodyless function first so recursive drops can call it
        let symbol = mir::Symbol::named(name).instantiate(&arguments, &self.tree);
        let function = mir::Function::declare(name, Vec::new(), parameters, void)
            .with_arguments(arguments)
            .with_symbol(symbol);
        let function = self.tree.insert(function);

        Some(function)
    }

    /// Return the generated destructor name for one type.
    fn drop_name(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        storage: mir::Storage,
    ) -> Option<StringId> {
        let stem = self.destructor_name_stem(ty)?;
        let storage = storage.label();
        let drop_name = format!("{stem}.destruct.{storage}");

        Some(self.strings.intern(&drop_name))
    }

    /// Return a stable name stem for one generated destructor.
    fn destructor_name_stem(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<String> {
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

    /// Return a stable name stem for one generated destructor.
    fn drop_name_stem(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<String> {
        // prefer source names for nominal types
        if let Some(declaration) = self.tree.type_declaration(ty) {
            let declaration = self.tree.get(declaration);

            return Some(self.strings.get(declaration.name).to_string());
        }

        self.structural_drop_name_stem(ty)
    }

    /// Return concrete arguments that uniquely identify one generated destructor.
    fn destructor_arguments(&mut self, ty: mir::TypeId) -> Vec<mir::StaticId> {
        let mut arguments = Vec::new();
        self.collect_drop_arguments(ty, &mut arguments);

        // nominal destructors mirror their declaration arguments directly
        if self.tree.type_declaration(ty).is_some() || arguments.is_empty() {
            return arguments;
        }

        // preserve structural nesting instead of flattening descendant arguments
        let argument = self.tree.intern_static(mir::Static::Type(ty));

        vec![argument]
    }

    /// Collect concrete arguments contributing to one generated destructor identity.
    fn collect_drop_arguments(&self, ty: mir::TypeId, arguments: &mut Vec<mir::StaticId>) {
        // nominal types own their concrete generic arguments
        if let Some(declaration) = self.tree.type_declaration(ty) {
            let declaration = self.tree.get(declaration);
            arguments.extend_from_slice(&declaration.arguments);

            return;
        }

        // structural wrappers inherit their nested nominal arguments
        match self.tree.get(ty) {
            mir::Type::Dynamic { constraint, .. } => {
                self.collect_drop_arguments(*constraint, arguments);
            }
            mir::Type::Reference { pointee, .. } => {
                self.collect_drop_arguments(*pointee, arguments);
            }
            mir::Type::Slice { element, .. } | mir::Type::FixedArray { element, .. } => {
                self.collect_drop_arguments(*element, arguments);
            }
            mir::Type::Tuple { elements, .. } => {
                for element in elements {
                    self.collect_drop_arguments(*element, arguments);
                }
            }
            mir::Type::Variant {
                discriminant,
                cases,
                ..
            } => {
                self.collect_drop_arguments(*discriminant, arguments);
                for case in cases {
                    self.collect_drop_arguments(case.ty, arguments);
                }
            }
            _ => {}
        }
    }

    /// Return a stable name stem for unnamed structural types.
    fn structural_drop_name_stem(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<String> {
        let name = match self.tree.get(ty) {
            mir::Type::Void => "void".to_string(),
            mir::Type::Boolean => "boolean".to_string(),
            mir::Type::Character => "char".to_string(),
            mir::Type::Int { width, is_signed } => {
                if *is_signed {
                    format!("int{width}")
                } else {
                    format!("uint{width}")
                }
            }
            mir::Type::Isize => "isize".to_string(),
            mir::Type::Usize => "usize".to_string(),
            mir::Type::Dynamic {
                kind,
                constraint,
                storage,
                ..
            } => {
                let constraint = self.drop_name_stem(*constraint)?;
                format!("dynamic.{constraint}.{}.{}", kind.name(), storage.label())
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
            mir::Type::Variant {
                discriminant,
                cases,
                ..
            } => {
                let discriminant = self.drop_name_stem(*discriminant)?;
                let cases = cases
                    .iter()
                    .map(|case| self.drop_name_stem(case.ty))
                    .collect::<Option<Vec<_>>>()?;
                format!("variant.{discriminant}.{}", cases.join("."))
            }
            _ => return None,
        };

        Some(name)
    }

    /// Generate nested aggregate destructors before building this body.
    fn generate_child_destructors(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        storage: mir::Storage,
        building: &mut HashSet<(mir::LocalNodeId<mir::Type>, mir::Storage)>,
    ) {
        match self.tree.get(ty) {
            mir::Type::Struct { fields, .. } => {
                let field_count = fields.len();

                // generate field functions without materializing child lists
                for index in 0..field_count {
                    let field_ty = match self.tree.get(ty) {
                        mir::Type::Struct { fields, .. } => self.tree.get(fields[index]).ty,
                        _ => unreachable!("type changed during destructor generation"),
                    };
                    self.generate_reachable_destructor(field_ty, storage, building);
                }
            }
            mir::Type::Tuple { elements, .. } => {
                let element_count = elements.len();

                // generate element functions without materializing child lists
                for index in 0..element_count {
                    let element = match self.tree.get(ty) {
                        mir::Type::Tuple { elements, .. } => elements[index],
                        _ => unreachable!("type changed during destructor generation"),
                    };
                    self.generate_reachable_destructor(element, storage, building);
                }
            }
            mir::Type::Newtype { inner, .. } => {
                let inner = *inner;
                self.generate_reachable_destructor(inner, storage, building);
            }
            mir::Type::FixedArray {
                element, length, ..
            } => {
                let element = *element;
                let length = *length;
                if length > 0 {
                    self.generate_reachable_destructor(element, storage, building);
                }
            }
            mir::Type::Slice {
                kind: mir::ReferenceKind::Unique,
                element,
                storage,
                ..
            } => {
                let element = *element;
                let storage = *storage;
                self.generate_reachable_destructor(element, storage, building);
            }
            mir::Type::Variant { cases, .. } => {
                let case_count = cases.len();

                // generate case functions without materializing child lists
                for index in 0..case_count {
                    let case_ty = match self.tree.get(ty) {
                        mir::Type::Variant { cases, .. } => cases[index].ty,
                        _ => unreachable!("type changed during destructor generation"),
                    };
                    self.generate_reachable_destructor(case_ty, storage, building);
                }
            }
            _ => {}
        }
    }

    /// Build the destructor body.
    fn build_destructor(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        storage: mir::Storage,
        function: mir::LocalNodeId<mir::Function>,
    ) {
        // turn the declaration into a real body
        let mut builder = mir::FunctionBuilder::from_declared(
            &mut self.tree,
            &mut self.effects,
            self.target.pointer_bits(),
            function,
        )
        .unwrap_or_else(|error| unreachable!("{error}"));

        let entry = builder.block();
        builder.switch_to_block(entry);
        let pointer = builder.function_parameter(0);

        // destroy one stored value without releasing the containing storage
        DestructorEmitter::new(&self.drops, &mut builder).drop_at(ty, pointer, storage);
        builder.return_(None);
        builder
            .finish()
            .unwrap_or_else(|error| unreachable!("{error}"));
    }
}

/// MIR destructor emitter.
struct DestructorEmitter<'a, 'b> {
    /// The destructors and hooks available to generated code.
    drops: &'a mir::DropTable,
    /// The function under construction.
    builder: &'a mut mir::FunctionBuilder<'b>,
}

impl<'a, 'b> DestructorEmitter<'a, 'b> {
    /// Create a destructor emitter for one function body.
    fn new(drops: &'a mir::DropTable, builder: &'a mut mir::FunctionBuilder<'b>) -> Self {
        Self { drops, builder }
    }

    /// Emit drop for one inline value at its storage address.
    fn drop_inline(&mut self, ty: mir::TypeId, pointer: mir::Value, storage: mir::Storage) {
        match self.builder.tree().get(ty).clone() {
            mir::Type::Struct { fields, .. } => {
                for (index, field) in fields.iter().enumerate().rev() {
                    let field_ty = self.builder.tree().get(*field).ty;
                    let field_pointer_type = self.storage_pointer_type(field_ty, storage);
                    let field_pointer =
                        self.builder
                            .field_addr(pointer, index as u32, field_pointer_type);

                    self.drop_at(field_ty, field_pointer, storage);
                }
            }
            mir::Type::Tuple { elements, .. } => {
                for (index, element) in elements.iter().enumerate().rev() {
                    let element_pointer_type = self.storage_pointer_type(*element, storage);
                    let element_pointer =
                        self.builder
                            .field_addr(pointer, index as u32, element_pointer_type);

                    self.drop_at(*element, element_pointer, storage);
                }
            }
            mir::Type::Newtype { inner, .. } => {
                let inner_pointer_type = self.storage_pointer_type(inner, storage);
                let inner_pointer = self.builder.bitcast(pointer, inner_pointer_type);

                self.drop_at(inner, inner_pointer, storage);
            }
            mir::Type::FixedArray {
                element, length, ..
            } => {
                let element_pointer_type = self.storage_pointer_type(element, storage);

                for index in (0..length).rev() {
                    let index = self.builder.usize_const(u128::from(index));
                    let element_pointer =
                        self.builder
                            .element_addr(pointer, index, element_pointer_type);

                    self.drop_at(element, element_pointer, storage);
                }
            }
            mir::Type::Slice {
                kind: mir::ReferenceKind::Unique,
                element,
                storage,
                access,
                ..
            } => {
                if !Self::type_emits(self.drops, self.builder.tree(), element, storage) {
                    return;
                }

                let value = self.builder.load(pointer, ty);

                self.drop_slice(element, storage, access, value);
            }
            mir::Type::Variant {
                discriminant,
                cases,
                ..
            } => self.drop_variant(ty, pointer, discriminant, storage, cases),
            _ => {}
        }
    }

    /// Emit drop for an owning slice descriptor.
    fn drop_slice(
        &mut self,
        element: mir::TypeId,
        storage: mir::Storage,
        access: mir::Access,
        value: mir::Value,
    ) {
        // build loop state
        let usize_type = self.builder.tree_mut().intern_type(mir::Type::Usize);
        let element_pointer = self.reference_type(
            mir::ReferenceKind::Unique,
            element,
            access,
            storage,
            mir::Nullability::None,
        );
        let index = self.builder.variable(usize_type);
        let header = self.builder.block();
        let body = self.builder.block();
        let done = self.builder.block();
        let length = self.builder.slice_length(value);
        let zero = self.builder.usize_const(0);

        // initialize one-past the last element
        self.builder.define_variable(index, length);
        self.builder.jump(header);

        // branch while stored elements remain
        self.builder.switch_to_block(header);
        let current = self.builder.use_variable(index);
        let has_element = self.builder.icmp_ne(current, zero);
        self.builder.branch(has_element, body, done);

        // drop the preceding element
        self.builder.switch_to_block(body);
        let one = self.builder.usize_const(1);
        let next = self.builder.isub(current, one);
        let pointer = self.builder.element_addr(value, next, element_pointer);
        self.drop_at(element, pointer, storage);

        // advance toward the first element
        self.builder.define_variable(index, next);
        self.builder.jump(header);

        self.builder.switch_to_block(done);
    }

    /// Emit drop for one logical variant value.
    fn drop_variant(
        &mut self,
        variant_type: mir::TypeId,
        pointer: mir::Value,
        discriminant_type: mir::TypeId,
        storage: mir::Storage,
        cases: Vec<mir::VariantCase>,
    ) {
        let cases = cases
            .into_iter()
            .enumerate()
            .filter(|(_, case)| Self::type_emits(self.drops, self.builder.tree(), case.ty, storage))
            .collect::<Vec<_>>();
        if cases.is_empty() {
            return;
        }

        // read only the encoded discriminant before dispatching its active case
        let discriminant = self.builder.variant_tag_load(pointer, variant_type);

        // build one payload block per nontrivial case
        let case_blocks = cases
            .iter()
            .map(|_| self.builder.block())
            .collect::<Vec<_>>();
        let done = self.builder.block();
        let check_blocks = cases
            .iter()
            .skip(1)
            .map(|_| self.builder.block())
            .collect::<Vec<_>>();
        let mut check = self.builder.current_block();
        let case_count = cases.len();

        // dispatch on the active tag
        for (dispatch_index, (case_index, case)) in cases.into_iter().enumerate() {
            let case_block = case_blocks[dispatch_index];
            let failure = if dispatch_index + 1 == case_count {
                done
            } else {
                check_blocks[dispatch_index]
            };

            self.builder.switch_to_block(check);
            let expected = self
                .builder
                .constant(case.discriminant.clone(), discriminant_type);
            let is_case = self.builder.icmp_eq(discriminant, expected);
            self.builder.branch(is_case, case_block, failure);

            // drop matching payload
            self.builder.switch_to_block(case_block);
            let payload_pointer_type = self.storage_pointer_type(case.ty, storage);
            let payload_pointer =
                self.builder
                    .variant_payload_addr(pointer, case_index as u32, payload_pointer_type);
            self.drop_at(case.ty, payload_pointer, storage);
            self.builder.jump(done);

            check = failure;
        }

        self.builder.switch_to_block(done);
    }

    /// Emit drop for one value at its storage address.
    fn drop_at(&mut self, ty: mir::TypeId, pointer: mir::Value, storage: mir::Storage) {
        // run the user hook before destroying owned children
        if let Some(hook) = self.drops.hook(ty, storage) {
            self.call_hook(pointer, hook);
        }

        // recurse through unique indirection at runtime
        let unique = match self.builder.tree().get(ty) {
            mir::Type::Reference {
                kind: mir::ReferenceKind::Unique,
                pointee,
                storage,
                ..
            } => Some((*pointee, *storage)),
            _ => None,
        };
        if let Some((pointee, storage)) = unique {
            let value = self.builder.load(pointer, ty);
            self.drop_unique_reference(value, pointee, storage);
            self.builder.free(value);

            return;
        }

        self.drop_inline(ty, pointer, storage);

        // release owning descriptors after destroying their contents
        if self.builder.tree().get(ty).is_unique_storage() {
            let value = self.builder.load(pointer, ty);
            self.builder.free(value);
        }
    }

    /// Call one user-authored drop hook.
    fn call_hook(&mut self, pointer: mir::Value, function: mir::FunctionId) {
        let parameter = self.builder.tree().get(function).parameters[0].ty;
        let receiver = self.builder.bitcast(pointer, parameter);
        let void = self.builder.tree_mut().void_type();
        let signature = self
            .builder
            .tree_mut()
            .intern_type(mir::Type::FunctionSignature {
                lifetimes: Vec::new(),
                parameters: vec![mir::SignatureParameter::new(parameter)],
                result: void,
            });

        self.builder
            .call(mir::Callee::Direct { function }, signature, vec![receiver]);
    }

    /// Return one exclusive borrowed reference type for generated destruction code.
    fn storage_pointer_type(&mut self, pointee: mir::TypeId, storage: mir::Storage) -> mir::TypeId {
        self.reference_type(
            mir::ReferenceKind::Borrowed,
            pointee,
            mir::Access::Exclusive,
            storage,
            mir::Nullability::None,
        )
    }

    /// Return a reference type for generated destruction code.
    fn reference_type(
        &mut self,
        kind: mir::ReferenceKind,
        pointee: mir::TypeId,
        access: mir::Access,
        storage: mir::Storage,
        nullability: mir::Nullability,
    ) -> mir::TypeId {
        let reference = mir::Type::Reference {
            kind,
            lifetime: mir::Lifetime::empty(),
            storage,
            access,
            pointee,
            nullability,
        };

        self.builder.tree_mut().intern_type(reference)
    }

    /// Emit drop for one unique reference's pointee.
    fn drop_unique_reference(
        &mut self,
        value: mir::Value,
        pointee: mir::TypeId,
        storage: mir::Storage,
    ) {
        let Some(function) = self.drops.destructor(pointee, storage) else {
            return;
        };
        let parameters = self.builder.tree().get(function).parameters.clone();
        let pointer = self.builder.bitcast(value, parameters[0].ty);
        let void = self.builder.tree_mut().void_type();
        let signature = self
            .builder
            .tree_mut()
            .intern_type(mir::Type::FunctionSignature {
                lifetimes: Vec::new(),
                parameters: parameters
                    .iter()
                    .map(|parameter| mir::SignatureParameter::new(parameter.ty))
                    .collect(),
                result: void,
            });

        self.builder
            .call(mir::Callee::Direct { function }, signature, vec![pointer]);
    }

    /// Return whether dropping a value of this type emits MIR.
    fn type_emits(
        drops: &mir::DropTable,
        tree: &mir::Tree,
        ty: mir::TypeId,
        storage: mir::Storage,
    ) -> bool {
        if drops.destructor(ty, storage).is_some() || tree.get(ty).is_unique_storage() {
            return true;
        }

        match tree.get(ty) {
            mir::Type::Reference {
                kind: mir::ReferenceKind::Unique,
                pointee,
                storage,
                ..
            } => Self::type_emits(drops, tree, *pointee, *storage),
            _ => false,
        }
    }
}

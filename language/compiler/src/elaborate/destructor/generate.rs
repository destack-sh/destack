use std::collections::HashSet;

use destack_core::StringId;
use destack_mir as mir;

use crate::elaborate::ElaborateState;

impl ElaborateState<'_> {
    /// Generate missing destructors.
    pub(in crate::elaborate) fn generate_destructors(&mut self) {
        let mut types = Vec::new();
        let mut building = HashSet::new();
        self.collect_drop_roots(&mut types);
        self.collect_allocation_drop_types(&mut types);
        types.sort_unstable();
        types.dedup();

        // generate destructors for every reachable value and allocation type
        for ty in types {
            self.generate_reachable_destructor(ty, &mut building);
        }
    }

    /// Collect types stored by managed heap allocation instructions.
    fn collect_allocation_drop_types(&self, allocations: &mut Vec<mir::TypeId>) {
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
                        } if self.is_managed_storage_type(*result_type) => {
                            allocations.push(*storage_type);
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
                        } if self.is_managed_storage_type(*result_type) => {
                            allocations.push(*element);
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
                    } if self.target_is_managed(success) => allocations.push(*storage_type),
                    mir::Terminator::NewSliceZeroedTry {
                        element, success, ..
                    }
                    | mir::Terminator::NewSliceUninitTry {
                        element, success, ..
                    } if self.target_is_managed(success) => {
                        allocations.push(*element);
                    }
                    _ => {}
                }
            }
        }
    }

    /// Return whether one block target receives managed storage.
    fn target_is_managed(&self, target: &mir::BlockTarget) -> bool {
        self.tree
            .get(target.block)
            .parameters
            .first()
            .is_some_and(|parameter| self.is_managed_storage_type(parameter.ty))
    }

    /// Return whether one type represents managed heap storage.
    fn is_managed_storage_type(&self, ty: mir::TypeId) -> bool {
        let mut ty = ty;

        // peel transparent and initialization wrappers to the allocation carrier
        loop {
            ty = self.tree.repr_type(ty);
            let mir::Type::Uninit { value } = self.tree.get(ty) else {
                break;
            };

            ty = *value;
        }

        self.tree.get(ty).reference_kind() == Some(mir::ReferenceKind::Managed)
    }

    /// Collect types that may need destructors from module values.
    fn collect_drop_roots(&self, roots: &mut Vec<mir::LocalNodeId<mir::Type>>) {
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

            for local in function.locals() {
                let local = self.tree.get(*local);
                roots.push(local.ty);
            }

            for ty in function.value_types().iter().flatten() {
                roots.push(*ty);
            }
        }
    }

    /// Generate destructors reachable through one value type.
    fn generate_reachable_destructor(
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
                self.destructor(*pointee, building);
            }
            _ => {
                self.destructor(ty, building);
            }
        }
    }

    /// Return the existing or generated destructor for one type.
    fn destructor(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        building: &mut HashSet<mir::LocalNodeId<mir::Type>>,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        // reuse an existing destructor
        if let Some(destructor) = self.drops.destructor(ty) {
            return Some(destructor);
        }

        // skip copy and scalar shapes
        if !self.type_needs_destructor(ty) {
            return None;
        }

        // declare before recursion so cycles can refer to the symbol
        let function = self.declare_destructor(ty)?;
        self.drops.set_destructor(ty, function);

        // build the body once per active recursion chain
        if building.insert(ty) {
            self.generate_child_destructors(ty, building);
            self.build_destructor(ty, function);
            building.remove(&ty);
        }

        Some(function)
    }

    /// Return whether a type needs a generated destructor.
    fn type_needs_destructor(&self, ty: mir::LocalNodeId<mir::Type>) -> bool {
        // copy values never need destructors
        if self.tree.get(ty).copy(&self.tree).is_yes() {
            return false;
        }
        if self.drops.hook(ty).is_some() {
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
        if DestructorEmitter::type_emits(&self.drops, &self.tree, ty) {
            return true;
        }
        if self.tree.get(ty).copy(&self.tree).is_yes() || !seen.insert(ty) {
            return false;
        }

        let emits_drop = self.type_children_emit_drop(ty, seen);
        seen.remove(&ty);

        emits_drop
    }

    /// Declare the destructor for one type.
    fn declare_destructor(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        let name = self.drop_name(ty)?;
        let arguments = self.destructor_arguments(ty);
        let pointer_type = mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::empty(),
            space: mir::Space::Local,
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
    fn drop_name(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<StringId> {
        let stem = self.destructor_name_stem(ty)?;
        let drop_name = format!("{stem}.destruct");

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
                constraint, space, ..
            } => {
                let constraint = self.drop_name_stem(*constraint)?;
                format!("dynamic.{constraint}.{}", space.label())
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
        building: &mut HashSet<mir::LocalNodeId<mir::Type>>,
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
                    self.generate_reachable_destructor(field_ty, building);
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
                    self.generate_reachable_destructor(element, building);
                }
            }
            mir::Type::Newtype { inner, .. } => {
                let inner = *inner;
                self.generate_reachable_destructor(inner, building);
            }
            mir::Type::FixedArray {
                element, length, ..
            } => {
                let element = *element;
                let length = *length;
                if length > 0 {
                    self.generate_reachable_destructor(element, building);
                }
            }
            mir::Type::Slice {
                kind: mir::ReferenceKind::Unique,
                element,
                ..
            } => {
                let element = *element;
                self.generate_reachable_destructor(element, building);
            }
            mir::Type::Variant { cases, .. } => {
                let case_count = cases.len();

                // generate case functions without materializing child lists
                for index in 0..case_count {
                    let case_ty = match self.tree.get(ty) {
                        mir::Type::Variant { cases, .. } => cases[index].ty,
                        _ => unreachable!("type changed during destructor generation"),
                    };
                    self.generate_reachable_destructor(case_ty, building);
                }
            }
            _ => {}
        }
    }

    /// Build the destructor body.
    fn build_destructor(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
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
        DestructorEmitter::new(&self.drops, &mut builder).drop_at(ty, pointer);
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
    fn drop_inline(&mut self, ty: mir::TypeId, pointer: mir::Value) {
        match self.builder.tree().get(ty).clone() {
            mir::Type::Struct { fields, .. } => {
                for (index, field) in fields.iter().enumerate().rev() {
                    let field_ty = self.builder.tree().get(*field).ty;
                    let field_pointer_type = self.storage_pointer_type(field_ty);
                    let field_pointer =
                        self.builder
                            .field_addr(pointer, index as u32, field_pointer_type);

                    self.drop_at(field_ty, field_pointer);
                }
            }
            mir::Type::Tuple { elements, .. } => {
                for (index, element) in elements.iter().enumerate().rev() {
                    let element_pointer_type = self.storage_pointer_type(*element);
                    let element_pointer =
                        self.builder
                            .field_addr(pointer, index as u32, element_pointer_type);

                    self.drop_at(*element, element_pointer);
                }
            }
            mir::Type::Newtype { inner, .. } => {
                let inner_pointer_type = self.storage_pointer_type(inner);
                let inner_pointer = self.builder.bitcast(pointer, inner_pointer_type);

                self.drop_at(inner, inner_pointer);
            }
            mir::Type::FixedArray {
                element, length, ..
            } => {
                let element_pointer_type = self.storage_pointer_type(element);

                for index in (0..length).rev() {
                    let index = self.builder.usize_const(u128::from(index));
                    let element_pointer =
                        self.builder
                            .element_addr(pointer, index, element_pointer_type);

                    self.drop_at(element, element_pointer);
                }
            }
            mir::Type::Slice {
                kind: mir::ReferenceKind::Unique,
                element,
                space,
                access,
                ..
            } => {
                if !Self::type_emits(self.drops, self.builder.tree(), element) {
                    return;
                }

                let value = self.builder.load(pointer, ty);

                self.drop_slice(element, space, access, value);
            }
            mir::Type::Variant {
                discriminant,
                storage,
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
        space: mir::Space,
        access: mir::Access,
        value: mir::Value,
    ) {
        // build loop state
        let usize_type = self.builder.tree_mut().intern_type(mir::Type::Usize);
        let element_pointer = self.reference_type(
            mir::ReferenceKind::Unique,
            element,
            access,
            space,
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
        self.drop_at(element, pointer);

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
        storage_type: mir::TypeId,
        cases: Vec<mir::VariantCase>,
    ) {
        let cases = cases
            .into_iter()
            .filter(|case| Self::type_emits(self.drops, self.builder.tree(), case.ty))
            .collect::<Vec<_>>();
        if cases.is_empty() {
            return;
        }

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
        let variant = self.builder.load(pointer, variant_type);
        let discriminant = self.builder.field_get(variant, 0);
        let storage = self.builder.field_get(variant, 1);
        let storage_local = self.builder.local(storage_type, mir::Mutability::Mutable);
        self.builder.local_set(storage_local, storage);
        let storage_pointer_type = self.frame_pointer_type(storage_type);
        let storage_pointer = self.builder.local_addr(storage_local, storage_pointer_type);
        let case_count = cases.len();

        // dispatch on the active tag
        for (index, case) in cases.into_iter().enumerate() {
            let case_block = case_blocks[index];
            let failure = if index + 1 == case_count {
                done
            } else {
                check_blocks[index]
            };

            self.builder.switch_to_block(check);
            let expected = self
                .builder
                .constant(case.discriminant.clone(), discriminant_type);
            let is_case = self.builder.icmp_eq(discriminant, expected);
            self.builder.branch(is_case, case_block, failure);

            // drop matching payload
            self.builder.switch_to_block(case_block);
            let payload_pointer =
                self.variant_payload_pointer(storage_pointer, storage_type, case.ty);
            self.drop_at(case.ty, payload_pointer);
            self.builder.jump(done);

            check = failure;
        }

        self.builder.switch_to_block(done);
    }

    /// Return the address of one active variant payload.
    fn variant_payload_pointer(
        &mut self,
        storage_pointer: mir::Value,
        storage_type: mir::TypeId,
        payload_type: mir::TypeId,
    ) -> mir::Value {
        if storage_type == payload_type {
            return storage_pointer;
        }

        // follow boxed variant storage before reinterpreting its pointee
        if let mir::Type::Reference {
            kind,
            access,
            space,
            nullability,
            ..
        } = self.builder.tree().get(storage_type).clone()
        {
            let storage = self.builder.load(storage_pointer, storage_type);
            let payload_reference =
                self.reference_type(kind, payload_type, access, space, nullability);

            return self.builder.bitcast(storage, payload_reference);
        }

        // reinterpret inline variant storage in place
        let payload_pointer_type = self.frame_pointer_type(payload_type);

        self.builder.bitcast(storage_pointer, payload_pointer_type)
    }

    /// Emit drop for one value at its storage address.
    fn drop_at(&mut self, ty: mir::TypeId, pointer: mir::Value) {
        // run the user hook before destroying owned children
        if let Some(hook) = self.drops.hook(ty) {
            self.call_hook(pointer, hook);
        }

        // recurse through unique indirection at runtime
        if let mir::Type::Reference {
            kind: mir::ReferenceKind::Unique,
            pointee,
            ..
        } = self.builder.tree().get(ty)
        {
            let pointee = *pointee;
            let value = self.builder.load(pointer, ty);
            self.drop_unique_reference(value, pointee);
            self.builder.free(value);

            return;
        }

        self.drop_inline(ty, pointer);

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
    fn storage_pointer_type(&mut self, pointee: mir::TypeId) -> mir::TypeId {
        self.reference_type(
            mir::ReferenceKind::Borrowed,
            pointee,
            mir::Access::Exclusive,
            mir::Space::Local,
            mir::Nullability::None,
        )
    }

    /// Return one exclusive borrowed frame reference type for generated temporaries.
    fn frame_pointer_type(&mut self, pointee: mir::TypeId) -> mir::TypeId {
        self.reference_type(
            mir::ReferenceKind::Borrowed,
            pointee,
            mir::Access::Exclusive,
            mir::Space::Frame,
            mir::Nullability::None,
        )
    }

    /// Return a reference type for generated destruction code.
    fn reference_type(
        &mut self,
        kind: mir::ReferenceKind,
        pointee: mir::TypeId,
        access: mir::Access,
        space: mir::Space,
        nullability: mir::Nullability,
    ) -> mir::TypeId {
        let reference = mir::Type::Reference {
            kind,
            lifetime: mir::Lifetime::empty(),
            space,
            access,
            pointee,
            nullability,
        };

        self.builder.tree_mut().intern_type(reference)
    }

    /// Emit drop for one unique reference's pointee.
    fn drop_unique_reference(&mut self, value: mir::Value, pointee: mir::TypeId) {
        let Some(function) = self.drops.destructor(pointee) else {
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
    fn type_emits(drops: &mir::DropTable, tree: &mir::Tree, ty: mir::TypeId) -> bool {
        if drops.destructor(ty).is_some() || tree.get(ty).is_unique_storage() {
            return true;
        }

        match tree.get(ty) {
            mir::Type::Reference {
                kind: mir::ReferenceKind::Unique,
                pointee,
                ..
            } => Self::type_emits(drops, tree, *pointee),
            _ => false,
        }
    }
}

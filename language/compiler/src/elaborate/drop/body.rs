use destack_mir as mir;

/// Body of one generated MIR destructor.
pub(super) struct DestructorBody<'a, 'b> {
    /// The destructors and hooks available to generated code.
    drops: &'a mir::DropTable,
    /// The function under construction.
    builder: &'a mut mir::FunctionBuilder<'b>,
}

impl<'a, 'b> DestructorBody<'a, 'b> {
    /// Create one generated destructor body.
    pub(super) fn new(
        drops: &'a mir::DropTable,
        builder: &'a mut mir::FunctionBuilder<'b>,
    ) -> Self {
        Self { drops, builder }
    }

    /// Emit the body of one generated destructor.
    pub(super) fn drop_root(
        &mut self,
        ty: mir::TypeId,
        pointer: mir::Value,
        storage: mir::Storage,
    ) {
        // run the user hook before destroying owned children
        if let Some(hook) = self.drops.hook(ty, storage) {
            self.call_hook(pointer, hook);
        }

        self.drop_storage(ty, pointer, storage);
    }

    /// Emit drop for one inline value at its storage address.
    fn drop_inline(&mut self, ty: mir::TypeId, pointer: mir::Value, storage: mir::Storage) {
        match self.builder.tree().get(ty).clone() {
            // type Pair { left: File; right: File; }
            mir::Type::Struct { fields, .. } => {
                // drop fields in reverse declaration order
                for (index, field) in fields.iter().enumerate().rev() {
                    let field_ty = self.builder.tree().get(*field).ty;
                    let field_pointer_type = self.intern_pointer(field_ty, storage);
                    let field_pointer =
                        self.builder
                            .field_addr(pointer, index as u32, field_pointer_type);

                    self.drop_at(field_ty, field_pointer, storage);
                }
            }
            // type Pair = (File, File);
            mir::Type::Tuple { elements, .. } => {
                // drop elements in reverse declaration order
                for (index, element) in elements.iter().enumerate().rev() {
                    let element_pointer_type = self.intern_pointer(*element, storage);
                    let element_pointer =
                        self.builder
                            .field_addr(pointer, index as u32, element_pointer_type);

                    self.drop_at(*element, element_pointer, storage);
                }
            }
            // type Handle = newtype<File>;
            mir::Type::Newtype { inner, .. } => {
                // drop the transparent inner value
                let inner_pointer_type = self.intern_pointer(inner, storage);
                let inner_pointer = self.builder.bitcast(pointer, inner_pointer_type);

                self.drop_at(inner, inner_pointer, storage);
            }
            // type Buffer = [File; 4];
            mir::Type::FixedArray {
                element, length, ..
            } => {
                let element_pointer_type = self.intern_pointer(element, storage);

                // drop elements from the last index to the first
                for index in (0..length).rev() {
                    let index = self.builder.usize_const(u128::from(index));
                    let element_pointer =
                        self.builder
                            .element_addr(pointer, index, element_pointer_type);

                    self.drop_at(element, element_pointer, storage);
                }
            }
            // type Result = variant<uint8> { 0uint8 = File; 1uint8 = void; };
            mir::Type::Variant {
                discriminant,
                cases,
                ..
            } => self.drop_variant(ty, pointer, discriminant, storage, cases),
            // int32 and other scalar or borrowed types
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
        let element_pointer = self.intern_reference(
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
        let has_element = self
            .builder
            .binary(mir::BinaryOperator::NotEqual, current, zero);
        self.builder.branch(has_element, body, done);

        // drop the preceding element
        self.builder.switch_to_block(body);
        let one = self.builder.usize_const(1);
        let next = self
            .builder
            .binary(mir::BinaryOperator::Subtract, current, one);
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
            .filter(|(_, case)| Self::emits(self.drops, self.builder.tree(), case.ty, storage))
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

            // check tag
            self.builder.switch_to_block(check);
            let expected = self
                .builder
                .constant(case.discriminant.clone(), discriminant_type);
            let is_case = self
                .builder
                .binary(mir::BinaryOperator::Equal, discriminant, expected);
            self.builder.branch(is_case, case_block, failure);

            // drop matching payload
            self.builder.switch_to_block(case_block);
            let payload_pointer_type = self.intern_pointer(case.ty, storage);
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
    pub(super) fn drop_at(&mut self, ty: mir::TypeId, pointer: mir::Value, storage: mir::Storage) {
        // call canonical drop glue for nontrivial stored values
        if let Some(function) = self.drops.destructor(ty, storage) {
            self.call(pointer, function);

            return;
        }

        self.drop_storage(ty, pointer, storage);
    }

    /// Emit drop for one value stored at an address.
    fn drop_storage(&mut self, ty: mir::TypeId, pointer: mir::Value, storage: mir::Storage) {
        match self.builder.tree().get(ty).clone() {
            // ref<File, unique, mutable>
            mir::Type::Reference {
                kind: mir::ReferenceKind::Unique,
                pointee,
                storage,
                ..
            } => {
                let value = self.builder.load(pointer, ty);
                self.drop_unique(value, pointee, storage);
                self.builder.free(value);
            }
            // slice<File, unique, mutable>
            mir::Type::Slice {
                kind: mir::ReferenceKind::Unique,
                element,
                storage,
                access,
                ..
            } => {
                let value = self.builder.load(pointer, ty);
                if Self::emits(self.drops, self.builder.tree(), element, storage) {
                    self.drop_slice(element, storage, access, value);
                }
                self.builder.free(value);
            }
            // dynamic<Writer, unique, mutable> or function<() => void, once, unique, mutable>
            mir::Type::Dynamic {
                kind: mir::ReferenceKind::Unique,
                ..
            }
            | mir::Type::Function {
                kind: mir::ReferenceKind::Unique,
                ..
            } => {
                let value = self.builder.load(pointer, ty);
                self.builder.drop_value(value);
                self.builder.free(value);
            }
            // Pair and other inline values
            _ => self.drop_inline(ty, pointer, storage),
        }
    }

    /// Call one user-authored drop hook with the storage reference.
    fn call_hook(&mut self, pointer: mir::Value, function: mir::FunctionId) {
        let (parameter, signature) = {
            let function = self.builder.tree().get(function);
            let [parameter] = function.parameters.as_slice() else {
                unreachable!("drop hook must accept one storage reference");
            };

            (parameter.ty, function.signature())
        };
        let pointer_type = self.builder.value_type(pointer);
        let receiver = if pointer_type == Some(parameter) {
            pointer
        } else {
            self.builder.bitcast(pointer, parameter)
        };
        let signature = self.builder.tree_mut().intern_type(signature);

        self.builder
            .call(mir::Callee::Direct { function }, signature, vec![receiver]);
    }

    /// Call one generated destructor.
    fn call(&mut self, pointer: mir::Value, function: mir::FunctionId) {
        let (parameter, signature) = {
            let function = self.builder.tree().get(function);
            let [parameter] = function.parameters.as_slice() else {
                unreachable!("generated destructor must accept one storage reference");
            };

            (parameter.ty, function.signature())
        };
        let pointer_type = self.builder.value_type(pointer);
        let pointer = if pointer_type == Some(parameter) {
            pointer
        } else {
            self.builder.bitcast(pointer, parameter)
        };
        let signature = self.builder.tree_mut().intern_type(signature);

        self.builder
            .call(mir::Callee::Direct { function }, signature, vec![pointer]);
    }

    /// Intern one exclusive borrowed reference for generated destruction code.
    fn intern_pointer(&mut self, pointee: mir::TypeId, storage: mir::Storage) -> mir::TypeId {
        self.intern_reference(
            mir::ReferenceKind::Borrowed,
            pointee,
            mir::Access::Exclusive,
            storage,
            mir::Nullability::None,
        )
    }

    /// Intern one reference type for generated destruction code.
    fn intern_reference(
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
    fn drop_unique(&mut self, value: mir::Value, pointee: mir::TypeId, storage: mir::Storage) {
        let Some(function) = self.drops.destructor(pointee, storage) else {
            return;
        };
        let (parameter, signature) = {
            let function = self.builder.tree().get(function);
            let [parameter] = function.parameters.as_slice() else {
                unreachable!("generated destructor must accept one storage reference");
            };

            (parameter.ty, function.signature())
        };
        let pointer = self.builder.bitcast(value, parameter);
        let signature = self.builder.tree_mut().intern_type(signature);

        self.builder
            .call(mir::Callee::Direct { function }, signature, vec![pointer]);
    }

    /// Return whether dropping a value of this type emits MIR.
    pub(super) fn emits(
        drops: &mir::DropTable,
        tree: &mir::Tree,
        ty: mir::TypeId,
        storage: mir::Storage,
    ) -> bool {
        drops.destructor(ty, storage).is_some() || tree.get(ty).is_unique_storage()
    }
}

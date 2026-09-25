use tspp_mir as mir;

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
        if let Some(hook) = self.drops.hook(ty) {
            self.call(pointer, hook);
        }

        self.drop_storage(ty, pointer, storage);
    }

    /// Emit drop for one inline value at its storage address.
    fn drop_inline(&mut self, ty: mir::TypeId, pointer: mir::Value, storage: mir::Storage) {
        // drop an application through the type it stands for
        let ty = mir::Substitution::resolve(ty, self.builder.tree());
        match self.builder.tree().type_definition(ty).clone() {
            // type Pair { left: File; right: File; }
            mir::Type::Struct { fields, .. } => {
                // drop fields in reverse declaration order
                for (index, field) in fields.iter().enumerate().rev() {
                    let field_ty = self.builder.tree().get(*field).ty;
                    let field_pointer_type = self.intern_pointer(field_ty);
                    let field_pointer = self.builder.address(
                        mir::Place::value(pointer)
                            .with_projection(mir::Projection::Deref)
                            .with_projection(mir::Projection::Field {
                                index: index as u32,
                            }),
                        field_pointer_type,
                    );

                    self.drop_at(field_ty, field_pointer, storage);
                }
            }
            // type Pair = (File, File);
            mir::Type::Tuple { elements, .. } => {
                // drop elements in reverse declaration order
                for (index, element) in elements.iter().enumerate().rev() {
                    let element_pointer_type = self.intern_pointer(*element);
                    let element_pointer = self.builder.address(
                        mir::Place::value(pointer)
                            .with_projection(mir::Projection::Deref)
                            .with_projection(mir::Projection::Field {
                                index: index as u32,
                            }),
                        element_pointer_type,
                    );

                    self.drop_at(*element, element_pointer, storage);
                }
            }
            // type Handle = newtype<File>;
            mir::Type::Newtype { value, .. } => {
                // drop the transparent value
                let value_pointer_type = self.intern_pointer(value);
                let value_pointer = self.builder.bitcast(pointer, value_pointer_type);

                self.drop_at(value, value_pointer, storage);
            }
            // type Buffer = [File; 4];
            mir::Type::FixedArray {
                element, length, ..
            } => {
                let element_pointer_type = self.intern_pointer(element);

                // drop elements from the last index to the first
                let length = self
                    .builder
                    .tree()
                    .static_value(length)
                    .length()
                    .unwrap_or_else(|| unreachable!("drop bodies close every array length"));
                for index in (0..length).rev() {
                    let index = self.builder.usize_const(u128::from(index));
                    let element_pointer = self.builder.address(
                        mir::Place::value(pointer)
                            .with_projection(mir::Projection::Deref)
                            .with_projection(mir::Projection::Index { index }),
                        element_pointer_type,
                    );

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
        let place = mir::Place::value(pointer).with_projection(mir::Projection::Deref);
        let discriminant = self.builder.variant_tag_load(place, variant_type);

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
            let payload_pointer_type = self.intern_pointer(case.ty);
            let payload_pointer = self.builder.address(
                mir::Place::value(pointer)
                    .with_projection(mir::Projection::Deref)
                    .with_projection(mir::Projection::Variant {
                        case: case_index as u32,
                    }),
                payload_pointer_type,
            );
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
        match self.builder.tree().type_definition(ty).clone() {
            // ref<File, unique, mutable>, slice<File, unique, mutable>, dynamic<Writer, unique>
            mir::Type::Reference {
                kind: mir::Reference::Unique,
                ..
            }
            | mir::Type::Slice {
                kind: mir::Reference::Unique,
                ..
            }
            | mir::Type::Dynamic {
                kind: mir::Reference::Unique,
                ..
            }
            | mir::Type::Function {
                kind: mir::Reference::Unique,
                ..
            } => {
                let place = mir::Place::value(pointer).with_projection(mir::Projection::Deref);
                let value = self.builder.load(place, ty);
                self.builder.release(value);
            }
            // Pair and other inline values
            _ => self.drop_inline(ty, pointer, storage),
        }
    }

    /// Call a destructor or drop hook with its storage reference.
    fn call(&mut self, pointer: mir::Value, function: mir::FunctionId) {
        let (parameter, signature) = {
            let function = self.builder.tree().get(function);
            let [parameter] = function.parameters.as_slice() else {
                unreachable!("generated destructor must accept one storage reference");
            };

            (parameter.ty, function.signature())
        };

        // reborrow the storage at the access the callee declares
        let pointer_type = self.builder.value_type(pointer);
        let pointer = if pointer_type == Some(parameter) {
            pointer
        } else {
            let place = mir::Place::value(pointer).with_projection(mir::Projection::Deref);
            self.builder.address(place, parameter)
        };
        let signature = self.builder.tree_mut().intern_type(signature);
        let result = self.builder.signature_result(signature);

        self.builder.call(
            mir::Callee::Direct {
                function,
                arguments: Vec::new(),
            },
            signature,
            vec![pointer],
            result,
        );
    }

    /// Intern one exclusive borrowed reference for generated destruction code.
    fn intern_pointer(&mut self, pointee: mir::TypeId) -> mir::TypeId {
        let reference = mir::Type::Reference {
            kind: mir::Reference::Borrowed,
            lifetime: mir::Lifetime::bound(0),
            access: mir::Access::Exclusive,
            pointee,
        };

        self.builder.tree_mut().intern_type(reference)
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

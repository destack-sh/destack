use destack_mir as mir;

/// Inserter for the box operations the boxed variant cases of instantiated MIR need.
pub(in crate::elaborate) struct BoxInserter<'a> {
    /// The MIR tree receiving explicit box operations.
    tree: &'a mut mir::Tree,
}

impl<'a> BoxInserter<'a> {
    /// Create one MIR box inserter.
    pub(in crate::elaborate) fn new(tree: &'a mut mir::Tree) -> Self {
        Self { tree }
    }

    /// Rewrite every variant operation on a boxed case into its box operations.
    pub(in crate::elaborate) fn insert(&mut self, functions: &[mir::LocalNodeId<mir::Function>]) {
        for &function in functions {
            let blocks = self.tree.get(function).blocks().to_vec();
            for block in blocks {
                let instructions = self.tree.get(block).instructions.clone();
                let mut rewritten = Vec::with_capacity(instructions.len());
                for instruction in instructions {
                    self.rewrite(function, instruction, &mut rewritten);
                }
                self.tree.get_mut(block).instructions = rewritten;
            }
        }
    }

    /// Rewrite one instruction in place, adding the box operations around it.
    fn rewrite(
        &mut self,
        function: mir::LocalNodeId<mir::Function>,
        id: mir::LocalNodeId<mir::Instruction>,
        rewritten: &mut Vec<mir::LocalNodeId<mir::Instruction>>,
    ) {
        match self.tree.get(id).clone() {
            // move the payload into a fresh box the case owns
            mir::Instruction::VariantNew {
                destination,
                case,
                payload: Some(payload),
                result_type,
            } if let Some(box_type) = self.boxed_case(result_type, case) => {
                let boxed = self.value(function, box_type);
                rewritten.push(self.tree.insert(mir::Instruction::NewComplete {
                    destination: boxed,
                    value: payload,
                    result_type: box_type,
                }));
                *self.tree.get_mut(id) = mir::Instruction::VariantNew {
                    destination,
                    case,
                    payload: Some(boxed),
                    result_type,
                };
                rewritten.push(id);
            }
            // take the payload out of the box and return the shell
            mir::Instruction::VariantPayload {
                destination,
                variant,
                case,
            } if let Some(box_type) = self.boxed_case(self.value_type(function, variant), case) => {
                let boxed = self.value(function, box_type);
                let payload_type = self.value_type(function, destination);
                *self.tree.get_mut(id) = mir::Instruction::VariantPayload {
                    destination: boxed,
                    variant,
                    case,
                };
                rewritten.push(id);
                rewritten.push(self.tree.insert(mir::Instruction::Load {
                    destination,
                    pointer: boxed,
                    result_type: payload_type,
                }));
                rewritten.push(self.tree.insert(mir::Instruction::Release { value: boxed }));
            }
            // address the box slot and read the box as the borrow of its contents
            mir::Instruction::VariantPayloadAddr {
                destination,
                variant,
                case,
                result_type,
                kind: _,
            } if let Some(box_type) = self.boxed_case(self.pointee(function, variant), case) => {
                let (storage, access, lifetime) = match self.tree.get(result_type) {
                    mir::Type::Reference {
                        storage,
                        access,
                        lifetime,
                        ..
                    } => (*storage, *access, lifetime.clone()),
                    _ => (
                        mir::Storage::heap(mir::Space::Local),
                        mir::Access::Mutable,
                        mir::Lifetime::frame(),
                    ),
                };
                let slot_type = self.tree.intern_type(mir::Type::Reference {
                    kind: mir::ReferenceKind::Borrowed,
                    lifetime,
                    storage,
                    access,
                    pointee: box_type,
                });
                let slot = self.value(function, slot_type);
                *self.tree.get_mut(id) = mir::Instruction::VariantPayloadAddr {
                    destination: slot,
                    variant,
                    case,
                    result_type: slot_type,
                    kind: mir::AddressKind::Projection,
                };
                rewritten.push(id);
                rewritten.push(self.tree.insert(mir::Instruction::Load {
                    destination,
                    pointer: slot,
                    result_type,
                }));
            }
            _ => rewritten.push(id),
        }
    }

    /// Return the box type one variant stores at a case, when the case is boxed.
    fn boxed_case(&self, variant: mir::TypeId, case: u32) -> Option<mir::TypeId> {
        let stored = self.tree.storage_type(variant);
        let mir::Type::Variant { cases, .. } = self.tree.get(stored) else {
            return None;
        };

        cases
            .get(case as usize)
            .filter(|case| case.is_boxed)
            .map(|case| case.ty)
    }

    /// Return the type one function value holds.
    fn value_type(
        &self,
        function: mir::LocalNodeId<mir::Function>,
        value: mir::Value,
    ) -> mir::TypeId {
        self.tree
            .get(function)
            .value_type(value)
            .unwrap_or_else(|| unreachable!("a variant operation on an untyped value"))
    }

    /// Return the type behind one function's reference value.
    fn pointee(&self, function: mir::LocalNodeId<mir::Function>, value: mir::Value) -> mir::TypeId {
        let held = self.value_type(function, value);
        match self.tree.get(held) {
            mir::Type::Reference { pointee, .. } | mir::Type::Pointer { pointee, .. } => *pointee,
            _ => held,
        }
    }

    /// Allocate one typed value in a function.
    fn value(&mut self, function: mir::LocalNodeId<mir::Function>, ty: mir::TypeId) -> mir::Value {
        self.tree.get_mut(function).next_typed_value(ty)
    }
}

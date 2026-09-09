use destack_mir as mir;

/// MIR instruction emitter for one planned drop.
pub(in crate::elaborate) struct DropEmitter<'a> {
    /// The MIR tree receiving generated values and types.
    tree: &'a mut mir::Tree,
    /// Canonical MIR drop table.
    drops: &'a mir::DropTable,
    /// Function receiving the instructions.
    function: mir::LocalNodeId<mir::Function>,
    /// Block where the instructions execute.
    block: mir::LocalNodeId<mir::Block>,
    /// Move paths planned against the original function.
    paths: &'a mir::MoveTable,
    /// Instructions emitted so far.
    instructions: Vec<mir::Instruction>,
}

impl<'a> DropEmitter<'a> {
    /// Create an emitter at one function block.
    pub(in crate::elaborate) fn new(
        tree: &'a mut mir::Tree,
        drops: &'a mir::DropTable,
        function: mir::LocalNodeId<mir::Function>,
        block: mir::LocalNodeId<mir::Block>,
        paths: &'a mir::MoveTable,
    ) -> Self {
        Self {
            tree,
            drops,
            function,
            block,
            paths,
            instructions: Vec::new(),
        }
    }

    /// Emit destruction for one initialized move path.
    pub(in crate::elaborate) fn emit(mut self, path: mir::MovePathId) -> Vec<mir::Instruction> {
        // select the value that carries the planned path
        let root = self.paths.root(path);
        let origin = self.paths.get(root).place.origin;
        let value = match (root == path, origin) {
            // use a complete SSA root directly
            (true, mir::PlaceOrigin::Value(_)) => self.root_value(root),
            // load a complete frame local
            (true, mir::PlaceOrigin::Local(local)) => {
                let ty = self.paths.get(path).ty;
                let value = self.allocate_value(ty);
                self.instructions.push(mir::Instruction::LocalGet {
                    destination: value,
                    local,
                });

                value
            }
            // extract a projected SSA child
            (false, mir::PlaceOrigin::Value(_)) => {
                let value = self.root_value(root);

                self.extract(value, path)
            }
            // load a projected frame child
            (false, mir::PlaceOrigin::Local(local)) => self.load(local, path),
            (_, mir::PlaceOrigin::Global(_)) => {
                unreachable!("function drop path cannot own global storage")
            }
        };
        let ty = self.paths.get(path).ty;
        self.emit_value(value, ty);

        self.instructions
    }

    /// Return the SSA value carrying one root in this block.
    fn root_value(&self, path: mir::MovePathId) -> mir::Value {
        // prefer the block parameter carrying an incoming owner
        let parameter = self
            .tree
            .get(self.block)
            .parameters
            .iter()
            .map(|parameter| parameter.value)
            .find(|value| self.paths.value(*value) == Some(path));
        if let Some(parameter) = parameter {
            return parameter;
        }

        let mir::PlaceOrigin::Value(value) = self.paths.get(path).place.origin else {
            unreachable!("non-value move path requested as an SSA value");
        };

        value
    }

    /// Extract one projected child from an SSA aggregate.
    fn extract(&mut self, root: mir::Value, path: mir::MovePathId) -> mir::Value {
        let mut value = root;

        // extract each structural child toward the selected path
        for child in self.path_chain(path).into_iter().skip(1) {
            let child = self.paths.get(child);
            let projection = child
                .place
                .path
                .projections
                .last()
                .unwrap_or_else(|| unreachable!("child move path has no projection"));
            let destination = self.allocate_value(child.ty);
            let instruction = match projection {
                mir::Projection::Field { index } => mir::Instruction::FieldGet {
                    destination,
                    aggregate: value,
                    field: *index,
                },
                mir::Projection::Element { index } => mir::Instruction::ElementGet {
                    destination,
                    aggregate: value,
                    index: *index,
                },
                _ => unreachable!("move path contains a dynamic projection"),
            };
            self.instructions.push(instruction);
            value = destination;
        }

        value
    }

    /// Load one projected child from a frame local.
    fn load(&mut self, local: mir::LocalId, path: mir::MovePathId) -> mir::Value {
        // address the root frame local
        let chain = self.path_chain(path);
        let root = self.paths.get(chain[0]);
        let pointer_type = self.intern_reference(root.ty);
        let mut pointer = self.allocate_value(pointer_type);
        self.instructions.push(mir::Instruction::LocalAddr {
            destination: pointer,
            local,
            result_type: pointer_type,
            kind: mir::AddressKind::Projection,
        });

        // project an address through every structural child
        for child in chain.into_iter().skip(1) {
            let child = self.paths.get(child);
            let result_type = self.intern_reference(child.ty);
            let destination = self.allocate_value(result_type);
            let projection = child
                .place
                .path
                .projections
                .last()
                .unwrap_or_else(|| unreachable!("child move path has no projection"));
            match projection {
                mir::Projection::Field { index } => {
                    self.instructions.push(mir::Instruction::FieldAddr {
                        destination,
                        aggregate: pointer,
                        field: *index,
                        result_type,
                        kind: mir::AddressKind::Projection,
                    });
                }
                mir::Projection::Element { index } => {
                    let index_type = self.tree.intern_type(mir::Type::Usize);
                    let index_value = self.allocate_value(index_type);
                    self.instructions.push(mir::Instruction::Const {
                        destination: index_value,
                        value: mir::Constant::uint64(u64::from(*index)),
                    });
                    self.instructions.push(mir::Instruction::ElementAddr {
                        destination,
                        base: pointer,
                        index: index_value,
                        result_type,
                        kind: mir::AddressKind::Projection,
                    });
                }
                _ => unreachable!("move path contains a dynamic projection"),
            }
            pointer = destination;
        }

        let ty = self.paths.get(path).ty;
        let value = self.allocate_value(ty);
        self.instructions.push(mir::Instruction::Load {
            destination: value,
            pointer,
            result_type: ty,
        });

        value
    }

    /// Return one move path from its root through the selected child.
    fn path_chain(&self, path: mir::MovePathId) -> Vec<mir::MovePathId> {
        // walk from the child back to its root
        let mut chain = vec![path];
        let mut current = path;

        while let Some(parent) = self.paths.get(current).parent {
            chain.push(parent);
            current = parent;
        }
        chain.reverse();

        chain
    }

    /// Emit destruction for one concrete SSA value.
    fn emit_value(&mut self, value: mir::Value, ty: mir::TypeId) {
        // detect whether Drop returns this representation
        let requires_destructor =
            self.drops
                .requires_destructor(ty, mir::Storage::Frame, self.tree);
        self.emit_contents(value, ty);

        // return trivial unique storage after its contents
        if self.tree.get(ty).is_unique_storage() && !requires_destructor {
            self.instructions.push(mir::Instruction::Release { value });
        }
    }

    /// Emit destruction for one value's contents.
    fn emit_contents(&mut self, value: mir::Value, ty: mir::TypeId) {
        if self
            .drops
            .requires_destructor(ty, mir::Storage::Frame, self.tree)
        {
            self.instructions.push(mir::Instruction::Drop { value });

            return;
        }

        match self.tree.get(ty).clone() {
            // dispatch erased payload and closure environment destruction at runtime
            mir::Type::Dynamic {
                kind: mir::ReferenceKind::Unique,
                ..
            }
            | mir::Type::Function {
                kind: mir::ReferenceKind::Unique,
                ..
            } => self.instructions.push(mir::Instruction::Drop { value }),
            // statically destroy concrete unique pointees
            mir::Type::Reference {
                kind: mir::ReferenceKind::Unique,
                pointee,
                storage,
                ..
            } => self.emit_unique(value, pointee, storage),
            _ => {}
        }
    }

    /// Emit the destructor call for one unique pointee.
    fn emit_unique(&mut self, value: mir::Value, pointee: mir::TypeId, storage: mir::Storage) {
        let Some(function) = self.drops.destructor(pointee, storage) else {
            return;
        };
        let (parameter, signature) = {
            let function = self.tree.get(function);
            let [parameter] = function.parameters.as_slice() else {
                unreachable!("generated destructor must accept one storage reference");
            };

            (parameter.ty, function.signature())
        };

        // borrow the pointee for the call, filling the destructor's binder with the frame
        let parameter = mir::instantiate_slots(self.tree, parameter, &[mir::Lifetime::frame()]);

        // enter the storage-specific destructor at the allocation address
        let value_type = self.tree.get(self.function).expect_value_type(value);
        let pointer = if value_type == parameter {
            value
        } else {
            let pointer = self.allocate_value(parameter);
            self.instructions.push(mir::Instruction::Cast {
                destination: pointer,
                operator: mir::CastOperator::Bitcast,
                argument: value,
                to_type: parameter,
            });

            pointer
        };
        let signature = self.tree.intern_type(signature);
        let arguments = self.tree.add_values(&[pointer]);
        self.instructions.push(mir::Instruction::Call {
            destination: None,
            call: mir::Call::new(
                mir::Callee::Direct {
                    function,
                    arguments: Vec::new(),
                },
                arguments,
                signature,
            ),
        });
    }

    /// Intern the exclusive frame reference used to load a drop path.
    fn intern_reference(&mut self, pointee: mir::TypeId) -> mir::TypeId {
        self.tree.intern_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::frame(),
            storage: mir::Storage::Frame,
            access: mir::Access::Mutable,
            pointee,
        })
    }

    /// Allocate one typed SSA value in the rewritten function.
    fn allocate_value(&mut self, ty: mir::TypeId) -> mir::Value {
        self.tree.get_mut(self.function).next_typed_value(ty)
    }
}

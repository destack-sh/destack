use tspp_mir as mir;

use super::DropAction;

/// MIR instruction emitter for one planned drop.
pub(in crate::elaborate) struct DropEmitter<'a> {
    /// The MIR tree receiving generated values and types.
    tree: &'a mut mir::Tree,
    /// Canonical MIR drop table.
    drops: &'a mir::DropTable,
    /// Function receiving the instructions.
    function: mir::LocalNodeId<mir::Function>,
    /// The block receiving the generated instructions.
    block: mir::BlockId,
    /// Move paths planned against the original function.
    paths: &'a mir::MoveTable,
    /// Instructions emitted so far.
    instructions: Vec<mir::Instruction>,
}

impl<'a> DropEmitter<'a> {
    /// Create an emitter for one function.
    pub(in crate::elaborate) fn new(
        tree: &'a mut mir::Tree,
        drops: &'a mir::DropTable,
        function: mir::LocalNodeId<mir::Function>,
        block: mir::BlockId,
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

    /// Emit the planned destruction or allocation release.
    pub(in crate::elaborate) fn emit(mut self, action: DropAction) -> Vec<mir::Instruction> {
        // select the complete storage path recorded by ownership analysis
        let path = action.path();
        let ty = self.paths.get(path).ty;
        if matches!(action, DropAction::Drop(_)) && !self.destroys(ty) {
            return self.instructions;
        }
        let root = self.paths.root(path);
        let root_place = &self.paths.get(root).place;
        let mut place = self.paths.get(path).place.clone();

        // select the incoming owner when the allocation crosses a block parameter
        if root_place.path.projections.last() == Some(&mir::Projection::Deref)
            && let Some(parameter) = self
                .tree
                .get(self.block)
                .parameters
                .iter()
                .find(|parameter| self.paths.pointee(parameter.value) == Some(root))
        {
            let suffix = &place.path.projections[root_place.path.projections.len()..];
            let mut incoming =
                mir::Place::value(parameter.value).with_projection(mir::Projection::Deref);
            incoming.path.projections.extend_from_slice(suffix);
            place = incoming;
        }

        // extract SSA aggregates or move from the selected storage
        let value = match place.origin {
            mir::PlaceOrigin::Value(value)
                if !place.path.projections.contains(&mir::Projection::Deref) =>
            {
                if root == path {
                    value
                } else {
                    self.extract(value, path)
                }
            }
            mir::PlaceOrigin::Value(_) | mir::PlaceOrigin::Local(_) => {
                let ty = self.paths.get(path).ty;
                let value = self.allocate_value(ty);
                self.instructions.push(mir::Instruction::Load {
                    destination: value,
                    place,
                    result_type: ty,
                });

                value
            }
            mir::PlaceOrigin::Global(_) => {
                unreachable!("function drop path cannot own global storage")
            }
        };

        // destroy the value or release its emptied allocation
        match action {
            DropAction::Drop(_) => self.emit_value(value, ty),
            DropAction::Release(_) => self.emit_storage_release(value, ty),
        }

        self.instructions
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

    /// Destroy the value one store overwrites through a reference, ahead of the store.
    pub(in crate::elaborate) fn emit_overwrite(
        mut self,
        place: mir::Place,
    ) -> Vec<mir::Instruction> {
        let Some(mir::PlaceType::Value(ty)) = place.ty(self.function, self.tree) else {
            unreachable!("an overwritten place selects one value");
        };
        if !self.destroys(ty) {
            return self.instructions;
        }

        // read the old value out of the overwritten storage and destroy it
        let value = self.allocate_value(ty);
        self.instructions.push(mir::Instruction::Load {
            destination: value,
            place,
            result_type: ty,
        });
        self.emit_value(value, ty);

        self.instructions
    }

    /// Return whether destroying a value of one type emits an instruction.
    fn destroys(&self, ty: mir::TypeId) -> bool {
        self.tree.get(ty).is_unique_storage()
            || self
                .drops
                .requires_destructor(ty, mir::Storage::Frame, self.tree)
    }

    /// Emit destruction for one concrete SSA value.
    fn emit_value(&mut self, value: mir::Value, ty: mir::TypeId) {
        // return unique storage to the heap, which destroys its values
        if self.tree.get(ty).is_unique_storage() {
            self.instructions.push(mir::Instruction::Release { value });
        } else {
            self.instructions.push(mir::Instruction::Drop { value });
        }
    }

    /// Release one allocation whose values moved out, typing its storage uninitialized.
    fn emit_storage_release(&mut self, value: mir::Value, ty: mir::TypeId) {
        // retype the referent as uninitialized storage
        let moved = self.tree.emptied_type(ty);

        // release the allocation under its uninitialized type
        let storage = self.allocate_value(moved);
        self.instructions.push(mir::Instruction::Cast {
            destination: storage,
            operator: mir::CastOperator::Bitcast,
            argument: value,
            to_type: moved,
        });
        self.instructions
            .push(mir::Instruction::Release { value: storage });
    }

    /// Allocate one typed SSA value in the rewritten function.
    fn allocate_value(&mut self, ty: mir::TypeId) -> mir::Value {
        self.tree.get_mut(self.function).next_typed_value(ty)
    }
}

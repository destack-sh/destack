use std::collections::HashMap;

use destack_mir as mir;

use crate::verify::VerifyState;

use super::plan::{DropPlan, DropPoint};

impl VerifyState<'_> {
    /// Insert explicit last use drops for verified owned values.
    pub(in crate::verify) fn insert_drops(&mut self) {
        let functions = self
            .tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();

        // build and insert drops for each function
        for function_id in functions {
            if self.is_generated_drop_glue(function_id) {
                continue;
            }

            let function = self.tree.get(function_id).clone();
            if function.entry.is_none() {
                continue;
            }

            let drop_receiver = self.drop_receiver(function_id);
            let drops = DropPlan::build(&function, &self.tree, drop_receiver);
            self.insert_function_drops(function_id, drops);
        }
    }

    /// Return whether one function is generated drop glue.
    fn is_generated_drop_glue(&self, function_id: mir::LocalNodeId<mir::Function>) -> bool {
        self.tree
            .metadata
            .drop
            .glue_by_type
            .values()
            .any(|glue| glue.is_generated_function(function_id))
    }

    /// Return the value consumed by a custom drop function.
    fn drop_receiver(&self, function_id: mir::LocalNodeId<mir::Function>) -> Option<mir::Value> {
        self.tree
            .metadata
            .drop
            .hooks_by_type
            .values()
            .any(|hook| hook.is_function(function_id))
            .then_some(mir::Value::new(0))
    }

    /// Insert drops into one function body.
    fn insert_function_drops(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        drops_by_block: HashMap<mir::LocalNodeId<mir::Block>, Vec<DropPoint>>,
    ) {
        for (block_id, drops) in drops_by_block {
            let mut inserted = 0;
            for drop in drops {
                let instructions = self.instructions_for_drop(function_id, drop.place);
                let count = instructions.len();
                if count == 0 {
                    continue;
                }

                let instruction_ids = instructions
                    .into_iter()
                    .map(|instruction| self.tree.insert(instruction))
                    .collect::<Vec<_>>();

                let block = self.tree.get_mut(block_id);
                block.instructions.splice(
                    drop.index + inserted..drop.index + inserted,
                    instruction_ids,
                );
                inserted += count;
            }
        }
    }

    /// Build executable drop instructions for one place.
    fn instructions_for_drop(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        place: mir::Place,
    ) -> Vec<mir::Instruction> {
        let Some((value, ty, mut instructions)) = self.extract_drop_value(function_id, &place)
        else {
            return Vec::new();
        };

        instructions.extend(self.instructions_for_value(function_id, value, ty));

        instructions
    }

    /// Extract a projected drop place into an SSA value when needed.
    fn extract_drop_value(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        place: &mir::Place,
    ) -> Option<(
        mir::Value,
        mir::LocalNodeId<mir::Type>,
        Vec<mir::Instruction>,
    )> {
        let mir::PlaceOrigin::Value(root) = place.origin else {
            return None;
        };

        let mut value = root;
        let mut ty = self.tree.get(function_id).value_type(root)?;
        let mut instructions = Vec::new();

        // materialize each projection as a normal MIR extraction
        for projection in &place.path.projections {
            let projection_ty = self.type_for_projection(ty, projection)?;
            let destination = self.next_typed_value(function_id, projection_ty);
            let instruction = match projection {
                mir::Projection::Field { index } => mir::Instruction::FieldGet {
                    destination,
                    aggregate: value,
                    index: *index,
                },
                mir::Projection::Element { index } => mir::Instruction::FieldGet {
                    destination,
                    aggregate: value,
                    index: *index,
                },
                mir::Projection::Variant { tag } => mir::Instruction::VariantPayload {
                    destination,
                    variant: value,
                    tag: tag.clone(),
                },
                _ => return None,
            };

            instructions.push(instruction);
            value = destination;
            ty = projection_ty;
        }

        Some((value, ty, instructions))
    }

    /// Build drop instructions for a concrete SSA value.
    fn instructions_for_value(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        value: mir::Value,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Vec<mir::Instruction> {
        let mut instructions = self.instructions_for_contents(function_id, value, ty);

        // release owned storage after its contents are destroyed
        if self.tree.get(ty).is_unique_storage() {
            instructions.push(mir::Instruction::Free { value });
        }

        instructions
    }

    /// Build drop instructions for one value's contents.
    fn instructions_for_contents(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        value: mir::Value,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Vec<mir::Instruction> {
        if let Some(glue) = self.tree.metadata.drop.drop_glue(ty).cloned() {
            return self
                .instruction_for_drop_glue(value, ty, glue)
                .into_iter()
                .collect();
        }

        let mir::Type::Reference {
            kind: mir::ReferenceKind::Unique,
            pointee,
            ..
        } = self.tree.get(ty)
        else {
            return Vec::new();
        };

        self.instructions_for_unique_reference_contents(function_id, value, *pointee)
    }

    /// Build drop instructions for one unique reference's pointee.
    fn instructions_for_unique_reference_contents(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        value: mir::Value,
        pointee: mir::LocalNodeId<mir::Type>,
    ) -> Vec<mir::Instruction> {
        let mut instructions = Vec::new();

        // drop pointee contents before caller releases the allocation
        if self.type_emits_drop_code(pointee) {
            let loaded = self.next_typed_value(function_id, pointee);
            instructions.push(mir::Instruction::Load {
                destination: loaded,
                pointer: value,
                result_type: pointee,
            });
            instructions.extend(self.instructions_for_value(function_id, loaded, pointee));
        }

        instructions
    }

    /// Return whether dropping a value of this type emits MIR.
    fn type_emits_drop_code(&self, ty: mir::LocalNodeId<mir::Type>) -> bool {
        if self
            .tree
            .metadata
            .drop
            .drop_glue(ty)
            .is_some_and(|glue| !matches!(glue, mir::DropGlue::None))
        {
            return true;
        }
        if self.tree.get(ty).is_unique_storage() {
            return true;
        }

        match self.tree.get(ty) {
            mir::Type::Reference {
                kind: mir::ReferenceKind::Unique,
                pointee,
                ..
            } => self.type_emits_drop_code(*pointee),
            _ => false,
        }
    }

    /// Build one drop call for explicit drop glue.
    fn instruction_for_drop_glue(
        &mut self,
        value: mir::Value,
        ty: mir::LocalNodeId<mir::Type>,
        glue: mir::DropGlue,
    ) -> Option<mir::Instruction> {
        let void = self.tree.void_type();
        let signature = self.tree.insert_type(mir::Type::FunctionSignature {
            lifetimes: Vec::new(),
            parameters: vec![mir::SignatureParameter::new(ty)],
            result: void,
        });

        match glue {
            mir::DropGlue::Generated { function } => {
                let arguments = self.tree.add_values(&[value]);

                Some(mir::Instruction::Call {
                    destination: None,
                    function,
                    call: mir::Call::new(arguments, signature),
                })
            }
            mir::DropGlue::Dynamic { slot } => {
                let arguments = self.tree.add_values(&[]);
                let mir::Type::Dynamic { constraint } = self.tree.get(ty) else {
                    unreachable!("dynamic drop glue requires a dynamic value type");
                };

                Some(mir::Instruction::CallDynamic {
                    destination: None,
                    receiver: value,
                    constraint: *constraint,
                    slot,
                    call: mir::Call::new(arguments, signature),
                })
            }
            mir::DropGlue::None => None,
        }
    }

    /// Allocate one typed SSA value in the rewritten function.
    fn next_typed_value(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> mir::Value {
        let function = self.tree.get_mut(function_id);
        function.next_typed_value(ty)
    }

    /// Return the type reached by one place projection.
    fn type_for_projection(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        projection: &mir::Projection,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        match (self.tree.get(ty), projection) {
            (mir::Type::Struct { fields, .. }, mir::Projection::Field { index }) => fields
                .get(*index as usize)
                .map(|field| self.tree.get(*field).ty),
            (mir::Type::Tuple { elements, .. }, mir::Projection::Field { index }) => {
                elements.get(*index as usize).copied()
            }
            (mir::Type::Newtype { inner, .. }, mir::Projection::Field { index }) => {
                (*index == 0).then_some(*inner)
            }
            (mir::Type::FixedArray { element, .. }, mir::Projection::Element { .. }) => {
                Some(*element)
            }
            (mir::Type::Variant { cases, .. }, mir::Projection::Variant { tag }) => cases
                .iter()
                .find(|case| case.tag == *tag)
                .map(|case| case.ty),
            _ => None,
        }
    }
}

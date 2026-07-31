use destack_artifact::{FramePlace, FramePoint, FrameSlot, FrameState, MirOptimized};
use destack_mir as mir;
use destack_source::ModuleId;

use crate::EmitError;

use super::point::PointIndex;

/// Emit engine-neutral logical frame states from MIR liveness.
pub(super) struct FrameEmitter<'a> {
    /// Optimized MIR containing the emitted functions.
    optimized: &'a MirOptimized,
    /// Object-local operation identities.
    points: &'a PointIndex,
    /// Module receiving frame diagnostics.
    module: ModuleId,
}

impl<'a> FrameEmitter<'a> {
    /// Create one logical frame emitter.
    pub(super) const fn new(
        module: ModuleId,
        optimized: &'a MirOptimized,
        points: &'a PointIndex,
    ) -> Self {
        Self {
            optimized,
            points,
            module,
        }
    }

    /// Emit every runtime-visible frame state in logical coordinate order.
    pub(super) fn emit(&self) -> Result<Vec<FrameState>, EmitError> {
        let mut states = Vec::new();

        // emit functions in stable MIR identity order
        for (function_id, function) in self.optimized.tree.iter_nodes::<mir::Function>() {
            let Some(body) = &function.body else {
                continue;
            };
            let liveness = mir::FunctionLiveness::build(function, &self.optimized.tree);

            // retain the complete callable input before a coroutine starts
            if function.coroutine.is_some() {
                let slots = self.entry_slots(function);
                states.push(FrameState::new(FramePoint::entry(function_id), slots));
            }

            // retain every logical operation that execution can resume at
            let mut blocks = body.blocks().to_vec();
            self.points.order_blocks(&mut blocks);
            for block_id in blocks {
                let block = self.optimized.tree.get(block_id);

                for (index, instruction_id) in block.instructions.iter().enumerate() {
                    let values = liveness
                        .value_live_before_instruction(&self.optimized.tree, block_id, index)
                        .into_iter();
                    let locals = liveness
                        .local_live_before_instruction(&self.optimized.tree, block_id, index)
                        .into_iter();
                    let slots = self.slots(function, values, locals)?;
                    let point = self.points.instruction(*instruction_id);
                    states.push(FrameState::new(FramePoint::operation(point), slots));
                }

                let values = liveness
                    .value_live_before_terminator(&self.optimized.tree, block_id)
                    .into_iter();
                let locals = liveness.local_live_before_terminator(block_id).into_iter();
                let slots = self.slots(function, values, locals)?;
                let point = self.points.terminator(block_id);
                states.push(FrameState::new(FramePoint::operation(point), slots));
            }
        }

        // order states for direct point lookup and engine projection
        states.sort_unstable_by_key(|state| state.point);

        Ok(states)
    }

    /// Build one coroutine's initial logical slots.
    fn entry_slots(&self, function: &mir::Function) -> Vec<FrameSlot> {
        let mut slots = Vec::new();

        // retain the hidden environment first
        if let Some(environment) = function.environment {
            slots.push(FrameSlot::new(FramePlace::Environment, environment));
        }

        // retain explicit parameters in calling order
        for parameter in &function.parameters {
            slots.push(FrameSlot::new(
                FramePlace::Value(parameter.value),
                parameter.ty,
            ));
        }

        slots
    }

    /// Build one operation's logical slots in canonical acquisition order.
    fn slots(
        &self,
        function: &mir::Function,
        values: impl IntoIterator<Item = mir::Value>,
        locals: impl IntoIterator<Item = mir::LocalId>,
    ) -> Result<Vec<FrameSlot>, EmitError> {
        let mut values = values.into_iter().collect::<Vec<_>>();
        values.sort_unstable();
        let mut locals = locals.into_iter().collect::<Vec<_>>();
        locals.sort_unstable();
        let mut slots = Vec::new();

        // retain the hidden environment first
        if let Some(environment) = function.environment {
            slots.push(FrameSlot::new(FramePlace::Environment, environment));
        }

        // retain live function parameters in calling order
        for parameter in &function.parameters {
            if values.binary_search(&parameter.value).is_ok() {
                slots.push(FrameSlot::new(
                    FramePlace::Value(parameter.value),
                    parameter.ty,
                ));
            }
        }

        // retain live locals in declaration order
        for local in locals {
            let ty = self.optimized.tree.get(local).ty;
            slots.push(FrameSlot::new(FramePlace::Local(local), ty));
        }

        // retain remaining live SSA values in creation order
        for value in values {
            if function
                .parameters
                .iter()
                .any(|parameter| parameter.value == value)
            {
                continue;
            }
            let ty = function
                .value_type(value)
                .ok_or_else(|| EmitError::Internal {
                    anchor: self.module.into(),
                    module: self.module,
                    message: "live MIR value has no type".to_string(),
                })?;
            slots.push(FrameSlot::new(FramePlace::Value(value), ty));
        }

        Ok(slots)
    }
}

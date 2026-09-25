use tspp_artifact::MirOptimized;
use tspp_mir as mir;
use tspp_program::object::{FramePlace, FramePoint, FrameSlot, FrameState, Point};
use tspp_source::ModuleId;

use crate::EmitError;

use super::ObjectEmitter;
use super::point::PointMap;
use super::site::SiteEmitter;

/// Logical frame-state emitter for optimized MIR.
pub(super) struct FrameEmitter<'a> {
    /// Optimized MIR containing the emitted functions.
    optimized: &'a MirOptimized,
    /// Object-local operation identities.
    points: &'a PointMap,
    /// Engine-neutral runtime sites.
    sites: &'a SiteEmitter,
    /// Module receiving frame diagnostics.
    module: ModuleId,
}

impl<'a> FrameEmitter<'a> {
    /// Create one logical frame emitter.
    pub(super) const fn new(
        module: ModuleId,
        optimized: &'a MirOptimized,
        points: &'a PointMap,
        sites: &'a SiteEmitter,
    ) -> Self {
        Self {
            optimized,
            points,
            sites,
            module,
        }
    }

    /// Emit every runtime-visible frame state in logical operation order.
    pub(super) fn emit(
        &self,
        analyses: &mut mir::ModuleCache,
    ) -> Result<Vec<FrameState>, EmitError> {
        // collect frame states from the emitted functions
        let mut states = Vec::new();
        let points = self.frame_points()?;

        // emit functions in stable MIR identity order
        for (function_id, function) in self.optimized.tree.iter_nodes::<mir::Function>() {
            let Some(body) = &function.body else {
                continue;
            };

            // reuse liveness for the immutable optimized function
            let liveness = analyses.liveness(function_id, &self.optimized.tree);

            // materialize exact liveness only at selected frame points
            let mut blocks = body.blocks().to_vec();
            self.points.order_blocks(&mut blocks);
            for block_id in blocks {
                let block = self.optimized.tree.get(block_id);

                for (index, instruction_id) in block.instructions.iter().enumerate() {
                    let point = FramePoint::operation(self.points.instruction(*instruction_id));
                    if points.binary_search(&point).is_err() {
                        continue;
                    }
                    let values = liveness
                        .value_live_before_instruction(&self.optimized.tree, block_id, index)
                        .into_iter();
                    let locals = liveness
                        .local_live_before_instruction(&self.optimized.tree, block_id, index)
                        .into_iter();
                    let slots = self.slots(function, values, locals)?;
                    states.push(FrameState::new(point, slots));
                }

                let point = FramePoint::operation(self.points.terminator(block_id));
                if points.binary_search(&point).is_err() {
                    continue;
                }
                let values = liveness
                    .value_live_before_terminator(&self.optimized.tree, block_id)
                    .into_iter();
                let locals = liveness.local_live_before_terminator(block_id).into_iter();
                let slots = self.slots(function, values, locals)?;
                states.push(FrameState::new(point, slots));
            }
        }

        // order states for direct point lookup and engine projection
        states.sort_unstable_by_key(|state| state.point);

        Ok(states)
    }

    /// Collect canonical frame points in logical order.
    fn frame_points(&self) -> Result<Vec<FramePoint>, EmitError> {
        // collect the operations that require frame states
        let mut points = Vec::new();

        // retain runtime sites that may inspect or move the active frame
        points.extend(
            self.sites
                .allocations
                .iter()
                .map(|site| FramePoint::operation(site.point)),
        );
        points.extend(
            self.sites
                .calls
                .iter()
                .map(|site| FramePoint::operation(site.point)),
        );

        // retain explicit engine transitions
        for (_, function) in self.optimized.tree.iter_nodes::<mir::Function>() {
            let Some(body) = &function.body else {
                continue;
            };
            let mut blocks = body.blocks().to_vec();
            self.points.order_blocks(&mut blocks);
            for block_id in blocks {
                let block = self.optimized.tree.get(block_id);

                for &instruction_id in &block.instructions {
                    let instruction = self.optimized.tree.get(instruction_id);
                    let point = self.points.instruction(instruction_id);
                    if let Some(point) = self.instruction_point(function, instruction, point)? {
                        points.push(point);
                    }
                }
            }
        }

        // collapse operations selected by more than one runtime concern
        points.sort_unstable();
        points.dedup();

        Ok(points)
    }

    /// Return the frame point required by one MIR instruction.
    fn instruction_point(
        &self,
        function: &mir::Function,
        instruction: &mir::Instruction,
        point: Point,
    ) -> Result<Option<FramePoint>, EmitError> {
        // select the frame position for the instruction
        let point = match instruction {
            // retain callers before generated destruction
            mir::Instruction::Drop { .. } => Some(point),

            // retain callers while the runtime destroys a released allocation's values
            mir::Instruction::Release { value } => {
                let ty = function
                    .value_type(*value)
                    .ok_or_else(|| self.internal("released MIR value has no type"))?;
                let is_destroying =
                    ObjectEmitter::release_destroys(self.module, self.optimized, ty)?;

                is_destroying.then_some(point)
            }

            // record runtime entry after the operation
            mir::Instruction::Poll | mir::Instruction::Breakpoint => Some(point.next()),

            // defer other runtime sites to SiteEmitter
            _ => None,
        };

        Ok(point.map(FramePoint::operation))
    }

    /// Build one operation's logical slots in canonical acquisition order.
    fn slots(
        &self,
        function: &mir::Function,
        values: impl IntoIterator<Item = mir::Value>,
        locals: impl IntoIterator<Item = mir::LocalId>,
    ) -> Result<Vec<FrameSlot>, EmitError> {
        // collect and sort the live values
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
                .ok_or_else(|| self.internal("live MIR value has no type"))?;
            slots.push(FrameSlot::new(FramePlace::Value(value), ty));
        }

        Ok(slots)
    }

    /// Build one internal frame diagnostic.
    fn internal(&self, message: &str) -> EmitError {
        EmitError::Internal {
            anchor: self.module.into(),
            module: self.module,
            message: message.to_owned(),
        }
    }
}

use destack_artifact::MirOptimized;
use destack_mir as mir;
use destack_program::object::{FramePlace, FramePoint, FrameSlot, FrameState, Point};
use destack_source::ModuleId;

use crate::EmitError;

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

    /// Emit every runtime-visible frame state in logical coordinate order.
    pub(super) fn emit(&self) -> Result<Vec<FrameState>, EmitError> {
        let mut states = Vec::new();
        let points = self.frame_points();

        // emit functions in stable MIR identity order
        for (_, function) in self.optimized.tree.iter_nodes::<mir::Function>() {
            let Some(body) = &function.body else {
                continue;
            };
            let liveness = mir::FunctionLiveness::build(function, &self.optimized.tree);

            // materialize exact liveness only at selected frame coordinates
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

    /// Collect canonical frame coordinates in logical order.
    fn frame_points(&self) -> Vec<FramePoint> {
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
                    if let Some(point) = Self::instruction_point(instruction, point) {
                        points.push(point);
                    }
                }
            }
        }

        // collapse operations selected by more than one runtime concern
        points.sort_unstable();
        points.dedup();

        points
    }

    /// Return the frame coordinate required by one MIR instruction.
    fn instruction_point(instruction: &mir::Instruction, point: Point) -> Option<FramePoint> {
        let point = match instruction {
            // retain callers before generated destruction
            mir::Instruction::Drop { .. } => point,

            // runtime entry after the operation
            mir::Instruction::Poll | mir::Instruction::Breakpoint => point.next(),

            // remaining runtime sites were selected from SiteEmitter
            _ => return None,
        };

        Some(FramePoint::operation(point))
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

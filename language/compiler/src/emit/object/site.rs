use std::collections::HashMap;

use destack_artifact::{
    AllocationSite, CallMode, CallSite, ContinuationSite, CounterSite, EdgeSite, MemorySite,
    MirOptimized, Point, SampleSite, Suspension, SuspensionSite,
};
use destack_mir as mir;
use destack_source::ModuleId;

use crate::EmitError;

use super::ObjectEmitter;
use super::point::PointIndex;

/// Object-local sites emitted from optimized MIR.
#[derive(Debug, Default)]
pub(super) struct Sites {
    /// Allocation sites.
    pub(super) allocations: Vec<AllocationSite>,
    /// Allocation indices keyed by object-local program point.
    pub(super) allocation_indices: HashMap<Point, u32>,
    /// Addressable memory sites.
    pub(super) memory: Vec<MemorySite>,
    /// Function call sites.
    pub(super) calls: Vec<CallSite>,
    /// Continuation control sites.
    pub(super) continuations: Vec<ContinuationSite>,
    /// Control flow edges.
    pub(super) edges: Vec<EdgeSite>,
    /// Coroutine suspension sites.
    pub(super) suspensions: Vec<SuspensionSite>,
    /// Explicit profile counter sites.
    pub(super) counters: Vec<CounterSite>,
    /// Explicit profile sample sites.
    pub(super) samples: Vec<SampleSite>,
}

#[allow(clippy::too_many_arguments)]
impl Sites {
    /// Emit every object-local site in stable operation order.
    pub(super) fn emit(
        module: ModuleId,
        optimized: &MirOptimized,
        points: &PointIndex,
    ) -> Result<Self, EmitError> {
        let mut sites = Self::default();

        // walk each defined function in object order
        for (_, function) in optimized.tree.iter_nodes::<mir::Function>() {
            let Some(body) = &function.body else {
                continue;
            };

            // emit instruction and terminator sites in logical operation order
            for block_id in body.blocks() {
                let block = optimized.tree.get(*block_id);
                for instruction_id in &block.instructions {
                    sites.emit_instruction(module, optimized, points, function, *instruction_id)?;
                }

                sites.emit_terminator(module, optimized, points, function, *block_id)?;
            }
        }

        Ok(sites)
    }

    /// Emit sites attached to one instruction.
    fn emit_instruction(
        &mut self,
        module: ModuleId,
        optimized: &MirOptimized,
        points: &PointIndex,
        function: &mir::Function,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
    ) -> Result<(), EmitError> {
        let point = points.instruction(instruction_id);
        let instruction = optimized.tree.get(instruction_id);

        // record allocation metadata
        match instruction {
            mir::Instruction::NewZeroed {
                storage_type,
                result_type,
                ..
            }
            | mir::Instruction::NewUninit {
                storage_type,
                result_type,
                ..
            } => self.push_allocation(Self::allocation(
                module,
                optimized,
                point,
                *storage_type,
                *result_type,
            )?),
            mir::Instruction::NewSliceZeroed {
                element,
                result_type,
                ..
            }
            | mir::Instruction::NewSliceUninit {
                element,
                result_type,
                ..
            } => self.push_allocation(Self::allocation(
                module,
                optimized,
                point,
                *element,
                *result_type,
            )?),
            _ => {}
        }

        // record every explicit memory access at this operation
        if let Some(accesses) = optimized.memory.memory_accesses(instruction_id) {
            for access in accesses {
                self.memory
                    .push(Self::memory(module, optimized, function, point, access)?);
            }
        }

        // record calls and explicit profiling operations
        match instruction {
            mir::Instruction::Call { call, .. } => {
                let resume = Point::new(point.function, point.operation + 1);
                self.calls.push(Self::call(
                    module,
                    optimized,
                    function,
                    point,
                    Some(resume),
                    None,
                    CallMode::Return,
                    call,
                )?);
            }
            mir::Instruction::ProfileIncrement { counter } => {
                self.counters.push(CounterSite {
                    point,
                    counter: *counter,
                });
            }
            mir::Instruction::ProfileSample { sampler, value } => {
                let value_type = function
                    .value_type(*value)
                    .ok_or_else(|| ObjectEmitter::invalid(module, "missing sample value type"))?;
                self.samples.push(SampleSite {
                    point,
                    sampler: *sampler,
                    value_type,
                });
            }
            _ => {}
        }

        Ok(())
    }

    /// Emit sites attached to one terminator.
    fn emit_terminator(
        &mut self,
        module: ModuleId,
        optimized: &MirOptimized,
        points: &PointIndex,
        function: &mir::Function,
        block_id: mir::BlockId,
    ) -> Result<(), EmitError> {
        let block = optimized.tree.get(block_id);
        let terminator = optimized.tree.get(block.terminator);
        let point = points.terminator(block_id);

        // record fallible allocation metadata
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
            } => {
                let result_type = Self::success_type(module, optimized, success)?;
                self.push_allocation(Self::allocation(
                    module,
                    optimized,
                    point,
                    *storage_type,
                    result_type,
                )?);
            }
            mir::Terminator::NewSliceZeroedTry {
                element, success, ..
            }
            | mir::Terminator::NewSliceUninitTry {
                element, success, ..
            } => {
                let result_type = Self::success_type(module, optimized, success)?;
                self.push_allocation(Self::allocation(
                    module,
                    optimized,
                    point,
                    *element,
                    result_type,
                )?);
            }
            _ => {}
        }

        // record calls
        match terminator {
            mir::Terminator::Invoke {
                call,
                target,
                unwind,
            } => {
                self.calls.push(Self::call(
                    module,
                    optimized,
                    function,
                    point,
                    Some(points.block(target.block)),
                    Some(points.block(unwind.block)),
                    CallMode::Return,
                    call,
                )?);
            }
            mir::Terminator::TailCall { call } => {
                self.calls.push(Self::call(
                    module,
                    optimized,
                    function,
                    point,
                    None,
                    None,
                    CallMode::Tail,
                    call,
                )?);
            }
            _ => {}
        }

        // record continuation control metadata
        match terminator {
            mir::Terminator::ContinuationResume {
                yielded,
                returned,
                unwind,
                ..
            }
            | mir::Terminator::ContinuationComplete {
                yielded,
                returned,
                unwind,
                ..
            } => self.continuations.push(ContinuationSite {
                point,
                yielded: points.block(yielded.block),
                returned: points.block(returned.block),
                unwind: unwind.as_ref().map(|target| points.block(target.block)),
            }),
            _ => {}
        }

        // record coroutine suspension metadata
        match terminator {
            mir::Terminator::Await {
                value,
                resume,
                cancel,
                unwind,
                ..
            } => self.suspensions.push(Self::suspension(
                module,
                optimized,
                points,
                function,
                point,
                Suspension::Await,
                *value,
                resume,
                Some(cancel),
                None,
                unwind.as_ref(),
            )?),
            mir::Terminator::Yield {
                value,
                resume,
                complete,
                unwind,
            } => self.suspensions.push(Self::suspension(
                module,
                optimized,
                points,
                function,
                point,
                Suspension::Yield,
                *value,
                resume,
                None,
                Some(complete),
                unwind.as_ref(),
            )?),
            _ => {}
        }

        // record every explicit control flow edge
        self.edges.extend(
            terminator
                .targets(&optimized.tree, block_id)
                .into_iter()
                .map(|(_, target)| EdgeSite {
                    source: point,
                    target: points.block(target.block),
                }),
        );

        Ok(())
    }

    /// Build one coroutine suspension site.
    fn suspension(
        module: ModuleId,
        optimized: &MirOptimized,
        points: &PointIndex,
        function: &mir::Function,
        point: Point,
        operation: Suspension,
        value: mir::Value,
        resume: &mir::BlockTarget,
        cancel: Option<&mir::BlockTarget>,
        complete: Option<&mir::BlockTarget>,
        unwind: Option<&mir::BlockTarget>,
    ) -> Result<SuspensionSite, EmitError> {
        let value_type = function
            .value_type(value)
            .ok_or_else(|| ObjectEmitter::invalid(module, "missing suspension value type"))?;
        let resume_type = Self::success_type(module, optimized, resume)?;
        let complete_type = complete
            .map(|target| Self::success_type(module, optimized, target))
            .transpose()?;

        Ok(SuspensionSite {
            point,
            resume: points.block(resume.block),
            cancel: cancel.map(|target| points.block(target.block)),
            complete: complete.map(|target| points.block(target.block)),
            unwind: unwind.map(|target| points.block(target.block)),
            operation,
            value_type,
            resume_type,
            complete_type,
        })
    }

    /// Append one allocation site under its dense object-local identity.
    fn push_allocation(&mut self, site: AllocationSite) {
        let index = self.allocations.len() as u32;
        let previous = self.allocation_indices.insert(site.point, index);
        assert!(previous.is_none(), "duplicate allocation point");
        self.allocations.push(site);
    }

    /// Build one allocation site.
    fn allocation(
        module: ModuleId,
        optimized: &MirOptimized,
        point: Point,
        storage_type: mir::TypeId,
        result_type: mir::TypeId,
    ) -> Result<AllocationSite, EmitError> {
        let space = Self::reference_storage(optimized, result_type)
            .and_then(mir::Storage::heap_space)
            .ok_or_else(|| ObjectEmitter::invalid(module, "missing allocation space"))?;

        Ok(AllocationSite {
            point,
            space,
            result_type,
            storage_type,
        })
    }

    /// Build one memory site.
    fn memory(
        module: ModuleId,
        optimized: &MirOptimized,
        function: &mir::Function,
        point: Point,
        access: &mir::MemoryAccess,
    ) -> Result<MemorySite, EmitError> {
        let (storage, value_type) = match access.target {
            mir::MemoryTarget::Reference(value) => {
                let ty = function.value_type(value).ok_or_else(|| {
                    ObjectEmitter::invalid(module, "missing memory reference type")
                })?;
                let storage = Self::reference_storage(optimized, ty).ok_or_else(|| {
                    ObjectEmitter::invalid(module, "missing memory reference storage")
                })?;
                let value_type = Self::reference_value_type(optimized, ty).ok_or_else(|| {
                    ObjectEmitter::invalid(module, "missing memory reference value type")
                })?;

                (storage, value_type)
            }
            mir::MemoryTarget::Local(local) => {
                let local = optimized.tree.get(local);

                (mir::Storage::Frame, Self::storage_type(optimized, local.ty))
            }
            mir::MemoryTarget::Global(global) => {
                let global = optimized.tree.get(global);

                (
                    mir::Storage::Global(global.storage),
                    Self::storage_type(optimized, global.ty),
                )
            }
        };

        Ok(MemorySite {
            point,
            access: access.operation,
            storage,
            value_type,
        })
    }

    /// Build one call site.
    fn call(
        module: ModuleId,
        optimized: &MirOptimized,
        function: &mir::Function,
        point: Point,
        resume: Option<Point>,
        unwind: Option<Point>,
        mode: CallMode,
        call: &mir::Call,
    ) -> Result<CallSite, EmitError> {
        let (space, dispatch_type) = match call.callee {
            mir::Callee::Virtual {
                receiver, class, ..
            } => {
                let receiver_type = function.value_type(receiver).ok_or_else(|| {
                    ObjectEmitter::invalid(module, "missing virtual receiver type")
                })?;
                let space = Self::reference_storage(optimized, receiver_type)
                    .and_then(mir::Storage::heap_space)
                    .ok_or_else(|| {
                        ObjectEmitter::invalid(module, "missing virtual receiver space")
                    })?;

                (Some(space), Some(class))
            }
            mir::Callee::Dynamic {
                receiver,
                constraint,
                ..
            } => {
                let receiver_type = function.value_type(receiver).ok_or_else(|| {
                    ObjectEmitter::invalid(module, "missing dynamic receiver type")
                })?;
                let space = Self::dynamic_space(optimized, receiver_type).ok_or_else(|| {
                    ObjectEmitter::invalid(module, "missing dynamic receiver space")
                })?;

                (Some(space), Some(constraint))
            }
            mir::Callee::Direct { .. } | mir::Callee::Indirect { .. } => (None, None),
        };

        Ok(CallSite {
            point,
            resume,
            unwind,
            mode,
            dispatch: call.callee.dispatch(),
            space,
            target: call.callee.function(),
            dispatch_type,
            signature: call.signature,
        })
    }

    /// Return the implicit result type of one successful terminator edge.
    fn success_type(
        module: ModuleId,
        optimized: &MirOptimized,
        target: &mir::BlockTarget,
    ) -> Result<mir::TypeId, EmitError> {
        optimized
            .tree
            .get(target.block)
            .parameters
            .first()
            .map(|parameter| parameter.ty)
            .ok_or_else(|| ObjectEmitter::invalid(module, "missing successor result type"))
    }

    /// Return the transparent storage type for one MIR type.
    fn storage_type(optimized: &MirOptimized, mut ty: mir::TypeId) -> mir::TypeId {
        loop {
            match optimized.tree.get(ty) {
                mir::Type::Atomic { value }
                | mir::Type::WithLifetimes { base: value, .. }
                | mir::Type::Uninit { value }
                | mir::Type::ManuallyDrop { value }
                | mir::Type::Newtype { inner: value, .. } => ty = *value,
                _ => return ty,
            }
        }
    }

    /// Return the storage carried by one reference-like MIR type.
    fn reference_storage(optimized: &MirOptimized, ty: mir::TypeId) -> Option<mir::Storage> {
        match optimized.tree.get(Self::storage_type(optimized, ty)) {
            mir::Type::Reference { storage, .. }
            | mir::Type::Slice { storage, .. }
            | mir::Type::TensorView { storage, .. } => Some(*storage),
            mir::Type::Tensor { space, .. } => Some(mir::Storage::Heap(*space)),
            _ => None,
        }
    }

    /// Return the stored value type addressed by one reference-like MIR type.
    fn reference_value_type(optimized: &MirOptimized, ty: mir::TypeId) -> Option<mir::TypeId> {
        match optimized.tree.get(Self::storage_type(optimized, ty)) {
            mir::Type::Reference { pointee, .. } => Some(Self::storage_type(optimized, *pointee)),
            mir::Type::Slice { element, .. } | mir::Type::TensorView { element, .. } => {
                Some(Self::storage_type(optimized, *element))
            }
            _ => None,
        }
    }

    /// Return the payload space carried by one dynamic MIR type.
    fn dynamic_space(optimized: &MirOptimized, ty: mir::TypeId) -> Option<mir::Space> {
        match optimized.tree.get(Self::storage_type(optimized, ty)) {
            mir::Type::Dynamic { space, .. } => Some(*space),
            _ => None,
        }
    }
}

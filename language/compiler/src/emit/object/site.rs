use tspp_artifact::MirOptimized;
use tspp_mir as mir;
use tspp_program::object::{
    AllocationSite, CallMode, CallSite, CounterSite, EdgeSite, MemorySite, Point, SampleSite,
};
use tspp_source::ModuleId;

use crate::EmitError;

use super::ObjectEmitter;
use super::point::PointMap;

/// Object-local site emitter for optimized MIR.
#[derive(Debug, Default)]
pub(super) struct SiteEmitter {
    /// Allocation sites.
    pub(super) allocations: Vec<AllocationSite>,
    /// Addressable memory sites.
    pub(super) memory: Vec<MemorySite>,
    /// Function call sites.
    pub(super) calls: Vec<CallSite>,
    /// Control flow edges.
    pub(super) edges: Vec<EdgeSite>,
    /// Explicit profile counter sites.
    pub(super) counters: Vec<CounterSite>,
    /// Explicit profile sample sites.
    pub(super) samples: Vec<SampleSite>,
}

impl SiteEmitter {
    /// Emit every object-local site in stable operation order.
    pub(super) fn emit(
        module: ModuleId,
        optimized: &MirOptimized,
        points: &PointMap,
    ) -> Result<Self, EmitError> {
        // initialize the object site tables
        let mut sites = Self::default();

        // walk each defined function in object order
        for (function_id, function) in optimized.tree.iter_nodes::<mir::Function>() {
            let Some(body) = &function.body else {
                continue;
            };

            // emit instruction and terminator sites in logical operation order
            let mut blocks = body.blocks().to_vec();
            points.order_blocks(&mut blocks);
            for block_id in blocks {
                let block = optimized.tree.get(block_id);
                for instruction_id in &block.instructions {
                    sites.emit_instruction(
                        module,
                        optimized,
                        points,
                        function_id,
                        function,
                        *instruction_id,
                    )?;
                }

                sites.emit_terminator(module, optimized, points, function, block_id)?;
            }
        }

        Ok(sites)
    }

    /// Emit sites attached to one instruction.
    fn emit_instruction(
        &mut self,
        module: ModuleId,
        optimized: &MirOptimized,
        points: &PointMap,
        function_id: mir::FunctionId,
        function: &mir::Function,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
    ) -> Result<(), EmitError> {
        // read the instruction and its object point
        let point = points.instruction(instruction_id);
        let instruction = optimized.tree.get(instruction_id);

        // record allocation sites
        match instruction {
            mir::Instruction::NewZeroed {
                storage_type,
                result_type,
                space,
                ..
            }
            | mir::Instruction::NewUninit {
                storage_type,
                result_type,
                space,
                ..
            } => self.allocations.push(AllocationSite {
                point,
                space: *space,
                result_type: *result_type,
                storage_type: *storage_type,
            }),
            mir::Instruction::ContextBind {
                node_type,
                result_type,
                ..
            } => self.allocations.push(Self::allocation(
                module,
                optimized,
                point,
                *node_type,
                *result_type,
            )?),
            mir::Instruction::NewSliceZeroed {
                element,
                result_type,
                space,
                ..
            }
            | mir::Instruction::NewSliceUninit {
                element,
                result_type,
                space,
                ..
            } => self.allocations.push(AllocationSite {
                point,
                space: *space,
                result_type: *result_type,
                storage_type: *element,
            }),
            _ => {}
        }

        // record the memory access of the place this operation selects
        if let Some((place, operation)) = instruction.place_access(function_id, &optimized.tree) {
            self.memory.push(Self::memory(
                module,
                optimized,
                function_id,
                point,
                place,
                operation,
            )?);
        }

        // record dynamic entry reads
        if let mir::Instruction::DynamicRead {
            dynamic,
            result_type,
            ..
        } = instruction
        {
            self.memory.push(Self::dynamic_memory(
                module,
                optimized,
                function,
                point,
                *dynamic,
                *result_type,
            )?);
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
                // read the name the profile table records for this counter
                let name = match optimized
                    .profile
                    .function(point.function)
                    .and_then(|table| table.counter_site(*counter))
                {
                    Some(mir::CounterSite::Named(name)) => Some(*name),
                    _ => None,
                };

                self.counters.push(CounterSite {
                    name,
                    point,
                    counter: *counter,
                });
            }
            mir::Instruction::ProfileSample { sampler, value } => {
                let value_type = function
                    .value_type(*value)
                    .ok_or_else(|| ObjectEmitter::internal(module, "missing sample value type"))?;

                // read the name the profile table records for this sampler
                let name = match optimized
                    .profile
                    .function(point.function)
                    .and_then(|table| table.sample_site(*sampler))
                {
                    Some(mir::SampleSite::Named(name)) => Some(*name),
                    _ => None,
                };

                self.samples.push(SampleSite {
                    name,
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
        points: &PointMap,
        function: &mir::Function,
        block_id: mir::BlockId,
    ) -> Result<(), EmitError> {
        // read the block terminator and its object point
        let block = optimized.tree.get(block_id);
        let terminator = optimized.tree.get(block.terminator);
        let point = points.terminator(block_id);

        // record fallible allocation sites
        match terminator {
            mir::Terminator::NewZeroedTry {
                storage_type,
                space,
                success,
                ..
            }
            | mir::Terminator::NewUninitTry {
                storage_type,
                space,
                success,
                ..
            } => self.allocations.push(AllocationSite {
                point,
                space: *space,
                result_type: Self::success_type(module, optimized, success)?,
                storage_type: *storage_type,
            }),
            mir::Terminator::NewSliceZeroedTry {
                element,
                space,
                success,
                ..
            }
            | mir::Terminator::NewSliceUninitTry {
                element,
                space,
                success,
                ..
            } => self.allocations.push(AllocationSite {
                point,
                space: *space,
                result_type: Self::success_type(module, optimized, success)?,
                storage_type: *element,
            }),
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

    /// Build the allocation site of one context node in its managed space.
    fn allocation(
        module: ModuleId,
        optimized: &MirOptimized,
        point: Point,
        storage_type: mir::TypeId,
        result_type: mir::TypeId,
    ) -> Result<AllocationSite, EmitError> {
        // require a heap space for the allocation
        let space = Self::reference_storage(optimized, result_type)
            .and_then(mir::Storage::heap_space)
            .ok_or_else(|| ObjectEmitter::internal(module, "missing allocation space"))?;

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
        function: mir::FunctionId,
        point: Point,
        place: &mir::Place,
        access: mir::MemoryOperation,
    ) -> Result<MemorySite, EmitError> {
        // resolve the accessed storage and value type
        let Some(mir::PlaceType::Value(value_type)) = place.ty(function, &optimized.tree) else {
            return Err(ObjectEmitter::internal(
                module,
                "memory site selects a referent",
            ));
        };

        Ok(MemorySite {
            point,
            access,
            storage: place.storage(function, &optimized.tree),
            value_type: optimized.tree.storage_type(value_type),
        })
    }

    /// Build one dynamic entry memory site.
    fn dynamic_memory(
        module: ModuleId,
        optimized: &MirOptimized,
        function: &mir::Function,
        point: Point,
        dynamic: mir::Value,
        result_type: mir::TypeId,
    ) -> Result<MemorySite, EmitError> {
        // read the dynamic receiver type
        let dynamic_type = function
            .value_type(dynamic)
            .ok_or_else(|| ObjectEmitter::internal(module, "missing dynamic value type"))?;
        let storage = Self::reference_storage(optimized, dynamic_type);

        Ok(MemorySite {
            point,
            access: mir::MemoryOperation::Read,
            storage,
            value_type: result_type,
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
        // resolve the callee storage and dispatch type
        let (space, dispatch_type) = match call.callee {
            mir::Callee::Virtual {
                receiver, class, ..
            } => {
                let receiver_type = function.value_type(receiver).ok_or_else(|| {
                    ObjectEmitter::internal(module, "missing virtual receiver type")
                })?;
                let space = Self::reference_storage(optimized, receiver_type)
                    .and_then(mir::Storage::heap_space)
                    .ok_or_else(|| {
                        ObjectEmitter::internal(module, "missing virtual receiver space")
                    })?;

                (Some(space), Some(class))
            }
            mir::Callee::Dynamic {
                receiver,
                constraint,
                ..
            } => {
                let receiver_type = function.value_type(receiver).ok_or_else(|| {
                    ObjectEmitter::internal(module, "missing dynamic receiver type")
                })?;
                let space = Self::reference_storage(optimized, receiver_type)
                    .and_then(mir::Storage::heap_space)
                    .ok_or_else(|| {
                        ObjectEmitter::internal(module, "missing dynamic receiver space")
                    })?;

                (Some(space), Some(constraint))
            }
            mir::Callee::Direct { .. }
            | mir::Callee::Indirect { .. }
            | mir::Callee::Witness { .. } => (None, None),
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
            .ok_or_else(|| ObjectEmitter::internal(module, "missing successor result type"))
    }

    /// Return the storage of one reference-like MIR type.
    fn reference_storage(optimized: &MirOptimized, ty: mir::TypeId) -> Option<mir::Storage> {
        optimized
            .tree
            .type_definition(optimized.tree.storage_type(ty))
            .reference_storage()
    }
}

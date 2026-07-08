use destack_core::Optional;
use destack_mir as mir;
use destack_program::{
    AddressSpace, AllocationInitialization, AllocationOperation, AllocationSite, CallDispatch,
    CallMode, CallSite, MemoryAccess, MemorySite, ProgramPoint,
};

use crate::LinkResult;

use super::allocation;
use super::lower::BlockLowerer;

impl BlockLowerer<'_> {
    /// Build an executable allocation site for one MIR instruction.
    pub(super) fn allocation_site_for_instruction(
        &self,
        instruction: &mir::Instruction,
        pc: u32,
    ) -> LinkResult<Option<AllocationSite>> {
        let point = self.program_point(pc);

        let site = match instruction {
            mir::Instruction::NewZeroed {
                layout,
                result_type,
                ..
            } => Some(self.allocation_site(
                point,
                AllocationOperation::Object,
                *layout,
                *result_type,
                allocation::AllocationInitialization::Zeroed,
            )?),
            mir::Instruction::NewUninit {
                layout,
                result_type,
                ..
            } => Some(self.allocation_site(
                point,
                AllocationOperation::Object,
                *layout,
                *result_type,
                allocation::AllocationInitialization::Uninit,
            )?),
            mir::Instruction::NewSliceZeroed {
                element,
                result_type,
                ..
            } => Some(self.allocation_site(
                point,
                AllocationOperation::Slice,
                *element,
                *result_type,
                allocation::AllocationInitialization::Zeroed,
            )?),
            mir::Instruction::NewSliceUninit {
                element,
                result_type,
                ..
            } => Some(self.allocation_site(
                point,
                AllocationOperation::Slice,
                *element,
                *result_type,
                allocation::AllocationInitialization::Uninit,
            )?),
            _ => None,
        };

        Ok(site)
    }

    /// Build an executable allocation site for one MIR terminator.
    pub(super) fn allocation_site_for_terminator(
        &self,
        terminator: &mir::Terminator,
        pc: u32,
    ) -> LinkResult<Option<AllocationSite>> {
        let point = self.program_point(pc);

        let site = match terminator {
            mir::Terminator::NewZeroedTry {
                layout, success, ..
            } => {
                let result_type = self.allocation_success_type(success)?;
                Some(self.allocation_site(
                    point,
                    AllocationOperation::Object,
                    *layout,
                    result_type,
                    allocation::AllocationInitialization::Zeroed,
                )?)
            }
            mir::Terminator::NewUninitTry {
                layout, success, ..
            } => {
                let result_type = self.allocation_success_type(success)?;
                Some(self.allocation_site(
                    point,
                    AllocationOperation::Object,
                    *layout,
                    result_type,
                    allocation::AllocationInitialization::Uninit,
                )?)
            }
            mir::Terminator::NewSliceZeroedTry {
                element, success, ..
            } => {
                let result_type = self.allocation_success_type(success)?;
                Some(self.allocation_site(
                    point,
                    AllocationOperation::Slice,
                    *element,
                    result_type,
                    allocation::AllocationInitialization::Zeroed,
                )?)
            }
            mir::Terminator::NewSliceUninitTry {
                element, success, ..
            } => {
                let result_type = self.allocation_success_type(success)?;
                Some(self.allocation_site(
                    point,
                    AllocationOperation::Slice,
                    *element,
                    result_type,
                    allocation::AllocationInitialization::Uninit,
                )?)
            }
            _ => None,
        };

        Ok(site)
    }

    /// Append executable memory sites for one MIR instruction.
    pub(super) fn push_memory_sites_for_instruction(
        &self,
        instruction: &mir::Instruction,
        pc: u32,
        sites: &mut Vec<MemorySite>,
    ) -> LinkResult<()> {
        let point = self.program_point(pc);

        match instruction {
            mir::Instruction::Load {
                pointer,
                result_type,
                ..
            } => {
                sites.push(self.memory_site(point, MemoryAccess::Read, *pointer, *result_type)?);
            }
            mir::Instruction::Store { pointer, value } => {
                let value_type = self.value_type_for_value(*value)?;
                sites.push(self.memory_site(point, MemoryAccess::Write, *pointer, value_type)?);
            }
            mir::Instruction::AtomicLoad {
                pointer,
                result_type,
                ..
            } => {
                sites.push(self.memory_site(point, MemoryAccess::Read, *pointer, *result_type)?);
            }
            mir::Instruction::AtomicStore { pointer, value, .. } => {
                let value_type = self.value_type_for_value(*value)?;
                sites.push(self.memory_site(point, MemoryAccess::Write, *pointer, value_type)?);
            }
            mir::Instruction::AtomicCompareExchange {
                pointer, expected, ..
            } => {
                let value_type = self.value_type_for_value(*expected)?;
                sites.push(self.memory_site(
                    point,
                    MemoryAccess::ReadWrite,
                    *pointer,
                    value_type,
                )?);
            }
            mir::Instruction::AtomicRmw { pointer, value, .. } => {
                let value_type = self.value_type_for_value(*value)?;
                sites.push(self.memory_site(
                    point,
                    MemoryAccess::ReadWrite,
                    *pointer,
                    value_type,
                )?);
            }
            mir::Instruction::TensorLoad { view, .. } => {
                sites.push(self.tensor_memory_site(point, MemoryAccess::Read, *view)?);
            }
            mir::Instruction::TensorStore { view, .. }
            | mir::Instruction::TensorFill { view, .. } => {
                sites.push(self.tensor_memory_site(point, MemoryAccess::Write, *view)?);
            }
            mir::Instruction::TensorCopy { target, source } => {
                sites.push(self.tensor_memory_site(point, MemoryAccess::Write, *target)?);
                sites.push(self.tensor_memory_site(point, MemoryAccess::Read, *source)?);
            }
            _ => {}
        }

        Ok(())
    }

    /// Build an executable call site for one MIR instruction.
    pub(super) fn call_site_for_instruction(
        &self,
        instruction: &mir::Instruction,
        pc: u32,
    ) -> LinkResult<Option<CallSite>> {
        let point = self.program_point(pc);

        let site = match instruction {
            mir::Instruction::Call { function, call, .. } => {
                Some(self.direct_call_site(point, CallMode::Return, *function, call.signature))
            }
            mir::Instruction::CallVirtual {
                receiver,
                class,
                slot,
                call,
                ..
            } => Some(self.dispatch_call_site(
                point,
                CallMode::Return,
                CallDispatch::Virtual,
                *receiver,
                *class,
                *slot,
                call.signature,
            )?),
            mir::Instruction::CallDynamic {
                receiver,
                constraint,
                slot,
                call,
                ..
            } => Some(self.dispatch_call_site(
                point,
                CallMode::Return,
                CallDispatch::Dynamic,
                *receiver,
                *constraint,
                *slot,
                call.signature,
            )?),
            mir::Instruction::CallIndirect { call, .. } => {
                Some(self.indirect_call_site(point, CallMode::Return, call.signature))
            }
            _ => None,
        };

        Ok(site)
    }

    /// Build an executable call site for one MIR terminator.
    pub(super) fn call_site_for_terminator(
        &self,
        terminator: &mir::Terminator,
        pc: u32,
    ) -> LinkResult<Option<CallSite>> {
        let point = self.program_point(pc);

        let site = match terminator {
            mir::Terminator::Call { function, call, .. } => {
                Some(self.direct_call_site(point, CallMode::Return, *function, call.signature))
            }
            mir::Terminator::CallIndirect { call, .. } => {
                Some(self.indirect_call_site(point, CallMode::Return, call.signature))
            }
            mir::Terminator::CallVirtual {
                receiver,
                class,
                slot,
                call,
                ..
            } => Some(self.dispatch_call_site(
                point,
                CallMode::Return,
                CallDispatch::Virtual,
                *receiver,
                *class,
                *slot,
                call.signature,
            )?),
            mir::Terminator::CallDynamic {
                receiver,
                constraint,
                slot,
                call,
                ..
            } => Some(self.dispatch_call_site(
                point,
                CallMode::Return,
                CallDispatch::Dynamic,
                *receiver,
                *constraint,
                *slot,
                call.signature,
            )?),
            mir::Terminator::TailCall { function, call, .. } => {
                Some(self.direct_call_site(point, CallMode::Tail, *function, call.signature))
            }
            mir::Terminator::TailCallIndirect { call, .. } => {
                Some(self.indirect_call_site(point, CallMode::Tail, call.signature))
            }
            mir::Terminator::TailCallVirtual {
                receiver,
                class,
                slot,
                call,
                ..
            } => Some(self.dispatch_call_site(
                point,
                CallMode::Tail,
                CallDispatch::Virtual,
                *receiver,
                *class,
                *slot,
                call.signature,
            )?),
            mir::Terminator::TailCallDynamic {
                receiver,
                constraint,
                slot,
                call,
                ..
            } => Some(self.dispatch_call_site(
                point,
                CallMode::Tail,
                CallDispatch::Dynamic,
                *receiver,
                *constraint,
                *slot,
                call.signature,
            )?),
            _ => None,
        };

        Ok(site)
    }

    /// Return the executable point for one lowered instruction offset.
    fn program_point(&self, pc: u32) -> ProgramPoint {
        let function = self.function.program_function(self.function.function_id);
        let operation = self.block_start + pc;

        ProgramPoint::new(function, operation)
    }

    /// Build one heap allocation site row.
    fn allocation_site(
        &self,
        point: ProgramPoint,
        allocation: AllocationOperation,
        storage_type: mir::TypeId,
        result_type: mir::TypeId,
        initialization: allocation::AllocationInitialization,
    ) -> LinkResult<AllocationSite> {
        let storage_layout = self.layout_for_type(storage_type)?.layout_id;
        let address_space = match allocation {
            AllocationOperation::Object => self.function.address_space_for_type(result_type)?,
            AllocationOperation::Slice => self.slice_backing_address_space(result_type)?,
        };

        Ok(AllocationSite {
            point,
            operation: allocation,
            initialization: self.allocation_initialization(initialization),
            address_space,
            result_type: self.function.program.type_id(result_type),
            storage_type: self.function.program.type_id(storage_type),
            storage_layout,
        })
    }

    /// Return the result type for one fallible allocation success edge.
    fn allocation_success_type(&self, target: &mir::BlockTarget) -> LinkResult<mir::TypeId> {
        let target_index = self
            .function
            .block_index_by_id
            .get(&target.block)
            .copied()
            .ok_or_else(|| self.invalid_instruction("allocation success block"))?;
        let target_parameters = self.function.block_parameters[target_index].as_slice();
        let Some(&result) = target_parameters.first() else {
            return Err(self.invalid_input("allocation success parameter"));
        };

        self.value_type_for_value(result)
    }

    /// Convert compiler allocation initialization into the program row shape.
    fn allocation_initialization(
        &self,
        initialization: allocation::AllocationInitialization,
    ) -> AllocationInitialization {
        match initialization {
            allocation::AllocationInitialization::Zeroed => AllocationInitialization::Zeroed,
            allocation::AllocationInitialization::Uninit => AllocationInitialization::Uninit,
        }
    }

    /// Build one direct memory access site row.
    fn memory_site(
        &self,
        point: ProgramPoint,
        access: MemoryAccess,
        pointer: mir::Value,
        value_type: mir::TypeId,
    ) -> LinkResult<MemorySite> {
        let address_space = self
            .operand_map()
            .address_space(pointer)
            .ok_or_else(|| self.invalid_pointer_type(format!("{pointer:?}")))?;

        Ok(self.memory_site_entry(point, access, address_space, value_type))
    }

    /// Build one tensor memory access site row.
    fn tensor_memory_site(
        &self,
        point: ProgramPoint,
        access: MemoryAccess,
        view: mir::Value,
    ) -> LinkResult<MemorySite> {
        let view_type = self.value_type_for_value(view)?;
        let address_space = self
            .tensor_view_address_space(view_type)
            .ok_or_else(|| self.invalid_instruction("tensor view address space"))?;
        let value_type = self
            .tensor_element_type(view_type)
            .ok_or_else(|| self.invalid_instruction("tensor view element"))?;

        Ok(self.memory_site_entry(point, access, address_space, value_type))
    }

    /// Build one memory site row from resolved components.
    fn memory_site_entry(
        &self,
        point: ProgramPoint,
        access: MemoryAccess,
        address_space: AddressSpace,
        value_type: mir::TypeId,
    ) -> MemorySite {
        MemorySite {
            point,
            access,
            address_space,
            value_type: self.function.program.type_id(value_type),
        }
    }

    /// Build one direct call site row.
    fn direct_call_site(
        &self,
        point: ProgramPoint,
        mode: CallMode,
        function: mir::FunctionId,
        signature_type: mir::TypeId,
    ) -> CallSite {
        CallSite {
            point,
            mode,
            dispatch: CallDispatch::Direct,
            address_space: Optional::none(),
            target: Optional::some(self.function.program_function(function)),
            dispatch_type: Optional::none(),
            signature_type: self.function.program.type_id(signature_type),
            slot: Optional::none(),
        }
    }

    /// Build one virtual or dynamic call site row.
    fn dispatch_call_site(
        &self,
        point: ProgramPoint,
        mode: CallMode,
        dispatch: CallDispatch,
        receiver: mir::Value,
        dispatch_type: mir::TypeId,
        slot: mir::DispatchSlot,
        signature_type: mir::TypeId,
    ) -> LinkResult<CallSite> {
        let address_space = self
            .operand_map()
            .address_space(receiver)
            .ok_or_else(|| self.invalid_pointer_type(format!("{receiver:?}")))?;

        Ok(CallSite {
            point,
            mode,
            dispatch,
            address_space: Optional::some(address_space),
            target: Optional::none(),
            dispatch_type: Optional::some(self.function.program.type_id(dispatch_type)),
            signature_type: self.function.program.type_id(signature_type),
            slot: Optional::some(slot.0),
        })
    }

    /// Build one indirect call site row.
    fn indirect_call_site(
        &self,
        point: ProgramPoint,
        mode: CallMode,
        signature_type: mir::TypeId,
    ) -> CallSite {
        CallSite {
            point,
            mode,
            dispatch: CallDispatch::Indirect,
            address_space: Optional::none(),
            target: Optional::none(),
            dispatch_type: Optional::none(),
            signature_type: self.function.program.type_id(signature_type),
            slot: Optional::none(),
        }
    }
}

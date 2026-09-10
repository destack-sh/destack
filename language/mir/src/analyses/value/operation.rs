use crate as mir;

// TODO #Incomplete: include traps and memory accesses in instruction effect queries

/// Return whether an instruction must remain when its result is unused.
pub fn instruction_has_side_effects(instruction: &mir::Instruction) -> bool {
    // classify instructions by side effects
    match instruction {
        mir::Instruction::Error => {
            panic!("recovered MIR instruction reached optimizer");
        }

        // classify scalar and aggregate instructions as pure
        mir::Instruction::Const { .. }
        | mir::Instruction::Binary { .. }
        | mir::Instruction::Unary { .. }
        | mir::Instruction::Cast { .. }
        | mir::Instruction::Select { .. }
        | mir::Instruction::Aggregate { .. }
        | mir::Instruction::FieldGet { .. }
        | mir::Instruction::FieldSet { .. }
        | mir::Instruction::ElementGet { .. }
        | mir::Instruction::ElementSet { .. }
        | mir::Instruction::VariantNew { .. }
        | mir::Instruction::VariantTag { .. }
        | mir::Instruction::VariantTagLoad { .. }
        | mir::Instruction::VariantPayload { .. }
        | mir::Instruction::FieldAddr { .. }
        | mir::Instruction::ElementAddr { .. }
        | mir::Instruction::VariantPayloadAddr { .. }
        | mir::Instruction::SliceView { .. }
        | mir::Instruction::SliceLength { .. }
        | mir::Instruction::DynamicBind { .. }
        | mir::Instruction::DynamicPayload { .. }
        | mir::Instruction::DynamicType { .. }
        | mir::Instruction::DynamicRead { .. }
        | mir::Instruction::DynamicFind { .. }
        | mir::Instruction::VectorSplat { .. }
        | mir::Instruction::VectorExtract { .. }
        | mir::Instruction::VectorInsert { .. }
        | mir::Instruction::VectorShuffle { .. }
        | mir::Instruction::VectorSelect { .. }
        | mir::Instruction::VectorReduce { .. }
        | mir::Instruction::VectorCompare { .. }
        | mir::Instruction::VectorConvert { .. }
        | mir::Instruction::GlobalAddr { .. }
        | mir::Instruction::FunctionAddr { .. }
        | mir::Instruction::FunctionBind { .. }
        | mir::Instruction::FunctionEnvironment { .. }
        | mir::Instruction::FunctionEnvironmentCurrent { .. }
        | mir::Instruction::ContextCurrent { .. }
        | mir::Instruction::ContextGet { .. }
        | mir::Instruction::LocalAddr { .. }
        | mir::Instruction::NewComplete { .. }
        | mir::Instruction::Assume { .. } => false,

        // classify nonvolatile reads as pure
        mir::Instruction::LocalGet { .. } | mir::Instruction::Load { .. } => false,

        // preserve memory writes
        mir::Instruction::LocalSet { .. }
        | mir::Instruction::Store { .. }
        | mir::Instruction::AtomicLoad { .. }
        | mir::Instruction::AtomicStore { .. }
        | mir::Instruction::AtomicCompareExchange { .. }
        | mir::Instruction::AtomicRmw { .. }
        | mir::Instruction::AtomicFence { .. }
        | mir::Instruction::BarrierWrite { .. } => true,

        // preserve calls
        mir::Instruction::Call { .. }
        | mir::Instruction::ContextReplace { .. }
        | mir::Instruction::ContextBind { .. }
        | mir::Instruction::Drop { .. } => true,

        // preserve allocations
        mir::Instruction::NewZeroed { .. }
        | mir::Instruction::NewUninit { .. }
        | mir::Instruction::NewSliceZeroed { .. }
        | mir::Instruction::NewSliceUninit { .. } => true,

        // preserve storage release
        mir::Instruction::Release { .. } => true,

        // preserve profile instrumentation
        mir::Instruction::ProfileIncrement { .. } | mir::Instruction::ProfileSample { .. } => true,

        // preserve runtime and debugger control
        mir::Instruction::Poll | mir::Instruction::Breakpoint => true,

        // check whether the intrinsic has side effects
        mir::Instruction::Intrinsic { intrinsic, .. } => {
            !intrinsic.is_pure() || matches!(intrinsic, mir::Intrinsic::BlackBox)
        }
    }
}

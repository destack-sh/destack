use crate::{
    Callee, CheckConstraint, Constant, FunctionId, GenericArgument, GlobalId, Instruction, LocalId,
    PlaceOrigin, Terminator, TypeId,
};

/// Map declaration and local ids when copying a body.
pub trait IdRemap {
    /// Map one local id.
    fn map_local(&mut self, local: LocalId) -> LocalId;
    /// Map one type id.
    fn map_type(&mut self, ty: TypeId) -> TypeId;
    /// Map one function id.
    fn map_function(&mut self, function: FunctionId) -> FunctionId;
    /// Map one global id.
    fn map_global(&mut self, global: GlobalId) -> GlobalId;
}

impl Constant {
    /// Map every type id one constant names.
    pub fn map_ids(&mut self, remap: &mut dyn IdRemap) {
        match self {
            Constant::Layout { ty, .. } => *ty = remap.map_type(*ty),
            Constant::Witness {
                receiver,
                interface,
                ..
            } => {
                *receiver = remap.map_type(*receiver);
                *interface = remap.map_type(*interface);
            }
            _ => {}
        }
    }
}

impl Callee {
    /// Map every id one callee names.
    pub fn map_ids(&mut self, remap: &mut dyn IdRemap) {
        match self {
            Callee::Direct {
                function,
                arguments,
            } => {
                *function = remap.map_function(*function);
                for argument in arguments {
                    if let GenericArgument::Type(ty) = argument {
                        *ty = remap.map_type(*ty);
                    }
                }
            }
            Callee::Indirect { .. } => {}
            Callee::Virtual { class, .. } => *class = remap.map_type(*class),
            Callee::Dynamic { constraint, .. } => *constraint = remap.map_type(*constraint),
            Callee::Witness {
                receiver,
                interface,
                requirement,
                arguments,
            } => {
                *receiver = remap.map_type(*receiver);
                *interface = remap.map_type(*interface);
                *requirement = remap.map_function(*requirement);
                for argument in arguments {
                    if let GenericArgument::Type(ty) = argument {
                        *ty = remap.map_type(*ty);
                    }
                }
            }
        }
    }
}

impl Instruction {
    /// Map every declaration and local id one instruction names.
    pub fn map_ids(&mut self, remap: &mut dyn IdRemap) {
        // remap storage selected by memory operands
        let place = match self {
            Self::Load { place, .. }
            | Self::VariantTagLoad { place, .. }
            | Self::Store { place, .. }
            | Self::Address { place, .. }
            | Self::AtomicLoad { place, .. }
            | Self::AtomicStore { place, .. }
            | Self::AtomicCompareExchange { place, .. }
            | Self::AtomicRmw { place, .. } => Some(place),
            _ => None,
        };
        if let Some(place) = place {
            match &mut place.origin {
                PlaceOrigin::Local(local) => *local = remap.map_local(*local),
                PlaceOrigin::Global(global) => *global = remap.map_global(*global),
                PlaceOrigin::Value(_) => {}
            }
        }

        match self {
            Instruction::Const { value, .. } => value.map_ids(remap),
            Instruction::Cast { to_type, .. } => *to_type = remap.map_type(*to_type),
            Instruction::Address { result_type, .. }
            | Instruction::Load { result_type, .. }
            | Instruction::VariantNew { result_type, .. }
            | Instruction::DynamicPayload { result_type, .. }
            | Instruction::DynamicRead { result_type, .. }
            | Instruction::DynamicFind { result_type, .. }
            | Instruction::NewComplete { result_type, .. }
            | Instruction::AtomicLoad { result_type, .. } => {
                *result_type = remap.map_type(*result_type);
            }
            Instruction::FunctionAddr {
                function,
                arguments,
                ..
            }
            | Instruction::FunctionBind {
                function,
                arguments,
                ..
            } => {
                *function = remap.map_function(*function);
                for argument in arguments {
                    if let GenericArgument::Type(ty) = argument {
                        *ty = remap.map_type(*ty);
                    }
                }
            }
            Instruction::ContextBind {
                node_type,
                result_type,
                ..
            }
            | Instruction::ContextGet {
                node_type,
                result_type,
                ..
            } => {
                *node_type = remap.map_type(*node_type);
                *result_type = remap.map_type(*result_type);
            }
            Instruction::DynamicBind { concrete, .. } => *concrete = remap.map_type(*concrete),
            Instruction::Call { call, .. } => {
                call.callee.map_ids(remap);
                call.signature = remap.map_type(call.signature);
            }
            Instruction::NewZeroed {
                storage_type,
                result_type,
                ..
            }
            | Instruction::NewUninit {
                storage_type,
                result_type,
                ..
            } => {
                *storage_type = remap.map_type(*storage_type);
                *result_type = remap.map_type(*result_type);
            }
            Instruction::NewSliceZeroed {
                element,
                result_type,
                ..
            }
            | Instruction::NewSliceUninit {
                element,
                result_type,
                ..
            } => {
                *element = remap.map_type(*element);
                *result_type = remap.map_type(*result_type);
            }
            Instruction::Error
            | Instruction::Copy { .. }
            | Instruction::Binary { .. }
            | Instruction::Unary { .. }
            | Instruction::Select { .. }
            | Instruction::FunctionEnvironment { .. }
            | Instruction::FunctionEnvironmentCurrent { .. }
            | Instruction::ContextCurrent { .. }
            | Instruction::ContextReplace { .. }
            | Instruction::Store { .. }
            | Instruction::Aggregate { .. }
            | Instruction::FieldGet { .. }
            | Instruction::FieldSet { .. }
            | Instruction::ElementGet { .. }
            | Instruction::ElementSet { .. }
            | Instruction::VariantTag { .. }
            | Instruction::VariantTagLoad { .. }
            | Instruction::VariantPayload { .. }
            | Instruction::SliceLength { .. }
            | Instruction::DynamicType { .. }
            | Instruction::VectorSplat { .. }
            | Instruction::VectorExtract { .. }
            | Instruction::VectorInsert { .. }
            | Instruction::VectorShuffle { .. }
            | Instruction::VectorSelect { .. }
            | Instruction::VectorReduce { .. }
            | Instruction::VectorCompare { .. }
            | Instruction::VectorConvert { .. }
            | Instruction::Drop { .. }
            | Instruction::Release { .. }
            | Instruction::BarrierWrite { .. }
            | Instruction::AtomicStore { .. }
            | Instruction::AtomicCompareExchange { .. }
            | Instruction::AtomicRmw { .. }
            | Instruction::AtomicFence { .. }
            | Instruction::Assume { .. }
            | Instruction::ProfileIncrement { .. }
            | Instruction::ProfileSample { .. }
            | Instruction::Poll
            | Instruction::Breakpoint
            | Instruction::Intrinsic { .. } => {}
        }
    }
}

impl Terminator {
    /// Map every type and function id one terminator names.
    pub fn map_ids(&mut self, remap: &mut dyn IdRemap) {
        match self {
            Terminator::Invoke { call, .. } | Terminator::TailCall { call } => {
                call.callee.map_ids(remap);
                call.signature = remap.map_type(call.signature);
            }
            Terminator::NewZeroedTry { storage_type, .. }
            | Terminator::NewUninitTry { storage_type, .. } => {
                *storage_type = remap.map_type(*storage_type);
            }
            Terminator::NewSliceZeroedTry { element, .. }
            | Terminator::NewSliceUninitTry { element, .. } => {
                *element = remap.map_type(*element);
            }
            Terminator::Check { constraint, .. } => match constraint {
                CheckConstraint::IsType { expected, .. }
                | CheckConstraint::IsSubtype { expected, .. } => {
                    *expected = remap.map_type(*expected);
                }
                CheckConstraint::Bounds { .. }
                | CheckConstraint::Null { .. }
                | CheckConstraint::DivZero { .. }
                | CheckConstraint::ShiftRange { .. }
                | CheckConstraint::Narrow { .. }
                | CheckConstraint::Overflow { .. } => {}
            },
            Terminator::Error
            | Terminator::Return { .. }
            | Terminator::Jump { .. }
            | Terminator::Branch { .. }
            | Terminator::Switch { .. }
            | Terminator::VariantSwitch { .. }
            | Terminator::Panic { .. }
            | Terminator::UnwindResume
            | Terminator::Abort { .. }
            | Terminator::Unreachable => {}
        }
    }
}

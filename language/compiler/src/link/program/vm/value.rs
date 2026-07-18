use destack_mir as mir;
use destack_program::vm::IntrinsicOperand;
use destack_program::{CellLayout, ReferenceFlags, TypeId};

use super::super::TypeLinker;
use super::linker::Linker;

/// Lowering-only operand used for VM op selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Operand {
    /// Void value.
    Void,
    /// Boolean value.
    Boolean,
    /// Signed or unsigned integer with width.
    Int { width: u16, signed: bool },
    /// Floating point value with format.
    Float { format: mir::FloatType },
    /// Reference value with pointee type.
    Reference {
        pointee: TypeId,
        space: mir::Space,
        cell_layout: CellLayout,
        reference: ReferenceFlags,
    },
    /// Function pointer value with result type.
    FunctionPointer { result: TypeId },
    /// Aggregate value materialized in frame storage.
    Aggregate { ty: TypeId },
    /// Inline indexed value with element type and count.
    Sequence { element: TypeId, length: u64 },
}

impl Operand {
    /// Return whether this operand is represented by frame bytes.
    pub(super) const fn is_frame_backed(self) -> bool {
        matches!(self, Self::Aggregate { .. } | Self::Sequence { .. })
    }

    /// Return the intrinsic operand shape for this lowered operand.
    pub(super) const fn intrinsic(self) -> Option<IntrinsicOperand> {
        match self {
            Self::Void => Some(IntrinsicOperand::Void),
            Self::Boolean => Some(IntrinsicOperand::Boolean),
            Self::Int { width, signed } => Some(IntrinsicOperand::Int {
                width,
                is_signed: signed,
            }),
            Self::Float { format } => Some(IntrinsicOperand::Float { format }),
            Self::Reference { cell_layout, .. } => {
                Some(IntrinsicOperand::Reference { cell_layout })
            }
            Self::FunctionPointer { .. } | Self::Aggregate { .. } | Self::Sequence { .. } => None,
        }
    }
}

/// MIR type to VM operand projection.
pub(super) struct OperandLowerer<'a> {
    /// Shared MIR to program type projection.
    type_linker: TypeLinker<'a>,
    /// The MIR tree being lowered.
    tree: &'a mir::Tree,
}

impl<'a> OperandLowerer<'a> {
    /// Create one operand lowerer.
    pub(super) fn new(
        tree: &'a mir::Tree,
        target_layout: &'a mir::TargetLayout,
        types: &'a mir::TypeTable,
        program: &'a Linker<'a>,
    ) -> Self {
        Self {
            type_linker: TypeLinker::new(tree, target_layout, types, program.program()),
            tree,
        }
    }

    /// Return the program pointer byte width.
    pub(super) fn pointer_bytes(&self) -> u8 {
        self.type_linker.pointer_bytes()
    }

    /// Return the program type id for one MIR type.
    pub(super) fn type_id(&self, ty: mir::LocalNodeId<mir::Type>) -> TypeId {
        self.type_linker.type_id(ty)
    }

    /// Return the operand for one MIR type.
    pub(super) fn operand(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<Operand> {
        let repr = self.type_linker.storage_type(ty);

        self.operand_node(self.type_id(repr), self.tree.get(repr))
    }

    /// Return reference flags for a MIR type when available.
    pub(super) fn reference_meta(&self, ty: mir::LocalNodeId<mir::Type>) -> ReferenceFlags {
        match self.operand(ty) {
            Some(Operand::Reference { reference, .. }) => reference,
            _ => ReferenceFlags::NONE,
        }
    }

    /// Return the integer operand layout for one MIR type.
    pub(super) fn integer_layout(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<(u16, bool)> {
        match self.operand(ty) {
            Some(Operand::Int { width, signed }) => Some((width, signed)),
            _ => None,
        }
    }

    /// Return the operand for one MIR type.
    fn operand_node(&self, ty: TypeId, type_shape: &mir::Type) -> Option<Operand> {
        match type_shape {
            mir::Type::Error => None,
            mir::Type::WithLifetimes { .. }
            | mir::Type::Uninit { .. }
            | mir::Type::ManuallyDrop { .. }
            | mir::Type::Atomic { .. }
            | mir::Type::Newtype { .. } => {
                unreachable!("storage_type must peel storage wrappers")
            }
            mir::Type::Void => Some(Operand::Void),
            mir::Type::Boolean => Some(Operand::Boolean),
            mir::Type::Int { width, is_signed } => Some(Operand::Int {
                width: *width,
                signed: *is_signed,
            }),
            mir::Type::Isize => Some(Operand::Int {
                width: u16::from(self.pointer_bytes()) * 8,
                signed: true,
            }),
            mir::Type::Usize | mir::Type::TypeDescriptor => Some(Operand::Int {
                width: u16::from(self.pointer_bytes()) * 8,
                signed: false,
            }),
            mir::Type::TypeId => Some(Operand::Int {
                width: u32::BITS as u16,
                signed: false,
            }),
            mir::Type::Float(float_type) => Some(Operand::Float {
                format: *float_type,
            }),
            mir::Type::Reference {
                kind,
                space,
                access,
                pointee,
                nullability,
                ..
            } => {
                let cell_layout = self.type_linker.reference_cell_layout(*space, *kind);

                Some(Operand::Reference {
                    pointee: self.type_id(*pointee),
                    space: *space,
                    cell_layout,
                    reference: ReferenceFlags::new(*kind, *space, *access, *nullability),
                })
            }
            mir::Type::FunctionSignature { .. } => None,
            mir::Type::FunctionPointer { signature } => match self.tree.get(*signature) {
                mir::Type::FunctionSignature { result, .. } => Some(Operand::FunctionPointer {
                    result: self.type_id(*result),
                }),
                _ => None,
            },
            mir::Type::FixedArray {
                element, length, ..
            } => Some(Operand::Sequence {
                element: self.type_id(*element),
                length: *length,
            }),
            mir::Type::Vector { element, lanes, .. } => Some(Operand::Sequence {
                element: self.type_id(*element),
                length: u64::from(*lanes),
            }),
            mir::Type::Slice { .. } | mir::Type::Function { .. } => Some(Operand::Aggregate { ty }),
            mir::Type::Dynamic { .. } => Some(Operand::Aggregate { ty }),
            mir::Type::Tuple { .. }
            | mir::Type::Struct { .. }
            | mir::Type::Variant { .. }
            | mir::Type::Tensor { .. }
            | mir::Type::TensorView { .. } => Some(Operand::Aggregate { ty }),
        }
    }
}

/// One dense map from SSA value id to lowered operand.
pub(super) struct OperandMap {
    /// Operand for each SSA value id.
    operands: Vec<Option<Operand>>,
}

impl OperandMap {
    /// Create a new value operand table.
    fn new(value_count: usize) -> Self {
        Self {
            operands: vec![None; value_count],
        }
    }

    /// Lower one dense MIR value type table into VM operands.
    pub(super) fn lower(
        tree: &mir::Tree,
        target_layout: &mir::TargetLayout,
        types: &mir::TypeTable,
        program: &Linker<'_>,
        value_types: &[mir::LocalNodeId<mir::Type>],
    ) -> Self {
        let operand_lowerer = OperandLowerer::new(tree, target_layout, types, program);
        let mut operands = Self::new(value_types.len());

        for (index, ty) in value_types.iter().enumerate() {
            let value = mir::Value(index as u32);
            let operand = operand_lowerer.operand(*ty);

            operands.replace(value, operand);
        }

        operands
    }

    /// Get the operand for a value.
    pub(super) fn get(&self, value: mir::Value) -> Option<Operand> {
        self.operands
            .get(value.0 as usize)
            .and_then(|operand| *operand)
    }

    /// Replace the operand for a value.
    fn replace(&mut self, value: mir::Value, operand: Option<Operand>) {
        if let Some(entry) = self.operands.get_mut(value.0 as usize) {
            *entry = operand;
        }
    }

    /// Return the storage space for a value when available.
    pub(super) fn space(&self, value: mir::Value) -> Option<mir::Space> {
        match self.get(value) {
            Some(Operand::Reference { space, .. }) => Some(space),
            Some(Operand::Aggregate { .. } | Operand::Sequence { .. }) => Some(mir::Space::Frame),
            _ => None,
        }
    }

    /// Return the pointer cell layout for a value when available.
    pub(super) fn pointer_cell_layout(&self, value: mir::Value) -> Option<CellLayout> {
        match self.get(value) {
            Some(Operand::Reference { cell_layout, .. }) => Some(cell_layout),
            Some(Operand::Aggregate { .. } | Operand::Sequence { .. }) => {
                Some(CellLayout::FramePointer)
            }
            _ => None,
        }
    }

    /// Return the heap pointee type from a reference operand when available.
    pub(super) fn heap_pointee_type(
        &self,
        program: &Linker<'_>,
        value: mir::Value,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        match self.get(value) {
            Some(Operand::Reference {
                pointee,
                space: mir::Space::Local | mir::Space::Shared,
                ..
            }) => program.type_by_id(pointee),
            _ => None,
        }
    }

    /// Return the raw pointee type from a reference operand when available.
    pub(super) fn raw_pointee_type(
        &self,
        program: &Linker<'_>,
        value: mir::Value,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        match self.get(value) {
            Some(Operand::Reference {
                pointee,
                space: mir::Space::Frame | mir::Space::Static,
                ..
            }) => program.type_by_id(pointee),
            Some(Operand::Reference {
                pointee,
                cell_layout: CellLayout::Address,
                ..
            }) => program.type_by_id(pointee),
            _ => None,
        }
    }
}

use destack_mir as mir;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ProgramIndex;
use crate::vm::{
    CellLayout, ReferenceMeta, ScalarLayout, ValueShape, address_space_from_reference,
    cell_layout_from_address_space,
};

/// Durable runtime type id inside one program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct TypeId(pub u32);

impl TypeId {
    /// Return this id as a dense table index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl From<u32> for TypeId {
    /// Convert one raw program type id.
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<TypeId> for u32 {
    /// Convert one program type id into its raw value.
    fn from(id: TypeId) -> Self {
        id.0
    }
}

/// Runtime type metadata carried by one durable program.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TypeTable {
    /// Dense runtime type records keyed by program type id.
    types: Vec<Option<mir::Type>>,
    /// MIR type id by program type id.
    mir_types: Vec<Option<mir::LocalNodeId<mir::Type>>>,
    /// Program type id by MIR type id.
    type_ids: HashMap<mir::LocalNodeId<mir::Type>, TypeId>,
    /// Dense runtime layout ids keyed by program type id.
    layout_by_type: Vec<Option<mir::LayoutId>>,
}

impl TypeTable {
    /// Build one runtime type table from lowered MIR metadata.
    pub fn lower(tree: &mir::Tree, index: &ProgramIndex) -> Self {
        let mut types = Vec::new();
        let mut mir_types = Vec::new();
        let mut type_ids = HashMap::new();

        // copy type records into dense runtime slots
        for (mir_type, ty) in tree.iter_nodes::<mir::Type>() {
            let type_id = index.type_id(mir_type);
            let index = type_id.index();
            if index >= types.len() {
                types.resize_with(index + 1, || None);
                mir_types.resize_with(index + 1, || None);
            }
            types[index] = Some(ty.clone());
            mir_types[index] = Some(mir_type);
            type_ids.insert(mir_type, type_id);
        }

        // copy layout ids already assigned during VM lowerer
        let mut layout_by_type = Vec::new();
        for (mir_type, _) in tree.iter_nodes::<mir::Type>() {
            if let Some(layout_id) = tree.type_layout_id(mir_type) {
                let type_id = index.type_id(mir_type);
                let index = type_id.index();
                if index >= layout_by_type.len() {
                    layout_by_type.resize_with(index + 1, || None);
                }
                layout_by_type[index] = Some(layout_id);
            }
        }

        Self {
            types,
            mir_types,
            type_ids,
            layout_by_type,
        }
    }

    /// Return one runtime type record.
    pub fn get(&self, ty: TypeId) -> Option<&mir::Type> {
        self.types.get(ty.index()).and_then(Option::as_ref)
    }

    /// Return the program type id for one MIR type id.
    pub fn type_id(&self, ty: mir::LocalNodeId<mir::Type>) -> TypeId {
        self.type_ids[&ty]
    }

    /// Return the runtime value shape for one MIR type id.
    pub fn value_shape_for_mir(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        pointer_bytes: u8,
    ) -> Option<ValueShape> {
        self.value_shape(self.type_id(ty), pointer_bytes)
    }

    /// Return the MIR type id for one program type id.
    pub fn mir_type_id(&self, ty: TypeId) -> Option<mir::LocalNodeId<mir::Type>> {
        self.mir_types.get(ty.index()).copied().flatten()
    }

    /// Return the transparent representation type.
    pub fn repr_type(&self, mut ty: TypeId) -> TypeId {
        loop {
            match self.get(ty) {
                Some(mir::Type::Newtype { inner, .. })
                | Some(mir::Type::WithLifetimes { base: inner, .. }) => {
                    ty = self.type_id(*inner);
                }
                _ => return ty,
            }
        }
    }

    /// Return the runtime layout id for one type.
    pub fn layout_id(&self, ty: TypeId) -> Option<mir::LayoutId> {
        let ty = self.repr_type(ty);

        self.layout_by_type.get(ty.index()).copied().flatten()
    }

    /// Return whether one type is stored in one VM cell.
    pub fn is_cell_type(&self, ty: TypeId, pointer_bytes: u8) -> bool {
        self.cell_layout(ty, pointer_bytes)
            .map(|layout| layout.byte_len(pointer_bytes as usize) <= crate::vm::Cell::BYTE_LEN)
            .unwrap_or(false)
    }

    /// Return the native cell layout for one type.
    pub fn cell_layout(&self, ty: TypeId, pointer_bytes: u8) -> Option<CellLayout> {
        let ty = self.repr_type(ty);
        match self.get(ty)? {
            mir::Type::Void => Some(CellLayout::Void),
            mir::Type::Boolean => Some(CellLayout::Bool),
            mir::Type::Int { width, is_signed } => {
                let width = u8::try_from(*width).ok()?;

                if *is_signed {
                    Some(CellLayout::Int { width })
                } else {
                    Some(CellLayout::Uint { width })
                }
            }
            mir::Type::Isize => Some(CellLayout::Int {
                width: pointer_bytes * 8,
            }),
            mir::Type::Usize | mir::Type::TypeDescriptor | mir::Type::TypeId => {
                Some(CellLayout::Uint {
                    width: pointer_bytes * 8,
                })
            }
            mir::Type::Float(mir::FloatType::Float16) => Some(CellLayout::Float16),
            mir::Type::Float(mir::FloatType::Bfloat16) => Some(CellLayout::Bfloat16),
            mir::Type::Float(mir::FloatType::Float32) => Some(CellLayout::Float32),
            mir::Type::Float(mir::FloatType::Float64) => Some(CellLayout::Float64),
            mir::Type::Reference { kind, space, .. } => {
                let address_space = address_space_from_reference(space.clone(), *kind);

                cell_layout_from_address_space(address_space)
            }
            mir::Type::Uninit { value } | mir::Type::Atomic { value } => {
                self.cell_layout(self.type_id(*value), pointer_bytes)
            }
            mir::Type::FunctionPointer { .. } => Some(CellLayout::FunctionPointer),
            mir::Type::Tensor { .. } => Some(CellLayout::HeapReference),
            mir::Type::FunctionSignature { .. } | mir::Type::Function { .. } => None,
            _ => None,
        }
    }

    /// Return the scalar layout for one type.
    pub fn scalar_layout(&self, ty: TypeId, pointer_bytes: u8) -> Option<ScalarLayout> {
        match self.get(ty)? {
            mir::Type::Int { width, is_signed } => Some(ScalarLayout::Int {
                width: *width,
                is_signed: *is_signed,
            }),
            mir::Type::Isize => Some(ScalarLayout::Int {
                width: u16::from(pointer_bytes) * 8,
                is_signed: true,
            }),
            mir::Type::Usize | mir::Type::TypeDescriptor | mir::Type::TypeId => {
                Some(ScalarLayout::Int {
                    width: u16::from(pointer_bytes) * 8,
                    is_signed: false,
                })
            }
            mir::Type::Float(float_type) => Some(ScalarLayout::Float {
                format: *float_type,
            }),
            mir::Type::Boolean => Some(ScalarLayout::Bool),
            _ => None,
        }
    }

    /// Return the runtime value shape for one type.
    pub fn value_shape(&self, ty: TypeId, pointer_bytes: u8) -> Option<ValueShape> {
        match self.get(ty)? {
            mir::Type::Error => None,
            mir::Type::WithLifetimes { base, .. } => {
                self.value_shape(self.type_id(*base), pointer_bytes)
            }
            mir::Type::Void => Some(ValueShape::Void),
            mir::Type::Boolean => Some(ValueShape::Bool),
            mir::Type::Int { width, is_signed } => Some(ValueShape::Int {
                width: *width,
                signed: *is_signed,
            }),
            mir::Type::Isize => Some(ValueShape::Int {
                width: u16::from(pointer_bytes) * 8,
                signed: true,
            }),
            mir::Type::Usize => Some(ValueShape::Int {
                width: u16::from(pointer_bytes) * 8,
                signed: false,
            }),
            mir::Type::Float(float_type) => Some(ValueShape::Float {
                format: *float_type,
            }),
            mir::Type::TypeDescriptor | mir::Type::TypeId => Some(ValueShape::Int {
                width: u16::from(pointer_bytes) * 8,
                signed: false,
            }),
            mir::Type::Reference {
                kind,
                space,
                access,
                pointee,
                nullability,
                ..
            } => {
                let address_space = address_space_from_reference(space.clone(), *kind);

                Some(ValueShape::Pointer {
                    pointee: self.type_id(*pointee),
                    address_space,
                    reference: ReferenceMeta::new(*kind, space.clone(), *access, *nullability),
                })
            }
            mir::Type::FunctionSignature { .. } => None,
            mir::Type::FunctionPointer { signature } => match self.get(self.type_id(*signature))? {
                mir::Type::FunctionSignature { result, .. } => Some(ValueShape::FunctionPointer {
                    result: self.type_id(*result),
                }),
                _ => None,
            },
            mir::Type::FixedArray {
                element, length, ..
            } => Some(ValueShape::Array {
                element: self.type_id(*element),
                length: *length,
            }),
            mir::Type::Slice { .. } | mir::Type::Function { .. } => {
                Some(ValueShape::Aggregate { ty })
            }
            mir::Type::Uninit { value } | mir::Type::Atomic { value } => {
                self.value_shape(self.type_id(*value), pointer_bytes)
            }
            mir::Type::Dynamic { .. } => Some(ValueShape::Aggregate { ty }),
            mir::Type::Newtype { inner, .. } => {
                self.value_shape(self.type_id(*inner), pointer_bytes)
            }
            mir::Type::Tuple { .. }
            | mir::Type::Struct { .. }
            | mir::Type::Variant { .. }
            | mir::Type::Vector { .. }
            | mir::Type::Tensor { .. }
            | mir::Type::TensorView { .. } => Some(ValueShape::Aggregate { ty }),
        }
    }
}

use std::collections::HashMap;

use destack_dir::GlobalNodeIdAny;
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

/// Classify scalar types for lowering decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScalarType {
    /// Represent a boolean scalar.
    Bool,
    /// Represent a signed integer scalar.
    SignedInt {
        /// Store the integer bit width.
        width: u16,
    },
    /// Represent an unsigned integer scalar.
    UnsignedInt {
        /// Store the integer bit width.
        width: u16,
    },
    /// Represent a floating point scalar.
    Float {
        /// Store the float bit width.
        width: u16,
    },
}

/// Lower DIR types into MIR types with a shared cache.
#[derive(Debug)]
pub(crate) struct TypeLowerer {
    /// Cache lowered MIR types by DIR type id.
    type_cache: HashMap<dir::LocalTypeId, mir::LocalNodeId<mir::Type>>,
    /// Pointer width in bits for pointer-sized integers.
    pointer_width_bits: u16,
    /// Cache the MIR void type.
    pub(crate) ty_void: mir::LocalNodeId<mir::Type>,
    /// Cache the MIR bool type.
    pub(crate) ty_bool: mir::LocalNodeId<mir::Type>,
    /// Cache the MIR i32 type.
    pub(crate) ty_i32: mir::LocalNodeId<mir::Type>,
    /// Cache the MIR i64 type.
    pub(crate) ty_i64: mir::LocalNodeId<mir::Type>,
    /// Cache the MIR f32 type.
    pub(crate) ty_f32: mir::LocalNodeId<mir::Type>,
    /// Cache the MIR f64 type.
    pub(crate) ty_f64: mir::LocalNodeId<mir::Type>,
}

impl TypeLowerer {
    /// Create a new type lowerer with common MIR types initialized.
    pub(crate) fn new(builder: &mut mir::ModuleBuilder, pointer_bytes: u8) -> Self {
        // compute pointer width from the target pointer size
        let pointer_width_bits = u16::from(pointer_bytes) * 8;

        Self {
            type_cache: HashMap::new(),
            pointer_width_bits,
            ty_void: builder.type_void(),
            ty_bool: builder.type_bool(),
            ty_i32: builder.type_i32(),
            ty_i64: builder.type_i64(),
            ty_f32: builder.type_f32(),
            ty_f64: builder.type_f64(),
        }
    }

    /// Lower a DIR type to a MIR type.
    pub(crate) fn lower_type(
        &mut self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
        module_id: destack_source::ModuleId,
        node: GlobalNodeIdAny,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        if let Some(mir_type) = self.type_cache.get(&type_id) {
            return Ok(*mir_type);
        }

        let dir_type = types.get_type(type_id);
        let mir_type = match dir_type {
            dir::Type::PointerOf { right, .. } => {
                let pointee = self.lower_type(types, *right, module_id, node, builder)?;
                builder.type_raw_pointer(pointee)
            }
            _ => self
                .try_lower_type(dir_type, builder)
                .ok_or(LowerError::UnsupportedType {
                    node,
                    ty: type_id.into_global(module_id),
                    message: "unsupported type".to_string(),
                })?,
        };
        self.type_cache.insert(type_id, mir_type);
        Ok(mir_type)
    }

    /// Try to lower a DIR type to a MIR type.
    fn try_lower_type(
        &mut self,
        dir_type: &dir::Type,
        builder: &mut mir::ModuleBuilder,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        match dir_type {
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Void,
            } => Some(self.ty_void),
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Boolean),
            } => Some(self.ty_bool),
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Number),
            } => Some(self.ty_f64),
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Float(float_type)),
            } => match float_type.simplify() {
                dir::FloatType::Float32 => Some(self.ty_f32),
                dir::FloatType::Float64 => Some(self.ty_f64),
                dir::FloatType::Arbitrary { width } => Some(builder.type_float(width)),
            },
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Int(int_type)),
            } => match int_type.simplify() {
                dir::IntType::Int32 => Some(self.ty_i32),
                dir::IntType::Int64 => Some(self.ty_i64),
                dir::IntType::Uint32 => Some(builder.type_int(32, false)),
                dir::IntType::Uint64 => Some(builder.type_int(64, false)),
                dir::IntType::Int8 => Some(builder.type_int(8, true)),
                dir::IntType::Int16 => Some(builder.type_int(16, true)),
                dir::IntType::Int128 => Some(builder.type_int(128, true)),
                dir::IntType::Int256 => Some(builder.type_int(256, true)),
                dir::IntType::Uint8 => Some(builder.type_int(8, false)),
                dir::IntType::Uint16 => Some(builder.type_int(16, false)),
                dir::IntType::Uint128 => Some(builder.type_int(128, false)),
                dir::IntType::Uint256 => Some(builder.type_int(256, false)),
                dir::IntType::Isize => Some(builder.type_int(self.pointer_width_bits, true)),
                dir::IntType::Usize => Some(builder.type_int(self.pointer_width_bits, false)),
                dir::IntType::Arbitrary { width, is_signed } => {
                    Some(builder.type_int(width, is_signed))
                }
            },
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::ScalarLiteral(literal),
            } => match literal {
                dir::ScalarLiteral::Boolean(_) => Some(self.ty_bool),
                dir::ScalarLiteral::Integer(_) => Some(self.ty_i32),
                dir::ScalarLiteral::Float(_) => Some(self.ty_f64),
                dir::ScalarLiteral::Character(_)
                | dir::ScalarLiteral::String(_)
                | dir::ScalarLiteral::Bigint(_)
                | dir::ScalarLiteral::RegexString { .. } => None,
            },
            _ => None,
        }
    }

    /// Resolve a scalar type for a given DIR type.
    pub(crate) fn scalar_type_for_dir_type(&self, dir_type: &dir::Type) -> Option<ScalarType> {
        match dir_type {
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Boolean),
            } => Some(ScalarType::Bool),
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Number),
            } => Some(ScalarType::Float { width: 64 }),
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Float(float_type)),
            } => match float_type.simplify() {
                dir::FloatType::Float32 => Some(ScalarType::Float { width: 32 }),
                dir::FloatType::Float64 => Some(ScalarType::Float { width: 64 }),
                dir::FloatType::Arbitrary { width } => Some(ScalarType::Float { width }),
            },
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Int(int_type)),
            } => match int_type.simplify() {
                dir::IntType::Int8 => Some(ScalarType::SignedInt { width: 8 }),
                dir::IntType::Int16 => Some(ScalarType::SignedInt { width: 16 }),
                dir::IntType::Int32 => Some(ScalarType::SignedInt { width: 32 }),
                dir::IntType::Int64 => Some(ScalarType::SignedInt { width: 64 }),
                dir::IntType::Int128 => Some(ScalarType::SignedInt { width: 128 }),
                dir::IntType::Int256 => Some(ScalarType::SignedInt { width: 256 }),
                dir::IntType::Uint8 => Some(ScalarType::UnsignedInt { width: 8 }),
                dir::IntType::Uint16 => Some(ScalarType::UnsignedInt { width: 16 }),
                dir::IntType::Uint32 => Some(ScalarType::UnsignedInt { width: 32 }),
                dir::IntType::Uint64 => Some(ScalarType::UnsignedInt { width: 64 }),
                dir::IntType::Uint128 => Some(ScalarType::UnsignedInt { width: 128 }),
                dir::IntType::Uint256 => Some(ScalarType::UnsignedInt { width: 256 }),
                dir::IntType::Isize => Some(ScalarType::SignedInt {
                    width: self.pointer_width_bits,
                }),
                dir::IntType::Usize => Some(ScalarType::UnsignedInt {
                    width: self.pointer_width_bits,
                }),
                dir::IntType::Arbitrary { width, is_signed } => {
                    if is_signed {
                        Some(ScalarType::SignedInt { width })
                    } else {
                        Some(ScalarType::UnsignedInt { width })
                    }
                }
            },
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::ScalarLiteral(literal),
            } => match literal {
                dir::ScalarLiteral::Boolean(_) => Some(ScalarType::Bool),
                dir::ScalarLiteral::Integer(_) => Some(ScalarType::SignedInt { width: 32 }),
                dir::ScalarLiteral::Float(_) => Some(ScalarType::Float { width: 64 }),
                _ => None,
            },
            _ => None,
        }
    }
}

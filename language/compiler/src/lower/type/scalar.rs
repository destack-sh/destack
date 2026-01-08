use {destack_dir as dir, destack_mir as mir};

use super::TypeLowerer;

/// Scalar type classification for lowering decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScalarType {
    /// Boolean scalar.
    Bool,
    /// Signed integer scalar.
    SignedInt {
        /// Bit width.
        width: u16,
    },
    /// Unsigned integer scalar.
    UnsignedInt {
        /// Bit width.
        width: u16,
    },
    /// Floating point scalar.
    Float {
        /// Bit width.
        width: u16,
    },
}

impl TypeLowerer {
    /// Try to lower a DIR type to a MIR type.
    pub(crate) fn try_lower_type(
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

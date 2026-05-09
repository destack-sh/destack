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

impl ScalarType {
    /// 32-bit floating point scalar.
    pub(crate) const FLOAT32: Self = Self::Float { width: 32 };
    /// 64-bit floating point scalar.
    pub(crate) const FLOAT64: Self = Self::Float { width: 64 };
}

impl TypeLowerer<'_> {
    /// Try to lower a DIR type to a MIR type.
    pub(crate) fn try_lower_type(
        &mut self,
        dir_type: &dir::Type,
        builder: &mut mir::ModuleBuilder,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        match dir_type {
            dir::Type::Literal(dir::LiteralType {
                value: dir::TypeLiteral::Void,
            }) => Some(self.ty_void),
            dir::Type::Literal(dir::LiteralType {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Boolean),
            }) => Some(self.ty_bool),
            dir::Type::Literal(dir::LiteralType {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Float(float_type)),
            }) => match float_type {
                dir::FloatType::Float32 => Some(self.ty_f32),
                dir::FloatType::Float64 => Some(self.ty_f64),
            },
            dir::Type::Literal(dir::LiteralType {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::String),
            }) => self.ty_string,
            dir::Type::Literal(dir::LiteralType {
                value: dir::TypeLiteral::Null | dir::TypeLiteral::Undefined,
            }) => Some(self.ty_void),
            dir::Type::Literal(dir::LiteralType {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Integer(int_type)),
            }) => match int_type {
                dir::IntegerType::Fixed { width, is_signed } => {
                    Some(builder.type_int(*width, *is_signed))
                }
                dir::IntegerType::Pointer { is_signed: true } => Some(self.ty_isize),
                dir::IntegerType::Pointer { is_signed: false } => Some(self.ty_usize),
            },
            dir::Type::Literal(dir::LiteralType {
                value: dir::TypeLiteral::ScalarLiteral(literal),
            }) => match literal {
                dir::ScalarLiteral::Null => None,
                dir::ScalarLiteral::Boolean(_) => Some(self.ty_bool),
                dir::ScalarLiteral::Integer(_) => Some(self.ty_i32),
                dir::ScalarLiteral::Float(_) => Some(self.ty_f64),
                dir::ScalarLiteral::String(_) => self.ty_string,
                dir::ScalarLiteral::Character(_)
                | dir::ScalarLiteral::Bigint(_)
                | dir::ScalarLiteral::RegexString { .. } => None,
            },
            _ => None,
        }
    }

    /// Resolve a scalar type for a given DIR type.
    pub(crate) fn scalar_type_for_dir_type(&self, dir_type: &dir::Type) -> Option<ScalarType> {
        match dir_type {
            dir::Type::Literal(dir::LiteralType {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Boolean),
            }) => Some(ScalarType::Bool),
            dir::Type::Literal(dir::LiteralType {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Float(float_type)),
            }) => match float_type {
                dir::FloatType::Float32 => Some(ScalarType::FLOAT32),
                dir::FloatType::Float64 => Some(ScalarType::FLOAT64),
            },
            dir::Type::Literal(dir::LiteralType {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Integer(int_type)),
            }) => self.scalar_type_for_int_type(*int_type),
            dir::Type::Literal(dir::LiteralType {
                value: dir::TypeLiteral::ScalarLiteral(literal),
            }) => match literal {
                dir::ScalarLiteral::Boolean(_) => Some(ScalarType::Bool),
                dir::ScalarLiteral::Integer(_) => Some(ScalarType::SignedInt { width: 32 }),
                dir::ScalarLiteral::Float(_) => Some(ScalarType::FLOAT64),
                _ => None,
            },
            _ => None,
        }
    }

    /// Resolve a scalar type for an enum backing type.
    pub(crate) fn scalar_type_for_enum_backing(
        &self,
        backing: dir::EnumBackingType,
    ) -> Option<ScalarType> {
        match backing {
            dir::EnumBackingType::Integer(int_type) => self.scalar_type_for_int_type(int_type),
            dir::EnumBackingType::String => None,
        }
    }

    /// Resolve a scalar type for an integer type.
    pub(crate) fn scalar_type_for_int_type(
        &self,
        int_type: dir::IntegerType,
    ) -> Option<ScalarType> {
        match int_type {
            dir::IntegerType::Fixed { width, is_signed } => {
                if is_signed {
                    Some(ScalarType::SignedInt { width })
                } else {
                    Some(ScalarType::UnsignedInt { width })
                }
            }
            dir::IntegerType::Pointer { is_signed: true } => Some(ScalarType::SignedInt {
                width: self.pointer_width_bits,
            }),
            dir::IntegerType::Pointer { is_signed: false } => Some(ScalarType::UnsignedInt {
                width: self.pointer_width_bits,
            }),
        }
    }
}

use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{CompositeType, FloatType, IntType, ScalarLiteral, TypeLiteral};
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Lower a path to a DIR path.
    pub fn lower_scalar_literal(
        &mut self,
        source_id: SourceId,
        _ast: &ast::NodeTree,
        scalar_literal: &ast::ScalarLiteral,
    ) -> ScalarLiteral {
        match scalar_literal {
            ast::ScalarLiteral::Boolean(boolean) => ScalarLiteral::Boolean(*boolean),
            ast::ScalarLiteral::Byte(byte) => ScalarLiteral::Byte(*byte),
            ast::ScalarLiteral::Integer(integer) => ScalarLiteral::Integer(*integer),
            ast::ScalarLiteral::Float(float) => ScalarLiteral::Float(*float),
            ast::ScalarLiteral::Character(character) => ScalarLiteral::Character(*character),
            ast::ScalarLiteral::String(string) => {
                let string = self.intern_string(source_id, *string);
                ScalarLiteral::String(string)
            }
            ast::ScalarLiteral::ByteString(byte_string) => {
                ScalarLiteral::ByteString(byte_string.clone())
            }
        }
    }

    /// Lower an int type to a DIR int type.
    pub fn lower_int_type(&mut self, int_type: &ast::IntType) -> IntType {
        match int_type {
            ast::IntType {
                width: Some(8),
                is_signed: true,
            } => IntType::Int8,
            ast::IntType {
                width: Some(16),
                is_signed: true,
            } => IntType::Int16,
            ast::IntType {
                width: Some(32),
                is_signed: true,
            } => IntType::Int32,
            ast::IntType {
                width: Some(64),
                is_signed: true,
            } => IntType::Int64,
            ast::IntType {
                width: Some(128),
                is_signed: true,
            } => IntType::Int128,
            ast::IntType {
                width: Some(256),
                is_signed: true,
            } => IntType::Int256,
            ast::IntType {
                width: Some(8),
                is_signed: false,
            } => IntType::Uint8,
            ast::IntType {
                width: Some(16),
                is_signed: false,
            } => IntType::Uint16,
            ast::IntType {
                width: Some(32),
                is_signed: false,
            } => IntType::Uint32,
            ast::IntType {
                width: Some(64),
                is_signed: false,
            } => IntType::Uint64,
            ast::IntType {
                width: Some(128),
                is_signed: false,
            } => IntType::Uint128,
            ast::IntType {
                width: Some(256),
                is_signed: false,
            } => IntType::Uint256,
            ast::IntType {
                width: None,
                is_signed,
            } => IntType::Variable {
                width: self.options.default_int_width,
                is_signed: *is_signed,
            },
            ast::IntType {
                width: Some(width),
                is_signed,
            } => IntType::Variable {
                width: *width,
                is_signed: *is_signed,
            },
        }
    }

    /// Lower a float type to a DIR float type.
    pub fn lower_float_type(&mut self, float_type: &ast::FloatType) -> FloatType {
        match float_type {
            ast::FloatType { width: Some(16) } => FloatType::Float16,
            ast::FloatType { width: Some(32) } => FloatType::Float32,
            ast::FloatType { width: Some(64) } => FloatType::Float64,
            ast::FloatType { width: Some(80) } => FloatType::Float80,
            ast::FloatType { width: Some(128) } => FloatType::Float128,
            ast::FloatType { width: None } => FloatType::Variable {
                width: self.options.default_float_width,
            },
            ast::FloatType { width: Some(width) } => FloatType::Variable { width: *width },
        }
    }

    /// Lower a composite type to a DIR composite type.
    pub fn lower_composite_type(&mut self, composite_type: &ast::CompositeType) -> CompositeType {
        match composite_type {
            ast::CompositeType::Type => CompositeType::Type,
            ast::CompositeType::Struct => CompositeType::Struct,
            ast::CompositeType::Enum => CompositeType::Enum,
            ast::CompositeType::Union => CompositeType::Union,
            ast::CompositeType::Tuple => CompositeType::Tuple,
            ast::CompositeType::Trait => CompositeType::Trait,
            ast::CompositeType::Function => CompositeType::Function,
        }
    }

    /// Lower a type literal to a DIR type literal.
    pub fn lower_type_literal(
        &mut self,
        _source_id: SourceId,
        _ast: &ast::NodeTree,
        type_literal: &ast::TypeLiteral,
    ) -> TypeLiteral {
        match type_literal {
            ast::TypeLiteral::Never => TypeLiteral::Never,
            ast::TypeLiteral::Any => TypeLiteral::Any,
            ast::TypeLiteral::Infer => TypeLiteral::Infer,
            ast::TypeLiteral::Undefined => TypeLiteral::Undefined,
            ast::TypeLiteral::Void => TypeLiteral::Void,
            ast::TypeLiteral::Null => TypeLiteral::Null,
            ast::TypeLiteral::Boolean => TypeLiteral::Boolean,
            ast::TypeLiteral::Character => TypeLiteral::Character,
            ast::TypeLiteral::String => TypeLiteral::String,
            ast::TypeLiteral::Number => TypeLiteral::Number,
            ast::TypeLiteral::Int(int_type) => TypeLiteral::Int(self.lower_int_type(int_type)),
            ast::TypeLiteral::Float(float_type) => {
                TypeLiteral::Float(self.lower_float_type(float_type))
            }
            ast::TypeLiteral::Composite(composite_type) => {
                TypeLiteral::Composite(self.lower_composite_type(composite_type))
            }
            ast::TypeLiteral::Self_ => TypeLiteral::Self_,
        }
    }
}

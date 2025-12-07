use crate::{FunctionMode, Keyword, LocalNodeId, PrimitiveType, Type, TypeField, TypeLiteral};
use destack_fir::format::FormatResult;

use destack_fir::prelude::*;
use destack_fir::write;

use crate::format::argument::list_like;
use crate::format::property::{
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix_maybe,
};
use crate::{FormatNode, CodegenJsFormatContext, CodegenJsFormatter};

impl<'ast> Format<CodegenJsFormatContext<'ast>> for PrimitiveType {
    fn format(&self, f: &mut CodegenJsFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            PrimitiveType::Boolean => write!(f, [token("boolean")]),
            PrimitiveType::String => write!(f, [token("string")]),
            PrimitiveType::Bigint => write!(f, [token("bigint")]),
            PrimitiveType::Number => write!(f, [token("number")]),
            PrimitiveType::Symbol => write!(f, [token("symbol")]),
            PrimitiveType::UniqueSymbol => write!(f, [token("unique symbol")]),
        }
    }
}

impl<'ast> Format<CodegenJsFormatContext<'ast>> for TypeLiteral {
    fn format(&self, f: &mut CodegenJsFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            TypeLiteral::Never => write!(f, [token("never")]),
            TypeLiteral::Any => write!(f, [token("any")]),
            TypeLiteral::Undefined => write!(f, [token("undefined")]),
            TypeLiteral::Unknown => write!(f, [token("unknown")]),
            TypeLiteral::Void => write!(f, [token("void")]),
            TypeLiteral::Null => write!(f, [token("null")]),
            TypeLiteral::Primitive(primitive) => write!(f, [primitive]),
            TypeLiteral::ScalarLiteral(scalar_literal) => write!(f, [scalar_literal]),
        }
    }
}

impl<'ast> FormatNode<'ast, TypeField> for TypeField {
    fn format_node(
        &self,
        _node_id: LocalNodeId<TypeField>,
        f: &mut CodegenJsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            TypeField::Field { modifiers, key, ty } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // key
                write!(f, [key])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // type
                write!(f, [token(":"), space(), ty])?;
            }
            TypeField::Method {
                modifiers,
                key,
                signature,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // key
                write!(f, [key])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // mode
                if let Some(mode) = signature.mode
                    && mode == FunctionMode::New
                {
                    write!(f, [Keyword::New, space()])?;
                }
                // static parameters
                if let Some(static_parameters) = signature
                    .generics
                    .as_ref()
                    .and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }
                // dynamic parameters
                write!(f, [list_like("(", ")", ",", &signature.dynamic_parameters)])?;
                // return type
                if let Some(return_type) = signature.return_type {
                    write!(f, [token(":"), space(), return_type])?;
                }
            }
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Type> for Type {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Type>,
        f: &mut CodegenJsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Type::Scalar(scalar) => {
                write!(f, [scalar])?;
            }
            Type::Path {
                path,
                static_arguments,
            } => {
                write!(f, [path])?;
                if let Some(static_arguments) = static_arguments {
                    write!(f, [list_like("<", ">", ",", static_arguments)])?;
                }
            }
            Type::Expression(expression) => {
                write!(f, [expression])?;
            }

            Type::Unary { operator, right } => {
                write!(f, [operator, space(), right])?;
            }
            Type::Binary {
                left,
                operator,
                right,
            } => {
                write!(f, [left, space(), operator, space(), right])?;
            }

            Type::Array { element } => {
                if let Some(element) = element {
                    write!(f, [element, token("[]")])?;
                } else {
                    write!(f, [token("Array<any>")])?;
                }
            }
            Type::Tuple { elements } => {
                write!(f, [list_like("[", "]", ",", elements)])?;
            }
            Type::Object { properties } => {
                write!(f, [list_like("{", "}", ",", properties).include_space()])?;
            }
            Type::Union { elements } => {
                write!(f, [list_like("|", "|", ",", elements)])?;
            }
            Type::Intersection { elements } => {
                write!(f, [list_like("&", "&", ",", elements)])?;
            }
            Type::Function { signature } => {
                // mode
                if let Some(mode) = signature.mode
                    && mode == FunctionMode::New
                {
                    write!(f, [Keyword::New, space()])?;
                }
                // static parameters
                if let Some(static_parameters) = signature
                    .generics
                    .as_ref()
                    .and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }
                // dynamic parameters
                write!(f, [list_like("(", ")", ",", &signature.dynamic_parameters)])?;
                // return type
                if let Some(return_type) = signature.return_type {
                    write!(f, [token(":"), space(), return_type])?;
                }
            }

            Type::Error => {
                write!(f, [token("/* ERROR */")])?;
            }
        }
        Ok(())
    }
}

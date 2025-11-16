use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{NodeId, PrimitiveType, Type, TypeLiteral};

use dyst_fir::prelude::*;
use dyst_fir::write;

use crate::{FormatNode, JavaScriptFormatContext, JavaScriptFormatter};

impl<'ast> Format<JavaScriptFormatContext<'ast>> for PrimitiveType {
    fn format(&self, f: &mut JavaScriptFormatter<'ast, '_>) -> FormatResult<()> {
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

impl<'ast> Format<JavaScriptFormatContext<'ast>> for TypeLiteral {
    fn format(&self, f: &mut JavaScriptFormatter<'ast, '_>) -> FormatResult<()> {
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

impl<'ast> FormatNode<'ast, Type> for Type {
    fn format_node(
        &self,
        node_id: NodeId<Type>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Type::Scalar(scalar) => {
                write!(f, [scalar])?;
            }
            Type::Definition(definition) => {
                let alias = f
                    .context()
                    .get_alias_to_definition(node_id.into(), *definition);
                write!(f, [alias])?;
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
        }
        Ok(())
    }
}

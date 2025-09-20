use dyst_language_fir::format::FormatResult;

use crate::{
    DystFormatContext, DystFormatter, FloatType, FormatNode, IntType, Mutability, NodeId,
    PrimitiveType, Type,
};
use dyst_language_fir::prelude::*;
use dyst_language_fir::{format_args, write};

impl<'ast> FormatNode<'ast, Type> for Type {
    fn format_node(
        &self,
        _node_id: NodeId<Type>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Type::Infer => write!(f, [token("_")]),
            Type::Maybe(type_) => write!(f, [token("?"), type_]),
            Type::Not(type_) => write!(f, [token("!"), type_]),
            Type::Never => write!(f, [token("!")]),
            Type::This => write!(f, [token("Self")]),
            Type::Primitive(primitive) => write!(f, [primitive]),
            Type::Path {
                path,
                static_arguments,
            } => {
                if static_arguments.is_some() {
                    todo!()
                } else {
                    write!(f, [path])
                }
            }
            Type::Reference { mutability, target } => {
                write!(
                    f,
                    [
                        if *mutability == Mutability::Mutable {
                            token("&mut ")
                        } else {
                            token("&")
                        },
                        target
                    ]
                )
            }
            Type::Virtual(target) => {
                write!(f, [token("$"), target])
            }
            Type::Variadic(target) => {
                write!(f, [token(".."), target])
            }
            _ => todo!("format type"),
        }
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for PrimitiveType {
    fn format(&self, f: &mut Formatter<'_, DystFormatContext<'ast>>) -> FormatResult<()> {
        match self {
            PrimitiveType::Void => write!(f, [token("void")]),
            PrimitiveType::Null => write!(f, [token("null")]),
            PrimitiveType::Boolean => write!(f, [token("boolean")]),
            PrimitiveType::Character => write!(f, [token("char")]),
            PrimitiveType::Int(int_type) => write!(f, [int_type]),
            PrimitiveType::Float(float_type) => write!(f, [float_type]),
        }
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for IntType {
    fn format(&self, f: &mut Formatter<'_, DystFormatContext<'ast>>) -> FormatResult<()> {
        if self.is_signed {
            write!(f, [token("int"), text(&self.width.to_string())])
        } else {
            write!(f, [token("uint"), text(&self.width.to_string())])
        }
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for FloatType {
    fn format(&self, f: &mut Formatter<'_, DystFormatContext<'ast>>) -> FormatResult<()> {
        match self {
            FloatType::Float32 => write!(f, [token("float32")]),
            FloatType::Float64 => write!(f, [token("float64")]),
        }
    }
}

//! Type formatting.

use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{FormatMirNode, LocalNodeId, MirFormatter, Type};

impl<'a> FormatMirNode<'a, Type> for Type {
    fn format_node(
        &self,
        _id: LocalNodeId<Type>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        match self {
            Type::Void => write!(f, [token("void")]),
            Type::Boolean => write!(f, [token("bool")]),
            Type::Int { width, signed } => {
                let prefix = if *signed { "i" } else { "u" };
                write!(f, [text(&format!("{prefix}{width}"))])
            }
            Type::Float { width } => {
                write!(f, [text(&format!("f{width}"))])
            }
            Type::Pointer { pointee } => {
                write!(f, [token("ptr<"), pointee, token(">")])
            }
            Type::Array { element, length } => {
                write!(
                    f,
                    [
                        token("["),
                        element,
                        token("; "),
                        text(&length.to_string()),
                        token("]")
                    ]
                )
            }
            Type::Tuple { elements } => {
                write!(f, [token("(")])?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, [token(", ")])?;
                    }
                    write!(f, [elem])?;
                }
                write!(f, [token(")")])
            }
            Type::Struct { fields } => {
                write!(f, [token("struct { ")])?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, [token(", ")])?;
                    }
                    write!(f, [field.ty])?;
                }
                write!(f, [token(" }")])
            }
            Type::FunctionPointer { parameters, result } => {
                write!(f, [token("fn(")])?;
                for (i, param) in parameters.iter().enumerate() {
                    if i > 0 {
                        write!(f, [token(", ")])?;
                    }
                    write!(f, [param])?;
                }
                write!(f, [token(") -> "), result])
            }
        }
    }
}

//! Type formatting.

use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{FormatMirNode, LocalNodeId, MirFormatter, Type, TypeAlias};

impl<'a> FormatMirNode<'a, Type> for Type {
    fn format_node(&self, id: LocalNodeId<Type>, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        format_type_inner(f, id, self, true)
    }
}

pub(super) fn format_type_expanded<'a>(
    f: &mut MirFormatter<'a, '_>,
    id: LocalNodeId<Type>,
    ty: &Type,
) -> FormatResult<()> {
    format_type_inner(f, id, ty, false)
}

fn format_type_inner<'a>(
    f: &mut MirFormatter<'a, '_>,
    id: LocalNodeId<Type>,
    ty: &Type,
    use_alias: bool,
) -> FormatResult<()> {
    if use_alias && let Some(alias_name) = f.context().type_alias_name(id) {
        let name = f.context().strings.get(alias_name);
        return write!(f, [token("@"), text(name)]);
    }

    match ty {
        Type::Void => write!(f, [token("void")]),
        Type::Boolean => write!(f, [token("bool")]),
        Type::Int { width, signed } => {
            let prefix = if *signed { "i" } else { "u" };
            write!(f, [text(&format!("{prefix}{width}"))])
        }
        Type::Float { width } => {
            write!(f, [text(&format!("f{width}"))])
        }
        Type::RawPointer { pointee } => {
            write!(f, [token("rawptr<"), pointee, token(">")])
        }
        Type::ManagedReference {
            pointee,
            is_nullable: nullable,
        } => {
            if *nullable {
                write!(f, [token("ref?<"), pointee, token(">")])
            } else {
                write!(f, [token("ref<"), pointee, token(">")])
            }
        }
        Type::Array { element, length } => {
            write!(
                f,
                [
                    token("["),
                    element,
                    token(";"),
                    space(),
                    text(&length.to_string()),
                    token("]")
                ]
            )
        }
        Type::Tuple { elements } => {
            write!(f, [token("(")])?;
            for (i, elem) in elements.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(","), space()])?;
                }
                write!(f, [elem])?;
            }
            write!(f, [token(")")])
        }
        Type::Struct { fields } => {
            write!(f, [token("struct"), space(), token("{"), space()])?;
            for (i, field_id) in fields.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(","), space()])?;
                }
                let field = f.context().tree.get(*field_id);
                if let Some(name) = field.name {
                    let field_name = f.context().strings.get(name);
                    write!(f, [text(field_name), token(":"), space(), field.ty])?;
                } else {
                    write!(f, [field.ty])?;
                }
            }
            write!(f, [space(), token("}")])
        }
        Type::FunctionPointer { parameters, result } => {
            write!(f, [token("fn(")])?;
            for (i, param) in parameters.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(","), space()])?;
                }
                write!(f, [param])?;
            }
            write!(f, [token(")"), space(), token("->"), space(), result])
        }
    }
}

impl<'a> FormatMirNode<'a, TypeAlias> for TypeAlias {
    fn format_node(
        &self,
        _id: LocalNodeId<TypeAlias>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        let name = f.context().strings.get(self.name);

        write!(
            f,
            [
                token("type"),
                space(),
                token("@"),
                text(name),
                space(),
                token("="),
                space()
            ]
        )?;

        let ty = f.context().tree.get(self.ty);
        format_type_expanded(f, self.ty, ty)
    }
}

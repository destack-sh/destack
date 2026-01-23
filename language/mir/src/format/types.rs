use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    AddressSpace, FormatMirNode, LocalNodeId, MirFormatter, Mutability, ReferenceKind,
    TensorDimension, TensorLayout, Type, TypeAlias, format_attribute_inline,
    format_attribute_lines,
};

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
        let alias_name = alias_name.to_string();
        return write!(f, [token("@"), text(&alias_name)]);
    }

    match ty {
        Type::Void => write!(f, [token("void")]),
        Type::Boolean => write!(f, [token("bool")]),
        Type::Int {
            width,
            is_signed: signed,
        } => {
            let prefix = if *signed { "i" } else { "u" };
            write!(f, [text(&format!("{prefix}{width}"))])
        }
        Type::Isize => write!(f, [token("isize")]),
        Type::Usize => write!(f, [token("usize")]),
        Type::Float { width } => {
            write!(f, [text(&format!("f{width}"))])
        }
        Type::Type => write!(f, [token("type")]),
        Type::Reference {
            kind,
            address_space,
            mutability,
            pointee,
            is_nullable,
        } => {
            // reference header
            let ref_token = if *is_nullable { "ref?<" } else { "ref<" };
            let kind_token = match kind {
                ReferenceKind::Managed => "managed",
                ReferenceKind::Owned => "owned",
                ReferenceKind::Borrowed => "borrowed",
                ReferenceKind::Raw => "raw",
            };

            // address space clause
            let address_space_token = match address_space {
                AddressSpace::Generic => None,
                AddressSpace::Target(id) => Some(format!("addrspace({id})")),
                _ => address_space
                    .keyword()
                    .map(|name| format!("addrspace({name})")),
            };

            // render reference syntax
            if *mutability == Mutability::Mutable && address_space_token.is_some() {
                let addrspace = address_space_token.as_deref().unwrap_or("");
                write!(
                    f,
                    [
                        token(ref_token),
                        token(kind_token),
                        space(),
                        text(addrspace),
                        space(),
                        token("mut"),
                        space(),
                        pointee,
                        token(">")
                    ]
                )
            } else if *mutability == Mutability::Mutable {
                write!(
                    f,
                    [
                        token(ref_token),
                        token(kind_token),
                        space(),
                        token("mut"),
                        space(),
                        pointee,
                        token(">")
                    ]
                )
            } else if let Some(addrspace) = address_space_token {
                write!(
                    f,
                    [
                        token(ref_token),
                        token(kind_token),
                        space(),
                        text(&addrspace),
                        space(),
                        pointee,
                        token(">")
                    ]
                )
            } else {
                write!(
                    f,
                    [
                        token(ref_token),
                        token(kind_token),
                        space(),
                        pointee,
                        token(">")
                    ]
                )
            }
        }
        Type::Array {
            element,
            length,
            copyability: _,
        } => {
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
        Type::Tuple {
            elements,
            copyability: _,
        } => {
            write!(f, [token("(")])?;
            for (i, elem) in elements.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(","), space()])?;
                }
                write!(f, [elem])?;
            }
            write!(f, [token(")")])
        }
        Type::Struct {
            fields,
            copyability: _,
        } => {
            write!(f, [token("{"), space()])?;
            for (i, field_id) in fields.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(","), space()])?;
                }
                let field = f.context().tree.get(*field_id);
                let attributes = f.context().tree.attributes(*field_id);
                if !attributes.is_empty() {
                    format_attribute_inline(attributes, f)?;
                    write!(f, [space()])?;
                }
                if let Some(name) = field.name {
                    let field_name = f.context().strings.get(name);
                    write!(f, [text(field_name), token(":"), space(), field.ty])?;
                } else {
                    write!(f, [field.ty])?;
                }
            }
            write!(f, [space(), token("}")])
        }
        Type::Vector {
            element,
            lanes,
            copyability: _,
        } => {
            write!(
                f,
                [
                    token("vector"),
                    token("<"),
                    element,
                    token(","),
                    space(),
                    text(&lanes.to_string()),
                    token(">")
                ]
            )
        }
        Type::Tensor {
            element,
            shape,
            layout,
            copyability: _,
        } => {
            write!(
                f,
                [token("tensor"), token("<"), element, token(","), space()]
            )?;
            format_shape(shape, f)?;
            if *layout != TensorLayout::RowMajor {
                write!(f, [token(","), space(), token("layout=")])?;
                format_tensor_layout(layout, f)?;
            }
            write!(f, [token(">")])
        }
        Type::TensorReference {
            kind,
            address_space,
            mutability,
            element,
            shape,
            layout,
            is_nullable,
        } => {
            let view_token = if *is_nullable {
                "tensor_ref?<"
            } else {
                "tensor_ref<"
            };
            write!(f, [token(view_token)])?;
            format_view_header(*kind, *address_space, *mutability, *element, f)?;
            write!(f, [token(","), space()])?;
            format_shape(shape, f)?;
            if *layout != TensorLayout::RowMajor {
                write!(f, [token(","), space(), token("layout=")])?;
                format_tensor_layout(layout, f)?;
            }
            write!(f, [token(">")])
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

fn format_shape<'a>(shape: &[TensorDimension], f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    write!(f, [token("["),])?;
    for (i, dim) in shape.iter().enumerate() {
        if i > 0 {
            write!(f, [token(","), space()])?;
        }
        match dim {
            TensorDimension::Static(value) => {
                write!(f, [text(&value.to_string())])?;
            }
            TensorDimension::Dynamic => {
                write!(f, [token("dynamic")])?;
            }
        }
    }
    write!(f, [token("]")])
}

fn format_tensor_layout<'a>(
    layout: &TensorLayout,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    match layout {
        TensorLayout::RowMajor => write!(f, [token("row_major")]),
        TensorLayout::ColumnMajor => write!(f, [token("column_major")]),
        TensorLayout::Strided { strides } => {
            write!(f, [token("strided"), token("(")])?;
            format_shape(strides, f)?;
            write!(f, [token(")")])
        }
    }
}

fn format_view_header<'a>(
    kind: ReferenceKind,
    address_space: AddressSpace,
    mutability: Mutability,
    element: LocalNodeId<Type>,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let kind_token = match kind {
        ReferenceKind::Managed => "managed",
        ReferenceKind::Owned => "owned",
        ReferenceKind::Borrowed => "borrowed",
        ReferenceKind::Raw => "raw",
    };

    let address_space_token = match address_space {
        AddressSpace::Generic => None,
        AddressSpace::Target(id) => Some(format!("addrspace({id})")),
        _ => address_space
            .keyword()
            .map(|name| format!("addrspace({name})")),
    };

    if mutability == Mutability::Mutable && address_space_token.is_some() {
        let addrspace = address_space_token.as_deref().unwrap_or("");
        write!(
            f,
            [
                token(kind_token),
                space(),
                text(addrspace),
                space(),
                token("mut"),
                space(),
                element
            ]
        )
    } else if mutability == Mutability::Mutable {
        write!(
            f,
            [token(kind_token), space(), token("mut"), space(), element]
        )
    } else if let Some(addrspace) = address_space_token {
        write!(
            f,
            [
                token(kind_token),
                space(),
                text(&addrspace),
                space(),
                element
            ]
        )
    } else {
        write!(f, [token(kind_token), space(), element])
    }
}

impl<'a> FormatMirNode<'a, TypeAlias> for TypeAlias {
    fn format_node(
        &self,
        id: LocalNodeId<TypeAlias>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        let attributes = f.context().tree.attributes(id);
        if !attributes.is_empty() {
            format_attribute_lines(attributes, f)?;
        }

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

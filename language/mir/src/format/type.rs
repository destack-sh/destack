use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    AddressSpace, Attribute, Field, FormatMirNode, LocalNodeId, MirFormatter, Mutability,
    ReferenceKind, TensorDimension, TensorLayout, Type, TypeAlias, format_attribute_inline,
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

pub(super) fn format_type_declaration<'a>(
    name: &str,
    attributes: &[Attribute],
    type_id: LocalNodeId<Type>,
    ty: &Type,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    if !attributes.is_empty() {
        format_attribute_lines(attributes, f)?;
    }

    match ty {
        Type::Struct { fields, .. } => {
            write!(f, [token("type"), space(), text(name), space(), token("{")])?;

            if fields.is_empty() {
                return write!(f, [space(), token("}")]);
            }

            let field_ids = fields.clone();
            write!(f, [hard_line_break()])?;
            write!(
                f,
                [block_indent(&format_with(
                    |f: &mut Formatter<'_, crate::MirFormatContext<'a>>| {
                        for (index, field_id) in field_ids.iter().enumerate() {
                            if index > 0 {
                                write!(f, [hard_line_break()])?;
                            }

                            let field = f.context().tree.get(*field_id);
                            let field_attributes = f.context().tree.attributes(*field_id);
                            if !field_attributes.is_empty() {
                                format_attribute_lines(field_attributes, f)?;
                            }

                            format_struct_field(field, f)?;
                        }

                        Ok(())
                    }
                ))]
            )?;
            write!(f, [hard_line_break(), token("}")])
        }
        _ => {
            write!(f, [token("type"), space(), text(name), space()])?;
            format_type_expanded(f, type_id, ty)?;
            Ok(())
        }
    }
}

fn format_type_inner<'a>(
    f: &mut MirFormatter<'a, '_>,
    id: LocalNodeId<Type>,
    ty: &Type,
    use_alias: bool,
) -> FormatResult<()> {
    if use_alias && let Some(alias_name) = f.context().type_alias_name(id) {
        let alias_name = alias_name.to_string();
        return write!(f, [text(&alias_name)]);
    }

    match ty {
        Type::Void => write!(f, [token("void")]),
        Type::Boolean => write!(f, [token("boolean")]),
        Type::Int {
            width,
            is_signed: signed,
        } => {
            let prefix = if *signed { "int" } else { "uint" };
            write!(f, [text(&format!("{prefix}{width}"))])
        }
        Type::Isize => write!(f, [token("isize")]),
        Type::Usize => write!(f, [token("usize")]),
        Type::Float { width } => {
            write!(f, [text(&format!("float{width}"))])
        }
        Type::TypeDescriptor => write!(f, [token("typeDescriptor")]),
        Type::TypeId => write!(f, [token("typeId")]),
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
                AddressSpace::Target(id) => Some(format!("addressSpace({id})")),
                _ => address_space
                    .keyword()
                    .map(|name| format!("addressSpace({name})")),
            };

            // render reference syntax
            write!(
                f,
                [
                    token(ref_token),
                    pointee,
                    token(","),
                    space(),
                    token(kind_token)
                ]
            )?;
            if *mutability == Mutability::Immutable {
                write!(f, [token(","), space(), token("readonly")])?;
            }
            if let Some(addrspace) = address_space_token {
                write!(f, [token(","), space(), text(&addrspace)])?;
            }
            write!(f, [token(">")])
        }
        Type::Array {
            element,
            length,
            copyability: _,
        } => {
            write!(
                f,
                [element, token("["), text(&length.to_string()), token("]")]
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
        Type::Newtype {
            inner,
            copyability: _,
        } => {
            write!(f, [token("newtype"), token("<"), inner, token(">")])
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
                write!(f, [token(","), space(), token("layout"), token("(")])?;
                format_tensor_layout(layout, f)?;
                write!(f, [token(")")])?;
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
                "tensorRef?<"
            } else {
                "tensorRef<"
            };
            write!(f, [token(view_token)])?;
            format_view_header(*kind, *address_space, *mutability, *element, f)?;
            write!(f, [token(","), space()])?;
            format_shape(shape, f)?;
            if *layout != TensorLayout::RowMajor {
                write!(f, [token(","), space(), token("layout"), token("(")])?;
                format_tensor_layout(layout, f)?;
                write!(f, [token(")")])?;
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
        Type::Closure { signature, .. } => {
            let signature_type = f.context().tree.get(*signature);
            if let Type::FunctionPointer { parameters, result } = signature_type {
                write!(f, [token("closure(")])?;
                for (i, param) in parameters.iter().enumerate() {
                    if i > 0 {
                        write!(f, [token(","), space()])?;
                    }
                    write!(f, [param])?;
                }
                write!(f, [token(")"), space(), token("->"), space(), result])
            } else {
                write!(f, [token("closure<"), signature, token(">")])
            }
        }
    }
}

fn format_shape<'a>(shape: &[TensorDimension], f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    write!(f, [token("("),])?;
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
    write!(f, [token(")")])
}

fn format_tensor_layout<'a>(
    layout: &TensorLayout,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    match layout {
        TensorLayout::RowMajor => write!(f, [token("rowMajor")]),
        TensorLayout::ColumnMajor => write!(f, [token("columnMajor")]),
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
        AddressSpace::Target(id) => Some(format!("addressSpace({id})")),
        _ => address_space
            .keyword()
            .map(|name| format!("addressSpace({name})")),
    };

    write!(f, [element, token(","), space(), token(kind_token)])?;
    if mutability == Mutability::Immutable {
        write!(f, [token(","), space(), token("readonly")])?;
    }
    if let Some(addrspace) = address_space_token {
        write!(f, [token(","), space(), text(&addrspace)])?;
    }
    Ok(())
}

impl<'a> FormatMirNode<'a, TypeAlias> for TypeAlias {
    fn format_node(
        &self,
        id: LocalNodeId<TypeAlias>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        let attributes = f.context().tree.attributes(id);
        let name = f.context().strings.get(self.name);
        let name = f.context().format_alias_name(name);
        let ty = f.context().tree.get(self.ty);
        format_type_declaration(&name, attributes, self.ty, ty, f)
    }
}

fn format_struct_field<'a>(field: &Field, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    if let Some(name) = field.name {
        let field_name = f.context().strings.get(name);
        write!(
            f,
            [text(field_name), token(":"), space(), field.ty, token(";")]
        )
    } else {
        write!(f, [field.ty, token(";")])
    }
}

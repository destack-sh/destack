use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use super::attribute::{write_attributes, write_attributes_before_anchor, write_inline_attributes};

use crate::{
    Access, Attribute, AttributeIdentifier, BorrowObligation, Copy, Field, FieldSpan,
    FormatMirNode, Lifetime, LifetimeOrigin, LocalNodeId, MirFormatContext, MirFormatter,
    Nullability, ReferenceKind, Space, TensorDimension, TensorDimensionOrder, TensorLayout,
    TensorViewLayout, Type, TypeAlias, TypeDeclarationSpans, TypeReference, write_comments_before,
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
    alias_id: Option<LocalNodeId<TypeAlias>>,
    type_id: LocalNodeId<Type>,
    ty: &Type,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let attributes = alias_id
        .map(|alias_id| f.context().tree.attributes(alias_id))
        .unwrap_or(attributes);

    // synthetic copy marker
    if type_copy(ty) == Some(Copy::No) && !has_copy_marker(attributes, f) {
        write!(f, [token("@moveOnly"), hard_line_break()])?;
    }

    // declaration attributes
    if let Some(alias_id) = alias_id {
        let tree = f.context().tree;

        if !attributes.is_empty() {
            if let Some(keyword_span) = tree.keyword_span(alias_id) {
                write_attributes_before_anchor(
                    attributes,
                    tree.attribute_spans(alias_id),
                    keyword_span.start,
                    tree,
                    f,
                )?;
            } else {
                write_attributes(attributes, f)?;
            }
        }
    } else if !attributes.is_empty() {
        write_attributes(attributes, f)?;
    }

    match ty {
        Type::Struct { fields, .. } => format_struct_type_declaration(name, alias_id, fields, f),
        _ => {
            write!(
                f,
                [
                    token("type"),
                    space(),
                    text(name),
                    space(),
                    token("="),
                    space()
                ]
            )?;
            format_type_expanded(f, type_id, ty)?;
            write!(f, [token(";")])
        }
    }
}

/// Return the explicit copy property carried by one aggregate type.
fn type_copy(ty: &Type) -> Option<Copy> {
    match ty {
        Type::Array { copy, .. }
        | Type::Tuple { copy, .. }
        | Type::Struct { copy, .. }
        | Type::Newtype { copy, .. }
        | Type::Variant { copy, .. }
        | Type::Vector { copy, .. }
        | Type::Tensor { copy, .. } => Some(*copy),
        _ => None,
    }
}

/// Return whether attributes already include an explicit copy marker.
fn has_copy_marker(attributes: &[Attribute], f: &MirFormatter<'_, '_>) -> bool {
    attributes.iter().any(|attribute| {
        let AttributeIdentifier::Identifier(name) = attribute.name else {
            return false;
        };

        matches!(f.context().strings.get(name), "copy" | "moveOnly")
    })
}

/// Format one struct type declaration.
fn format_struct_type_declaration<'a>(
    name: &str,
    alias_id: Option<LocalNodeId<TypeAlias>>,
    fields: &[LocalNodeId<Field>],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token("type"), space(), text(name), space(), token("{")])?;

    if fields.is_empty() {
        return write!(f, [space(), token("}")]);
    }

    let tree = f.context().tree;
    let field_spans = alias_id
        .map(|alias_id| tree.type_field_spans(alias_id).to_vec())
        .unwrap_or_default();
    let declaration_spans =
        alias_id.and_then(|alias_id| tree.type_declaration_spans(alias_id).cloned());

    write!(f, [hard_line_break()])?;
    write!(
        f,
        [block_indent(&format_with(
            |f: &mut Formatter<'_, MirFormatContext<'a>>| {
                format_struct_fields(fields, &field_spans, declaration_spans.as_ref(), f)
            }
        ))]
    )?;
    write!(f, [hard_line_break(), token("}")])
}

/// Format the fields of one struct type declaration.
fn format_struct_fields<'a>(
    field_ids: &[LocalNodeId<Field>],
    field_spans: &[FieldSpan],
    declaration_spans: Option<&TypeDeclarationSpans>,
    f: &mut Formatter<'_, MirFormatContext<'a>>,
) -> FormatResult<()> {
    let tree = f.context().tree;

    // comments before the first field
    if let Some(declaration_spans) = declaration_spans
        && let Some(open_brace_span) = declaration_spans.open_brace
        && let Some(first_field_span) = field_spans.first()
    {
        write_comments_before(tree, open_brace_span.end, first_field_span.span.start, f)?;
    }

    for (index, field_id) in field_ids.iter().enumerate() {
        if index > 0 {
            write!(f, [hard_line_break()])?;
        }

        // comments between fields
        if let Some(previous_field_span) = index
            .checked_sub(1)
            .and_then(|previous_index| field_spans.get(previous_index))
            && let Some(field_span) = field_spans.get(index)
        {
            write_comments_before(tree, previous_field_span.span.end, field_span.span.start, f)?;
        }

        format_struct_field_entry(*field_id, field_spans.get(index), f)?;
    }

    // comments before the closing brace
    if let Some(declaration_spans) = declaration_spans
        && let Some(last_field_span) = field_spans.last()
        && let Some(close_brace_span) = declaration_spans.close_brace
    {
        write_comments_before(tree, last_field_span.span.end, close_brace_span.start, f)?;
    }

    Ok(())
}

/// Format one struct field entry.
fn format_struct_field_entry<'a>(
    field_id: LocalNodeId<Field>,
    field_span: Option<&FieldSpan>,
    f: &mut Formatter<'_, MirFormatContext<'a>>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let field = tree.get(field_id);
    let field_attributes = tree.attributes(field_id);

    // field attributes
    if !field_attributes.is_empty() {
        if let Some(field_span) = field_span {
            let field_head_start = field_span.name_span.unwrap_or(field_span.type_span).start;

            write_attributes_before_anchor(
                field_attributes,
                &field_span.attribute_spans,
                field_head_start,
                tree,
                f,
            )?;
        } else {
            write_attributes(field_attributes, f)?;
        }
    }

    // field body
    format_struct_field(field, f)
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
        Type::Float(float_type) => write!(f, [text(&format!("float{}", float_type.width()))]),
        Type::TypeDescriptor => write!(f, [token("typeDescriptor")]),
        Type::TypeId => write!(f, [token("typeId")]),
        Type::Atomic { value } => {
            write!(f, [token("atomic"), token("<"), value, token(">")])
        }
        Type::Dynamic { constraint } => {
            write!(f, [token("dynamic"), token("<"), constraint, token(">")])
        }
        Type::Uninit { value } => {
            write!(f, [token("uninit"), token("<"), value, token(">")])
        }
        Type::Reference {
            kind,
            lifetime,
            space: memory_space,
            access,
            pointee,
            nullability,
        } => {
            write!(f, [token("ref"), token("<")])?;
            format_view_header(
                *kind,
                lifetime,
                memory_space.clone(),
                *access,
                *nullability,
                *pointee,
                f,
            )?;
            write!(f, [token(">")])
        }
        Type::Array {
            element,
            length,
            copy: _,
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
        Type::Slice {
            kind,
            lifetime,
            element,
            space,
            access,
            nullability,
        } => {
            write!(f, [token("slice"), token("<"), element])?;
            format_reference_qualifiers(*kind, lifetime, space.clone(), *access, *nullability, f)?;
            write!(f, [token(">")])
        }
        Type::Tuple { elements, copy: _ } => {
            write!(f, [token("(")])?;
            for (i, elem) in elements.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(","), space()])?;
                }
                write!(f, [elem])?;
            }
            write!(f, [token(")")])
        }
        Type::Struct { fields, copy: _ } => {
            write!(f, [token("{"), space()])?;
            for (i, field_id) in fields.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(","), space()])?;
                }
                let field = f.context().tree.get(*field_id);
                let attributes = f.context().tree.attributes(*field_id);
                if !attributes.is_empty() {
                    write_inline_attributes(attributes, f)?;
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
        Type::Newtype { inner, copy: _ } => {
            write!(f, [token("newtype"), token("<"), inner, token(">")])
        }
        Type::Variant {
            tag,
            storage,
            cases,
            copy: _,
        } => {
            write!(
                f,
                [
                    token("variant"),
                    token("<"),
                    tag,
                    token(","),
                    space(),
                    storage
                ]
            )?;
            write!(f, [token(">"), space(), token("{")])?;
            if !cases.is_empty() {
                write!(f, [space()])?;
            }
            for (index, case) in cases.iter().enumerate() {
                if index > 0 {
                    write!(f, [space()])?;
                }
                write!(
                    f,
                    [&case.tag, space(), token("="), space(), case.ty, token(";")]
                )?;
            }
            if !cases.is_empty() {
                write!(f, [space()])?;
            }
            write!(f, [token("}")])
        }
        Type::Vector {
            element,
            lanes,
            copy: _,
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
            copy: _,
        } => {
            write!(
                f,
                [token("tensor"), token("<"), element, token(","), space()]
            )?;
            format_shape(shape, f)?;
            if *layout != TensorLayout::dense_row_major() {
                write!(f, [token(","), space(), token("layout"), token("(")])?;
                format_tensor_layout(layout, f)?;
                write!(f, [token(")")])?;
            }
            write!(f, [token(">")])
        }
        Type::TensorView {
            kind,
            lifetime,
            space: memory_space,
            access,
            element,
            shape,
            layout,
            nullability,
        } => {
            write!(f, [token("tensorView"), token("<")])?;
            format_view_header(
                *kind,
                lifetime,
                memory_space.clone(),
                *access,
                *nullability,
                *element,
                f,
            )?;
            write!(f, [token(","), space()])?;
            format_shape(shape, f)?;
            if *layout != TensorViewLayout::dense_row_major() {
                write!(f, [token(","), space(), token("layout"), token("(")])?;
                format_tensor_view_layout(layout, f)?;
                write!(f, [token(")")])?;
            }
            write!(f, [token(">")])
        }
        Type::FunctionSignature {
            parameters,
            result,
            borrow_obligations,
        } => {
            write!(f, [token("(")])?;
            for (i, param) in parameters.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(","), space()])?;
                }
                write!(f, [param])?;
            }
            write!(f, [token(")"), space(), token("->"), space(), result])?;
            format_borrow_obligations(borrow_obligations, f)
        }
        Type::FunctionPointer { signature } | Type::Closure { signature, .. } => {
            if let TypeReference::Type(signature) = *signature {
                let signature_type = f.context().tree.get(signature);
                if let Type::FunctionSignature {
                    parameters,
                    result,
                    borrow_obligations,
                } = signature_type
                {
                    write!(f, [token("(")])?;
                    for (i, param) in parameters.iter().enumerate() {
                        if i > 0 {
                            write!(f, [token(","), space()])?;
                        }
                        write!(f, [param])?;
                    }
                    let arrow = match ty {
                        Type::FunctionPointer { .. } => "->",
                        Type::Closure { .. } => "=>",
                        _ => unreachable!(),
                    };
                    write!(f, [token(")"), space(), token(arrow), space(), result])?;
                    format_borrow_obligations(borrow_obligations, f)?;
                    return Ok(());
                }
            }

            write!(
                f,
                [
                    token("("),
                    signature,
                    token(")"),
                    space(),
                    token("=>"),
                    space(),
                    token("<?>")
                ]
            )
        }
    }
}

fn format_shape<'a>(shape: &[TensorDimension], f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    write!(f, [token("(")])?;
    for (i, dim) in shape.iter().enumerate() {
        if i > 0 {
            write!(f, [token(","), space()])?;
        }
        match dim {
            TensorDimension::Static(value) => {
                write!(f, [text(&value.to_string())])?;
            }
            TensorDimension::Symbol(name) => write!(f, [text(name)])?,
            TensorDimension::Dynamic => write!(f, [token("dynamic")])?,
        }
    }
    write!(f, [token(")")])
}

fn format_tensor_layout<'a>(
    layout: &TensorLayout,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    match layout {
        TensorLayout::Dense {
            order: TensorDimensionOrder::RowMajor,
        } => write!(
            f,
            [token("dense"), token("("), token("rowMajor"), token(")")]
        ),
        TensorLayout::Dense {
            order: TensorDimensionOrder::ColumnMajor,
        } => write!(
            f,
            [token("dense"), token("("), token("columnMajor"), token(")")]
        ),
    }
}

fn format_tensor_view_layout<'a>(
    layout: &TensorViewLayout,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    match layout {
        TensorViewLayout::Dense {
            order: TensorDimensionOrder::RowMajor,
        } => write!(
            f,
            [token("dense"), token("("), token("rowMajor"), token(")")]
        ),
        TensorViewLayout::Dense {
            order: TensorDimensionOrder::ColumnMajor,
        } => write!(
            f,
            [token("dense"), token("("), token("columnMajor"), token(")")]
        ),
        TensorViewLayout::Strided => write!(f, [token("strided")]),
    }
}

fn format_view_header<'a>(
    kind: ReferenceKind,
    lifetime: &Lifetime,
    memory_space: Space,
    access: Access,
    nullability: Nullability,
    element: TypeReference,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [element])?;
    format_reference_qualifiers(kind, lifetime, memory_space, access, nullability, f)
}

fn format_reference_qualifiers<'a>(
    kind: ReferenceKind,
    lifetime: &Lifetime,
    memory_space: Space,
    access: Access,
    nullability: Nullability,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let kind_token = match kind {
        ReferenceKind::Managed => "managed",
        ReferenceKind::Unique => "unique",
        ReferenceKind::Borrowed => "borrowed",
        ReferenceKind::Raw => "raw",
    };

    write!(f, [token(","), space(), token(kind_token)])?;
    format_lifetime(lifetime, f)?;
    format_access(access, f)?;
    format_nullability(nullability, f)?;
    if !memory_space.is_local() {
        write!(
            f,
            [
                token(","),
                space(),
                token("space"),
                token("("),
                text(memory_space.label()),
                token(")")
            ]
        )?;
    }
    Ok(())
}

fn format_nullability<'a>(
    nullability: Nullability,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    match nullability {
        Nullability::None => Ok(()),
        Nullability::Null => write!(f, [token(","), space(), token("nullable")]),
        Nullability::Undefined => write!(f, [token(","), space(), token("undefined")]),
        Nullability::NullOrUndefined => write!(f, [token(","), space(), token("nullish")]),
    }
}

fn format_lifetime<'a>(lifetime: &Lifetime, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    if lifetime.is_empty() {
        return Ok(());
    }

    write!(f, [token(","), space(), token("lifetime")])?;
    format_lifetime_group(lifetime, f)
}

pub(super) fn format_borrow_obligations<'a>(
    obligations: &[BorrowObligation],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    for obligation in obligations {
        match obligation {
            BorrowObligation::SuspensionStable { lifetime } => {
                write!(f, [space(), token("@"), token("suspensionSafe")])?;
                format_lifetime_group(lifetime, f)?;
            }
        }
    }

    Ok(())
}

fn format_lifetime_group<'a>(
    lifetime: &Lifetime,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token("(")])?;
    for (index, source) in lifetime.origins.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        match source {
            LifetimeOrigin::Static => write!(f, [token("static")])?,
            LifetimeOrigin::Parameter(index) => write!(f, [text(&index.to_string())])?,
        }
    }
    write!(f, [token(")")])
}

fn format_access<'a>(access: Access, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    match access {
        Access::Readonly => write!(f, [token(","), space(), token("readonly")]),
        Access::Mutable => Ok(()),
        Access::Exclusive => write!(f, [token(","), space(), token("exclusive")]),
    }
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
        let TypeReference::Type(type_id) = self.ty else {
            write!(
                f,
                [
                    token("type"),
                    space(),
                    text(&name),
                    space(),
                    token("="),
                    space(),
                    self.ty,
                    token(";")
                ]
            )?;
            return Ok(());
        };
        let ty = f.context().tree.get(type_id);
        format_type_declaration(&name, attributes, Some(id), type_id, ty, f)
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

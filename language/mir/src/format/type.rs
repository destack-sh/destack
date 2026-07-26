use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use super::attribute::{write_attributes, write_attributes_before_anchor, write_inline_attributes};
use super::value::format_type_id;

use crate::{
    Access, Attribute, AttributeIdentifier, Copy, Field, FieldSpan, FormatMirNode, Lifetime,
    LifetimeParameter, LifetimeTerm, LocalNodeId, MirFormatContext, MirFormatter, Nullability,
    ReferenceKind, Space, TensorDimension, TensorDimensionOrder, TensorFormat, TensorReduction,
    TensorSharding, TensorShardingAxis, TensorViewFormat, Type, TypeDeclaration,
    TypeDeclarationSpans, TypeId, write_comments_before,
};

impl<'a> FormatMirNode<'a, Type> for Type {
    fn format_node(&self, id: LocalNodeId<Type>, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        format_type_inner(f, id, self, true)
    }
}

/// Formatter adapter for one nested type reference.
struct FormatTypeId(TypeId);

impl<'a> Format<'a, MirFormatContext<'a>> for FormatTypeId {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        format_type_id(self.0, f)
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
    declaration_id: Option<LocalNodeId<TypeDeclaration>>,
    type_id: LocalNodeId<Type>,
    ty: &Type,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let attributes = declaration_id
        .map(|declaration_id| f.context().tree.attributes(declaration_id))
        .unwrap_or(attributes);
    let lifetimes = declaration_id
        .map(|declaration_id| f.context().tree.get(declaration_id).lifetimes.clone())
        .unwrap_or_default();

    // synthetic copy marker
    if type_copy(ty) == Some(Copy::Yes) && !has_copy_attribute(attributes, f) {
        write!(f, [token("@copy"), hard_line_break()])?;
    }

    // declaration attributes
    if let Some(declaration_id) = declaration_id {
        let tree = f.context().tree;

        if !attributes.is_empty() {
            if let Some(keyword_span) = tree.keyword_span(declaration_id) {
                write_attributes_before_anchor(
                    attributes,
                    tree.attribute_spans(declaration_id),
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

    let previous_lifetimes =
        std::mem::replace(&mut f.context_mut().current_lifetimes, lifetimes.clone());
    let result = match ty {
        Type::Struct { fields, .. } => {
            format_struct_type_declaration(name, declaration_id, &lifetimes, fields, f)
        }
        _ => {
            write!(f, [token("type"), space(), copied_text(name)])?;
            format_lifetimes(&lifetimes, f)?;
            write!(f, [space(), token("="), space()])?;
            format_type_expanded(f, type_id, ty)?;
            write!(f, [token(";")])
        }
    };
    f.context_mut().current_lifetimes = previous_lifetimes;

    result
}

/// Return the explicit copy property carried by one aggregate type.
fn type_copy(ty: &Type) -> Option<Copy> {
    match ty {
        Type::FixedArray { copy, .. }
        | Type::Tuple { copy, .. }
        | Type::Struct { copy, .. }
        | Type::Newtype { copy, .. }
        | Type::Variant { copy, .. }
        | Type::Vector { copy, .. }
        | Type::Tensor { copy, .. } => Some(*copy),
        _ => None,
    }
}

/// Return whether attributes already include an explicit copy attribute.
fn has_copy_attribute(attributes: &[Attribute], f: &MirFormatter<'_, '_>) -> bool {
    attributes.iter().any(|attribute| {
        let AttributeIdentifier::Identifier(name) = attribute.name else {
            return false;
        };

        f.context().strings.get(name) == "copy"
    })
}

/// Format one struct type declaration.
fn format_struct_type_declaration<'a>(
    name: &str,
    declaration_id: Option<LocalNodeId<TypeDeclaration>>,
    lifetimes: &[LifetimeParameter],
    fields: &[LocalNodeId<Field>],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token("type"), space(), copied_text(name)])?;
    format_lifetimes(lifetimes, f)?;
    write!(f, [space(), token("{")])?;

    if fields.is_empty() {
        return write!(f, [space(), token("}")]);
    }

    let tree = f.context().tree;
    let field_spans = declaration_id
        .map(|declaration_id| tree.type_field_spans(declaration_id).to_vec())
        .unwrap_or_default();
    let declaration_spans = declaration_id
        .and_then(|declaration_id| tree.type_declaration_spans(declaration_id).cloned());

    write!(f, [hard_line_break()])?;
    write!(
        f,
        [block_indent(&format_with(
            |f: &mut Formatter<'_, 'a, MirFormatContext<'a>>| {
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
    f: &mut Formatter<'_, 'a, MirFormatContext<'a>>,
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
    f: &mut Formatter<'_, 'a, MirFormatContext<'a>>,
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
    use_declaration: bool,
) -> FormatResult<()> {
    if use_declaration && let Some(declaration_name) = f.context().type_declaration_name(id) {
        let declaration_name = declaration_name.to_string();
        return write!(f, [copied_text(&declaration_name)]);
    }

    match ty {
        Type::Error => write!(f, [token("<error>")]),
        Type::Never => write!(f, [token("never")]),
        Type::Void => write!(f, [token("void")]),
        Type::Boolean => write!(f, [token("boolean")]),
        Type::Character => write!(f, [token("char")]),
        Type::Int {
            width,
            is_signed: signed,
        } => {
            let prefix = if *signed { "int" } else { "uint" };
            write!(f, [copied_text(&format!("{prefix}{width}"))])
        }
        Type::Isize => write!(f, [token("isize")]),
        Type::Usize => write!(f, [token("usize")]),
        Type::Float(float_type) => write!(f, [token(float_type.label())]),
        Type::TypeDescriptor => write!(f, [token("typeDescriptor")]),
        Type::TypeId => write!(f, [token("typeId")]),
        Type::Atomic { value } => {
            write!(f, [token("atomic"), token("<")])?;
            format_type_id(*value, f)?;
            write!(f, [token(">")])
        }
        Type::Dynamic {
            constraint,
            nullability,
            space: ty_space,
        } => {
            write!(f, [token("dynamic"), token("<")])?;
            format_type_id(*constraint, f)?;
            format_nullability(*nullability, f)?;
            if !ty_space.is_local() {
                write!(f, [token(","), space()])?;
                format_space_group(*ty_space, f)?;
            }
            write!(f, [token(">")])
        }
        Type::WithLifetimes { base, lifetimes } => format_type_application(*base, lifetimes, f),
        Type::Uninit { value } => {
            write!(f, [token("uninit"), token("<")])?;
            format_type_id(*value, f)?;
            write!(f, [token(">")])
        }
        Type::ManuallyDrop { value } => {
            write!(f, [token("manual"), token("<")])?;
            format_type_id(*value, f)?;
            write!(f, [token(">")])
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
                *memory_space,
                *access,
                *nullability,
                pointee,
                f,
            )?;
            write!(f, [token(">")])
        }
        Type::FixedArray {
            element,
            length,
            copy: _,
        } => {
            write!(
                f,
                [
                    token("["),
                    FormatTypeId(*element),
                    token(";"),
                    space(),
                    copied_text(&length.to_string()),
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
            write!(f, [token("slice"), token("<")])?;
            format_type_id(*element, f)?;
            format_reference_qualifiers(*kind, lifetime, *space, *access, *nullability, f)?;
            write!(f, [token(">")])
        }
        Type::Tuple { elements, copy: _ } => {
            write!(f, [token("(")])?;
            for (i, elem) in elements.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(","), space()])?;
                }
                format_type_id(*elem, f)?;
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
                    write!(f, [copied_text(field_name), token(":"), space()])?;
                    format_type_id(field.ty, f)?;
                } else {
                    format_type_id(field.ty, f)?;
                }
            }
            write!(f, [space(), token("}")])
        }
        Type::Newtype { inner, copy: _ } => {
            write!(f, [token("newtype"), token("<")])?;
            format_type_id(*inner, f)?;
            write!(f, [token(">")])
        }
        Type::Variant {
            discriminant,
            storage,
            cases,
            copy: _,
        } => {
            write!(
                f,
                [
                    token("variant"),
                    token("<"),
                    FormatTypeId(*discriminant),
                    token(","),
                    space(),
                    FormatTypeId(*storage)
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
                    [
                        &case.discriminant,
                        space(),
                        token("="),
                        space(),
                        FormatTypeId(case.ty),
                        token(";")
                    ]
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
                    FormatTypeId(*element),
                    token(","),
                    space(),
                    copied_text(&lanes.to_string()),
                    token(">")
                ]
            )
        }
        Type::Tensor {
            element,
            space: memory_space,
            shape,
            format,
            sharding,
            copy: _,
        } => {
            write!(
                f,
                [
                    token("tensor"),
                    token("<"),
                    FormatTypeId(*element),
                    token(","),
                    space()
                ]
            )?;
            format_space_group(*memory_space, f)?;
            write!(f, [token(","), space()])?;
            format_shape(shape, f)?;
            if *format != TensorFormat::dense_row_major() {
                write!(f, [token(","), space(), token("format"), token("(")])?;
                format_tensor_format(format, f)?;
                write!(f, [token(")")])?;
            }
            if sharding != &TensorSharding::unsharded() {
                write!(f, [token(","), space(), token("sharding"), token("(")])?;
                format_tensor_sharding(sharding, f)?;
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
            format,
            sharding,
            nullability,
        } => {
            write!(f, [token("tensorView"), token("<")])?;
            format_view_header(
                *kind,
                lifetime,
                *memory_space,
                *access,
                *nullability,
                element,
                f,
            )?;
            write!(f, [token(","), space()])?;
            format_shape(shape, f)?;
            if *format != TensorViewFormat::dense_row_major() {
                write!(f, [token(","), space(), token("format"), token("(")])?;
                format_tensor_view_format(format, f)?;
                write!(f, [token(")")])?;
            }
            if sharding != &TensorSharding::unsharded() {
                write!(f, [token(","), space(), token("sharding"), token("(")])?;
                format_tensor_sharding(sharding, f)?;
                write!(f, [token(")")])?;
            }
            write!(f, [token(">")])
        }
        Type::FunctionSignature {
            lifetimes,
            parameters,
            result,
        } => format_function_signature(lifetimes, parameters, *result, f),
        Type::FunctionPointer { signature } | Type::Function { signature, .. } => {
            let signature_type = f.context().tree.get(*signature);
            if let Type::FunctionSignature {
                lifetimes,
                parameters,
                result,
            } = signature_type
            {
                if matches!(ty, Type::FunctionPointer { .. }) {
                    write!(f, [token("fn")])?;
                }

                format_function_signature(lifetimes, parameters, *result, f)?;
                return Ok(());
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
        Type::Continuation {
            resume_type,
            yield_type,
            return_type,
        } => write!(
            f,
            [
                token("continuation"),
                token("<"),
                FormatTypeId(*resume_type),
                token(","),
                space(),
                FormatTypeId(*yield_type),
                token(","),
                space(),
                FormatTypeId(*return_type),
                token(">")
            ]
        ),
        Type::Waiter { value_type } => write!(
            f,
            [
                token("waiter"),
                token("<"),
                FormatTypeId(*value_type),
                token(">")
            ]
        ),
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
                write!(f, [copied_text(&value.to_string())])?;
            }
            TensorDimension::Symbol(name) => write!(f, [copied_text(name)])?,
            TensorDimension::Dynamic => write!(f, [token("dynamic")])?,
        }
    }
    write!(f, [token(")")])
}

fn format_tensor_format<'a>(
    format: &TensorFormat,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    match format {
        TensorFormat::Dense {
            order: TensorDimensionOrder::RowMajor,
        } => write!(
            f,
            [token("dense"), token("("), token("rowMajor"), token(")")]
        ),
        TensorFormat::Dense {
            order: TensorDimensionOrder::ColumnMajor,
        } => write!(
            f,
            [token("dense"), token("("), token("columnMajor"), token(")")]
        ),
    }
}

fn format_tensor_view_format<'a>(
    format: &TensorViewFormat,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    match format {
        TensorViewFormat::Dense {
            order: TensorDimensionOrder::RowMajor,
        } => write!(
            f,
            [token("dense"), token("("), token("rowMajor"), token(")")]
        ),
        TensorViewFormat::Dense {
            order: TensorDimensionOrder::ColumnMajor,
        } => write!(
            f,
            [token("dense"), token("("), token("columnMajor"), token(")")]
        ),
        TensorViewFormat::Strided => write!(f, [token("strided")]),
    }
}

fn format_tensor_sharding<'a>(
    sharding: &TensorSharding,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    match sharding {
        TensorSharding::Unsharded => write!(f, [token("unsharded")]),
        TensorSharding::Sharding { axes } => {
            for (index, axis) in axes.iter().enumerate() {
                if index > 0 {
                    write!(f, [token(","), space()])?;
                }
                format_tensor_sharding_axis(axis, f)?;
            }

            Ok(())
        }
    }
}

fn format_tensor_sharding_axis<'a>(
    axis: &TensorShardingAxis,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    match axis {
        TensorShardingAxis::Shard { axis } => {
            write!(
                f,
                [
                    token("shard"),
                    token("("),
                    copied_text(&axis.to_string()),
                    token(")")
                ]
            )
        }
        TensorShardingAxis::Replicate => write!(f, [token("replicate")]),
        TensorShardingAxis::Partial { reduction } => {
            write!(f, [token("partial"), token("(")])?;
            format_tensor_reduction(*reduction, f)?;
            write!(f, [token(")")])
        }
    }
}

fn format_tensor_reduction<'a>(
    reduction: TensorReduction,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let name = match reduction {
        TensorReduction::Add => "add",
        TensorReduction::Multiply => "multiply",
        TensorReduction::Minimum => "minimum",
        TensorReduction::Maximum => "maximum",
        TensorReduction::And => "and",
        TensorReduction::Or => "or",
    };

    write!(f, [token(name)])
}

fn format_view_header<'a>(
    kind: ReferenceKind,
    lifetime: &Lifetime,
    memory_space: Space,
    access: Access,
    nullability: Nullability,
    element: &TypeId,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    format_type_id(*element, f)?;
    format_reference_qualifiers(kind, lifetime, memory_space, access, nullability, f)
}

/// Format one explicit memory-space group.
fn format_space_group<'a>(memory_space: Space, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    write!(
        f,
        [
            token("space"),
            token("("),
            token(memory_space.label()),
            token(")")
        ]
    )
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
        write!(f, [token(","), space()])?;
        format_space_group(memory_space, f)?;
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

    write!(f, [token(","), space()])?;
    format_lifetime_terms(lifetime, f)
}

fn format_type_application<'a>(
    base: TypeId,
    lifetimes: &[Lifetime],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    format_type_id(base, f)?;
    write!(f, [token("<")])?;
    for (index, lifetime) in lifetimes.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        format_lifetime_terms(lifetime, f)?;
    }
    write!(f, [token(">")])
}

pub(super) fn format_signature_parameter<'a>(
    parameter: &crate::SignatureParameter,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    format_type_id(parameter.ty, f)
}

pub(super) fn format_function_signature<'a>(
    lifetimes: &[LifetimeParameter],
    parameters: &[crate::SignatureParameter],
    result: TypeId,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    let previous_lifetimes =
        std::mem::replace(&mut f.context_mut().current_lifetimes, lifetimes.to_vec());
    format_lifetimes(lifetimes, f)?;

    write!(f, [token("(")])?;
    for (index, parameter) in parameters.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }
        format_signature_parameter(parameter, f)?;
    }
    write!(f, [token(")"), space(), token("=>"), space()])?;
    format_type_id(result, f)?;
    super::function::format_lifetime_where(lifetimes, f)?;

    f.context_mut().current_lifetimes = previous_lifetimes;

    Ok(())
}

pub(super) fn format_borrow_obligations<'a>(
    obligations: &[BorrowObligation],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    for obligation in obligations {
        match obligation {
            BorrowObligation::SuspensionStable { lifetime } => {
                write!(
                    f,
                    [space(), token("@"), token("suspensionSafe"), token("(")]
                )?;
                format_lifetime_terms(lifetime, f)?;
                write!(f, [token(")")])?;
            }
        }
    }

    Ok(())
}

pub(super) fn format_lifetime_terms<'a>(
    lifetime: &Lifetime,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    for (index, source) in lifetime.terms.iter().enumerate() {
        if index > 0 {
            write!(f, [space(), token("|"), space()])?;
        }

        match source {
            LifetimeTerm::Static => write!(f, [token("'static")])?,
            LifetimeTerm::Slot(index) => {
                if let Some(name) = f.context().lifetime_name(*index).map(str::to_string) {
                    write!(f, [copied_text(&name)])?;
                } else {
                    write!(f, [copied_text(&format!("'l{}", index.0))])?;
                }
            }
        }
    }

    Ok(())
}

/// Format a declaration lifetime header.
fn format_lifetimes<'a>(
    lifetimes: &[LifetimeParameter],
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()> {
    if lifetimes.is_empty() {
        return Ok(());
    }

    write!(f, [token("<")])?;
    for (index, lifetime) in lifetimes.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        let name = lifetime
            .name
            .map(|name| f.context().strings.get(name).to_string())
            .unwrap_or_else(|| format!("'l{index}"));
        write!(f, [copied_text(&name)])?;
    }

    write!(f, [token(">")])
}

fn format_access<'a>(access: Access, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    match access {
        Access::Readonly => write!(f, [token(","), space(), token("readonly")]),
        Access::Mutable => write!(f, [token(","), space(), token("mutable")]),
        Access::Exclusive => write!(f, [token(","), space(), token("exclusive")]),
    }
}

impl<'a> FormatMirNode<'a, TypeDeclaration> for TypeDeclaration {
    fn format_node(
        &self,
        id: LocalNodeId<TypeDeclaration>,
        f: &mut MirFormatter<'a, '_>,
    ) -> FormatResult<()> {
        let attributes = f.context().tree.attributes(id);
        let name = f
            .context()
            .type_declaration_name(self.ty)
            .unwrap_or_else(|| f.context().strings.get(self.name))
            .to_string();
        let type_id = self.ty;
        let ty = f.context().tree.get(type_id);
        format_type_declaration(&name, attributes, Some(id), type_id, ty, f)
    }
}

fn format_struct_field<'a>(field: &Field, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    if let Some(name) = field.name {
        let field_name = f.context().strings.get(name);
        write!(f, [copied_text(field_name), token(":"), space()])?;
        format_type_id(field.ty, f)?;
        write!(f, [token(";")])
    } else {
        format_type_id(field.ty, f)?;
        write!(f, [token(";")])
    }
}

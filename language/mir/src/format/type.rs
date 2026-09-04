use destack_fir::format::{Allocator, Format, FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use super::attribute::{write_attributes, write_attributes_before_anchor, write_inline_attributes};
use super::r#static::format_static;
use super::value::{format_function_id, format_type_id};

use crate::{
    Access, Attribute, AttributeIdentifier, Copy, Field, FieldSpan, FormatNode, Formatter,
    FunctionId, Lifetime, LifetimeParameter, LifetimeTerm, LocalNodeId, Nullability, ReferenceKind,
    SignatureParameter, StaticId, Storage, Type, TypeDeclaration, TypeDeclarationSpans,
    TypeHeritage, TypeId, Writer, write_comments_before,
};

impl FormatNode for Type {
    fn format_node<'a>(&self, id: LocalNodeId<Type>, f: &mut Writer<'a, '_>) -> FormatResult<()> {
        format_type_maybe_named(f, id, self, true)
    }
}

/// Formatter adapter for one nested type reference.
struct FormatTypeId(TypeId);

impl<'a> Format<'a, Formatter<'a>> for FormatTypeId {
    fn format(&self, f: &mut Writer<'a, '_>) -> FormatResult<()> {
        format_type_id(self.0, f)
    }
}

/// Formatter adapter for one function reference.
struct FormatFunctionId(FunctionId);

impl<'a> Format<'a, Formatter<'a>> for FormatFunctionId {
    fn format(&self, f: &mut Writer<'a, '_>) -> FormatResult<()> {
        format_function_id(self.0, f)
    }
}

impl Formatter<'_> {
    /// Format one function reference, its name with its instance arguments.
    pub fn format_function(&self, function: FunctionId) -> FormatResult<String> {
        let allocator = Allocator::default();
        let formatter = Formatter::new(self.tree, self.target_layout, self.strings, self.options);

        // build the FIR document from the reference
        let document = destack_fir::format!(&allocator, formatter, [FormatFunctionId(function)])?;

        // print the complete function reference
        let printed = document.print()?;

        Ok(printed.as_str().to_string())
    }

    /// Format one type reference.
    pub fn format_type(&self, ty: TypeId) -> FormatResult<String> {
        let allocator = Allocator::default();
        let formatter = Formatter::new(self.tree, self.target_layout, self.strings, self.options);

        // build the FIR document from the type
        let document = destack_fir::format!(&allocator, formatter, [FormatTypeId(ty)])?;

        // print the complete type reference
        let printed = document.print()?;

        Ok(printed.as_str().to_string())
    }
}

/// Format one type in its expanded form, ignoring any named declaration.
pub(super) fn format_type_expanded<'a>(
    f: &mut Writer<'a, '_>,
    id: LocalNodeId<Type>,
    ty: &Type,
) -> FormatResult<()> {
    format_type_maybe_named(f, id, ty, false)
}

/// Format one named type declaration with its attributes and body.
pub(super) fn format_type_declaration<'a>(
    name: &str,
    attributes: &[Attribute],
    declaration_id: Option<LocalNodeId<TypeDeclaration>>,
    type_id: LocalNodeId<Type>,
    ty: &Type,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    let attributes = declaration_id
        .map(|declaration_id| f.context().tree.attributes(declaration_id))
        .unwrap_or(attributes);
    let lifetimes = declaration_id
        .map(|declaration_id| f.context().tree.get(declaration_id).lifetimes.clone())
        .unwrap_or_default();
    let arguments = declaration_id
        .map(|declaration_id| f.context().tree.get(declaration_id).arguments.clone())
        .unwrap_or_default();
    let heritage = declaration_id
        .map(|declaration_id| f.context().tree.get(declaration_id).heritage.clone())
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

    let previous_lifetimes = f.context_mut().replace_lifetimes(lifetimes.clone());
    let result = match ty {
        Type::Struct { fields, .. } => format_struct_type_declaration(
            name,
            &arguments,
            declaration_id,
            &lifetimes,
            &heritage,
            fields,
            f,
        ),
        _ => {
            write!(f, [token("type"), space()])?;
            format_type_name(name, &arguments, &lifetimes, f)?;
            format_type_heritage(&heritage, f)?;
            write!(f, [space(), token("="), space()])?;
            format_type_expanded(f, type_id, ty)?;
            write!(f, [token(";")])
        }
    };
    f.context_mut().replace_lifetimes(previous_lifetimes);

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
        | Type::Vector { copy, .. } => Some(*copy),
        _ => None,
    }
}

/// Return whether attributes already include an explicit copy attribute.
fn has_copy_attribute(attributes: &[Attribute], f: &Writer<'_, '_>) -> bool {
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
    arguments: &[StaticId],
    declaration_id: Option<LocalNodeId<TypeDeclaration>>,
    lifetimes: &[LifetimeParameter],
    heritage: &TypeHeritage,
    fields: &[LocalNodeId<Field>],
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token("type"), space()])?;
    format_type_name(name, arguments, lifetimes, f)?;
    format_type_heritage(heritage, f)?;
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
        [block_indent(&format_with(|f: &mut Writer<'a, '_>| {
            format_struct_fields(fields, &field_spans, declaration_spans.as_ref(), f)
        }))]
    )?;
    write!(f, [hard_line_break(), token("}")])
}

/// Format direct nominal heritage after one declared type name.
fn format_type_heritage<'a>(heritage: &TypeHeritage, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    // inherited types
    if !heritage.extends.is_empty() {
        write!(f, [space(), token("extends"), space()])?;
        for (index, base) in heritage.extends.iter().enumerate() {
            if index > 0 {
                write!(f, [token(","), space()])?;
            }
            format_type_id(*base, f)?;
        }
    }

    // implemented interfaces
    if !heritage.implements.is_empty() {
        write!(f, [space(), token("implements"), space()])?;
        for (index, interface) in heritage.implements.iter().enumerate() {
            if index > 0 {
                write!(f, [token(","), space()])?;
            }
            format_type_id(*interface, f)?;
        }
    }

    Ok(())
}

/// Format the fields of one struct type declaration.
fn format_struct_fields<'a>(
    field_ids: &[LocalNodeId<Field>],
    field_spans: &[FieldSpan],
    declaration_spans: Option<&TypeDeclarationSpans>,
    f: &mut Writer<'a, '_>,
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
    f: &mut Writer<'a, '_>,
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

/// Format one type, printing its declared name when one is available.
fn format_type_maybe_named<'a>(
    f: &mut Writer<'a, '_>,
    id: LocalNodeId<Type>,
    ty: &Type,
    use_declaration: bool,
) -> FormatResult<()> {
    let tree = f.context().tree;
    if use_declaration && let Some(declaration_id) = tree.type_declaration(id) {
        let declaration = tree.get(declaration_id);
        let name = f.context().strings.get(declaration.name).to_string();

        return format_type_name(&name, &declaration.arguments, &[], f);
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
            kind,
            lifetime,
            constraint,
            storage,
            access,
            nullability,
        } => {
            write!(f, [token("dynamic"), token("<")])?;
            format_type_id(*constraint, f)?;
            format_reference_qualifiers(*kind, lifetime, *storage, *access, *nullability, f)?;
            write!(f, [token(">")])
        }
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
            storage,
            access,
            pointee,
            nullability,
        } => {
            write!(f, [token("ref"), token("<")])?;
            format_view_header(*kind, lifetime, *storage, *access, *nullability, pointee, f)?;
            write!(f, [token(">")])
        }
        Type::Pointer {
            pointee,
            access,
            nullability,
        } => {
            write!(f, [token("ptr"), token("<")])?;
            format_type_id(*pointee, f)?;
            format_access(*access, f)?;
            format_nullability(*nullability, f)?;
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
            storage,
            access,
            nullability,
        } => {
            write!(f, [token("slice"), token("<")])?;
            format_type_id(*element, f)?;
            format_reference_qualifiers(*kind, lifetime, *storage, *access, *nullability, f)?;
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
            cases,
            copy: _,
        } => {
            write!(
                f,
                [token("variant"), token("<"), FormatTypeId(*discriminant)]
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
        Type::FunctionSignature {
            lifetimes,
            parameters,
            result,
        } => format_function_signature(lifetimes, parameters, *result, f),
        Type::FunctionPointer { signature } => {
            let signature_type = f.context().tree.get(*signature);
            let Type::FunctionSignature {
                lifetimes,
                parameters,
                result,
            } = signature_type
            else {
                return Err(FormatError::SyntaxError {
                    message: "MIR function pointer does not reference a function signature",
                });
            };

            write!(f, [token("fn")])?;
            format_function_signature(lifetimes, parameters, *result, f)
        }
        Type::Function {
            multiplicity,
            kind,
            lifetime,
            signature,
            storage,
            access,
            nullability,
        } => {
            write!(f, [token("function"), token("<")])?;
            format_type_id(*signature, f)?;
            write!(f, [token(","), space(), token(multiplicity.name())])?;
            format_reference_qualifiers(*kind, lifetime, *storage, *access, *nullability, f)?;
            write!(f, [token(">")])
        }
        Type::Application { base, lifetimes } => format_type_application(*base, lifetimes, f),
    }
}

/// Format one fat descriptor's element followed by its reference qualifiers.
fn format_view_header<'a>(
    kind: ReferenceKind,
    lifetime: &Lifetime,
    storage: Storage,
    access: Access,
    nullability: Nullability,
    element: &TypeId,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    format_type_id(*element, f)?;
    format_reference_qualifiers(kind, lifetime, storage, access, nullability, f)
}

/// Format the kind, lifetime, access, nullability, and storage of one reference.
fn format_reference_qualifiers<'a>(
    kind: ReferenceKind,
    lifetime: &Lifetime,
    storage: Storage,
    access: Access,
    nullability: Nullability,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    let kind_token = match kind {
        ReferenceKind::Managed => "managed",
        ReferenceKind::Unique => "unique",
        ReferenceKind::Borrowed => "borrowed",
    };

    write!(f, [token(","), space(), token(kind_token)])?;
    format_lifetime(lifetime, f)?;
    format_access(access, f)?;
    format_nullability(nullability, f)?;
    write!(f, [token(","), space()])?;
    format_storage(storage, f)?;

    Ok(())
}

/// Format one reference storage qualifier.
fn format_storage<'a>(storage: Storage, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    match storage {
        Storage::SharedStatic => write!(f, [token("shared"), space(), token("static")]),
        _ => write!(f, [token(storage.label())]),
    }
}

/// Format one nullability qualifier.
fn format_nullability<'a>(nullability: Nullability, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    match nullability {
        Nullability::None => Ok(()),
        Nullability::Null => write!(f, [token(","), space(), token("nullable")]),
        Nullability::Undefined => write!(f, [token(","), space(), token("undefined")]),
        Nullability::NullOrUndefined => write!(f, [token(","), space(), token("nullish")]),
    }
}

/// Format one reference lifetime qualifier.
fn format_lifetime<'a>(lifetime: &Lifetime, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    if lifetime.is_empty() {
        return Ok(());
    }

    write!(f, [token(","), space()])?;
    format_lifetime_terms(lifetime, f)
}

/// Format one type use with its applied lifetime arguments.
fn format_type_application<'a>(
    base: TypeId,
    lifetimes: &[Lifetime],
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let declaration = tree.type_declaration(base).map(|id| tree.get(id));
    let arguments = declaration
        .as_ref()
        .map(|declaration| declaration.arguments.as_slice())
        .unwrap_or_default();
    if let Some(declaration) = declaration.as_ref() {
        let name = f.context().strings.get(declaration.name).to_string();
        write!(f, [copied_text(&name)])?;
    } else {
        format_type_id(base, f)?;
    }

    write!(f, [token("<")])?;
    let mut written = 0;
    for argument in arguments {
        if written > 0 {
            write!(f, [token(","), space()])?;
        }
        written += 1;

        format_static(*argument, f)?;
    }

    // erased lifetimes elide from the application
    for lifetime in lifetimes {
        if lifetime.is_empty() {
            continue;
        }
        if written > 0 {
            write!(f, [token(","), space()])?;
        }
        written += 1;

        format_lifetime_terms(lifetime, f)?;
    }
    write!(f, [token(">")])
}

/// Format one function signature parameter.
pub(super) fn format_signature_parameter<'a>(
    parameter: &SignatureParameter,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    format_type_id(parameter.ty, f)
}

/// Format one function signature's lifetimes, parameters, and result.
pub(super) fn format_function_signature<'a>(
    lifetimes: &[LifetimeParameter],
    parameters: &[SignatureParameter],
    result: TypeId,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    let previous_lifetimes = f.context_mut().replace_lifetimes(lifetimes.to_vec());
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

    f.context_mut().replace_lifetimes(previous_lifetimes);

    Ok(())
}

/// Format the terms of one lifetime as a union.
pub(super) fn format_lifetime_terms<'a>(
    lifetime: &Lifetime,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    for (index, source) in lifetime.terms.iter().enumerate() {
        if index > 0 {
            write!(f, [space(), token("|"), space()])?;
        }

        match source {
            LifetimeTerm::Static => write!(f, [token("'static")])?,
            LifetimeTerm::Frame => write!(f, [token("'frame")])?,
            LifetimeTerm::Managed => write!(f, [token("'managed")])?,
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
    f: &mut Writer<'a, '_>,
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

/// Format one declared type's concrete generic arguments and lifetime terms.
pub(super) fn format_type_name<'a>(
    name: &str,
    arguments: &[StaticId],
    lifetimes: &[LifetimeParameter],
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    write!(f, [copied_text(name)])?;

    if arguments.is_empty() && lifetimes.is_empty() {
        return Ok(());
    }

    write!(f, [token("<")])?;
    for (index, argument) in arguments.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        format_static(*argument, f)?;
    }

    for (index, lifetime) in lifetimes.iter().enumerate() {
        if index > 0 || !arguments.is_empty() {
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

/// Format one reference access qualifier.
fn format_access<'a>(access: Access, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    match access {
        Access::Readonly => write!(f, [token(","), space(), token("readonly")]),
        Access::Mutable => write!(f, [token(","), space(), token("mutable")]),
        Access::Exclusive => write!(f, [token(","), space(), token("exclusive")]),
    }
}

impl FormatNode for TypeDeclaration {
    fn format_node<'a>(
        &self,
        id: LocalNodeId<TypeDeclaration>,
        f: &mut Writer<'a, '_>,
    ) -> FormatResult<()> {
        let attributes = f.context().tree.attributes(id);
        let name = f.context().strings.get(self.name).to_string();
        let type_id = self.ty;
        let ty = f.context().tree.get(type_id);
        format_type_declaration(&name, attributes, Some(id), type_id, ty, f)
    }
}

/// Format one struct field's name and type.
fn format_struct_field<'a>(field: &Field, f: &mut Writer<'a, '_>) -> FormatResult<()> {
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

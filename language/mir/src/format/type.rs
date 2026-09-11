use destack_fir::format::{Allocator, Format, FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use super::attribute::{write_attributes, write_attributes_before_anchor, write_inline_attributes};
use super::r#static::format_static;
use super::value::{format_function_id, format_type_id};

use crate::{
    Access, Attribute, AttributeIdentifier, Copy, Extent, Field, FieldSpan, FormatNode, Formatter,
    FunctionId, GenericArgument, GenericParameter, GenericParameterDomain, Lifetime,
    LifetimeParameter, LocalNodeId, Reference, RegionBound, SignatureParameter, Space, SpaceJoinId,
    Storage, Type, TypeDeclaration, TypeDeclarationSpans, TypeHeritage, TypeId, Writer,
    write_comments_before,
};

impl FormatNode for Type {
    fn format_node<'a>(&self, id: LocalNodeId<Type>, f: &mut Writer<'a, '_>) -> FormatResult<()> {
        format_type_id(id, f)
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

    /// Format one type reference inside a function's lifetime and generic scope.
    pub fn format_type_in(&self, function: FunctionId, ty: TypeId) -> FormatResult<String> {
        let allocator = Allocator::default();
        let mut formatter =
            Formatter::new(self.tree, self.target_layout, self.strings, self.options);
        formatter.enter_function(function);

        // build the FIR document from the type
        let document = destack_fir::format!(&allocator, formatter, [FormatTypeId(ty)])?;

        // print the complete type reference
        let printed = document.print()?;

        Ok(printed.as_str().to_string())
    }
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
    let generics = declaration_id
        .map(|declaration_id| f.context().tree.get(declaration_id).generics.clone())
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

    // format the definition under the declaration's generic scope
    let previous_generics = f.context_mut().replace_generics(generics.clone());
    let is_imported = f
        .context()
        .selection()
        .is_some_and(|selection| declaration_id.is_some_and(|id| selection.imported.contains(&id)));
    let is_opaque = is_imported || !f.context().tree.is_defined_type(TypeId::from(type_id));
    let result = match ty {
        // print an opaque declaration without a definition
        _ if is_opaque => {
            write!(f, [token("type"), space()])?;
            format_type_name(name, &generics, f)?;
            write!(f, [token(";")])
        }
        Type::Struct { fields, .. } => {
            format_struct_type_declaration(name, &generics, declaration_id, &heritage, fields, f)
        }
        _ => {
            write!(f, [token("type"), space()])?;
            format_type_name(name, &generics, f)?;
            format_type_heritage(&heritage, f)?;
            write!(f, [space(), token("="), space()])?;
            format_type_expanded(f, type_id, ty)?;
            write!(f, [token(";")])
        }
    };
    f.context_mut().replace_generics(previous_generics);

    result
}

/// Return the explicit copy property carried by one aggregate type.
fn type_copy(ty: &Type) -> Option<Copy> {
    match ty {
        Type::Struct { copy, .. } | Type::Newtype { copy, .. } | Type::Variant { copy, .. } => {
            Some(*copy)
        }
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
    generics: &[GenericParameter],
    declaration_id: Option<LocalNodeId<TypeDeclaration>>,
    heritage: &TypeHeritage,
    fields: &[LocalNodeId<Field>],
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    write!(f, [token("type"), space()])?;
    format_type_name(name, generics, f)?;
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

/// Format one structural type or declared reference.
pub(super) fn format_type_expanded<'a>(
    f: &mut Writer<'a, '_>,
    id: LocalNodeId<Type>,
    ty: &Type,
) -> FormatResult<()> {
    let tree = f.context().tree;
    match ty {
        Type::Declaration { declaration } => {
            let declaration = tree.get(*declaration);
            if let Some(name) = declaration.name {
                let name = f.context().strings.get(name).to_string();

                write!(f, [copied_text(&name)])
            } else if let Some(definition) = declaration.definition {
                format_type_id(definition, f)
            } else {
                write!(f, [copied_text(&format!("type@{}", id.id))])
            }
        }
        Type::Error => write!(f, [token("<error>")]),
        Type::Never => write!(f, [token("never")]),
        Type::Void => write!(f, [token("void")]),
        Type::Null => write!(f, [token("null")]),
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
        Type::Parameter { index } => format_parameter(*index, f),
        Type::TypeId => write!(f, [token("typeId")]),
        Type::Dynamic {
            kind,
            lifetime,
            constraint,
            storage,
            access,
        } => {
            write!(f, [token("dynamic"), token("<")])?;
            format_type_id(*constraint, f)?;
            format_reference_qualifiers(*kind, lifetime, *storage, *access, f)?;
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
        } => {
            write!(f, [token("ref"), token("<")])?;
            format_view_header(*kind, lifetime, *storage, *access, pointee, f)?;
            write!(f, [token(">")])
        }
        Type::Pointer { pointee, access } => {
            write!(f, [token("ptr"), token("<")])?;
            format_type_id(*pointee, f)?;
            format_access(*access, f)?;
            write!(f, [token(">")])
        }
        Type::FixedArray { element, length } => {
            write!(
                f,
                [
                    token("["),
                    FormatTypeId(*element),
                    token(";"),
                    space(),
                    format_with(|f| format_static(*length, f)),
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
        } => {
            write!(f, [token("slice"), token("<")])?;
            format_type_id(*element, f)?;
            format_reference_qualifiers(*kind, lifetime, *storage, *access, f)?;
            write!(f, [token(">")])
        }
        Type::Tuple { elements } => {
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
                write!(f, [&case.discriminant, space(), token("="), space()])?;
                write!(f, [FormatTypeId(case.ty), token(";")])?;
            }
            if !cases.is_empty() {
                write!(f, [space()])?;
            }
            write!(f, [token("}")])
        }
        Type::Vector { element, lanes } => {
            write!(
                f,
                [
                    token("vector"),
                    token("<"),
                    FormatTypeId(*element),
                    token(","),
                    space(),
                    format_with(|f| format_static(*lanes, f)),
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
        } => {
            write!(f, [token("function"), token("<")])?;
            format_type_id(*signature, f)?;
            write!(f, [token(","), space(), token(multiplicity.name())])?;
            format_reference_qualifiers(*kind, lifetime, *storage, *access, f)?;
            write!(f, [token(">")])
        }
        Type::Application { base, arguments } => format_type_application(*base, arguments, f),
    }
}

/// Format one fat descriptor's element followed by its reference qualifiers.
fn format_view_header<'a>(
    kind: Reference,
    lifetime: &Lifetime,
    storage: Storage,
    access: Access,
    element: &TypeId,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    format_type_id(*element, f)?;
    format_reference_qualifiers(kind, lifetime, storage, access, f)
}

/// Format the kind, lifetime, access, and storage of one reference.
fn format_reference_qualifiers<'a>(
    kind: Reference,
    lifetime: &Lifetime,
    storage: Storage,
    access: Access,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    let kind_token = match kind {
        Reference::Managed => "managed",
        Reference::Unique => "unique",
        Reference::Borrowed => "borrowed",
        Reference::Raw => "raw",
    };

    write!(f, [token(","), space(), token(kind_token)])?;
    if matches!(kind, Reference::Borrowed) {
        write!(f, [token(","), space()])?;
        format_region_argument(lifetime, storage, f)?;
    } else {
        format_lifetime(lifetime, f)?;
    }
    format_access(access, f)?;

    if matches!(kind, Reference::Borrowed) {
        return Ok(());
    }
    write!(f, [token(","), space()])?;
    format_storage(storage, f)?;

    Ok(())
}

/// Format one reference storage qualifier.
pub(super) fn format_storage<'a>(storage: Storage, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    match storage {
        Storage::Frame => write!(f, [token("frame")]),
        Storage::Parameter(index) => format_parameter(index, f),
        Storage::Bound(bound) => format_extents(&Lifetime::new([Extent::Bound(bound)]), f),
        Storage::Join(id) => {
            let members = f.context().tree.storage_join(id).to_vec();
            for (index, storage) in members.into_iter().enumerate() {
                if index > 0 {
                    write!(f, [token("|")])?;
                }
                format_storage(storage, f)?;
            }

            Ok(())
        }
        Storage::Heap(storage_space @ (Space::Parameter(_) | Space::Bound(_) | Space::Join(_))) => {
            write!(f, [token("heap"), token("(")])?;
            format_space(storage_space, f)?;
            write!(f, [token(")")])
        }
        Storage::Heap(space) => format_space(space, f),
        Storage::Static(Space::Local) => write!(f, [token("static")]),
        Storage::Static(Space::Constant) => write!(f, [token("constant")]),
        Storage::Static(Space::Shared) => write!(f, [token("shared"), space(), token("static")]),
        Storage::Static(storage_space) => {
            write!(f, [token("static"), token("(")])?;
            format_space(storage_space, f)?;
            write!(f, [token(")")])
        }
    }
}

/// Format one space name.
pub(super) fn format_space<'a>(space: Space, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    match space.label() {
        Some(label) => write!(f, [token(label)]),
        None => match space {
            Space::Parameter(index) => format_parameter(index, f),
            Space::Bound(slot) => format_extents(&Lifetime::new([Extent::Bound(slot)]), f),
            Space::Join(id) => format_space_join(id, f),
            space => unreachable!("closed space {space:?} without a label"),
        },
    }
}

/// Format one space join as its spaces separated by pipes.
fn format_space_join<'a>(id: SpaceJoinId, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    let spaces = f.context().tree.space_join(id).to_vec();
    for (index, space) in spaces.into_iter().enumerate() {
        if index > 0 {
            write!(f, [token("|")])?;
        }
        format_space(space, f)?;
    }

    Ok(())
}

/// Format one generic argument.
pub(super) fn format_generic_argument<'a>(
    argument: &GenericArgument,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    match argument {
        GenericArgument::Type(ty) => format_type_id(*ty, f),
        GenericArgument::Region { lifetime, storage } => {
            format_region_argument(lifetime, *storage, f)
        }
        GenericArgument::Space(space) => format_space(*space, f),
        GenericArgument::Access(access) => format_access_name(*access, f),
        GenericArgument::Value(value) => format_static(*value, f),
    }
}

/// Format a complete region, abbreviated when one parameter supplies lifetime and storage.
fn format_region_argument<'a>(
    lifetime: &Lifetime,
    storage: Storage,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    // print an erased extent as the wildcard
    if lifetime.is_empty() {
        write!(f, [token("'_")])?;
    } else {
        format_extents(lifetime, f)?;
    }

    // one region parameter or slot names both coordinates
    if let ([Extent::Parameter(index)], Storage::Parameter(parameter)) =
        (lifetime.extents.as_slice(), storage)
        && *index == parameter
    {
        return Ok(());
    }
    if let ([Extent::Bound(slot)], Storage::Bound(space)) = (lifetime.extents.as_slice(), storage)
        && *slot == space
    {
        return Ok(());
    }

    write!(f, [space(), token("&"), space()])?;
    format_storage(storage, f)
}

/// Format one generic argument list, elided when empty.
pub(super) fn format_generic_arguments<'a>(
    arguments: &[GenericArgument],
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    if arguments.is_empty() {
        return Ok(());
    }

    write!(f, [token("<")])?;
    for (index, argument) in arguments.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }
        format_generic_argument(argument, f)?;
    }

    write!(f, [token(">")])
}

/// Format one reference lifetime qualifier.
fn format_lifetime<'a>(lifetime: &Lifetime, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    if lifetime.is_empty() {
        return Ok(());
    }

    write!(f, [token(","), space()])?;
    format_extents(lifetime, f)
}

/// Format one type use with its applied generic arguments.
fn format_type_application<'a>(
    base: TypeId,
    arguments: &[GenericArgument],
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    format_type_id(base, f)?;

    write!(f, [token("<")])?;
    for (index, argument) in arguments.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }
        format_generic_argument(argument, f)?;
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
    f.context_mut().push_lifetimes(lifetimes.to_vec());
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

    f.context_mut().pop_lifetimes();

    Ok(())
}

/// Format the terms of one lifetime as a union.
pub(super) fn format_extents<'a>(lifetime: &Lifetime, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    for (index, source) in lifetime.extents.iter().enumerate() {
        if index > 0 {
            write!(f, [space(), token("|"), space()])?;
        }

        match source {
            Extent::Static => write!(f, [token("'static")])?,
            Extent::Frame => write!(f, [token("'frame")])?,
            Extent::Managed => write!(f, [token("'managed")])?,
            Extent::Bound(index) => {
                if let Some(name) = f.context().lifetime_name(*index).map(str::to_string) {
                    write!(f, [copied_text(&name)])?;
                } else {
                    write!(f, [copied_text(&format!("'l{}", index.index))])?;
                }
            }
            Extent::Parameter(index) => {
                let name = f
                    .context()
                    .parameter_name(*index)
                    .unwrap_or_else(|| {
                        unreachable!("region parameter {index} outside its declaration")
                    })
                    .to_string();
                write!(f, [copied_text(&name)])?;
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
    for index in 0..lifetimes.len() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        let name = f
            .context()
            .lifetime_name(RegionBound::new(index as u32))
            .expect("a declared lifetime has a display name")
            .to_string();
        write!(f, [copied_text(&name)])?;
    }

    write!(f, [token(">")])
}

/// Format one declaration name with its generic and lifetime parameters.
pub(super) fn format_type_name<'a>(
    name: &str,
    generics: &[GenericParameter],
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    write!(f, [copied_text(name)])?;

    if generics.is_empty() {
        return Ok(());
    }

    write!(f, [token("<")])?;
    for (index, parameter) in generics.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }
        format_generic_parameter(parameter, f)?;
    }

    write!(f, [token(">")])
}

/// Format one generic parameter declaration with its domain, name, and bounds.
pub(super) fn format_generic_parameter<'a>(
    parameter: &GenericParameter,
    f: &mut Writer<'a, '_>,
) -> FormatResult<()> {
    let name = f.context().strings.get(parameter.name).to_string();
    match &parameter.domain {
        GenericParameterDomain::Type { bounds } => {
            write!(f, [copied_text(&name)])?;
            for (position, bound) in bounds.iter().enumerate() {
                match position {
                    0 => write!(f, [token(":"), space()])?,
                    _ => write!(f, [space(), token("&"), space()])?,
                }
                format_type_id(*bound, f)?;
            }

            Ok(())
        }
        GenericParameterDomain::Region { outlives } => {
            write!(f, [copied_text(&name)])?;
            for (position, outlived) in outlives.iter().enumerate() {
                match position {
                    0 => write!(f, [token(":"), space()])?,
                    _ => write!(f, [space(), token("&"), space()])?,
                }
                let outlived = f
                    .context()
                    .parameter_name(*outlived)
                    .unwrap_or_else(|| {
                        unreachable!("region parameter {outlived} outside its declaration")
                    })
                    .to_string();
                write!(f, [copied_text(&outlived)])?;
            }

            Ok(())
        }
        GenericParameterDomain::Space => write!(f, [token("space"), space(), copied_text(&name)]),
        GenericParameterDomain::Access => write!(f, [token("access"), space(), copied_text(&name)]),
        GenericParameterDomain::Value { ty } => {
            write!(
                f,
                [
                    token("const"),
                    space(),
                    copied_text(&name),
                    token(":"),
                    space()
                ]
            )?;
            format_type_id(*ty, f)
        }
    }
}

/// Format one generic parameter by its name in the enclosing scope.
pub(super) fn format_parameter<'a>(index: u32, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    let Some(name) = f.context().parameter_name(index).map(str::to_string) else {
        return Err(FormatError::SyntaxError {
            message: "MIR generic parameter formatted outside its template",
        });
    };

    write!(f, [copied_text(&name)])
}

/// Format one reference access qualifier.
fn format_access<'a>(access: Access, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    write!(f, [token(","), space()])?;
    format_access_name(access, f)
}

/// Format one access name.
fn format_access_name<'a>(access: Access, f: &mut Writer<'a, '_>) -> FormatResult<()> {
    match access.label() {
        Some(label) => write!(f, [token(label)]),
        None => match access {
            Access::Parameter(index) => format_parameter(index, f),
            access => unreachable!("closed access {access:?} without a label"),
        },
    }
}

impl FormatNode for TypeDeclaration {
    fn format_node<'a>(
        &self,
        id: LocalNodeId<TypeDeclaration>,
        f: &mut Writer<'a, '_>,
    ) -> FormatResult<()> {
        let attributes = f.context().tree.attributes(id);
        let Some(name) = self.name else {
            return Err(FormatError::SyntaxError {
                message: "anonymous type has no source declaration",
            });
        };
        let name = f.context().strings.get(name).to_string();
        let type_id = f
            .context()
            .tree
            .identified_type(self.symbol)
            .expect("a declaration has an interned type");
        let ty = f.context().tree.get(self.definition.unwrap_or(type_id));
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

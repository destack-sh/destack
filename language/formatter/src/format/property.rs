use crate::argument::list_like;
use crate::directive::{
    FormatterDirectiveKind, FormatterDirectivePosition, collect_ignore_ranges_for_nodes,
    directive_for_node, ignored_node_source, write_ignored_span,
};
use crate::key::{format_key_with_quote_policy, is_identifier_for_quotes};
use crate::signature::{
    FunctionHeaderStyle, signature_parameters_should_expand,
    signature_return_type_has_line_postfix_boundary_annotation, signature_return_type_is_multiline,
    signature_should_elide_space_before_body, single_parameter_should_hug,
    write_function_header_prefix, write_signature_dynamic_parameter_list,
};
use crate::r#where::format_where_clause_with_break;
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AbstractionModifier, AccessorKind, Annotation, BindingAnchor, BindingKind, BindingModifier,
    BindingOperator, Comment, Declaration, DeclarationKind, Expression, FunctionSignature, Key,
    Keyword, LocalNodeId, Member, Mutability, Name, Node, NodeTree, NodeTreeImpl, NodeType,
    Property, Timing, VarianceModifier,
};
use destack_fir::format::{FormatResult, text};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::Span;
use destack_workspace::QuoteProperty;

/// Format binding modifiers that appear before a name.
#[inline]
pub(crate) fn format_binding_modifiers_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifiers: BindingModifier,
) -> FormatResult<()> {
    // variance
    if let Some(variance) = modifiers.variance {
        match variance {
            VarianceModifier::In => write!(f, [token("in"), space()])?,
            VarianceModifier::Out => write!(f, [token("out"), space()])?,
            VarianceModifier::InOut => {
                write!(f, [token("in"), space(), token("out"), space()])?;
            }
        }
    }
    // visibility
    if let Some(visibility) = modifiers.visibility {
        write!(f, [visibility, space()])?;
    }
    // declaration
    if modifiers.declaration == Some(DeclarationKind::Declaration) {
        write!(f, [Keyword::Declare, space()])?;
    }
    // abstraction
    if let Some(abstraction) = modifiers.abstraction {
        match abstraction {
            AbstractionModifier::Abstract => write!(f, [Keyword::Abstract, space()])?,
            AbstractionModifier::Override => write!(f, [Keyword::Override, space()])?,
            AbstractionModifier::AbstractOverride => {
                write!(f, [Keyword::Abstract, space()])?;
                write!(f, [Keyword::Override, space()])?;
            }
        }
    }
    // scope
    if modifiers.anchor == Some(BindingAnchor::Static) {
        write!(f, [Keyword::Static, space()])?;
    }
    // mutability
    if modifiers.mutability == Some(Mutability::Immutable) {
        write!(f, [Keyword::Readonly, space()])?;
    }
    // operator
    if modifiers.operator == Some(BindingOperator::AsConst) {
        write!(f, [Keyword::Const, space()])?;
    }
    // accessor
    if modifiers.accessor == Some(AccessorKind::Accessor) {
        write!(f, [Keyword::Accessor, space()])?;
    }
    // timing
    if modifiers.timing == Some(Timing::Comptime) {
        write!(f, [Keyword::Comptime, space()])?;
    }
    Ok(())
}

/// Format optional binding modifiers before a name.
#[inline]
pub(crate) fn format_binding_modifiers_prefix_maybe<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifiers: Option<BindingModifier>,
) -> FormatResult<()> {
    if let Some(modifiers) = modifiers {
        format_binding_modifiers_prefix(f, modifiers)?;
    }
    Ok(())
}

/// Format binding modifiers that appear after a name.
#[inline]
pub(crate) fn format_binding_modifiers_postfix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifiers: BindingModifier,
) -> FormatResult<()> {
    // kind
    if modifiers.kind == Some(BindingKind::Must) {
        write!(f, [token("!")])?;
    } else if modifiers.kind == Some(BindingKind::Maybe) {
        write!(f, [token("?")])?;
    }
    Ok(())
}

/// Format optional binding modifiers after a name.
#[inline]
pub(crate) fn format_binding_modifiers_postfix_maybe<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifiers: Option<BindingModifier>,
) -> FormatResult<()> {
    if let Some(modifiers) = modifiers {
        format_binding_modifiers_postfix(f, modifiers)?;
    }
    Ok(())
}

/// Return whether method signature source spans multiple lines before the body.
fn method_signature_source_is_multiline(
    context: &DestackFormatContext<'_>,
    node_span: Span,
    body: Option<LocalNodeId<Expression>>,
) -> bool {
    let Some(body_id) = body else {
        return false;
    };

    let body_span = context.get_span(body_id);
    if node_span.file != body_span.file || node_span.start >= body_span.start {
        return false;
    }

    let signature_span = Span::new(node_span.file, node_span.start, body_span.start);
    context.get_span_str(signature_span).contains('\n')
}

/// Format a block of properties with empty-annotation and ignore-range handling.
#[allow(unused)]
pub(crate) fn format_block_of_properties<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    properties: &[LocalNodeId<Property>],
    separator: &'static str,
) -> FormatResult<()> {
    format_block_nodes_with_ignore_ranges(f, properties, |f, property_id| {
        let property = f.context().tree.get(property_id);
        property_id.format(f)?;

        // separators
        if matches!(
            property,
            Property::Field { .. } | Property::Method { .. } | Property::Spread { .. }
        ) {
            write!(f, [token(separator)])?;
        }

        Ok(())
    })
}

/// Format a block of members with empty-annotation and ignore-range handling.
#[allow(unused)]
pub(crate) fn format_block_of_members<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    members: &[LocalNodeId<Member>],
) -> FormatResult<()> {
    format_block_nodes_with_ignore_ranges(f, members, |f, member_id| member_id.format(f))
}

/// Format a block of nodes while honoring ignore ranges and spacing.
fn format_block_nodes_with_ignore_ranges<'ast, T, F>(
    f: &mut DestackFormatter<'ast, '_>,
    node_ids: &[LocalNodeId<T>],
    mut format_node: F,
) -> FormatResult<()>
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T> + NodeTreeImpl<Annotation> + NodeTreeImpl<Comment>,
    F: FnMut(&mut DestackFormatter<'ast, '_>, LocalNodeId<T>) -> FormatResult<()>,
{
    let comment_tokens = f.context().comment_tokens();
    let ignore_ranges = collect_ignore_ranges_for_nodes(f.context(), node_ids, comment_tokens);

    let mut skip_until: Option<u32> = None;
    for (index, node_id) in node_ids.iter().copied().enumerate() {
        let node_span = f.context().get_span(node_id);

        if let Some(skip_end) = skip_until {
            if node_span.start < skip_end {
                continue;
            }
            skip_until = None;
        }

        // blank line between entries
        if index > 0 {
            write!(f, [hard_line_break()])?;
        }

        if let Some(range_span) = ignore_ranges.get(&node_id.id) {
            write_ignored_span(f, *range_span)?;
            skip_until = Some(range_span.end);
            continue;
        }

        format_node(f, node_id)?;
    }

    Ok(())
}

/// Return whether a key requires quoting.
#[inline]
fn key_requires_quotes<'ast>(f: &DestackFormatter<'ast, '_>, key: Key) -> bool {
    let strings = f.context().strings;
    match key {
        Key::Name(Name::String(string_id)) => {
            let content = strings.get(string_id);
            !is_identifier_for_quotes(content)
        }
        _ => false,
    }
}

/// Return whether object properties should force quoted keys.
#[inline]
fn force_quote_keys_for_object<'ast>(
    f: &DestackFormatter<'ast, '_>,
    properties: &[LocalNodeId<Property>],
) -> bool {
    properties.iter().any(|property_id| {
        let property = f.context().tree.get(*property_id);
        match property {
            Property::Field { key, .. } | Property::Method { key, .. } => {
                key.is_some_and(|key| key_requires_quotes(f, key))
            }
            Property::Spread { .. } => false,
        }
    })
}

/// Return whether members should force quoted keys.
#[inline]
fn force_quote_keys_for_members<'ast>(
    f: &DestackFormatter<'ast, '_>,
    members: &[LocalNodeId<Member>],
) -> bool {
    members.iter().any(|member_id| {
        let member = f.context().tree.get(*member_id);
        match member {
            Member::Field { key, .. } | Member::Method { key, .. } => {
                key.is_some_and(|key| key_requires_quotes(f, key))
            }
            _ => false,
        }
    })
}

/// Return whether one property should force quoted keys.
#[inline]
fn should_force_quote_keys_for_property<'ast>(
    f: &DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Property>,
) -> bool {
    if f.context().options.quote_props != QuoteProperty::Consistent {
        return false;
    }

    if f.context().options.language_type.is_destack() {
        return false;
    }

    let Some((parent_id, parent_type)) = f.context().get_parent(node_id) else {
        return false;
    };

    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::ObjectExpression { properties, .. } = f.context().tree.get(parent_id) else {
        return false;
    };

    force_quote_keys_for_object(f, properties)
}

/// Return whether one member should force quoted keys.
#[inline]
fn should_force_quote_keys_for_member<'ast>(
    f: &DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Member>,
) -> bool {
    if f.context().options.quote_props != QuoteProperty::Consistent {
        return false;
    }

    if f.context().options.language_type.is_destack() {
        return false;
    }

    let Some((parent_id, parent_type)) = f.context().get_parent(node_id) else {
        return false;
    };

    if parent_type != NodeType::Declaration {
        return false;
    }

    let parent_id = LocalNodeId::<Declaration>::new(parent_id);
    let Declaration::Class { members, .. } = f.context().tree.get(parent_id) else {
        return false;
    };

    force_quote_keys_for_members(f, members)
}

/// Decide whether a field default should stay inline after `=`.
fn should_keep_field_default_inline<'ast>(
    f: &DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    if f.context().has_prefix_annotation(expression_id) {
        return false;
    }

    let is_call_like = matches!(
        f.context().tree.get(expression_id),
        Expression::Call { .. } | Expression::New { .. } | Expression::Instantiation { .. }
    );
    if !is_call_like {
        return false;
    }

    // preserve inline `= <expr>` when source already uses multiline rhs structure
    if f.context().node_has_newline(expression_id) {
        return true;
    }

    false
}

/// Write a field type annotation.
#[inline]
fn write_field_type_annotation<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if f.context().has_prefix_annotation(value) {
        write!(f, [token(":"), indent(&format_args![space(), value])])
    } else {
        write!(f, [token(":"), space(), value])
    }
}

/// Format shared property or member field output.
fn format_field_like<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifiers: Option<BindingModifier>,
    key: Option<Key>,
    value: Option<LocalNodeId<Expression>>,
    default: Option<LocalNodeId<Expression>>,
    force_quote_keys: bool,
) -> FormatResult<()> {
    // modifiers
    format_binding_modifiers_prefix_maybe(f, modifiers)?;
    // key
    if let Some(key) = key {
        format_key_with_quote_policy(f, key, force_quote_keys)?;
    }
    // modifiers
    format_binding_modifiers_postfix_maybe(f, modifiers)?;
    // value
    if let Some(value) = value {
        write_field_type_annotation(f, value)?;
    }
    // default
    if let Some(default) = default {
        if should_keep_field_default_inline(f, default) {
            write!(
                f,
                [group(&format_args![space(), token("="), space(), default])]
            )?;
        } else {
            write!(
                f,
                [group(&format_args![
                    space(),
                    token("="),
                    indent(&format_args![soft_line_break_or_space(), default]),
                ])]
            )?;
        }
    }

    Ok(())
}

/// Format shared property or member method output.
fn format_method_like<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifiers: Option<BindingModifier>,
    key: Option<Key>,
    signature: &FunctionSignature,
    body: Option<LocalNodeId<Expression>>,
    force_quote_keys: bool,
    signature_source_is_multiline: bool,
) -> FormatResult<()> {
    let generics = signature.generics.as_ref();

    // modifiers
    format_binding_modifiers_prefix_maybe(f, modifiers)?;

    // shared function header prefix
    write_function_header_prefix(f, signature, FunctionHeaderStyle::MethodLike, key.is_some())?;

    // key
    if let Some(key) = key {
        format_key_with_quote_policy(f, key, force_quote_keys)?;
    }

    // static parameters
    if let Some(static_parameters) =
        generics.and_then(|generics| generics.static_parameters.as_ref())
        && !static_parameters.is_empty()
    {
        write!(f, [list_like("<", ">", ",", static_parameters)])?;
    }

    // dynamic parameters
    let should_expand_parameters = signature_parameters_should_expand(
        f.context(),
        signature.mode,
        &signature.dynamic_parameters,
        signature.return_type,
        false,
    );
    if signature.dynamic_parameters.len() == 1
        && !should_expand_parameters
        && single_parameter_should_hug(f.context(), signature.dynamic_parameters[0])
        && !signature_return_type_is_multiline(f.context(), signature.return_type)
        && !signature_source_is_multiline
    {
        write!(f, [token("("), signature.dynamic_parameters[0], token(")")])?;
    } else {
        write_signature_dynamic_parameter_list(
            f,
            &signature.dynamic_parameters,
            should_expand_parameters,
            false,
        )?;
    }

    // return type
    if let Some(return_type) = signature.return_type {
        write!(f, [token(":"), space(), return_type])?;
    }

    // where clauses
    if let Some(where_clauses) = generics.and_then(|generics| generics.where_clauses.as_ref())
        && !where_clauses.is_empty()
    {
        format_where_clause_with_break(f, where_clauses)?;
    }

    // modifiers
    format_binding_modifiers_postfix_maybe(f, modifiers)?;

    // body
    if let Some(body) = body {
        if signature_return_type_has_line_postfix_boundary_annotation(
            f.context(),
            signature.return_type,
        ) {
            write!(f, [hard_line_break(), body])?;
        } else if signature_should_elide_space_before_body(f.context(), signature.return_type) {
            write!(f, [body])?;
        } else {
            write!(f, [space(), body])?;
        }
    }

    Ok(())
}

/// Format one node with shared directive handling and annotations.
fn format_node_with_directive<'ast, T, F>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
    mut format_node: F,
) -> FormatResult<()>
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T> + NodeTreeImpl<Annotation> + NodeTreeImpl<Comment>,
    F: FnMut(&mut DestackFormatter<'ast, '_>) -> FormatResult<()>,
{
    let directive = directive_for_node(f.context(), node_id);
    write!(f, [f.context().any_prefix_annotations(node_id)])?;

    if let Some(directive) = directive
        && directive.kind == FormatterDirectiveKind::IgnoreFormat
    {
        let raw = ignored_node_source(f.context(), node_id, directive);
        write!(f, [text(&raw)])?;

        if !matches!(
            directive.position,
            FormatterDirectivePosition::Postfix { .. }
        ) {
            write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
        }

        return Ok(());
    }

    format_node(f)?;
    write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

    Ok(())
}

impl<'ast> FormatNode<'ast, Property> for Property {
    fn format_node(
        &self,
        node_id: LocalNodeId<Property>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        format_node_with_directive(f, node_id, |f| {
            match self {
                Property::Field {
                    modifiers,
                    key,
                    value,
                    default,
                } => {
                    let force_quote_keys = should_force_quote_keys_for_property(f, node_id);
                    format_field_like(f, *modifiers, *key, *value, *default, force_quote_keys)?;
                }
                Property::Method {
                    modifiers,
                    key,
                    signature,
                    body,
                } => {
                    let force_quote_keys = should_force_quote_keys_for_property(f, node_id);
                    let signature_source_is_multiline = method_signature_source_is_multiline(
                        f.context(),
                        f.context().get_span(node_id),
                        *body,
                    );
                    format_method_like(
                        f,
                        *modifiers,
                        *key,
                        signature,
                        *body,
                        force_quote_keys,
                        signature_source_is_multiline,
                    )?;
                }
                Property::Spread { modifiers, value } => {
                    // modifiers
                    format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                    // keyword
                    write!(f, [token("...")])?;
                    // value
                    write!(f, [value])?;
                    // modifiers
                    format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                }
            }

            Ok(())
        })
    }
}

impl<'ast> FormatNode<'ast, Member> for Member {
    fn format_node(
        &self,
        node_id: LocalNodeId<Member>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        format_node_with_directive(f, node_id, |f| {
            match self {
                Member::Type {
                    modifiers,
                    name,
                    static_parameters,
                    where_clauses,
                    ty,
                    value,
                } => {
                    // modifiers
                    format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                    // keyword
                    write!(f, [Keyword::Type, space()])?;
                    // name
                    write!(f, [name])?;
                    // static parameters
                    if let Some(static_parameters) = static_parameters
                        && !static_parameters.is_empty()
                    {
                        write!(f, [list_like("<", ">", ",", static_parameters)])?;
                    }
                    // where clauses
                    if let Some(where_clauses) = where_clauses
                        && !where_clauses.is_empty()
                    {
                        write!(f, [space(), Keyword::Where, space()])?;
                        write!(f, [list_like("", "", ",", where_clauses)])?;
                    }
                    // type bound
                    if let Some(ty) = ty {
                        write!(f, [token(":"), space(), ty])?;
                    }
                    // value
                    if let Some(value) = value {
                        write!(f, [space(), token("="), space(), value])?;
                    }
                }
                Member::ComptimeConst {
                    modifiers,
                    name,
                    ty,
                    value,
                } => {
                    // keep non comptime modifiers before the associated keyword pair
                    if let Some(mut modifiers) = *modifiers {
                        modifiers.timing = None;
                        modifiers.operator = None;
                        format_binding_modifiers_prefix(f, modifiers)?;
                    }

                    // associated comptime constants are always emitted in canonical order
                    write!(
                        f,
                        [Keyword::Comptime, space(), Keyword::Const, space(), name]
                    )?;

                    // optional type annotation
                    if let Some(ty) = ty {
                        write!(f, [token(":"), space(), ty])?;
                    }

                    // optional initializer
                    if let Some(value) = value {
                        write!(f, [space(), token("="), space(), value])?;
                    }
                }
                Member::Field {
                    modifiers,
                    key,
                    value,
                    default,
                } => {
                    let force_quote_keys = should_force_quote_keys_for_member(f, node_id);
                    format_field_like(f, *modifiers, *key, *value, *default, force_quote_keys)?;
                }
                Member::Method {
                    modifiers,
                    key,
                    signature,
                    body,
                } => {
                    let force_quote_keys = should_force_quote_keys_for_member(f, node_id);
                    let signature_source_is_multiline = method_signature_source_is_multiline(
                        f.context(),
                        f.context().get_span(node_id),
                        *body,
                    );
                    format_method_like(
                        f,
                        *modifiers,
                        *key,
                        signature,
                        *body,
                        force_quote_keys,
                        signature_source_is_multiline,
                    )?;
                }
                Member::Embed { modifiers, value } => {
                    // modifiers
                    format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                    // keyword
                    write!(f, [token("...")])?;
                    // value
                    write!(f, [value])?;
                    // modifiers
                    format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                }
                Member::StaticBlock { body, .. } => {
                    // keyword
                    write!(f, [Keyword::Static, space()])?;
                    // body
                    write!(f, [body])?;
                }
                Member::ComptimeBlock { modifiers, body } => {
                    // modifiers prefix
                    if let Some(mut modifiers) = *modifiers {
                        modifiers.timing = None;
                        format_binding_modifiers_prefix(f, modifiers)?;
                    }
                    // keyword
                    write!(f, [Keyword::Comptime, space()])?;
                    // body
                    write!(f, [body])?;
                }
            }

            let needs_semicolon = matches!(
                self,
                Member::Field { .. } | Member::Method { body: None, .. }
            );
            if needs_semicolon {
                write!(f, [token(";")])?;
            }

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};
    use destack_ast::DeclarationDescriptor;
    use destack_source::LanguageType;

    #[test]
    fn test_format_struct_empty() {
        assert_format!(
            "struct Foo { }",
            "struct Foo {}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_struct_with_fields() {
        assert_format!(
            "struct Foo { a: int32; b: boolean }",
            "struct Foo {\n\ta: int32;\n\tb: boolean;\n}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_modified_fields() {
        assert_format!(
            "struct Foo { readonly a: int32; private b: boolean }",
            "struct Foo {\n\treadonly a: int32;\n\tprivate b: boolean;\n}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_name() {
        assert_format!(
            "struct Foo { a: int32 }",
            "struct Foo {\n\ta: int32;\n}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_fields_and_defaults() {
        assert_format!(
            "struct Foo { a?: int32 = 42; b: boolean }",
            "struct Foo {\n\ta?: int32 = 42;\n\tb: boolean;\n}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_class_with_abstract_override_field() {
        assert_format!(
            "class Foo { abstract override bar: int32 }",
            "class Foo {\n\tabstract override bar: int32;\n}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
            DestackFormatOptions {
                language_type: LanguageType::TypeScript,
                ..DestackFormatOptions::default_tab()
            }
        );
    }

    #[test]
    fn test_format_struct_with_static_parameters_and_inheritance() {
        assert_format!(
            "struct Foo<T: Numeric> extends Bar implements Baz { }",
            "struct Foo<T: Numeric> extends Bar implements Baz {}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
            DestackFormatOptions::default()
        );
    }
}

use std::collections::HashMap;

use crate::argument::list_like;
use crate::directive::{
    FormatterDirectiveKind, FormatterDirectivePosition, collect_comment_tokens, directive_for_node,
    ignore_range_for_node, ignored_node_source, write_ignored_span,
};
use crate::key::{format_key_with_quote_policy, is_identifier_for_quotes};
use crate::r#where::format_where_clause_with_break;
use crate::{DestackFormatter, FormatNode};
use destack_ast::{
    AbstractionModifier, AccessorKind, Asynchrony, BindingAnchor, BindingKind, BindingModifier,
    BindingOperator, Declaration, Expression, FunctionAbstraction, FunctionCardinality,
    FunctionMode, Key, Keyword, LocalNodeId, Member, Mutability, Name, NodeType, Property, Timing,
    VarianceModifier,
};
use destack_fir::format::{FormatResult, text};
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::Span;
use destack_workspace::QuoteProperty;

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

#[inline]
pub(crate) fn format_binding_modifiers_postfix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifiers: BindingModifier,
) -> FormatResult<()> {
    // kind
    if modifiers.kind == Some(BindingKind::Maybe) {
        write!(f, [token("?")])?;
    }
    Ok(())
}

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

/// Format a block of properties with appropriate empty annotations.
#[allow(unused)]
pub(crate) fn format_block_of_properties<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    properties: &[LocalNodeId<Property>],
    separator: &'static str,
) -> FormatResult<()> {
    let comment_tokens = collect_comment_tokens(f.context());
    let mut ignore_ranges: HashMap<u32, Span> = HashMap::new();
    for &property_id in properties {
        if let Some(range_span) = ignore_range_for_node(f.context(), property_id, &comment_tokens) {
            ignore_ranges.insert(property_id.id, range_span);
        }
    }

    let mut skip_until: Option<u32> = None;
    for (i, &property_id) in properties.iter().enumerate() {
        let property = f.context().tree.get(property_id);
        let property_span = f.context().get_span(property_id);

        if let Some(skip_end) = skip_until {
            if property_span.start < skip_end {
                continue;
            }
            skip_until = None;
        }

        // blank line between properties
        if i > 0 {
            write!(f, [hard_line_break()])?;
        }

        if let Some(range_span) = ignore_ranges.get(&property_id.id) {
            write_ignored_span(f, *range_span)?;
            skip_until = Some(range_span.end);
            continue;
        }

        property_id.format(f)?;
        // comma after field properties
        if matches!(
            property,
            Property::Field { .. } | Property::Method { .. } | Property::Spread { .. }
        ) {
            write!(f, [token(separator)])?;
        }
    }
    Ok(())
}

/// Format a block of members with appropriate empty annotations.
#[allow(unused)]
pub(crate) fn format_block_of_members<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    members: &[LocalNodeId<Member>],
) -> FormatResult<()> {
    let is_typescript = f.context().options.language_type.is_typescript();
    let comment_tokens = collect_comment_tokens(f.context());
    let mut ignore_ranges: HashMap<u32, Span> = HashMap::new();
    for &member_id in members {
        if let Some(range_span) = ignore_range_for_node(f.context(), member_id, &comment_tokens) {
            ignore_ranges.insert(member_id.id, range_span);
        }
    }

    let mut skip_until: Option<u32> = None;
    for (i, &member_id) in members.iter().enumerate() {
        let member = f.context().tree.get(member_id);
        let member_span = f.context().get_span(member_id);

        if let Some(skip_end) = skip_until {
            if member_span.start < skip_end {
                continue;
            }
            skip_until = None;
        }

        // blank line between members
        if i > 0 {
            write!(f, [hard_line_break()])?;
        }

        if let Some(range_span) = ignore_ranges.get(&member_id.id) {
            write_ignored_span(f, *range_span)?;
            skip_until = Some(range_span.end);
            continue;
        }

        member_id.format(f)?;
        if is_typescript {
            let needs_semicolon = matches!(
                member,
                Member::Field { .. } | Member::Method { body: None, .. }
            );
            if needs_semicolon {
                write!(f, [token(";")])?;
            }
        } else {
            // comma after field members
            if matches!(member, Member::Field { .. }) {
                write!(f, [token(",")])?;
            }
        }
    }
    Ok(())
}

/// Check whether a key requires quotes under identifier rules.
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

/// Check whether any object key forces consistent quoting.
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

/// Check whether any type member key forces consistent quoting.
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

/// Decide whether this property should force consistent key quoting.
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

/// Decide whether this member should force consistent key quoting.
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

impl<'ast> FormatNode<'ast, Property> for Property {
    fn format_node(
        &self,
        node_id: LocalNodeId<Property>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // ignore formatting when requested
        let directive = directive_for_node(f.context(), node_id);

        // prefix annotations
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // raw node formatting for ignore directives
        if let Some(directive) = directive
            && directive.kind == FormatterDirectiveKind::IgnoreFormat
        {
            let raw_property = ignored_node_source(f.context(), node_id, directive);
            write!(f, [text(&raw_property)])?;

            if !matches!(
                directive.position,
                FormatterDirectivePosition::Postfix { .. }
            ) {
                write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
            }

            return Ok(());
        }

        match self {
            Property::Field {
                modifiers,
                key,
                value,
                default,
            } => {
                let force_quote_keys = should_force_quote_keys_for_property(f, node_id);

                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // key
                if let Some(key) = key {
                    format_key_with_quote_policy(f, *key, force_quote_keys)?;
                }
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // value
                if let Some(value) = value {
                    write!(f, [token(":"), space(), value])?;
                }
                // default
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
            Property::Method {
                modifiers,
                key,
                signature,
                body,
            } => {
                let generics = signature.generics.as_ref();
                let force_quote_keys = should_force_quote_keys_for_property(f, node_id);
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;

                // abstraction
                match signature.abstraction {
                    FunctionAbstraction::Abstract => {
                        write!(f, [Keyword::Abstract, space()])?;
                    }
                    FunctionAbstraction::AbstractOverride => {
                        write!(f, [Keyword::Abstract, space()])?;
                        write!(f, [Keyword::Override, space()])?;
                    }
                    FunctionAbstraction::ConcreteOverride => {
                        write!(f, [Keyword::Override, space()])?;
                    }
                    FunctionAbstraction::Concrete => {}
                }

                // asynchrony
                if signature.asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Async, space()])?;
                }

                // mode
                if let Some(mode) = signature.mode {
                    if let Some(keyword) = mode.to_keyword() {
                        write!(f, [keyword])?;
                    }
                    if key.is_some() || matches!(mode, FunctionMode::New) {
                        write!(f, [space()])?;
                    }
                }

                // cardinality
                if signature.cardinality == FunctionCardinality::Generator {
                    write!(f, [token("*")])?;
                }

                // key
                if let Some(key) = key {
                    format_key_with_quote_policy(f, *key, force_quote_keys)?;
                }

                // static parameters
                if let Some(static_parameters) =
                    generics.and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // dynamic parameters
                write!(f, [list_like("(", ")", ",", &signature.dynamic_parameters)])?;

                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;

                // return type
                if let Some(return_type) = signature.return_type {
                    write!(f, [token(":"), space(), return_type])?;
                }

                // where clauses
                if let Some(where_clauses) =
                    generics.and_then(|generics| generics.where_clauses.as_ref())
                    && !where_clauses.is_empty()
                {
                    format_where_clause_with_break(f, where_clauses)?;
                }

                // body
                if let Some(body) = body {
                    write!(f, [space(), body])?;
                }
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

        // postfix annotations
        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Member> for Member {
    fn format_node(
        &self,
        node_id: LocalNodeId<Member>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // ignore formatting when requested
        let directive = directive_for_node(f.context(), node_id);

        // prefix annotations
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // raw node formatting for ignore directives
        if let Some(directive) = directive
            && directive.kind == FormatterDirectiveKind::IgnoreFormat
        {
            let raw_member = ignored_node_source(f.context(), node_id, directive);
            write!(f, [text(&raw_member)])?;

            if !matches!(
                directive.position,
                FormatterDirectivePosition::Postfix { .. }
            ) {
                write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
            }

            return Ok(());
        }

        match self {
            Member::Type {
                modifiers,
                name,
                ty,
                value,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // keyword
                write!(f, [Keyword::Type, space()])?;
                // name
                write!(f, [name])?;
                // type bound
                if let Some(ty) = ty {
                    write!(f, [token(":"), space(), ty])?;
                }
                // value
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

                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // key
                if let Some(key) = key {
                    format_key_with_quote_policy(f, *key, force_quote_keys)?;
                }
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // value
                if let Some(value) = value {
                    write!(f, [token(":"), space(), value])?;
                }
                // default
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
            Member::Method {
                modifiers,
                key,
                signature,
                body,
            } => {
                let generics = signature.generics.as_ref();
                let force_quote_keys = should_force_quote_keys_for_member(f, node_id);

                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;

                // abstraction
                match signature.abstraction {
                    FunctionAbstraction::Abstract => {
                        write!(f, [Keyword::Abstract, space()])?;
                    }
                    FunctionAbstraction::AbstractOverride => {
                        write!(f, [Keyword::Abstract, space()])?;
                        write!(f, [Keyword::Override, space()])?;
                    }
                    FunctionAbstraction::ConcreteOverride => {
                        write!(f, [Keyword::Override, space()])?;
                    }
                    FunctionAbstraction::Concrete => {}
                }

                // asynchrony
                if signature.asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Async, space()])?;
                }

                // mode
                if let Some(mode) = signature.mode {
                    if let Some(keyword) = mode.to_keyword() {
                        write!(f, [keyword])?;
                    }
                    if key.is_some() || matches!(mode, FunctionMode::New) {
                        write!(f, [space()])?;
                    }
                }

                // cardinality
                if signature.cardinality == FunctionCardinality::Generator {
                    write!(f, [token("*")])?;
                }

                // key
                if let Some(key) = key {
                    format_key_with_quote_policy(f, *key, force_quote_keys)?;
                }

                // static parameters
                if let Some(static_parameters) =
                    generics.and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // dynamic parameters
                write!(f, [list_like("(", ")", ",", &signature.dynamic_parameters)])?;

                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;

                // return type
                if let Some(return_type) = signature.return_type {
                    write!(f, [token(":"), space(), return_type])?;
                }

                // where clauses
                if let Some(where_clauses) =
                    generics.and_then(|generics| generics.where_clauses.as_ref())
                    && !where_clauses.is_empty()
                {
                    format_where_clause_with_break(f, where_clauses)?;
                }

                // body
                if let Some(body) = body {
                    write!(f, [space(), body])?;
                }
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

        // postfix annotations
        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
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
            "struct Foo { }",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default()),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_struct_with_fields() {
        assert_format!(
            "struct Foo { a: int32, b: boolean }",
            "struct Foo {\n\ta: int32,\n\tb: boolean,\n}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default()),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_modified_fields() {
        assert_format!(
            "struct Foo { readonly a: int32, private b: boolean }",
            "struct Foo {\n\treadonly a: int32,\n\tprivate b: boolean,\n}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default()),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_name() {
        assert_format!(
            "struct Foo { a: int32 }",
            "struct Foo {\n\ta: int32,\n}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default()),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_fields_and_defaults() {
        assert_format!(
            "struct Foo { a?: int32 = 42, b: boolean }",
            "struct Foo {\n\ta?: int32 = 42,\n\tb: boolean,\n}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default()),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_class_with_abstract_override_field() {
        assert_format!(
            "class Foo { abstract override bar: int32 }",
            "class Foo {\n\tabstract override bar: int32;\n}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default()),
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
            "struct Foo<T: Numeric> extends Bar implements Baz { }",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default()),
            DestackFormatOptions::default()
        );
    }
}

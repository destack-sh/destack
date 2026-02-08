use destack_fir::format::FormatResult;

use crate::argument::list_like;
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{Expression, LocalNodeId, Mutability, NodeTree, NodeType, Pattern, PatternField};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};

impl<'ast> Format<DestackFormatContext<'ast>> for Mutability {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Mutability::Immutable => write!(f, [token("const")]),
            Mutability::Mutable => write!(f, [token("var")]),
        }
    }
}

/// Decide whether a default expression in a pattern field should prefer multiline layout.
fn default_expression_prefers_multiline(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    matches!(
        tree.get(expression_id),
        Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. }
    )
}

/// Decide whether a pattern has nested structure that benefits from multiline layout.
fn pattern_prefers_multiline(tree: &NodeTree, pattern_id: LocalNodeId<Pattern>) -> bool {
    match tree.get(pattern_id) {
        Pattern::Binding {
            pattern: Some(pattern),
            ..
        } => pattern_prefers_multiline(tree, *pattern),
        Pattern::Object { fields } | Pattern::TaggedObject { fields, .. } => fields
            .iter()
            .copied()
            .any(|field_id| pattern_field_prefers_multiline(tree, field_id)),
        Pattern::Array { fields }
        | Pattern::Tuple { fields }
        | Pattern::TaggedTuple { fields, .. } => fields
            .iter()
            .copied()
            .any(|field_id| pattern_field_prefers_multiline(tree, field_id)),
        _ => false,
    }
}

/// Decide whether a pattern field has nested structure that benefits from multiline layout.
fn pattern_field_prefers_multiline(tree: &NodeTree, field_id: LocalNodeId<PatternField>) -> bool {
    match tree.get(field_id) {
        PatternField::Named {
            pattern, default, ..
        }
        | PatternField::Computed {
            pattern, default, ..
        } => {
            pattern.is_some_and(|pattern_id| pattern_prefers_multiline(tree, pattern_id))
                || default.is_some_and(|default_id| {
                    default_expression_prefers_multiline(tree, default_id)
                })
        }
        PatternField::Alias { default, .. } => {
            default.is_some_and(|default_id| default_expression_prefers_multiline(tree, default_id))
        }
        PatternField::Positional { pattern } => pattern_prefers_multiline(tree, *pattern),
        PatternField::Spread { pattern, .. } => {
            pattern.is_some_and(|pattern_id| pattern_prefers_multiline(tree, pattern_id))
        }
        PatternField::Elision => false,
    }
}

/// Return whether source text between delimiters contains a newline.
fn source_between_delimiters_has_newline(source: &str, open: char, close: char) -> bool {
    let Some(open_index) = source.find(open) else {
        return source.contains('\n');
    };
    let Some(close_index) = source.rfind(close) else {
        return source.contains('\n');
    };
    if close_index <= open_index {
        return source.contains('\n');
    }

    let content_start = open_index + open.len_utf8();
    source[content_start..close_index].contains('\n')
}

/// Return whether a pattern source is multiline inside its delimiters.
fn pattern_has_multiline_source(
    context: &DestackFormatContext<'_>,
    pattern_id: LocalNodeId<Pattern>,
) -> bool {
    let span = context.get_span(pattern_id);
    let source = context.get_span_str(span);

    match context.tree.get(pattern_id) {
        Pattern::Object { .. } | Pattern::TaggedObject { .. } => {
            source_between_delimiters_has_newline(source, '{', '}')
        }
        Pattern::Array { .. } => source_between_delimiters_has_newline(source, '[', ']'),
        Pattern::Tuple { .. } | Pattern::TaggedTuple { .. } => {
            source_between_delimiters_has_newline(source, '(', ')')
        }
        _ => context.has_newline(span),
    }
}

/// Decide whether a pattern field default should force expanded formatting.
fn should_expand_pattern_field_default<'ast>(
    f: &DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<PatternField>,
    default_id: LocalNodeId<Expression>,
) -> bool {
    let default_is_collection = matches!(
        f.context().tree.get(default_id),
        Expression::ObjectExpression { properties, .. } if !properties.is_empty()
    ) || matches!(
        f.context().tree.get(default_id),
        Expression::ArrayExpression { elements } if !elements.is_empty()
    );
    if !default_is_collection {
        return false;
    }

    let Some((parent_id, parent_type)) = f.context().get_parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Pattern {
        return false;
    }

    let parent_pattern = LocalNodeId::<Pattern>::new(parent_id);
    f.context()
        .has_newline(f.context().get_span(parent_pattern))
}

/// Decide whether an object parameter pattern should expand for readability.
fn should_expand_parameter_object_pattern(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Pattern>,
    fields: &[LocalNodeId<PatternField>],
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Parameter {
        return false;
    }

    let parameter = context
        .tree
        .get(LocalNodeId::<destack_ast::Parameter>::new(parent_id));
    let parameter_pattern = match parameter {
        destack_ast::Parameter::Pattern { pattern, .. }
        | destack_ast::Parameter::VariadicPattern { pattern, .. } => Some(*pattern),
        destack_ast::Parameter::Named { .. } | destack_ast::Parameter::VariadicNamed { .. } => None,
    };
    if parameter_pattern.is_none_or(|pattern_id| pattern_id.id != node_id.id) {
        return false;
    }

    if fields
        .iter()
        .copied()
        .any(|field_id| pattern_field_prefers_multiline(context.tree, field_id))
    {
        return true;
    }

    if fields.len() >= 3 {
        return true;
    }

    if fields.len() <= 1 {
        return false;
    }

    fields
        .iter()
        .any(|field_id| match context.tree.get(*field_id) {
            PatternField::Named { default, .. }
            | PatternField::Computed { default, .. }
            | PatternField::Alias { default, .. } => default.is_some(),
            PatternField::Positional { .. }
            | PatternField::Spread { .. }
            | PatternField::Elision => false,
        })
}

impl<'ast> FormatNode<'ast, Pattern> for Pattern {
    fn format_node(
        &self,
        node_id: LocalNodeId<Pattern>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            Pattern::Wildcard => write!(f, [token("_")])?,
            Pattern::Must(unwrap) => write!(f, [unwrap, token("!")])?,
            Pattern::ReferenceOf { right, mutability } => {
                write!(f, [token("&")])?;
                if let Some(mutability) = mutability
                    && *mutability == Mutability::Immutable
                {
                    write!(f, [token("readonly"), space()])?;
                }
                write!(f, [right])?;
            }
            Pattern::ValueOf { right, mutability } => {
                write!(f, [token("^")])?;
                if let Some(mutability) = mutability
                    && *mutability == Mutability::Immutable
                {
                    write!(f, [token("readonly"), space()])?;
                }
                write!(f, [right])?;
            }
            Pattern::Binding {
                mutability,
                name,
                pattern,
            } => {
                if let Some(mutability) = mutability
                    && *mutability == Mutability::Mutable
                {
                    write!(f, [mutability, space()])?;
                }
                write!(f, [name])?;
                if let Some(pattern) = pattern {
                    write!(f, [token(":"), space(), pattern])?;
                }
            }
            Pattern::Expression { value } => write!(f, [value])?,
            Pattern::Range { start, end, .. } => write!(f, [start, token(".."), end,])?,
            Pattern::Tuple { fields } => {
                write!(f, [list_like("(", ")", ",", fields).as_collection()])?
            }
            Pattern::TaggedTuple { ty, fields } => {
                write!(f, [ty])?;
                write!(f, [list_like("(", ")", ",", fields).as_collection()])?
            }
            Pattern::Array { fields } => {
                let has_newline = pattern_has_multiline_source(f.context(), node_id);
                let has_nested_fields = fields
                    .iter()
                    .copied()
                    .any(|field_id| pattern_field_prefers_multiline(f.context().tree, field_id));
                let has_field_annotations = fields
                    .iter()
                    .copied()
                    .any(|field_id| f.context().has_annotation(field_id));

                let should_expand = has_newline && (has_nested_fields || has_field_annotations);
                let mut list = list_like("[", "]", ",", fields);
                list.as_collection().should_expand(should_expand);

                write!(f, [list])?;
            }
            Pattern::Object { fields } => {
                let mut list = list_like("{", "}", ",", fields);
                list.as_collection().include_space();
                let has_newline = pattern_has_multiline_source(f.context(), node_id);
                let has_nested_fields = fields
                    .iter()
                    .copied()
                    .any(|field_id| pattern_field_prefers_multiline(f.context().tree, field_id));
                let has_field_annotations = fields
                    .iter()
                    .copied()
                    .any(|field_id| f.context().has_annotation(field_id));
                let should_expand_for_parameter =
                    should_expand_parameter_object_pattern(f.context(), node_id, fields);
                let should_expand_for_comments = has_newline && has_field_annotations;

                if (has_newline && has_nested_fields)
                    || should_expand_for_comments
                    || should_expand_for_parameter
                {
                    list.as_collection().should_expand(true);
                }
                if fields.last().is_some_and(|field_id| {
                    matches!(f.context().tree.get(*field_id), PatternField::Spread { .. })
                }) {
                    list.disallow_trailing_separator();
                }
                write!(f, [list])?;
            }
            Pattern::TaggedObject { ty, fields } => {
                write!(f, [ty, space()])?;
                let mut list = list_like("{", "}", ",", fields);
                list.as_collection().include_space();
                let has_newline = pattern_has_multiline_source(f.context(), node_id);
                let has_nested_fields = fields
                    .iter()
                    .copied()
                    .any(|field_id| pattern_field_prefers_multiline(f.context().tree, field_id));
                let has_field_annotations = fields
                    .iter()
                    .copied()
                    .any(|field_id| f.context().has_annotation(field_id));
                let should_expand_for_parameter =
                    should_expand_parameter_object_pattern(f.context(), node_id, fields);
                let should_expand_for_comments = has_newline && has_field_annotations;

                if (has_newline && has_nested_fields)
                    || should_expand_for_comments
                    || should_expand_for_parameter
                {
                    list.as_collection().should_expand(true);
                }
                if fields.last().is_some_and(|field_id| {
                    matches!(f.context().tree.get(*field_id), PatternField::Spread { .. })
                }) {
                    list.disallow_trailing_separator();
                }
                write!(f, [list])?;
            }
            Pattern::Union { patterns } => write!(
                f,
                [format_with(|f| f
                    .join_with(&format_args![space(), token("|"), space()])
                    .entries(patterns)
                    .finish())]
            )?,
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, PatternField> for PatternField {
    fn format_node(
        &self,
        node_id: LocalNodeId<PatternField>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            PatternField::Named {
                mutability,
                name,
                pattern,
                default,
            } => {
                if let Some(mutability) = mutability {
                    write!(f, [mutability, space()])?;
                }
                if let Some(pattern) = pattern {
                    write!(f, [name, token(":"), space(), pattern])?;
                } else {
                    write!(f, [name])?;
                }
                if let Some(default) = default {
                    let should_expand_default =
                        should_expand_pattern_field_default(f, node_id, *default);
                    if should_expand_default {
                        write!(
                            f,
                            [
                                space(),
                                token("="),
                                space(),
                                group(default).should_expand(true)
                            ]
                        )?;
                    } else {
                        write!(f, [space(), token("="), space(), default])?;
                    }
                }
            }
            PatternField::Computed {
                mutability,
                key,
                pattern,
                default,
            } => {
                if let Some(mutability) = mutability {
                    write!(f, [mutability, space()])?;
                }
                write!(f, [token("["), key, token("]")])?;
                if let Some(pattern) = pattern {
                    write!(f, [token(":"), space(), pattern])?;
                }
                if let Some(default) = default {
                    let should_expand_default =
                        should_expand_pattern_field_default(f, node_id, *default);
                    if should_expand_default {
                        write!(
                            f,
                            [
                                space(),
                                token("="),
                                space(),
                                group(default).should_expand(true)
                            ]
                        )?;
                    } else {
                        write!(f, [space(), token("="), space(), default])?;
                    }
                }
            }
            PatternField::Alias {
                mutability,
                name,
                alias,
                default,
            } => {
                if let Some(mutability) = mutability {
                    write!(f, [mutability, space()])?;
                }
                write!(f, [name, token(":"), space(), alias])?;
                if let Some(default) = default {
                    let should_expand_default =
                        should_expand_pattern_field_default(f, node_id, *default);
                    if should_expand_default {
                        write!(
                            f,
                            [
                                space(),
                                token("="),
                                space(),
                                group(default).should_expand(true)
                            ]
                        )?;
                    } else {
                        write!(f, [space(), token("="), space(), default])?;
                    }
                }
            }
            PatternField::Positional { pattern } => write!(f, [pattern])?,
            PatternField::Spread {
                mutability,
                pattern,
            } => {
                if let Some(mutability) = mutability {
                    write!(f, [mutability, space()])?;
                }
                write!(f, [token("...")])?;
                if let Some(pattern) = pattern {
                    write!(f, [pattern])?;
                }
            }
            PatternField::Elision => {
                // elision is represented by empty slot; comma is handled at list level
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};

    #[test]
    fn test_format_pattern_wildcard() {
        assert_format!("_", "_", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_reference() {
        assert_format!("&_", "&_", |p| p.eat_pattern());

        assert_format!("&1", "&1", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_must() {
        assert_format!("T!", "T!", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_identifier() {
        assert_format!("x", "x", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_path() {
        assert_format!("MyEnum.A", "MyEnum.A", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_tuple() {
        assert_format!("(x: 1, 2, ...)", "(x: 1, 2, ...)", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_tuple_with_path() {
        assert_format!("Result.Success(_, ...)", "Result.Success(_, ...)", |p| p
            .eat_pattern());
    }

    #[test]
    fn test_format_pattern_slice() {
        assert_format!("[1, 2, ...]", "[1, 2, ...]", |p| p.eat_pattern());
    }

    #[test]
    fn test_format_pattern_union() {
        assert_format!("1 | 2 | 3 | 4 | 5", "1 | 2 | 3 | 4 | 5", |p| p
            .eat_pattern());
    }
}

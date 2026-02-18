use crate::collection::{collection_nodes_have_annotations, collection_value_should_force_break};
use crate::expression::*;
use destack_fir::{format_args, write};

/// Return whether JSX argument formatting should force multiline mode.
pub(crate) fn has_multiline_jsx_argument(
    tree: &NodeTree,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    if arguments.len() != 1 {
        return false;
    }

    let Some(value_id) = argument_value(tree, arguments[0]) else {
        return false;
    };

    // check if it's a JSX element with children
    if let Expression::TreeExpression { elements, .. } = tree.get(value_id) {
        elements.as_ref().is_some_and(|e| !e.is_empty())
    } else {
        false
    }
}

/// Tree expression argument, using `=` for named arguments.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TreeExpressionArgument {
    pub(crate) argument_id: LocalNodeId<Argument>,
}

/// Format inline stub comments attached to one expression node.
fn format_inline_stub_expression_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    let Some(annotations) = f.context().annotations(expression_id) else {
        return Ok(false);
    };

    let mut first = true;
    for annotation_id in annotations {
        let Annotation::Comment { node, .. } = f.context().annotation(annotation_id) else {
            continue;
        };

        if !first {
            write!(f, [space()])?;
        }
        first = false;

        write!(f, [node])?;
    }

    Ok(!first)
}

/// Format inline stub comments attached to one argument node.
fn format_inline_stub_argument_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<bool> {
    let Some(annotations) = f.context().annotations(argument_id) else {
        return Ok(false);
    };

    let mut first = true;
    for annotation_id in annotations {
        let Annotation::Comment { node, .. } = f.context().annotation(annotation_id) else {
            continue;
        };

        if !first {
            write!(f, [space()])?;
        }
        first = false;

        write!(f, [node])?;
    }

    Ok(!first)
}

impl<'ast> Format<DestackFormatContext<'ast>> for TreeExpressionArgument {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(self.argument_id)])?;

        let argument = f.context().tree.get(self.argument_id);
        let mut stub_argument_annotations_rendered_inline = false;
        match argument {
            Argument::Named { name, value, .. } => {
                let value_expr = f.context().tree.get(*value);
                if let Expression::ScalarLiteral(ScalarLiteral::Boolean(true)) = value_expr {
                    // boolean shorthand
                    write!(f, [name])?;
                } else {
                    write!(f, [name])?;
                    let is_string_literal = matches!(
                        value_expr,
                        Expression::ScalarLiteral(ScalarLiteral::String(_))
                            | Expression::ScalarLiteral(ScalarLiteral::Character(_))
                    );
                    if is_string_literal {
                        write!(f, [token("="), value])?;
                    } else {
                        // try hugged format for object and array attribute values
                        format_tree_attribute_value(f, *value)?;
                    }
                }
            }
            Argument::Labeled { label, value, .. } => {
                // label
                write!(f, [label])?;
                // value
                write!(f, [token(":"), space(), value])?;
            }
            Argument::Positional { value, .. } => {
                // in tree expressions, expression children need braces too
                let value_expr = f.context().tree.get(*value);
                let argument_span = f.context().span(self.argument_id);
                let argument_span_str = f.context().span_str(argument_span);
                let argument_is_braced = argument_span_str.trim_start().starts_with('{')
                    && argument_span_str.trim_end().ends_with('}');
                let needs_braces = argument_is_braced
                    || !matches!(
                        value_expr,
                        Expression::ScalarLiteral(ScalarLiteral::String(_))
                            | Expression::TreeExpression { .. }
                    );
                if needs_braces {
                    if matches!(value_expr, Expression::Stub) {
                        write!(f, [token("{")])?;
                        let mut wrote_stub_comment =
                            format_inline_stub_expression_comments(f, *value)?;
                        if !wrote_stub_comment {
                            wrote_stub_comment =
                                format_inline_stub_argument_comments(f, self.argument_id)?;
                            if wrote_stub_comment {
                                stub_argument_annotations_rendered_inline = true;
                            }
                        }
                        write!(f, [token("}")])?;
                    } else {
                        // keep jsx expression containers inline for common expression forms
                        if tree_child_should_inline_braced_expression(f.context(), self.argument_id)
                        {
                            write!(f, [token("{"), value, token("}")])?;
                        } else {
                            write!(
                                f,
                                [group(&format_args![
                                    token("{"),
                                    soft_block_indent(&value),
                                    token("}")
                                ])]
                            )?;
                        }
                    }
                } else {
                    write!(f, [value])?;
                }
            }
            Argument::Spread { value, .. } => {
                // spread in jsx needs braces: {...props}
                write!(f, [token("{"), token("..."), value, token("}")])?;
            }
        }

        if !stub_argument_annotations_rendered_inline {
            write!(
                f,
                [f.context()
                    .any_infix_or_postfix_annotations(self.argument_id)]
            )?;
        }

        Ok(())
    }
}

/// Get the value expression for any tree attribute argument variant.
pub(crate) fn tree_attribute_value_id(
    tree: &NodeTree,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    match tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => Some(*value),
    }
}

/// Return an argument value expression with transparent wrappers removed.
fn argument_transparent_value_id(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    tree_attribute_value_id(context.tree, argument_id)
        .map(|value_id| transparent_inner_expression(context, value_id))
}

/// Return a function declaration id from an expression when present.
fn expression_function_declaration_id(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Declaration>> {
    let Expression::Declaration(declaration_id) = tree.get(expression_id) else {
        return None;
    };

    if matches!(tree.get(*declaration_id), Declaration::Function { .. }) {
        Some(*declaration_id)
    } else {
        None
    }
}

/// Return whether a function declaration is a lambda.
fn declaration_is_lambda(tree: &NodeTree, declaration_id: LocalNodeId<Declaration>) -> bool {
    matches!(
        tree.get(declaration_id),
        Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
    )
}

/// Return a lambda body expression id with transparent wrappers removed.
fn lambda_body_expression_id(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> Option<LocalNodeId<Expression>> {
    let Declaration::Function {
        signature,
        body: Some(body_id),
        ..
    } = context.tree.get(declaration_id)
    else {
        return None;
    };
    if signature.kind != FunctionKind::Lambda {
        return None;
    }

    Some(transparent_inner_expression(context, *body_id))
}

/// Return a lambda declaration id from an argument when present.
fn argument_lambda_declaration_id(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Declaration>> {
    let value_id = argument_transparent_value_id(context, argument_id)?;
    let declaration_id = expression_function_declaration_id(context.tree, value_id)?;
    declaration_is_lambda(context.tree, declaration_id).then_some(declaration_id)
}

/// Return the receiver of a postfix-like expression node.
fn expression_postfix_receiver_id(expression: &Expression) -> Option<LocalNodeId<Expression>> {
    match expression {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. }
        | Expression::Parenthesized { expression: left }
        | Expression::Statement(left) => Some(*left),
        _ => None,
    }
}

/// Check whether a property value is complex enough to force breaks.
pub(crate) fn property_has_complex_value(
    context: &DestackFormatContext<'_>,
    property_id: LocalNodeId<Property>,
) -> bool {
    let tree = context.tree;

    // annotations on the property force complexity
    if context.has_annotation(property_id) {
        return true;
    }

    let property = tree.get(property_id);

    // field values and defaults can be complex
    if let Property::Field { value, default, .. } = property {
        // inspect the field value
        let value_is_complex = value.is_some_and(|value_id| {
            let value_expr = tree.get(value_id);
            is_complex_expression(tree, value_expr) || context.has_annotation(value_id)
        });

        // inspect the field default
        let default_is_complex = default.is_some_and(|default_id| {
            let default_expr = tree.get(default_id);
            is_complex_expression(tree, default_expr) || context.has_annotation(default_id)
        });

        return value_is_complex || default_is_complex;
    }

    // methods with bodies are always complex in object literals
    if let Property::Method { body, .. } = property {
        return body.is_some();
    }

    // spread properties inherit complexity from their value
    if let Property::Spread { value, .. } = property {
        let value_expr = tree.get(*value);
        return is_complex_expression(tree, value_expr) || context.has_annotation(*value);
    }

    false
}

/// Check whether a property contains a complex type value.
pub(crate) fn property_has_complex_type_value(
    context: &DestackFormatContext<'_>,
    property_id: LocalNodeId<Property>,
) -> bool {
    let tree = context.tree;

    if context.has_annotation(property_id) {
        return true;
    }

    let is_complex_type_expression = |expression_id: LocalNodeId<Expression>| {
        let expression = tree.get(expression_id);
        context.has_annotation(expression_id)
            || is_expression_breakable(tree, expression)
            || !is_trivial_expression(tree, expression)
    };

    match tree.get(property_id) {
        Property::Field { value, default, .. } => {
            value.is_some_and(is_complex_type_expression)
                || default.is_some_and(is_complex_type_expression)
        }
        Property::Method { body, .. } => body.is_some(),
        Property::Spread { value, .. } => is_complex_type_expression(*value),
    }
}

/// Decide whether tree attributes should force the element to break.
pub(crate) fn should_force_break_tree_attributes(
    context: &DestackFormatContext<'_>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    let tree = context.tree;
    let line_width = usize::from(context.options.line_width);

    // comments on attributes force a break
    if collection_nodes_have_annotations(context, arguments) {
        return true;
    }

    for argument_id in arguments {
        let Some(value_id) = tree_attribute_value_id(tree, *argument_id) else {
            continue;
        };

        // comments on attribute values force a break
        if context.has_annotation(value_id) {
            return true;
        }

        // preserve explicit multiline attribute values
        let value_span = context.span(value_id);
        if context.has_newline(value_span) {
            return true;
        }

        // collect value signals for complexity checks
        let value_source_len = expression_source_len(context, value_id);
        let value_expr = tree.get(value_id);

        // long attribute values should force multiline element layout
        if value_source_len > line_width {
            return true;
        }

        // complex object and array values should break the element
        match value_expr {
            Expression::ObjectExpression { properties, .. } => {
                // collect object signals
                let has_many_properties = properties.len() > 1;
                let has_complex_property = properties
                    .iter()
                    .copied()
                    .any(|property_id| property_has_complex_value(context, property_id));

                // break when the object is clearly complex
                let should_break_object =
                    collection_value_should_force_break(has_many_properties, has_complex_property);

                if should_break_object {
                    return true;
                }
            }
            Expression::ArrayExpression { elements } => {
                // collect array signals
                let has_many_elements = elements.len() > 1;
                let has_complex_element = elements.iter().any(|element_id| {
                    // annotations on the element force complexity
                    let element_has_annotation = context.has_annotation(*element_id);

                    // inspect the element value when present
                    let element_value_is_complex = tree_attribute_value_id(tree, *element_id)
                        .is_some_and(|element_value_id| {
                            let element_expr = tree.get(element_value_id);
                            is_complex_expression(tree, element_expr)
                                || context.has_annotation(element_value_id)
                        });

                    element_has_annotation || element_value_is_complex
                });

                // break when the array is clearly complex
                let should_break_array =
                    collection_value_should_force_break(has_many_elements, has_complex_element);

                if should_break_array {
                    return true;
                }
            }
            Expression::TreeExpression { elements, .. } => {
                // nested trees with children force a break
                let has_children = elements
                    .as_ref()
                    .is_some_and(|elements| !elements.is_empty());

                if has_children {
                    return true;
                }
            }
            _ => {}
        }
    }

    false
}

/// Check if an expression is huggable with the given configuration.
/// Return whether an expression is huggable in JSX position.
#[inline]
pub(crate) fn is_huggable_expression(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
    config: &HugOptions,
) -> bool {
    match tree.get(expression_id) {
        Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. } => true,
        Expression::Declaration(declaration_id) if config.allow_arrow_functions => {
            // arrow functions stay hugged when used as the only call argument
            if let Declaration::Function {
                signature,
                body: Some(_),
                ..
            } = tree.get(*declaration_id)
            {
                signature.kind == FunctionKind::Lambda
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Store arrow-specific layout facts for hugged argument formatting.
#[derive(Clone, Copy, Default)]
struct HuggedArrowLayoutSignals {
    is_arrow_function: bool,
    arrow_force_expand: bool,
    arrow_trailing_line_break_if_breaks: bool,
    arrow_trailing_comma_if_breaks: bool,
}

/// Collect arrow-specific layout signals for one hugged argument value.
fn collect_hugged_arrow_layout_signals(
    context: &DestackFormatContext<'_>,
    value_id: LocalNodeId<Expression>,
    config: &HugOptions,
) -> HuggedArrowLayoutSignals {
    if !config.allow_arrow_functions {
        return HuggedArrowLayoutSignals::default();
    }

    let tree = context.tree;
    let arrow_declaration_id = match tree.get(value_id) {
        Expression::Declaration(declaration_id) => match tree.get(*declaration_id) {
            Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda => {
                Some(*declaration_id)
            }
            _ => None,
        },
        _ => None,
    };

    let Some(declaration_id) = arrow_declaration_id else {
        return HuggedArrowLayoutSignals::default();
    };

    let (arrow_body_is_block, arrow_body_is_tree, arrow_body_is_lambda) =
        if let Declaration::Function {
            body: Some(body_id),
            ..
        } = tree.get(declaration_id)
        {
            let body_id = transparent_inner_expression(context, *body_id);
            let body_expr = tree.get(body_id);
            let is_block = matches!(body_expr, Expression::Block(_));
            let is_tree = matches!(body_expr, Expression::TreeExpression { .. });
            let is_lambda = matches!(
                body_expr,
                Expression::Declaration(nested_declaration_id)
                    if matches!(
                        tree.get(*nested_declaration_id),
                        Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
                    )
            );
            (is_block, is_tree, is_lambda)
        } else {
            (false, false, false)
        };

    let arrow_force_expand = lambda_expression_should_break(context, declaration_id)
        || expression_source_len(context, value_id) > usize::from(context.options.line_width);
    let arrow_trailing_line_break_if_breaks =
        !arrow_body_is_block && !arrow_body_is_tree && !arrow_body_is_lambda;
    let arrow_trailing_comma_if_breaks =
        arrow_trailing_line_break_if_breaks && !arrow_body_is_lambda;

    HuggedArrowLayoutSignals {
        is_arrow_function: true,
        arrow_force_expand,
        arrow_trailing_line_break_if_breaks,
        arrow_trailing_comma_if_breaks,
    }
}

/// Choose between inline and hugged candidate docs for one single hugged argument.
#[allow(clippy::too_many_arguments)]
fn choose_hugged_argument_layout<'ast, InlineDoc, HuggedDoc>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
    value_id: LocalNodeId<Expression>,
    config: &HugOptions,
    force_expand: bool,
    signals: HuggedArrowLayoutSignals,
    inline_format: InlineDoc,
    hugged_format: HuggedDoc,
) -> FormatResult<()>
where
    InlineDoc: Format<DestackFormatContext<'ast>>,
    HuggedDoc: Format<DestackFormatContext<'ast>>,
{
    // forced expansion always selects hugged output
    if force_expand {
        hugged_format.format(f)?;
        return Ok(());
    }

    // compute inline overflow bounds once
    let line_width = usize::from(f.context().options.line_width);
    let value_span = f.context().span(value_id);
    let (inline_compact_value_len, inline_source_value_len) =
        span_inline_char_bounds(f.context(), value_span);
    let inline_compact_candidate_len = config
        .open
        .len()
        .saturating_add(config.close.len())
        .saturating_add(inline_compact_value_len)
        .saturating_add(usize::from(config.force_trailing));
    let inline_source_candidate_len = config
        .open
        .len()
        .saturating_add(config.close.len())
        .saturating_add(inline_source_value_len)
        .saturating_add(usize::from(config.force_trailing));

    // fast inline path for non-arrow simple cases
    let can_use_inline_fast_path = !f.context().has_annotation(argument_id)
        && !f.context().has_annotation(value_id)
        && !signals.is_arrow_function
        && inline_source_candidate_len <= line_width;
    if can_use_inline_fast_path {
        f.context()
            .increment_counter("profile.jsx.hug.inline.fast_path", 1);
        inline_format.format(f)?;
        return Ok(());
    }

    // fast inline path for arrow values that do not force expand
    let can_use_arrow_inline_fast_path = !f.context().has_annotation(argument_id)
        && !f.context().has_annotation(value_id)
        && signals.is_arrow_function
        && !signals.arrow_force_expand
        && inline_source_candidate_len <= line_width;
    if can_use_arrow_inline_fast_path {
        f.context()
            .increment_counter("profile.jsx.hug.inline.arrow_fast_path", 1);
        inline_format.format(f)?;
        return Ok(());
    }

    // overflow guard picks hugged output without probing
    if inline_compact_candidate_len > line_width {
        f.context()
            .increment_counter("profile.jsx.hug.skip_probe_overflow", 1);
        hugged_format.format(f)?;
        return Ok(());
    }

    // deterministic fallback path
    let is_annotated =
        f.context().has_annotation(argument_id) || f.context().has_annotation(value_id);
    let should_hug = is_annotated || signals.arrow_force_expand;
    if should_hug {
        f.context()
            .increment_counter("profile.jsx.hug.deterministic.hug", 1);
        hugged_format.format(f)?;
    } else {
        f.context()
            .increment_counter("profile.jsx.hug.deterministic.inline", 1);
        inline_format.format(f)?;
    }

    Ok(())
}

/// Format a single-element argument list with hugging for expandable elements.
///
/// When a single object/array (or arrow function for calls) is the only argument,
/// format as `foo({...})` instead of `foo(\n    {...},\n)`.
/// Returns true if hugging was applied, false if regular list_like should be used.
pub(crate) fn format_hugged<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    arguments: &[LocalNodeId<Argument>],
    config: HugOptions,
    group_id: Option<GroupId>,
    force_expand: bool,
) -> FormatResult<bool> {
    // only hug single positional arguments
    if arguments.len() != 1 {
        return Ok(false);
    }

    // resolve and normalize the single argument value
    let argument_id = arguments[0];
    let Some(value_id) = argument_value(f.context().tree, argument_id) else {
        return Ok(false);
    };
    let value_id = transparent_inner_expression(f.context(), value_id);

    // guard non-huggable value kinds
    if !is_huggable_expression(f.context().tree, value_id, &config) {
        return Ok(false);
    }

    // multiline empty collections are not stable hugging candidates
    if !force_expand
        && f.context().node_has_newline(value_id)
        && match f.context().tree.get(value_id) {
            Expression::ObjectExpression { properties, .. } => properties.is_empty(),
            Expression::ArrayExpression { elements } => elements.is_empty(),
            _ => false,
        }
    {
        return Ok(false);
    }

    // multiline collection values can opt out of hugging by configuration
    if !force_expand
        && !config.allow_multiline_collection
        && f.context().node_has_newline(value_id)
        && matches!(
            f.context().tree.get(value_id),
            Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. }
        )
    {
        return Ok(false);
    }

    // collect arrow-specific layout facts once
    let signals = collect_hugged_arrow_layout_signals(f.context(), value_id, &config);
    let trailing_if_breaks = config.trailing_if_breaks;

    // build inline candidate doc
    let inline_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let inline_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write!(f, [token(config.open), argument_id])?;
            if config.force_trailing {
                write!(f, [token(",")])?;
            }
            write!(f, [token(config.close)])
        });

        if let Some(group_id) = group_id {
            group(&inline_inner).with_id(Some(group_id)).format(f)
        } else {
            write!(f, [inline_inner])
        }
    });

    // build hugged candidate doc
    let hugged_format_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        // argument annotations
        if config.handle_annotations {
            if signals.is_arrow_function {
                write!(f, [f.context().block_prefix_annotations(argument_id)])?;
            } else {
                write!(f, [f.context().any_prefix_annotations(argument_id)])?;
            }
        }

        // force expansion when arrow policy requires it
        if signals.arrow_force_expand {
            write!(f, [expand_parent()])?;
        }

        // opening delimiter
        write!(f, [token(config.open)])?;

        // value payload
        match f.context().tree.get(value_id) {
            Expression::ObjectExpression { ty, properties } => {
                if let Some(ty) = ty {
                    write!(f, [ty, space()])?;
                }
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write!(
                            f,
                            [
                                token("{"),
                                block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                    f.join_with(&format_args![
                                        token(","),
                                        soft_line_break_or_space()
                                    ])
                                    .entries(properties)
                                    .finish()?;
                                    write!(f, [if_group_breaks(&token(","))])
                                })),
                                token("}")
                            ]
                        )
                    }))
                    .should_expand(true)]
                )?;
            }
            Expression::ArrayExpression { elements } => {
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write!(
                            f,
                            [
                                token("["),
                                block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                    f.join_with(&format_args![
                                        token(","),
                                        soft_line_break_or_space()
                                    ])
                                    .entries(elements)
                                    .finish()?;
                                    write!(f, [if_group_breaks(&token(","))])
                                })),
                                token("]")
                            ]
                        )
                    }))
                    .should_expand(true)]
                )?;
            }
            Expression::Declaration(declaration_id) if config.allow_arrow_functions => {
                if let Some(group_id) = group_id {
                    write!(
                        f,
                        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                            write!(f, [declaration_id])
                        }))
                        .with_id(Some(group_id))]
                    )?;
                } else {
                    write!(f, [declaration_id])?;
                }
            }
            _ => {
                write!(f, [value_id])?;
            }
        }

        // trailing comma behavior
        if config.force_trailing {
            write!(f, [token(",")])?;
        } else if signals.arrow_trailing_comma_if_breaks || trailing_if_breaks {
            let comma = token(",");
            let trailing_comma = if let Some(group_id) = group_id {
                if_group_breaks(&comma).with_group_id(Some(group_id))
            } else {
                if_group_breaks(&comma)
            };
            write!(f, [trailing_comma])?;
        }

        // trailing line break behavior for arrow values
        if signals.arrow_trailing_line_break_if_breaks {
            let line_break = hard_line_break();
            let break_doc = if let Some(group_id) = group_id {
                if_group_breaks(&line_break).with_group_id(Some(group_id))
            } else {
                if_group_breaks(&line_break)
            };
            write!(f, [break_doc])?;
        }

        // closing delimiter
        write!(f, [token(config.close)])?;

        // trailing annotations
        if config.handle_annotations {
            write!(
                f,
                [f.context().any_infix_or_postfix_annotations(argument_id)]
            )?;
        }
        Ok(())
    });

    let hugged_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if trailing_if_breaks || group_id.is_some() {
            write!(
                f,
                [group(&hugged_format_inner)
                    .with_id(group_id)
                    .should_expand(signals.arrow_force_expand)]
            )?;
        } else {
            write!(f, [hugged_format_inner])?;
        }
        Ok(())
    });

    // choose final candidate
    choose_hugged_argument_layout(
        f,
        argument_id,
        value_id,
        &config,
        force_expand,
        signals,
        inline_format,
        hugged_format,
    )?;

    Ok(true)
}

/// Format one tree attribute by choosing between inline and hugged docs.
fn format_tree_attribute_inline_or_hugged<'ast, InlineDoc, HuggedDoc>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
    inline_format: InlineDoc,
    hugged_format: HuggedDoc,
) -> FormatResult<()>
where
    InlineDoc: Format<DestackFormatContext<'ast>>,
    HuggedDoc: Format<DestackFormatContext<'ast>>,
{
    // evaluate inline overflow bounds
    let line_width = usize::from(f.context().options.line_width);
    let value_span = f.context().span(value_id);
    let (inline_compact_value_len, inline_source_value_len) =
        span_inline_char_bounds(f.context(), value_span);
    let inline_compact_candidate_len = 3usize.saturating_add(inline_compact_value_len);
    let inline_source_candidate_len = 3usize.saturating_add(inline_source_value_len);

    // inline fast path for unannotated short values
    let can_use_inline_fast_path =
        !f.context().has_annotation(value_id) && inline_source_candidate_len <= line_width;
    if can_use_inline_fast_path {
        f.context()
            .increment_counter("profile.jsx.attribute.inline.fast_path", 1);
        inline_format.format(f)?;
        return Ok(());
    }

    // overflow guard picks hugged output
    if inline_compact_candidate_len > line_width {
        f.context()
            .increment_counter("profile.jsx.attribute.skip_probe_overflow", 1);
        hugged_format.format(f)?;
        return Ok(());
    }

    // deterministic fallback
    if f.context().has_annotation(value_id) {
        f.context()
            .increment_counter("profile.jsx.attribute.deterministic.hug", 1);
        hugged_format.format(f)?;
    } else {
        f.context()
            .increment_counter("profile.jsx.attribute.deterministic.inline", 1);
        inline_format.format(f)?;
    }

    Ok(())
}

/// Format a tree attribute object value using inline-or-hugged selection.
fn format_tree_attribute_object_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
    ty: Option<LocalNodeId<Expression>>,
    properties: Vec<LocalNodeId<Property>>,
) -> FormatResult<()> {
    let inline_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(f, [token("="), token("{"), value_id, token("}")])
    });

    let hugged_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(
            f,
            [
                token("="),
                token("{"),
                format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    if let Some(ty) = ty {
                        write!(f, [ty, space()])?;
                    }
                    write!(
                        f,
                        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                            write!(
                                f,
                                [
                                    token("{"),
                                    block_indent(&format_with(
                                        |f: &mut DestackFormatter<'ast, '_>| {
                                            f.join_with(&format_args![
                                                token(","),
                                                soft_line_break_or_space()
                                            ])
                                            .entries(&properties)
                                            .finish()?;
                                            write!(f, [if_group_breaks(&token(","))])
                                        }
                                    )),
                                    token("}")
                                ]
                            )
                        }))
                        .should_expand(true)]
                    )
                }),
                token("}")
            ]
        )
    });

    format_tree_attribute_inline_or_hugged(f, value_id, inline_format, hugged_format)
}

/// Format a tree attribute array value using inline-or-hugged selection.
fn format_tree_attribute_array_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
    elements: Vec<LocalNodeId<Argument>>,
) -> FormatResult<()> {
    let inline_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(f, [token("="), token("{"), value_id, token("}")])
    });

    let hugged_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(
            f,
            [
                token("="),
                token("{"),
                group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    write!(
                        f,
                        [
                            token("["),
                            block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                f.join_with(&format_args![token(","), soft_line_break_or_space()])
                                    .entries(&elements)
                                    .finish()?;
                                write!(f, [if_group_breaks(&token(","))])
                            })),
                            token("]")
                        ]
                    )
                }))
                .should_expand(true),
                token("}")
            ]
        )
    });

    format_tree_attribute_inline_or_hugged(f, value_id, inline_format, hugged_format)
}

/// Format a tree/JSX attribute value with hugging for objects and arrays.
pub(crate) fn format_tree_attribute_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    // tree attribute layout
    let tree = f.context().tree;

    match tree.get(value_id) {
        // object attribute value
        Expression::ObjectExpression { ty, properties } => {
            format_tree_attribute_object_value(f, value_id, *ty, properties.clone())?;
        }

        // array attribute value
        Expression::ArrayExpression { elements } => {
            format_tree_attribute_array_value(f, value_id, elements.clone())?;
        }

        // non-huggable values use regular braced formatting
        _ => {
            write!(f, [token("="), token("{"), value_id, token("}")])?;
        }
    }

    Ok(())
}

/// Return source text for a tree-related span.
pub(crate) fn tree_text_span_str(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<String> {
    let tree = context.tree;
    let Argument::Positional { value, .. } = tree.get(argument_id) else {
        return None;
    };

    let Expression::ScalarLiteral(ScalarLiteral::String(_)) = tree.get(*value) else {
        return None;
    };

    let span = context.span(*value);
    let span_str = context.file.get_span_str(span).unwrap_or_default();
    Some(span_str.to_owned())
}

/// Check whether a tree text child is whitespace-only.
pub(crate) fn tree_text_is_whitespace_only(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<(bool, bool)> {
    let tree = context.tree;
    let strings = context.strings;
    let Argument::Positional { value, .. } = tree.get(argument_id) else {
        return None;
    };

    if let Expression::ScalarLiteral(ScalarLiteral::String(string_id)) = tree.get(*value) {
        let span_str = tree_text_span_str(context, argument_id)?;
        if span_str.starts_with('"') || span_str.starts_with('\'') {
            let content = strings.get(*string_id);
            let has_non_whitespace = content.chars().any(|c| !c.is_whitespace());
            if has_non_whitespace {
                return Some((false, false));
            }
            let has_newline = content.contains(['\n', '\r']);
            return Some((true, has_newline));
        }
    }

    if let Expression::ScalarLiteral(ScalarLiteral::Character(value)) = tree.get(*value) {
        if !value.is_whitespace() {
            return Some((false, false));
        }
        let has_newline = matches!(value, '\n' | '\r');
        return Some((true, has_newline));
    }

    let span_str = tree_text_span_str(context, argument_id)?;
    let has_non_whitespace = span_str.chars().any(|c| !c.is_whitespace());
    if has_non_whitespace {
        return Some((false, false));
    }

    let has_newline = span_str.contains(['\n', '\r']);
    Some((true, has_newline))
}

/// Check whether a tree text child needs separator spaces for newline boundaries.
pub(crate) fn tree_text_boundary_separator_space(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<(bool, bool)> {
    let tree = context.tree;
    let strings = context.strings;

    // locate the raw text content
    let Argument::Positional { value, .. } = tree.get(argument_id) else {
        return None;
    };

    let Expression::ScalarLiteral(ScalarLiteral::String(string_id)) = tree.get(*value) else {
        return None;
    };

    let span_str = tree_text_span_str(context, argument_id)?;
    let text = if span_str.starts_with('"') || span_str.starts_with('\'') {
        Cow::Borrowed(strings.get(*string_id))
    } else {
        Cow::Owned(span_str)
    };

    // identify the boundary whitespace runs
    let leading_end = text
        .char_indices()
        .find(|(_, c)| !c.is_whitespace())
        .map_or(text.len(), |(index, _)| index);
    let trailing_start = text
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_whitespace())
        .map_or(0, |(index, c)| index + c.len_utf8());

    let leading_whitespace = &text[..leading_end];
    let trailing_whitespace = &text[trailing_start..];

    let has_leading_whitespace = !leading_whitespace.is_empty();
    let has_trailing_whitespace = !trailing_whitespace.is_empty();

    let leading_is_inline = has_leading_whitespace && !leading_whitespace.contains(['\n', '\r']);
    let trailing_is_inline = has_trailing_whitespace && !trailing_whitespace.contains(['\n', '\r']);

    let needs_leading_separator = has_leading_whitespace && !leading_is_inline;
    let needs_trailing_separator = has_trailing_whitespace && !trailing_is_inline;

    Some((needs_leading_separator, needs_trailing_separator))
}

/// Return whether source preserves an empty line between two tree child arguments.
pub(crate) fn tree_children_have_blank_line_between(
    context: &DestackFormatContext<'_>,
    previous_argument_id: LocalNodeId<Argument>,
    next_argument_id: LocalNodeId<Argument>,
) -> bool {
    let previous_span = context.span(previous_argument_id);
    let next_span = context.span(next_argument_id);
    if previous_span.file != next_span.file {
        return false;
    }
    if previous_span.end >= next_span.start {
        return false;
    }

    let between_span = Span::new(previous_span.file, previous_span.end, next_span.start);
    let between_source = context.span_str(between_span);
    between_source.contains("\n\n") || between_source.contains("\r\n\r\n")
}

/// Check whether a tree child expression should stay inline inside `{ ... }`.
pub(crate) fn tree_child_should_inline_braced_expression(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = argument_transparent_value_id(context, argument_id) else {
        return false;
    };
    let value_expr = context.tree.get(value_id);
    let argument_span = context.span(argument_id);
    let value_span = context.span(value_id);

    if argument_span.file == value_span.file {
        if argument_span.start < value_span.start
            && span_has_comment(
                context,
                Span::new(argument_span.file, argument_span.start, value_span.start),
            )
        {
            return false;
        }

        if value_span.end < argument_span.end
            && span_has_comment(
                context,
                Span::new(argument_span.file, value_span.end, argument_span.end),
            )
        {
            return false;
        }
    }

    if argument_has_line_comment_annotation(context, argument_id) {
        return false;
    }

    match value_expr {
        Expression::ScalarLiteral(ScalarLiteral::String(_))
        | Expression::ScalarLiteral(ScalarLiteral::Character(_)) => true,
        Expression::ArrayExpression { .. }
        | Expression::ObjectExpression { .. }
        | Expression::Call { .. }
        | Expression::TemplateExpression { .. }
        | Expression::TaggedTemplateExpression { .. }
        | Expression::Await { .. }
        | Expression::AwaitMaybe { .. }
        | Expression::Binary { .. }
        | Expression::Member { .. }
        | Expression::PrivateMember { .. }
        | Expression::Index { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. } => !expression_has_line_comment_annotation(context, value_id),
        Expression::If {
            kind: IfKind::Ternary,
            condition,
            then_expression,
            else_expression,
            ..
        } => {
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => return false,
            };
            if expression_has_line_comment_annotation(context, value_id)
                || expression_has_line_comment_annotation(context, condition_id)
                || expression_has_line_comment_annotation(context, *then_expression)
                || else_expression
                    .is_some_and(|else_id| expression_has_line_comment_annotation(context, else_id))
            {
                return false;
            }
            true
        }
        Expression::Declaration(declaration_id) => matches!(
            context.tree.get(*declaration_id),
            Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
        ),
        _ => false,
    }
}

/// Return whether one expression has a line-oriented slash comment annotation.
fn expression_has_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotation_ids) = context.annotations(expression_id) else {
        return false;
    };

    annotation_ids.into_iter().any(|annotation_id| {
        let Annotation::Comment { node, position } = context.annotation(annotation_id) else {
            return false;
        };

        let is_line_position = matches!(
            position,
            AnnotationPosition::LinePrefix
                | AnnotationPosition::LinePostfix
                | AnnotationPosition::LinePostfixBoundary
        );
        if !is_line_position {
            return false;
        }

        let comment = context.tree.get::<destack_ast::Comment>(node);
        comment.style == destack_ast::CommentStyle::Slash
    })
}

/// Return whether one tree argument has a line-oriented slash comment annotation.
fn argument_has_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotation_ids) = context.annotations(argument_id) else {
        return false;
    };

    annotation_ids.into_iter().any(|annotation_id| {
        let Annotation::Comment { node, position } = context.annotation(annotation_id) else {
            return false;
        };

        let is_line_position = matches!(
            position,
            AnnotationPosition::LinePrefix
                | AnnotationPosition::LinePostfix
                | AnnotationPosition::LinePostfixBoundary
        );
        if !is_line_position {
            return false;
        }

        let comment = context.tree.get::<destack_ast::Comment>(node);
        comment.style == destack_ast::CommentStyle::Slash
    })
}

/// Check whether a tree child forces the element to break.
pub(crate) fn tree_child_breaks_element(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = argument_transparent_value_id(context, argument_id) else {
        return false;
    };
    let value_expr = context.tree.get(value_id);
    let span = context.span(value_id);
    let is_text_node = matches!(
        value_expr,
        Expression::ScalarLiteral(ScalarLiteral::String(_))
    );
    let is_tree_node = matches!(value_expr, Expression::TreeExpression { .. });
    if !is_text_node && !is_tree_node && context.has_newline(span) {
        return true;
    }

    let has_line_comment_annotation = argument_has_line_comment_annotation(context, argument_id)
        || expression_has_line_comment_annotation(context, value_id);
    if (context.has_annotation(argument_id) || context.has_annotation(value_id))
        && !is_text_node
        && !has_line_comment_annotation
    {
        return true;
    }

    match value_expr {
        Expression::Stub => true,
        Expression::If {
            kind: IfKind::Ternary,
            ..
        } => expression_source_len(context, value_id) > usize::from(context.options.line_width),
        Expression::Block(_) | Expression::Match { .. } => true,
        Expression::Declaration(declaration_id) => {
            lambda_body_is_complex_for_tree(context, *declaration_id)
        }
        Expression::TreeExpression { .. } => false,
        _ => expression_has_complex_callback(context, value_id),
    }
}

/// Check whether a lambda body is complex enough to force tree breaking.
pub(crate) fn lambda_body_is_complex_for_tree(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let Some(body_id) = lambda_body_expression_id(context, declaration_id) else {
        return false;
    };

    matches!(
        context.tree.get(body_id),
        Expression::Block(_) | Expression::TreeExpression { .. }
    )
}

/// Check whether an argument is a lambda with a complex body for tree literals.
pub(crate) fn argument_is_complex_callback(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(declaration_id) = argument_lambda_declaration_id(context, argument_id) else {
        return false;
    };

    lambda_body_is_complex_for_tree(context, declaration_id)
}

/// Check whether an argument is a lambda with a block body.
pub(crate) fn argument_is_block_callback(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(declaration_id) = argument_lambda_declaration_id(context, argument_id) else {
        return false;
    };

    lambda_body_expression_id(context, declaration_id)
        .is_some_and(|body_id| matches!(context.tree.get(body_id), Expression::Block(_)))
}

/// Check whether an argument is an object literal expression.
pub(crate) fn argument_is_object_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_transparent_value_id(context, argument_id).is_some_and(|value_id| {
        matches!(
            context.tree.get(value_id),
            Expression::ObjectExpression { .. }
        )
    })
}

/// Check whether an argument is an array literal expression.
pub(crate) fn argument_is_array_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_transparent_value_id(context, argument_id).is_some_and(|value_id| {
        matches!(
            context.tree.get(value_id),
            Expression::ArrayExpression { .. }
        )
    })
}

/// Check whether an argument is a template literal expression.
pub(crate) fn argument_is_template_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_transparent_value_id(context, argument_id).is_some_and(|value_id| {
        matches!(
            context.tree.get(value_id),
            Expression::TemplateExpression { .. }
        )
    })
}

/// Check whether an argument is a lambda expression.
pub(crate) fn argument_is_lambda_expression(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_lambda_declaration_id(context, argument_id).is_some()
}

/// Check whether an argument is a function expression.
pub(crate) fn argument_is_function_expression(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = argument_transparent_value_id(context, argument_id) else {
        return false;
    };
    let Some(declaration_id) = expression_function_declaration_id(context.tree, value_id) else {
        return false;
    };

    !declaration_is_lambda(context.tree, declaration_id)
}

/// Check whether an expression contains a call with a complex callback.
pub(crate) fn expression_has_complex_callback(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let tree = context.tree;

    // unwrap transparent wrappers
    let expression_id = transparent_inner_expression(context, expression_id);

    // walk the expression shape looking for callback lambdas
    match tree.get(expression_id) {
        Expression::Call {
            left,
            dynamic_arguments,
            ..
        }
        | Expression::New {
            left,
            dynamic_arguments,
            ..
        } => {
            // check the call arguments
            let has_complex_argument = dynamic_arguments
                .iter()
                .any(|arg_id| argument_is_complex_callback(context, *arg_id));

            if has_complex_argument {
                return true;
            }

            // check chained receivers
            expression_has_complex_callback(context, *left)
        }
        expression if let Some(left) = expression_postfix_receiver_id(expression) => {
            expression_has_complex_callback(context, left)
        }
        Expression::Declaration(declaration_id) => {
            lambda_body_is_complex_for_tree(context, *declaration_id)
        }
        Expression::Block(block_id) => {
            let block = tree.get(*block_id);

            // scan block expressions for complex callbacks
            block
                .expressions
                .iter()
                .any(|expr_id| expression_has_complex_callback(context, *expr_id))
        }
        _ => false,
    }
}

/// Decide whether a tree literal should break across multiple lines.
pub(crate) fn tree_literal_should_break(
    context: &DestackFormatContext<'_>,
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
) -> bool {
    let force_break_attributes = arguments
        .as_ref()
        .is_some_and(|arguments| should_force_break_tree_attributes(context, arguments));

    let Some(elements) = elements else {
        return force_break_attributes;
    };

    if elements.is_empty() {
        return force_break_attributes;
    }

    let tree = context.tree;
    let element_children_count = elements
        .iter()
        .filter(|elem_id| {
            let arg = tree.get(**elem_id);
            if let Argument::Positional { value, .. } = arg {
                matches!(tree.get(*value), Expression::TreeExpression { .. })
            } else {
                false
            }
        })
        .count();

    let has_breaking_child = elements
        .iter()
        .any(|elem_id| tree_child_breaks_element(context, *elem_id));

    let has_tree_child = element_children_count > 0;
    let has_single_text_child = elements.len() == 1 && !has_tree_child && !has_breaking_child;

    force_break_attributes || has_breaking_child || (has_tree_child && !has_single_text_child)
}

/// Return whether a tree literal should be wrapped in parentheses when it breaks.
pub(crate) fn tree_literal_wraps_on_break(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return true;
    };

    match parent_type {
        NodeType::Expression => {
            let parent_id = LocalNodeId::<Expression>::new(parent_id);
            match context.tree.get(parent_id) {
                // explicit parentheses already control wrapping
                Expression::Parenthesized { .. } => false,
                // jsx-like containers and conditional branches keep children unwrapped
                Expression::ArrayExpression { .. }
                | Expression::TupleExpression { .. }
                | Expression::TreeExpression { .. }
                | Expression::If {
                    kind: IfKind::Ternary,
                    ..
                } => false,
                // standalone jsx statements stay unwrapped
                Expression::Statement(_) => false,
                // return handles jsx wrapping at the statement formatter level
                Expression::Return { .. } => false,
                _ => true,
            }
        }
        NodeType::Argument => {
            let Some((grand_id, grand_type)) = context.parent_by_id(parent_id) else {
                return true;
            };
            if grand_type != NodeType::Expression {
                return true;
            }

            let grand_id = LocalNodeId::<Expression>::new(grand_id);
            !matches!(
                context.tree.get(grand_id),
                Expression::Call { .. }
                    | Expression::New { .. }
                    | Expression::ArrayExpression { .. }
                    | Expression::TupleExpression { .. }
                    | Expression::TreeExpression { .. }
                    | Expression::If {
                        kind: IfKind::Ternary,
                        ..
                    }
            )
        }
        _ => true,
    }
}

/// Format a tree literal expression with optional wrap-on-break parentheses.
pub(crate) fn format_tree_literal_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: &Option<LocalNodeId<Expression>>,
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
) -> FormatResult<()> {
    if !tree_literal_wraps_on_break(f.context(), node_id) {
        return format_tree_literal(f, node_id, left, arguments, elements);
    }

    let should_expand = tree_literal_should_break(f.context(), arguments, elements);

    write!(
        f,
        [group(&format_with(|f| {
            write!(f, [if_group_breaks(&token("("))])?;

            let formatted_tree =
                format_with(|f| format_tree_literal(f, node_id, left, arguments, elements));
            if should_expand {
                write!(f, [block_indent(&formatted_tree)])?;
            } else {
                write!(f, [soft_block_indent(&formatted_tree)])?;
            }

            write!(f, [if_group_breaks(&token(")"))])?;
            Ok(())
        }))
        .should_expand(should_expand)]
    )
}

/// Format a tree literal.
/// Format a tree literal expression.
#[inline]
pub(crate) fn format_tree_literal<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _expression_id: LocalNodeId<Expression>,
    left: &Option<LocalNodeId<Expression>>,
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
) -> FormatResult<()> {
    let force_break_attributes = arguments
        .as_ref()
        .is_some_and(|arguments| should_force_break_tree_attributes(f.context(), arguments));

    write!(
        f,
        [group(&format_with(|f| {
            // header
            write!(
                f,
                [group(&format_with(|f| {
                    // <
                    write!(f, [token("<")])?;
                    // left
                    if let Some(left) = left {
                        write!(f, [left])?;
                    }
                    // arguments
                    if let Some(arguments) = arguments {
                        let single_attr_per_line = f.context().options.single_attribute_per_line;
                        let bracket_same_line = f.context().options.bracket_same_line;

                        // separator between attributes
                        let attr_separator: &dyn Format<DestackFormatContext<'ast>> =
                            if force_break_attributes
                                || (single_attr_per_line && arguments.len() > 1)
                            {
                                &hard_line_break()
                            } else {
                                &soft_line_break_or_space()
                            };

                        // format attribute list
                        let format_attrs = format_with(|f| {
                            f.join_with(attr_separator)
                                .entries(arguments.iter().map(|argument| TreeExpressionArgument {
                                    argument_id: *argument,
                                }))
                                .finish()
                        });

                        // complex attributes should expand the element
                        if force_break_attributes {
                            write!(f, [expand_parent()])?;
                        }

                        // when bracket_same_line is true, don't add trailing line break before >
                        // when false (default), soft_block_indent adds trailing soft_line_break
                        if bracket_same_line {
                            // avoid a trailing break before `>` when bracket_same_line is enabled
                            if force_break_attributes {
                                write!(
                                    f,
                                    [
                                        if_group_fits_on_line(&space()),
                                        indent(&format_args![hard_line_break(), format_attrs])
                                    ]
                                )?;
                            } else {
                                write!(
                                    f,
                                    [
                                        if_group_fits_on_line(&space()),
                                        indent(&format_args![soft_line_break(), format_attrs])
                                    ]
                                )?;
                            }
                        } else {
                            // force expansion for complex attributes in the default layout
                            if force_break_attributes {
                                write!(
                                    f,
                                    [
                                        if_group_fits_on_line(&space()),
                                        group(&soft_block_indent(&format_attrs))
                                            .should_expand(true)
                                    ]
                                )?;
                            } else {
                                write!(
                                    f,
                                    [
                                        if_group_fits_on_line(&space()),
                                        soft_block_indent(&format_attrs)
                                    ]
                                )?;
                            }
                        }
                    }
                    // /
                    if elements.is_none() {
                        let bracket_same_line = f.context().options.bracket_same_line;
                        let has_attributes = arguments.is_some();
                        if left.is_some() || has_attributes {
                            if has_attributes {
                                // space before /> when inline, or when bracket_same_line is true
                                if bracket_same_line {
                                    write!(f, [if_group_breaks(&space())])?;
                                }
                                write!(f, [if_group_fits_on_line(&space())])?;
                            } else {
                                write!(f, [space()])?;
                            }
                        }
                        write!(f, [token("/")])?;
                    }
                    // >
                    write!(f, [token(">")])?;
                    Ok(())
                }))]
            )?;

            // body
            if let Some(elements) = elements {
                let write_closing_tag = |f: &mut DestackFormatter<'ast, '_>| -> FormatResult<()> {
                    write!(f, [token("</")])?;
                    if let Some(left) = left {
                        let left_expression = f.context().tree.get(*left);

                        if let Expression::Path { path, .. } = left_expression {
                            write!(f, [path])?;
                        } else {
                            write!(f, [left])?;
                        }
                    }
                    write!(f, [token(">")])?;
                    Ok(())
                };

                // preserve compact empty paired tags
                if elements.is_empty() {
                    write_closing_tag(f)?;
                    return Ok(());
                }

                // check child types for formatting decisions
                let tree = f.context().tree;
                let element_children_count = elements
                    .iter()
                    .filter(|elem_id| {
                        let arg = tree.get(**elem_id);
                        if let Argument::Positional { value, .. } = arg {
                            matches!(tree.get(*value), Expression::TreeExpression { .. })
                        } else {
                            false
                        }
                    })
                    .count();
                let all_tree_children = element_children_count == elements.len();
                let only_tree_or_comment_children = elements.iter().all(|elem_id| {
                    let arg = tree.get(*elem_id);
                    let value_id = match arg {
                        Argument::Positional { value, .. }
                        | Argument::Spread { value, .. }
                        | Argument::Named { value, .. }
                        | Argument::Labeled { value, .. } => *value,
                    };
                    let value_id = transparent_inner_expression(f.context(), value_id);
                    matches!(
                        tree.get(value_id),
                        Expression::TreeExpression { .. } | Expression::Stub
                    )
                });

                // check if any child forces a break
                let child_breaks = elements
                    .iter()
                    .map(|elem_id| tree_child_breaks_element(f.context(), *elem_id))
                    .collect::<Vec<_>>();
                let has_breaking_child = child_breaks.iter().any(|breaks| *breaks);

                // force breaking when:
                // - attributes require a break, OR
                // - any child has complex content, such as callbacks with block bodies, OR
                // - there is at least one tree child, unless the only child is plain text
                let has_tree_child = element_children_count > 0;
                let has_single_text_child =
                    elements.len() == 1 && !has_tree_child && !has_breaking_child;
                let force_break = force_break_attributes
                    || has_breaking_child
                    || (has_tree_child && !has_single_text_child);

                // format children using TreeExpressionArgument for proper brace handling
                let format_children = format_with(|f| {
                    // multiline tree literals keep one child per line for stable layout
                    if force_break && elements.len() > 1 {
                        let mut wrote_child = false;
                        let mut pending_blank_line = false;
                        let mut previous_emitted_argument: Option<LocalNodeId<Argument>> = None;

                        for elem_id in elements {
                            let whitespace_info =
                                tree_text_is_whitespace_only(f.context(), *elem_id);
                            let is_whitespace_only = whitespace_info
                                .is_some_and(|(is_whitespace_only, _)| is_whitespace_only);
                            let has_blank_line =
                                whitespace_info.is_some_and(|(_, has_blank_line)| has_blank_line);

                            let argument_span = f.context().span(*elem_id);
                            let argument_source = f.context().span_str(argument_span);
                            let is_braced_whitespace =
                                argument_source.trim_start().starts_with('{')
                                    && argument_source.trim_end().ends_with('}')
                                    && is_whitespace_only;

                            if is_whitespace_only && !is_braced_whitespace {
                                if has_blank_line && wrote_child {
                                    pending_blank_line = true;
                                }
                                continue;
                            }

                            // keep explicit `{ " " }` style separators attached to previous child
                            if is_braced_whitespace {
                                if !wrote_child {
                                    continue;
                                }

                                write!(
                                    f,
                                    [TreeExpressionArgument {
                                        argument_id: *elem_id
                                    }]
                                )?;
                                continue;
                            }

                            if wrote_child {
                                let source_has_blank_line =
                                    previous_emitted_argument.is_some_and(|previous_argument_id| {
                                        tree_children_have_blank_line_between(
                                            f.context(),
                                            previous_argument_id,
                                            *elem_id,
                                        )
                                    });

                                if pending_blank_line || source_has_blank_line {
                                    write!(f, [empty_line()])?;
                                    pending_blank_line = false;
                                } else {
                                    write!(f, [hard_line_break()])?;
                                }
                            }

                            write!(
                                f,
                                [TreeExpressionArgument {
                                    argument_id: *elem_id
                                }]
                            )?;

                            wrote_child = true;
                            previous_emitted_argument = Some(*elem_id);
                        }

                        return Ok(());
                    }

                    // when all children are tree elements, keep one element per line
                    if (all_tree_children || only_tree_or_comment_children) && elements.len() > 1 {
                        for (index, elem_id) in elements.iter().enumerate() {
                            if index > 0 {
                                write!(f, [hard_line_break()])?;
                            }

                            write!(
                                f,
                                [TreeExpressionArgument {
                                    argument_id: *elem_id
                                }]
                            )?;
                        }

                        return Ok(());
                    }

                    // otherwise, use fill so mixed content can share lines when it fits
                    let inline_elements = elements
                        .iter()
                        .copied()
                        .filter(|elem_id| {
                            let whitespace_info =
                                tree_text_is_whitespace_only(f.context(), *elem_id);
                            let is_whitespace_only = whitespace_info
                                .is_some_and(|(is_whitespace_only, _)| is_whitespace_only);
                            if !is_whitespace_only {
                                return true;
                            }

                            let argument_span = f.context().span(*elem_id);
                            let argument_source = f.context().span_str(argument_span);
                            let is_braced_whitespace =
                                argument_source.trim_start().starts_with('{')
                                    && argument_source.trim_end().ends_with('}');

                            is_braced_whitespace
                        })
                        .collect::<Vec<_>>();
                    if inline_elements.is_empty() {
                        return Ok(());
                    }

                    let separators = {
                        let context = f.context();

                        let whitespace_flags = inline_elements
                            .iter()
                            .map(|elem_id| tree_text_is_whitespace_only(context, *elem_id))
                            .map(|info| info.unwrap_or((false, false)))
                            .collect::<Vec<_>>();
                        let boundary_spaces = inline_elements
                            .iter()
                            .map(|elem_id| {
                                tree_text_boundary_separator_space(context, *elem_id)
                                    .unwrap_or((false, false))
                            })
                            .collect::<Vec<_>>();
                        let inline_child_breaks = inline_elements
                            .iter()
                            .map(|elem_id| tree_child_breaks_element(context, *elem_id))
                            .collect::<Vec<_>>();

                        // compute the spacing decisions between adjacent children
                        let mut separators = Vec::with_capacity(inline_elements.len());
                        separators.push((false, false));

                        for index in 1..inline_elements.len() {
                            let prev_is_whitespace_only = whitespace_flags[index - 1].0;
                            let current_is_whitespace_only = whitespace_flags[index].0;
                            let force_hard_break =
                                inline_child_breaks[index - 1] || inline_child_breaks[index];

                            if prev_is_whitespace_only || current_is_whitespace_only {
                                separators.push((false, force_hard_break));
                                continue;
                            }

                            let prev_trailing_space = boundary_spaces[index - 1].1;
                            let current_leading_space = boundary_spaces[index].0;
                            let should_insert_space_inline =
                                prev_trailing_space || current_leading_space;
                            separators.push((should_insert_space_inline, force_hard_break));
                        }

                        separators
                    };

                    // render children using fill with the precomputed separators
                    let mut fill = f.fill();

                    for (index, elem_id) in inline_elements.iter().enumerate() {
                        let (should_insert_space_inline, force_break) = separators[index];

                        // build a separator doc for fill
                        let separator = format_with(|f| {
                            if index == 0 {
                                return Ok(());
                            }

                            if force_break {
                                write!(f, [hard_line_break()])?;
                            } else if should_insert_space_inline {
                                write!(f, [soft_line_break_or_space()])?;
                            } else {
                                write!(f, [token("")])?;
                            }

                            Ok(())
                        });

                        // add the child to the fill output
                        let entry = TreeExpressionArgument {
                            argument_id: *elem_id,
                        };
                        fill.entry(&separator, &entry);
                    }

                    fill.finish()
                });

                if force_break {
                    write!(f, [block_indent(&group(&format_children))])?;
                } else {
                    // use soft indent: stays on one line if it fits
                    write!(f, [group(&soft_block_indent(&format_children))])?;
                }

                // closing tag uses path only, no static arguments
                write_closing_tag(f)?;
            }

            Ok(())
        }))]
    )
}

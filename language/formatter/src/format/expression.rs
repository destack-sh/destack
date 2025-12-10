use destack_ast::{
    Argument, Asynchrony, Declarator, DependencyItem, DependencyKind, DependencyMode, Expression,
    ForEachKind, IfKind, Keyword, LocalNodeId, Mutability, NodeTree, PostfixPosition, Property,
    TypeUnaryOperator, WhileKind, YieldCardinality,
};
use destack_fir::format::{BestFittingMode, FormatError};
use destack_fir::prelude::*;
use destack_fir::{best_fitting, format_args, write};
use destack_source::StringId;
use smallvec::{SmallVec, smallvec};

use crate::argument::list_like;
use crate::block::format_block;
use crate::literal::{format_scalar_literal, format_template_literal};
use crate::{
    DestackFormatContext, DestackFormatter, FormatNode, empty_block_with_infix_annotations,
};

/// Tree fragment argument (with `=` instead of `: `)
#[derive(Debug, Clone, PartialEq)]
struct TreeExpressionArgument {
    argument_id: LocalNodeId<Argument>,
}

impl<'ast> Format<DestackFormatContext<'ast>> for TreeExpressionArgument {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(self.argument_id)])?;

        let argument = f.context().tree.get(self.argument_id);
        match argument {
            Argument::Named { name, value } => {
                // name
                write!(f, [name])?;
                // value
                write!(f, [token("="), value])?;
            }
            Argument::Labeled { label, value } => {
                // label
                write!(f, [label])?;
                // value
                write!(f, [token(":"), space(), value])?;
            }
            Argument::Positional { value } => {
                // value
                write!(f, [value])?;
            }
            Argument::Spread { value } => {
                // keyword
                write!(f, [token("...")])?;
                // value
                write!(f, [value])?;
            }
        }

        write!(
            f,
            [f.context()
                .any_infix_or_postfix_annotations(self.argument_id)]
        )?;

        Ok(())
    }
}

/// Walk a chain of if expressions and collect the if/else if/else nodes.
pub(crate) fn format_if_else_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    // walk the chain
    let mut next_if_id = node_id;
    loop {
        let if_node = f.context().tree.get(next_if_id);
        match if_node {
            // if or else if
            Expression::If {
                kind: _, // we turn everything into regular ifs
                condition,
                then_expression: then_expression_id,
                else_expression: else_expression_id,
            } => {
                // if <condition>
                write!(
                    f,
                    [
                        Keyword::If,
                        space(),
                        token("("),
                        condition,
                        token(")"),
                        space()
                    ]
                )?;

                // then block
                let then_expression = f.context().tree.get(*then_expression_id);
                match then_expression {
                    Expression::Block(block_id) => {
                        write!(f, [f.context().any_prefix_annotations(*then_expression_id)])?;
                        format_block(f, *block_id)?;
                        write!(
                            f,
                            [f.context()
                                .any_infix_or_postfix_annotations(*then_expression_id)]
                        )?;
                    }
                    // something else
                    _ => write!(f, [*then_expression_id])?,
                }

                // postfix annotations
                if next_if_id != node_id {
                    write!(f, [f.context().any_postfix_annotations(next_if_id)])?;
                }

                // next node
                if let Some(else_expression) = else_expression_id {
                    if !f.context().has_prefix_annotation(*else_expression)
                        && !f.context().has_postfix_annotation(*then_expression_id)
                    {
                        write!(f, [space()])?;
                    }
                    match f.context().tree.get(*else_expression) {
                        // else if
                        Expression::If { .. } => {
                            write!(f, [f.context().any_prefix_annotations(*else_expression)])?;
                            write!(f, [Keyword::Else, space()])?;
                            // (postfix is covered by the next if above)
                            next_if_id = *else_expression;
                        }
                        // else
                        Expression::Block(else_block_id) => {
                            write!(f, [f.context().any_prefix_annotations(*else_expression)])?;
                            write!(f, [Keyword::Else, space()])?;
                            format_block(f, *else_block_id)?;
                            write!(f, [f.context().any_postfix_annotations(*else_expression)])?;
                            break;
                        }
                        // something else
                        _ => {
                            write!(f, [Keyword::Else, space(), *else_expression])?;
                            break;
                        }
                    }
                } else {
                    // bare if
                    break;
                }
            }
            // shouldn't be anything else
            _ => {
                return Err(FormatError::SyntaxError {
                    message: "unexpected expression kind for if chain",
                });
            }
        }
    }
    Ok(())
}

/// Format a member expression without considering chaining.
#[inline]
fn format_member_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Member {
        left,
        name,
        static_arguments,
    } = f.context().tree.get(node_id)
    {
        write!(f, [*left, token("."), *name])?;
        if let Some(static_arguments) = static_arguments {
            write!(f, [list_like("<", ">", ",", static_arguments)])?;
        }
    } else {
        debug_assert!(false, "unexpected expression kind for member formatter");
    }
    Ok(())
}

/// Format an index expression without considering chaining.
#[inline]
fn format_index_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Index {
        position,
        left,
        index,
    } = f.context().tree.get(node_id)
    {
        write!(f, [*left])?;
        if *position == PostfixPosition::Indirect {
            write!(f, [token(".")])?;
        }
        if let Some(index) = index {
            write!(f, [token("["), *index, token("]")])?;
        } else {
            write!(f, [token("[]")])?;
        }
    } else {
        debug_assert!(false, "unexpected expression kind for index formatter");
    }
    Ok(())
}

/// Format a call expression without considering chaining.
#[inline]
fn format_call_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Call {
        position,
        left,
        static_arguments,
        dynamic_arguments,
    } = f.context().tree.get(node_id)
    {
        write!(f, [*left])?;
        if *position == PostfixPosition::Indirect {
            write!(f, [token(".")])?;
        }
        if let Some(static_arguments) = static_arguments {
            write!(f, [list_like("<", ">", ",", static_arguments)])?;
        }
        write!(f, [list_like("(", ")", ",", dynamic_arguments)])?;
    } else {
        debug_assert!(false, "unexpected expression kind for call formatter");
    }
    Ok(())
}

/// Format a maybe expression without considering chaining.
#[inline]
fn format_maybe_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Maybe { left, position } = f.context().tree.get(node_id) {
        write!(f, [*left])?;
        match position {
            PostfixPosition::Direct => write!(f, [token("?")])?,
            PostfixPosition::Indirect => write!(f, [token("."), token("?")])?,
        }
    } else {
        debug_assert!(false, "unexpected expression kind for maybe formatter");
    }
    Ok(())
}

/// The head of a chain before any postfix operations.
#[derive(Clone)]
enum ChainExpressionBaseHead {
    Path {
        segment: StringId,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    Expression(LocalNodeId<Expression>),
}

/// The initial portion of the chain including direct postfix ops.
#[derive(Clone)]
struct ChainExpressionBase {
    head: ChainExpressionBaseHead,
    body: Vec<ChainExpression>,
}

/// One operation in an expression chain.
#[derive(Clone)]
enum ChainExpression {
    /// Member expression.
    Member {
        segment: StringId,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Call expression.
    Call {
        position: PostfixPosition,
        dynamic_arguments: Vec<LocalNodeId<Argument>>,
    },
    /// Index expression.
    Index {
        position: PostfixPosition,
        index: Option<LocalNodeId<Expression>>,
    },
    /// Maybe expression.
    Maybe { position: PostfixPosition },
}

/// Format the base portion of the chain.
fn format_chain_base<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    base: &ChainExpressionBase,
) -> FormatResult<()> {
    match &base.head {
        ChainExpressionBaseHead::Path {
            segment,
            static_arguments,
        } => {
            write!(f, [*segment])?;
            if let Some(arguments) = static_arguments {
                write!(f, [list_like("<", ">", ",", arguments)])?;
            }
        }
        ChainExpressionBaseHead::Expression(node_id) => {
            let expression = f.context().tree.get(*node_id);
            debug_assert!(
                !matches!(
                    expression,
                    Expression::Member { .. }
                        | Expression::Call { .. }
                        | Expression::Index { .. }
                        | Expression::Maybe { .. }
                ),
                "chain base expression should not be another chain node"
            );
            write!(f, [*node_id])?;
        }
    }

    for op in &base.body {
        format_chain_expression(f, op)?;
    }

    Ok(())
}

/// Format one chained operation.
fn format_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    op: &ChainExpression,
) -> FormatResult<()> {
    match op {
        ChainExpression::Member {
            segment,
            static_arguments,
        } => {
            write!(f, [token(".")])?;
            write!(f, [*segment])?;
            if let Some(arguments) = static_arguments {
                write!(f, [list_like("<", ">", ",", arguments)])?;
            }
        }
        ChainExpression::Call {
            position,
            dynamic_arguments,
        } => {
            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }
            write!(f, [list_like("(", ")", ",", dynamic_arguments)])?;
        }
        ChainExpression::Index { position, index } => {
            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }
            if let Some(index) = index {
                write!(f, [token("["), *index, token("]")])?;
            } else {
                write!(f, [token("[]")])?;
            }
        }
        ChainExpression::Maybe { position } => match position {
            PostfixPosition::Direct => write!(f, [token("?")])?,
            PostfixPosition::Indirect => {
                write!(f, [token("."), token("?")])?;
            }
        },
    }
    Ok(())
}

/// Format all operations for one chain line.
fn format_chain_expression_line<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    ops: &[ChainExpression],
) -> FormatResult<()> {
    for op in ops {
        format_chain_expression(f, op)?;
    }
    Ok(())
}

/// Group chain operations into the segments that should share lines.
fn group_chain_expression_lines(
    operations: Vec<ChainExpression>,
) -> Vec<SmallVec<[ChainExpression; 2]>> {
    let mut lines = Vec::new();
    let mut iter = operations.into_iter().peekable();
    while let Some(op) = iter.next() {
        let mut line = smallvec![op.clone()];
        match op {
            // (maybe)
            ChainExpression::Maybe { .. } => {
                match iter.peek() {
                    // (maybe, member)
                    Some(ChainExpression::Member { .. }) => {
                        line.push(iter.next().unwrap());
                        // (maybe, member, index | call)
                        if let Some(ChainExpression::Index { .. } | ChainExpression::Call { .. }) =
                            iter.peek()
                        {
                            line.push(iter.next().unwrap());
                        }
                    }
                    // (maybe, index | call)
                    Some(ChainExpression::Index { .. } | ChainExpression::Call { .. }) => {
                        line.push(iter.next().unwrap());
                    }
                    _ => {}
                }
            }
            // (member)
            ChainExpression::Member { .. } => {
                // (member, index | call)
                if let Some(ChainExpression::Call { .. } | ChainExpression::Index { .. }) =
                    iter.peek()
                {
                    line.push(iter.next().unwrap());
                }
            }
            // (index | call)
            ChainExpression::Index { .. } | ChainExpression::Call { .. } => {
                // end of line
            }
        }
        lines.push(line);
    }

    lines
}

/// Check whether the expression is part of a member/call/maybe/index chain.
fn is_expression_chain(tree: &NodeTree, node_id: LocalNodeId<Expression>) -> bool {
    match tree.get(node_id) {
        Expression::Member { left, .. }
        | Expression::Call { left, .. }
        | Expression::Index { left, .. }
        | Expression::Maybe { left, .. } => is_chain_expression(tree.get(*left)),
        _ => false,
    }
}

/// Format a member/call/maybe/index chain with prettier-style breaking.
pub(crate) fn format_expression_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let tree = f.context().tree;

    // collect the nodes that belong to this chain
    let mut chain = Vec::new();
    let mut current = node_id;
    loop {
        chain.push(current);
        let next = match tree.get(current) {
            Expression::Member { left, .. }
            | Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left, .. } => Some(*left),
            _ => None,
        };

        if let Some(next_id) = next {
            current = next_id;
        } else {
            break;
        }
    }
    chain.reverse();
    let root_id = *chain
        .first()
        .expect("member/call/maybe/index chain must contain at least one node");

    // gather operations while breaking path roots into individual segments
    let mut body: Vec<ChainExpression> = Vec::new();
    let mut base_head = ChainExpressionBaseHead::Expression(root_id);

    // break leading path expression into individual segments
    if let Expression::Path {
        path,
        static_arguments,
    } = tree.get(root_id)
    {
        let mut segments = path.segments.clone().into_iter();
        let static_arguments = static_arguments.clone();
        let first_segment = segments.next().expect("path is not empty");
        let remaining_segments: Vec<StringId> = segments.collect();

        // path base (keeps static arguments if there are no remaining segments)
        let base_static_arguments = if remaining_segments.is_empty() {
            static_arguments.clone()
        } else {
            None
        };
        base_head = ChainExpressionBaseHead::Path {
            segment: first_segment,
            static_arguments: base_static_arguments,
        };

        // path rest (tail segments)
        let tail_len = remaining_segments.len();
        for (index, segment) in remaining_segments.into_iter().enumerate() {
            // last segment keeps static arguments if there are any
            let static_args = if tail_len != 0 && index + 1 == tail_len {
                static_arguments.clone()
            } else {
                None
            };
            body.push(ChainExpression::Member {
                segment,
                static_arguments: static_args,
            });
        }
    }
    let mut base = ChainExpressionBase {
        head: base_head,
        body: Vec::new(),
    };

    // convert the chain into individual chain expression
    for &expression_id in &chain[1..] {
        let chain_expression = match tree.get(expression_id) {
            Expression::Member { name, .. } => ChainExpression::Member {
                segment: *name,
                static_arguments: None,
            },
            Expression::Call {
                position,
                dynamic_arguments,
                ..
            } => ChainExpression::Call {
                position: *position,
                dynamic_arguments: dynamic_arguments.clone(),
            },
            Expression::Index {
                position, index, ..
            } => ChainExpression::Index {
                position: *position,
                index: *index,
            },
            Expression::Maybe { position, .. } => ChainExpression::Maybe {
                position: *position,
            },
            _ => {
                return Err(FormatError::SyntaxError {
                    message: "unexpected expression kind for chain expression",
                });
            }
        };
        body.push(chain_expression);
    }

    // keep a leading call with the base so alignment stays stable
    if let Some(first_op) = body.first()
        && matches!(
            first_op,
            ChainExpression::Call {
                position: PostfixPosition::Direct,
                ..
            }
        )
    {
        base.body.push(first_op.clone());
        body.remove(0);
    }

    // group the chain operations into lines
    let lines = group_chain_expression_lines(body);

    // inline variant keeps everything on one line when it fits
    let format_inline = format_with(|f| {
        format_chain_base(f, &base)?;
        for line in &lines {
            format_chain_expression_line(f, line)?;
        }
        Ok(())
    });
    // chain variant breaks each operation onto its own line
    let format_chain = format_with(|f| {
        group(&format_with(|f| {
            // always print the base first so indentation aligns subsequent lines
            format_chain_base(f, &base)?;
            // indent chained entries so each operation sits on its own line
            if !lines.is_empty() {
                write!(
                    f,
                    [block_indent(&format_with(|f| {
                        // each chain line renders in isolation to mirror prettier style
                        let mut join = f.join_with(hard_line_break());
                        for line in &lines {
                            join.entry(&format_with(|f| format_chain_expression_line(f, line)));
                        }
                        join.finish()
                    }))]
                )?;
            }
            Ok(())
        }))
        .format(f)
    });
    // prefer inline, otherwise chain
    best_fitting![format_inline, format_chain]
        .with_mode(BestFittingMode::AllLines)
        .format(f)
}

/// Format a match expression.
#[inline]
pub(crate) fn format_match<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    include_prefix: bool,
) -> FormatResult<()> {
    let match_node = f.context().tree.get(node_id);
    let Expression::Match {
        kind: _,
        value,
        cases,
    } = &match_node
    else {
        return Err(FormatError::SyntaxError {
            message: "invalid match expression",
        });
    };

    if include_prefix {
        // match <expression>
        write!(f, [Keyword::Match, space()])?;
    }

    write!(f, [token("("), value, token(")")])?;

    // empty match body
    if cases.is_empty() {
        write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
        return Ok(());
    }

    // match cases
    write!(f, [space(), token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&format_args![block_indent(&format_with(|f| f
            .join_with(hard_line_break())
            .entries(cases)
            .finish())),])]
    )?;
    write!(f, [f.context().block_infix_annotations(node_id)])?;
    write!(f, [hard_line_break(), token("}")])?;

    Ok(())
}

/// Check whether the expression is a chain expression.
fn is_chain_expression(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Member { .. }
            | Expression::Call { .. }
            | Expression::Index { .. }
            | Expression::Maybe { .. }
    )
}

/// Whether an expression is "trivial" (prefers to be fully inline).
pub fn is_trivial_expression(tree: &NodeTree, expression: &Expression) -> bool {
    match expression {
        Expression::ScalarLiteral(_) | Expression::TypeLiteral(_) => true,
        Expression::ObjectExpression { ty, properties, .. } => {
            ty.is_none()
                && properties.len() <= 5
                && properties
                    .iter()
                    .all(|property| is_trivial_property(tree, tree.get(*property)))
        }
        Expression::Unary { operator: _, right } => is_trivial_expression(tree, tree.get(*right)),
        Expression::Index { left, index, .. } => {
            is_trivial_expression(tree, tree.get(*left)) && index.is_none()
                || is_trivial_expression(tree, tree.get(*index.as_ref().unwrap()))
        }
        Expression::ReferenceOf {
            mutability: _,
            variance: _,
            right,
        } => is_trivial_expression(tree, tree.get(*right)),
        Expression::Member { left, .. } => is_trivial_expression(tree, tree.get(*left)),
        Expression::Path {
            path,
            static_arguments,
        } => path.segments.len() <= 3 && static_arguments.is_none(),
        _ => false,
    }
}

/// Whether an expression is "complex" (prefers to be multiline).
pub fn is_complex_expression(_tree: &NodeTree, expression: &Expression) -> bool {
    match expression {
        Expression::Statement { .. } => true,
        Expression::ObjectExpression { ty, properties, .. } => ty.is_some() || properties.len() > 1,
        Expression::TreeExpression { .. } => true,
        _ => false,
    }
}

/// Whether an argument is "trivial" (prefers to be inline).
pub fn is_trivial_argument(tree: &NodeTree, argument: &Argument) -> bool {
    match argument {
        Argument::Named { name: _, value, .. }
        | Argument::Labeled {
            label: _, value, ..
        }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => is_trivial_expression(tree, tree.get(*value)),
    }
}

/// Whether a property is "trivial" (prefers to be inline).
pub fn is_trivial_property(tree: &NodeTree, property: &Property) -> bool {
    match property {
        Property::Field { value, default, .. } => {
            value.is_none_or(|value| is_trivial_expression(tree, tree.get(value)))
                && default.is_none_or(|default| is_trivial_expression(tree, tree.get(default)))
        }
        Property::Method { body, .. } => {
            body.is_none_or(|body| is_trivial_expression(tree, tree.get(body)))
        }
        Property::Spread { value, .. } => is_trivial_expression(tree, tree.get(*value)),
    }
}

/// Whether an argument is "complex" (prefers to be multiline).
pub fn is_complex_argument(tree: &NodeTree, argument: &Argument) -> bool {
    match argument {
        Argument::Named { name: _, value, .. }
        | Argument::Labeled {
            label: _, value, ..
        }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => is_complex_expression(tree, tree.get(*value)),
    }
}

/// Whether the expression can break itself across multiple lines.
pub fn is_expression_breakable(tree: &NodeTree, expression: &Expression) -> bool {
    match expression {
        Expression::ArrayExpression { elements, .. } => !elements.is_empty(),
        Expression::TupleExpression { elements, .. } => !elements.is_empty(),
        Expression::ObjectExpression { ty, properties, .. } => {
            ty.is_some_and(|ty| is_expression_breakable(tree, tree.get(ty)))
                || !properties.is_empty()
        }
        Expression::TreeExpression {
            arguments,
            elements,
            ..
        } => {
            arguments
                .as_ref()
                .is_some_and(|arguments| !arguments.is_empty())
                || elements
                    .as_ref()
                    .is_some_and(|elements| !elements.is_empty())
        }
        Expression::Call {
            dynamic_arguments, ..
        } => !dynamic_arguments.is_empty(),
        Expression::Match { .. } => true,
        Expression::Path {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|static_arguments| !static_arguments.is_empty()),
        Expression::If { .. }
        | Expression::Loop { .. }
        | Expression::Try { .. }
        | Expression::Block { .. }
        | Expression::ForEach { .. }
        | Expression::For { .. }
        | Expression::While { .. }
        | Expression::Import { .. }
        | Expression::Export { .. } => true,
        Expression::Binary { .. } | Expression::TypeBinary { .. } => true,
        _ => false,
    }
}

/// Format a struct literal.
#[inline]
pub(crate) fn format_struct_literal<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    ty: &Option<LocalNodeId<Expression>>,
    properties_ids: &Vec<LocalNodeId<Property>>,
) -> FormatResult<()> {
    if let Some(ty) = ty {
        write!(f, [ty, space()])?;
    }

    let properties = properties_ids
        .iter()
        .map(|property| f.context().tree.get(*property))
        .collect::<SmallVec<[_; 3]>>();

    let is_trivial = properties.is_empty()
        || properties.len() <= 5
            && properties
                .iter()
                .all(|property| is_trivial_property(f.context().tree, property));
    let has_annotations = f.context().has_infix_annotation(expression_id)
        || properties_ids
            .iter()
            .any(|property| f.context().has_annotation(*property));
    let keep_newline =
        f.context().has_newline(f.context().get_span(expression_id)) && properties.len() > 1;

    write!(
        f,
        [list_like("{", "}", ",", properties_ids)
            .include_space()
            .should_expand(!is_trivial || has_annotations || keep_newline)]
    )?;
    Ok(())
}

/// Format a tree literal.
#[inline]
pub(crate) fn format_tree_literal<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    left: &Option<LocalNodeId<Expression>>,
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
) -> FormatResult<()> {
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
                        write!(
                            f,
                            [
                                if_group_fits_on_line(&space()),
                                soft_block_indent(&format_with(|f| {
                                    f.join_with(&format_args![soft_line_break_or_space()])
                                        .entries(arguments.iter().map(|argument| {
                                            TreeExpressionArgument {
                                                argument_id: *argument,
                                            }
                                        }))
                                        .finish()
                                }))
                            ]
                        )?;
                    }
                    // /
                    if elements.is_none() {
                        if left.is_some() {
                            write!(f, [if_group_fits_on_line(&space())])?;
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
                let span = f.context().get_span(expression_id);
                let force_newline =
                    f.context().has_newline(span) || f.context().is_at_line_start(expression_id.id);

                // elements
                if !force_newline {
                    write!(
                        f,
                        [group(&format_args![soft_block_indent(&format_with(|f| {
                            f.join_with(soft_line_break()).entries(elements).finish()
                        }))])]
                    )?;
                } else {
                    write!(
                        f,
                        [group(&format_args![block_indent(&format_with(|f| {
                            f.join_with(hard_line_break()).entries(elements).finish()
                        }))])]
                    )?;
                }

                // closing tag
                write!(f, [token("</")])?;
                if let Some(left) = left {
                    write!(f, [left])?;
                }
                write!(f, [token(">")])?;
            }

            Ok(())
        }))]
    )
}

/// Format an expression (without prefix and postfix annotations)
pub(crate) fn format_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<()> {
    let tree = f.context().tree;

    match expression {
        // declaration
        Expression::Declaration(node) => node.format(f)?,

        // block
        Expression::Block(node) => node.format(f)?,

        // statement
        Expression::Statement(node) => {
            write!(f, [*node, token(";")])?;
        }

        // import
        Expression::Import {
            kind,
            target,
            items,
            arguments,
        } => {
            // keyword
            write!(f, [Keyword::Import, space()])?;
            if *kind == DependencyKind::Type {
                write!(f, [Keyword::Type, space()])?;
            }

            // items
            let first_item = items.first().map(|item| tree.get(*item));
            // namespace
            if items.len() == 1 && first_item.unwrap().mode == DependencyMode::Namespace {
                write!(
                    f,
                    [
                        token("*"),
                        space(),
                        Keyword::As,
                        space(),
                        first_item.unwrap().alias
                    ]
                )?;
            }
            // items
            else {
                // default
                if let Some(first_item) = first_item
                    && first_item.mode == DependencyMode::Default
                {
                    write!(f, [first_item.alias, token(","), space()])?;
                    let rest_items: Vec<LocalNodeId<DependencyItem>> =
                        items.iter().skip(1).copied().collect();
                    if !rest_items.is_empty() {
                        write!(f, [list_like("{", "}", ",", &rest_items).include_space(),])?;
                    }
                }
                // items
                else if !items.is_empty() {
                    write!(f, [list_like("{", "}", ",", items).include_space()])?;
                }
            }

            // from
            if !items.is_empty() {
                write!(f, [space(), Keyword::From, space()])?;
            }

            // target
            write!(f, [token("\""), target, token("\"")])?;

            // arguments
            if let Some(arguments) = arguments {
                write!(
                    f,
                    [
                        space(),
                        Keyword::With,
                        space(),
                        list_like("{", "}", ",", arguments).include_space()
                    ]
                )?;
            }
        }

        // export
        Expression::Export {
            kind,
            target,
            items,
        } => {
            // keyword
            write!(f, [Keyword::Export, space()])?;
            if *kind == DependencyKind::Type {
                write!(f, [Keyword::Type, space()])?;
            }

            // items
            let first_item = items.first().map(|item| tree.get(*item));
            // namespace
            if items.len() == 1 && first_item.unwrap().mode == DependencyMode::Namespace {
                write!(f, [token("*")])?;
                if let Some(alias) = first_item.unwrap().alias {
                    write!(f, [space(), Keyword::As, space(), alias])?;
                }
            }
            // items
            else if !items.is_empty() {
                write!(f, [list_like("{", "}", ",", items).include_space()])?;
            }

            // target
            if let Some(target) = target {
                write!(
                    f,
                    [
                        space(),
                        Keyword::From,
                        space(),
                        token("\""),
                        target,
                        token("\"")
                    ]
                )?;
            }
        }

        // let
        Expression::Let {
            mutability,
            descriptor,
            declarators,
        } => {
            // keyword header (export + const/let)
            let keyword_header = format_with(|f| {
                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }
                // keyword
                if *mutability == Mutability::Immutable {
                    write!(f, [Keyword::Const])?;
                } else {
                    write!(f, [Keyword::Let])?;
                }
                Ok(())
            });

            // Format declarators (comma-separated)
            write!(f, [keyword_header])?;
            for (i, declarator_id) in declarators.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(",")])?;
                }
                write!(f, [space()])?;
                format_declarator(f, tree, *declarator_id)?;
            }
        }

        // if (ternary)
        Expression::If {
            kind: IfKind::Ternary,
            condition,
            then_expression,
            else_expression,
            ..
        } => {
            write!(
                f,
                [group(&format_args![
                    condition,
                    if_group_fits_on_line(&space()),
                    soft_block_indent(&format_args![
                        token("?"),
                        space(),
                        then_expression,
                        soft_line_break(),
                        if_group_fits_on_line(&space()),
                        token(":"),
                        space(),
                        else_expression
                    ]),
                ])]
            )?;
        }

        // if (regular)
        Expression::If {
            kind: IfKind::If, ..
        } => {
            write!(
                f,
                [group(&format_with(|f| format_if_else_chain(f, node_id)))]
            )?;
        }

        // while
        Expression::While {
            kind,
            condition,
            body,
        } => match *kind {
            WhileKind::While => {
                write!(
                    f,
                    [
                        Keyword::While,
                        space(),
                        token("("),
                        condition,
                        token(")"),
                        space(),
                        body
                    ]
                )?;
            }
            WhileKind::DoWhile => {
                write!(
                    f,
                    [
                        Keyword::Do,
                        space(),
                        body,
                        space(),
                        Keyword::While,
                        space(),
                        token("("),
                        condition,
                        token(")"),
                    ]
                )?;
            }
        },

        // for each
        Expression::ForEach {
            asynchrony,
            kind,
            pattern,
            iterator,
            body,
        } => {
            write!(f, [Keyword::For, space()])?;
            if *asynchrony == Asynchrony::Async {
                write!(f, [Keyword::Await, space()])?;
            }
            let keyword = match kind {
                ForEachKind::In => Keyword::In,
                ForEachKind::Of => Keyword::Of,
            };
            write!(
                f,
                [
                    token("("),
                    pattern,
                    space(),
                    keyword,
                    space(),
                    iterator,
                    token(")"),
                    space()
                ]
            )?;
            write!(f, [body])?;
        }

        // for condition
        Expression::For {
            initialization,
            condition,
            increment,
            body,
        } => {
            write!(
                f,
                [
                    Keyword::For,
                    space(),
                    token("("),
                    initialization,
                    token(";"),
                    space(),
                    condition,
                    token(";"),
                    space(),
                    increment,
                    token(")"),
                    space(),
                    body
                ]
            )?;
        }

        // loop
        Expression::Loop { body } => {
            write!(f, [Keyword::Loop, space(), body])?;
        }

        // try
        Expression::Try {
            try_expression,
            catch_pattern,
            catch_expression,
            finally_expression,
        } => {
            // try <expression>
            write!(f, [Keyword::Try, space(), try_expression])?;

            // catch <expression>
            if let Some(catch) = catch_expression {
                write!(f, [space(), Keyword::Catch, space()])?;
                if let Some(catch_pattern) = catch_pattern {
                    write!(f, [token("("), catch_pattern, token(")"), space()])?;
                }
                write!(f, [catch])?;
            }

            // finally <expression>
            if let Some(finally) = finally_expression {
                write!(f, [space(), Keyword::Finally, space(), finally])?;
            }
        }

        // match
        Expression::Match { .. } => {
            format_match(f, node_id, true)?;
        }

        // break
        Expression::Break { label, value } => {
            write!(f, [Keyword::Break])?;
            if let Some(label) = label {
                write!(f, [space(), token(":"), label])?;
            }
            if let Some(value) = value {
                write!(f, [space(), value])?;
            }
        }

        // continue
        Expression::Continue { label } => {
            write!(f, [Keyword::Continue])?;
            if let Some(label) = label {
                write!(f, [space(), token(":"), label])?;
            }
        }

        // await
        Expression::Await { expression } => {
            write!(f, [Keyword::Await, space(), expression])?;
        }

        // yield
        Expression::Yield { cardinality, value } => {
            write!(f, [Keyword::Yield])?;
            if *cardinality == YieldCardinality::Generator {
                write!(f, [token("*")])?;
            }
            write!(f, [space(), value])?;
        }

        // throw
        Expression::Throw { value } => {
            write!(f, [token("throw")])?;
            if let Some(value) = value {
                write!(f, [space(), value])?;
            }
        }

        // return
        Expression::Return { value } => {
            write!(f, [token("return")])?;
            if let Some(value) = value {
                write!(f, [space(), value])?;
            }
        }

        // path
        Expression::Path {
            path,
            static_arguments,
        } => {
            write!(f, [path])?;

            // static arguments
            if let Some(static_arguments) = static_arguments
                && !static_arguments.is_empty()
            {
                write!(f, [list_like("<", ">", ",", static_arguments)])?;
            }
        }

        // scalar literal
        Expression::ScalarLiteral(node) => {
            format_scalar_literal(node, tree.get_span(node_id), f)?;
        }

        // template literal
        Expression::TemplateExpression { value } => {
            format_template_literal(value, tree.get_span(node_id), f)?;
        }

        // tagged template literal
        Expression::TaggedTemplateExpression { tag, value } => {
            write!(f, [tag])?;
            format_template_literal(value, tree.get_span(node_id), f)?;
        }

        // type literal
        Expression::TypeLiteral(node) => node.format(f)?,

        // range literal
        Expression::RangeExpression {
            start,
            end,
            is_inclusive,
        } => {
            if *is_inclusive {
                write!(f, [start, token("..="), end,])?;
            } else {
                write!(f, [start, token(".."), end,])?;
            }
        }

        // array literal
        Expression::ArrayExpression {
            elements: elements_ids,
        } => {
            let span = f.context().get_span(node_id);
            let elements = elements_ids
                .iter()
                .map(|id| tree.get(*id))
                .collect::<SmallVec<[_; 3]>>();
            let should_expand = elements.len() > 1
                && elements
                    .iter()
                    .any(|element| is_complex_argument(tree, element))
                || f.context().has_newline(span) && elements.len() > 1;
            write!(
                f,
                [list_like("[", "]", ",", elements_ids).should_expand(should_expand)]
            )?;
        }

        // tuple literal
        Expression::TupleExpression {
            elements: elements_ids,
        } => {
            if elements_ids.is_empty() {
                write!(f, [token("()")])?;
            } else {
                let span = f.context().get_span(node_id);
                let elements = elements_ids
                    .iter()
                    .map(|id| tree.get(*id))
                    .collect::<SmallVec<[_; 3]>>();
                let should_expand = elements.len() > 1
                    && elements
                        .iter()
                        .any(|element| is_complex_argument(tree, element))
                    || f.context().has_newline(span) && elements.len() > 1;
                write!(
                    f,
                    [list_like("(", ")", ",", elements_ids)
                        .force_trailing_separator()
                        .should_expand(should_expand)]
                )?;
            }
        }

        // struct literal
        Expression::ObjectExpression { ty, properties } => {
            format_struct_literal(f, node_id, ty, properties)?;
        }

        // tree literal
        Expression::TreeExpression {
            left,
            arguments,
            elements,
        } => {
            format_tree_literal(f, node_id, left, arguments, elements)?;
        }

        // parenthesized
        Expression::Parenthesized { expression } => {
            let inner_expression = tree.get(*expression);
            if let Expression::TreeExpression { .. } = inner_expression {
                write!(f, [token("("), soft_block_indent(&expression), token(")")])?;
            } else {
                write!(f, [token("("), expression, token(")")])?;
            }
        }

        // unary
        Expression::Unary { operator, right } => {
            if operator.is_prefix() {
                write!(f, [operator, right])?;
            } else {
                write!(f, [right, operator])?;
            }
        }

        // type unary
        Expression::TypeUnary { operator, right } => match operator {
            TypeUnaryOperator::Newtype
            | TypeUnaryOperator::Type
            | TypeUnaryOperator::Readonly
            | TypeUnaryOperator::Typeof
            | TypeUnaryOperator::Keyof
            | TypeUnaryOperator::Infer
            | TypeUnaryOperator::Asserts => {
                write!(f, [operator, space(), right])?;
            }
            TypeUnaryOperator::AsConst => {
                write!(f, [right, token(" as const")])?;
            }
        },

        // value
        Expression::ValueOf {
            mutability,
            variance,
            right,
        } => {
            write!(f, [token("^")])?;
            if let Some(mutability) = mutability {
                write!(f, [*mutability])?;
            }
            if let Some(variance) = variance {
                write!(f, [variance.to_keyword(), space()])?;
            }
            right.format(f)?;
        }

        // reference
        Expression::ReferenceOf {
            mutability,
            variance,
            right,
        } => {
            write!(f, [token("&")])?;
            if let Some(mutability) = mutability {
                write!(f, [*mutability])?;
            }
            if let Some(variance) = variance {
                write!(f, [variance.to_keyword(), space()])?;
            }
            right.format(f)?;
        }

        // member
        Expression::Member { .. } => {
            if is_expression_chain(tree, node_id) {
                format_expression_chain(f, node_id)?;
            } else {
                format_member_expression(f, node_id)?;
            }
        }

        // index
        Expression::Index { .. } => {
            if is_expression_chain(tree, node_id) {
                format_expression_chain(f, node_id)?;
            } else {
                format_index_expression(f, node_id)?;
            }
        }

        // call
        Expression::Call { .. } => {
            if is_expression_chain(tree, node_id) {
                format_expression_chain(f, node_id)?;
            } else {
                format_call_expression(f, node_id)?;
            }
        }

        // new
        Expression::New {
            left,
            static_arguments,
            dynamic_arguments,
        } => {
            write!(f, [token("new"), space(), left])?;
            if let Some(static_arguments) = static_arguments {
                write!(f, [list_like("<", ">", ",", static_arguments)])?;
            }
            write!(f, [list_like("(", ")", ",", dynamic_arguments)])?;
        }

        // delete
        Expression::Delete { value } => {
            write!(f, [token("delete"), space(), value])?;
        }

        // maybe
        Expression::Maybe { .. } => {
            if is_expression_chain(tree, node_id) {
                format_expression_chain(f, node_id)?;
            } else {
                format_maybe_expression(f, node_id)?;
            }
        }

        // must
        Expression::Must { position, left } => {
            write!(f, [left])?;
            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }
            write!(f, [token("!")])?;
        }

        // binary
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            write!(f, [left])?;
            if !f.context().has_postfix_annotation(*left) {
                write!(f, [space()])?;
            }
            write!(f, [operator, space(), right])?;
        }

        // type binary
        Expression::TypeBinary {
            left,
            operator,
            right,
        } => {
            write!(f, [left])?;
            if !f.context().has_postfix_annotation(*left) {
                write!(f, [space()])?;
            }
            write!(f, [operator, space(), right])?;
        }

        // assign
        Expression::Assign {
            left,
            operator,
            right,
        } => {
            write!(f, [left])?;
            if !f.context().has_postfix_annotation(*left) {
                write!(f, [space()])?;
            }
            write!(f, [operator, space(), right])?;
        }

        // stub (placeholder for annotation-only files)
        Expression::Stub => {}

        // error
        Expression::Error => {
            write!(f, [token("/* ERROR */")])?;
        }
    };

    Ok(())
}

/// Format a declarator (pattern, optional type, optional value).
fn format_declarator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    tree: &NodeTree,
    declarator_id: LocalNodeId<Declarator>,
) -> FormatResult<()> {
    let declarator = tree.get(declarator_id);
    let Declarator { pattern, ty, value } = declarator;

    // header: pattern + optional type
    let header = format_with(|f| {
        write!(f, [pattern])?;
        if let Some(ty_id) = ty {
            write!(f, [token(":"), space(), ty_id])?;
        }
        Ok(())
    });

    let Some(value_id) = value else {
        write!(f, [header])?;
        return Ok(());
    };

    // prefer keeping the value on a single line
    let format_inline = format_with(|f| {
        write!(f, [header, space(), token("="), space(), *value_id])?;
        Ok(())
    });
    // expand inline if breakable (like let x = [\n ... ])
    let format_inline_expanded = format_with(|f| {
        write!(
            f,
            [
                header,
                space(),
                token("="),
                space(),
                fits_expanded(&group(value_id).should_expand(true)),
            ]
        )
    });
    // expand and indent the value
    let format_indented = format_with(|f| {
        group(&format_args![
            header,
            space(),
            token("="),
            block_indent(value_id)
        ])
        .format(f)
    });

    if is_expression_breakable(tree, tree.get(*value_id)) {
        best_fitting![format_inline, format_inline_expanded, format_indented]
            .with_mode(BestFittingMode::AllLines)
            .format(f)?;
    } else {
        best_fitting![format_inline, format_indented].format(f)?;
    }

    Ok(())
}

impl<'ast> FormatNode<'ast, Declarator> for Declarator {
    fn format_node(
        &self,
        node_id: LocalNodeId<Declarator>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        format_declarator(f, f.context().tree, node_id)?;

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Expression> for Expression {
    fn format_node(
        &self,
        node_id: LocalNodeId<Expression>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        format_expression(f, node_id, self)?;

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};

    /// Simple expressions should stay on one line.
    #[test]
    fn test_format_expression_simple() {
        assert_format!(
            "1 + 2 * 3 - a / b % c",
            "1 + 2 * 3 - a / b % c",
            |p| p.eat_expression(),
            DestackFormatOptions::default_tab()
        );
    }

    /// Parenthesized expressions should retain their parentheses.
    #[test]
    fn test_format_expression_parenthesized() {
        assert_format!(
            "(((1 + 2) * 3) - a / (b % c))",
            "(((1 + 2) * 3) - a / (b % c))",
            |p| p.eat_expression(),
            DestackFormatOptions::default_tab()
        );
    }

    /// Empty parenthesis/arguments/tuplestuples should be respected.
    #[test]
    fn test_format_expression_nested_empty_parenthesis() {
        assert_format!(
            "(((())))",
            "(((())))",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    /// Empty parenthesis/arguments/tuplestuples should be respected.
    #[test]
    fn test_format_expression_nested_empty_arguments() {
        assert_format!(
            "foo<()>(((())))",
            "foo<()>(((())))",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_struct_literal_trivial() {
        assert_format!(
            "({ a: 1, ...B })",
            "({ a: 1, ...B })",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_struct_literal_spread() {
        assert_format!(
            "Foo { ...B }",
            "Foo { ...B }",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_if_ternary() {
        assert_format!(
            "true ? 1 : 2",
            "true ? 1 : 2",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_index_call_mixed_postfix() {
        assert_format!(
            "x?.[f]?.[2]?.(a, b)",
            "x?.[f]?.[2]?.(a, b)",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_member_call_chain_line() {
        assert_format!(
            "call().followed().by().many().calls()",
            "call().followed().by().many().calls()",
            |p| p.eat_expression(),
            DestackFormatOptions::default_tab_with_line_width(100)
        );
    }

    #[test]
    fn test_format_member_call_chain_retains_breaks() {
        assert_format!(
            "call()\n\t.followed()\n\t.by()\n\t.many()\n\t.calls()\n",
            "call()\n\t.followed()\n\t.by()\n\t.many()\n\t.calls()\n",
            |p| p.eat_expression(),
            DestackFormatOptions::default_tab_with_line_width(20)
        );
    }

    #[test]
    fn test_format_member_call_chain_breaks() {
        assert_format!(
            "call().followed().by().many().calls()\n",
            "call()\n\t.followed()\n\t.by()\n\t.many()\n\t.calls()\n",
            |p| p.eat_expression(),
            DestackFormatOptions::default_tab_with_line_width(20)
        );
    }

    #[test]
    fn test_format_member_call_chain_breaks_with_maybe_and_index() {
        assert_format!(
            "call().followed()?.by()[0]?.many()?.calls()\n",
            "call()\n\t.followed()\n\t?.by()\n\t[0]\n\t?.many()\n\t?.calls()\n",
            |p| p.eat_expression(),
            DestackFormatOptions::default_tab_with_line_width(20)
        );
    }

    #[test]
    fn test_format_path_member_call_chain_breaks() {
        assert_format!(
            "long.base.path.followed().by().many().calls()\n",
            "long\n\t.base\n\t.path\n\t.followed()\n\t.by()\n\t.many()\n\t.calls()\n",
            |p| p.eat_expression(),
            DestackFormatOptions::default_tab_with_line_width(20)
        );
    }

    #[test]
    fn test_format_index_member_chain_breaks() {
        assert_format!(
            "identifier1.identifier2.identifier3[indexA].identifier4[indexB]?.[indexC][indexD]\n",
            "identifier1\n\t.identifier2\n\t.identifier3[indexA]\n\t.identifier4[indexB]\n\t?.[indexC]\n\t[indexD]\n",
            |p| p.eat_expression(),
            DestackFormatOptions::default_tab_with_line_width(20)
        );
    }

    #[test]
    fn test_format_expression_tree_literal_without_arguments() {
        assert_format!(
            "<Entity/>",
            "<Entity />",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_tree_literal_with_arguments() {
        assert_format!(
            "<Entity a=1 b = 2 />",
            "<Entity a=1 b=2 />",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_tree_literal_parenthesized() {
        let source = r"(
    <Entity a=1 b=2>
        <Entity a=1 b=2 />
    </Entity>
)";
        assert_format!(
            source,
            source,
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_tree_literal_nested() {
        let source = r#"<A x=4 y=4>
    <B x="hey">
        <C>
            <D />
            2
        </C>
    </B>
</A>"#;
        assert_format!(
            source,
            source,
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_tree_literal_with_array_of_struct_element() {
        let source = r#"<Menu
    items=[
        { to: "/posts" },
        { to: "/posts/$postId", params: { postId: "postId" } },
    ]
/>"#;
        assert_format!(
            source,
            source,
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_call_with_struct_literal() {
        let source = r#"Destack.serve({
    fetch: function (req: Request) {
        return Response("Success!")
    },
    run: true,
})"#;
        assert_format!(
            source,
            source,
            |p| p.eat_expression(),
            DestackFormatOptions::default_with_line_width(40)
        );
    }

    #[test]
    fn test_format_expression_let_call() {
        let source = r#"const ast = await parseAsync(text, {
    sourceFileName: "file",
    parserOpts: {
        plugins: ["typescript", "jsx"],
    },
    sourceType: "module",
    configFile: false,
    babelrc: false,
})"#;
        assert_format!(
            source,
            source,
            |p| p.eat_expression(),
            DestackFormatOptions::default_with_line_width(40)
        );
    }
}

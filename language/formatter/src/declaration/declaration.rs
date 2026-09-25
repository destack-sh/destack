use crate::annotation::{
    DanglingIndentMode, FormatDanglingComments, FormatLeadingComments, format_comment,
    format_dangling_comments, infix_or_postfix_annotations, postfix_annotations,
    prefix_annotations,
};
use crate::collection::member::format_block_of_members;
use crate::context::FormatNodeWithoutTrailingComments;
use crate::declaration::function::format_function_declaration;
use crate::declaration::sequence::format_block_statement_sequence;
use crate::declaration::signature::{
    function_grouping_generic_parameter_is_plain, write_declaration_generic_parameters,
    write_declaration_where_clauses,
};
use crate::declaration::r#type::{
    format_class_declaration, format_enum_declaration, format_interface_declaration,
    format_struct_declaration,
};
use crate::declaration::{
    FunctionCacheMode, empty_block_with_infix_annotations, format_lambda_declaration,
    write_keyword_prefix, write_statement_terminator_after_anchor,
};
use crate::expression::{format_declarator, write_control_branch_after_head};
use crate::operator::{
    AssignmentLikeLayout, write_assignment_like_right,
    write_type_expression_with_inline_prefix_annotations,
};
use crate::{FormatNode, TsppFormatContext, TsppFormatter};
use tspp_dir::{
    Asynchrony, Comment, Declaration, Declarator, ExportKind, Expression, ExtensionDeclaration,
    FunctionDeclaration, FunctionForm, GlobalDeclaration, Keyword, LetKind, LocalNodeId,
    ModuleDeclaration, Mutability, Node, NodeType, TokenSpan, TypeDeclaration, TypeExpression,
};
use tspp_fir::format::{
    FormatError, FormatLayout, FormatResult, Formatter as FirFormatter, GroupId, InstructionTape,
};
use tspp_fir::prelude::*;
use tspp_fir::{format_args, write};
use tspp_source::{NodeSpanBoundary, NodeSpanRegion, NodeSpanType, Span};

const MIN_OVERLAP_FOR_BREAK: u32 = 3;

/// Return the `export` token for one declaration, if present.
pub(crate) fn declaration_export_token(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
) -> Option<TokenSpan> {
    let declaration_span = context.span(node_id);

    context
        .tokens
        .iter()
        .copied()
        .filter(|token| {
            token.span.file == declaration_span.file
                && token.span.start >= declaration_span.start
                && token.span.end <= declaration_span.end
                && context.token_keyword(*token) == Some(Keyword::Export)
        })
        .min_by_key(|token| token.span.start)
}

/// Return comments between `export` and the declaration head.
fn declaration_export_head_comments(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
    export: ExportKind,
) -> Vec<Comment> {
    let declaration_span = context.span(node_id);
    let export_token = declaration_export_token(context, node_id);
    let Some(export_token) = export_token else {
        return Vec::new();
    };

    let mut comments: Vec<Comment> = Vec::new();

    // export separator
    if let Some(next_token) = context.next_token_after_span(export_token.span)
        && next_token.span.file == export_token.span.file
        && next_token.span.start > export_token.span.end
    {
        let comment_cursor = context.comments();
        comments
            .extend(comment_cursor.comments_in_range(export_token.span.end, next_token.span.start));
    }

    // default separator
    if export == ExportKind::Default {
        let default_token = context
            .tokens
            .iter()
            .copied()
            .filter(|token| {
                token.span.file == declaration_span.file
                    && token.span.start >= export_token.span.end
                    && token.span.end <= declaration_span.end
                    && context.token_keyword(*token) == Some(Keyword::Default)
            })
            .min_by_key(|token| token.span.start);

        if let Some(default_token) = default_token
            && let Some(next_token) = context.next_token_after_span(default_token.span)
            && next_token.span.file == default_token.span.file
            && next_token.span.start > default_token.span.end
        {
            let comment_cursor = context.comments();
            comments.extend(
                comment_cursor.comments_in_range(default_token.span.end, next_token.span.start),
            );
        }
    }

    comments.sort_by_key(|comment| comment.span.start);
    comments.dedup();

    comments
}

/// Write comments between `export` and the declaration head.
pub(crate) fn write_declaration_export_head_comments<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    export: ExportKind,
) -> FormatResult<()> {
    let comments = declaration_export_head_comments(f.context(), node_id, export);

    // empty separator
    if comments.is_empty() {
        return Ok(());
    }

    // comment sequence
    for comment in comments {
        format_comment(f, comment)?;

        let is_line_comment = comment.is_line();

        if is_line_comment {
            write!(f, [hard_line_break()])?;
        } else {
            write!(f, [space()])?;
        }
    }

    Ok(())
}

/// Write one export prefix.
pub(crate) fn format_declaration_export_modifier<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    export: Option<ExportKind>,
) -> FormatResult<()> {
    // export
    match export {
        Some(ExportKind::Named) => {
            write!(f, [Keyword::Export, space()])?;
            write_declaration_export_head_comments(f, node_id, ExportKind::Named)?;
        }
        Some(ExportKind::Default) => {
            write!(f, [Keyword::Export, space(), Keyword::Default, space()])?;
            write_declaration_export_head_comments(f, node_id, ExportKind::Default)?;
        }
        None => {}
    }

    Ok(())
}

/// Capture one type declaration head for layout selection.
fn capture_type_declaration_left<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &TypeDeclaration,
) -> FormatResult<(InstructionTape<'ast>, bool, bool)> {
    let mut formatter = FirFormatter::new(f.state_mut());

    // prefixes
    format_declaration_export_modifier(&mut formatter, node_id, declaration.export)?;
    write_keyword_prefix(&mut formatter, Keyword::Declare, declaration.is_ambient)?;
    write_keyword_prefix(&mut formatter, Keyword::Shared, declaration.is_shared)?;

    // modifiers
    if declaration.is_nominal {
        write!(&mut formatter, [Keyword::Newtype, space()])?;
    } else if declaration.mutability == Some(Mutability::Immutable) {
        write!(&mut formatter, [Keyword::Readonly, space()])?;
    }

    // head
    if !declaration.is_nominal {
        write!(&mut formatter, [Keyword::Type, space()])?;
    }

    write!(&mut formatter, [declaration.name])?;

    // generic parameters
    write_declaration_generic_parameters(&mut formatter, &declaration.generic_parameters)?;

    // where clauses
    write_declaration_where_clauses(&mut formatter, &declaration.where_clauses)?;

    // layout shape
    let instructions = formatter.into_tape();
    let is_left_short = instructions.single_line_width().is_some_and(|width| {
        width < (u32::from(f.context().options.indent_width) + MIN_OVERLAP_FOR_BREAK)
    });
    let left_may_break = instructions.may_directly_break();

    Ok((instructions, is_left_short, left_may_break))
}

/// Return whether one type expression counts as generic in one conditional head.
fn type_expression_is_assignment_like_generic_condition(
    context: &TsppFormatContext<'_>,
    type_id: LocalNodeId<TypeExpression>,
) -> bool {
    match context.tree.get(type_id) {
        TypeExpression::Reference {
            generic_arguments, ..
        }
        | TypeExpression::Member {
            generic_arguments, ..
        } => !generic_arguments.is_empty(),

        TypeExpression::Function(function) => !function.generic_parameters.is_empty(),
        TypeExpression::Constructor(function) => !function.generic_parameters.is_empty(),

        _ => false,
    }
}

/// Return whether documentation precedes a type declaration value.
fn type_declaration_has_documentation_before_value(
    context: &TsppFormatContext<'_>,
    declaration: &TypeDeclaration,
) -> bool {
    let value_start = context.span(declaration.value).start;
    let content_start = context
        .tree
        .get_side_span(
            declaration.value,
            NodeSpanType::Boundary(NodeSpanBoundary::LeadingOperator),
        )
        .map_or(value_start, |span| span.start);
    let content_span = Span::new(context.file.id, content_start, content_start);
    let Some(previous_token) = context.previous_token_before_span(content_span) else {
        return false;
    };

    context
        .source_comments_in_range(previous_token.span.end, content_start)
        .iter()
        .copied()
        .any(|comment| comment.is_documentation() && comment.followed_by_newline())
}

/// Return whether one type declaration rhs should break after `=`.
fn type_declaration_should_break_after_operator(
    context: &TsppFormatContext<'_>,
    declaration: &TypeDeclaration,
) -> bool {
    let value_start = context.span(declaration.value).start;
    let comments = context.comments();

    match context.tree.get(declaration.value) {
        TypeExpression::Conditional {
            left, extends_type, ..
        } => {
            type_expression_is_assignment_like_generic_condition(context, *left)
                || type_expression_is_assignment_like_generic_condition(context, *extends_type)
                || comments.has_comment_before(value_start)
        }

        // break before union leading documentation
        TypeExpression::Union { .. } => {
            type_declaration_has_documentation_before_value(context, declaration)
        }

        _ => comments.has_comment_before(value_start),
    }
}

/// Return whether one type alias has a complex generic head.
fn type_declaration_has_complex_generic_head(
    context: &TsppFormatContext<'_>,
    declaration: &TypeDeclaration,
) -> bool {
    declaration.generic_parameters.len() > 1
        && declaration
            .generic_parameters
            .iter()
            .copied()
            .any(|parameter_id| {
                !function_grouping_generic_parameter_is_plain(context, parameter_id)
            })
}

/// Return one assignment-like layout for one type declaration.
fn type_declaration_layout(
    context: &TsppFormatContext<'_>,
    declaration: &TypeDeclaration,
    is_left_short: bool,
    left_may_break: bool,
) -> AssignmentLikeLayout {
    if type_declaration_should_break_after_operator(context, declaration) {
        return AssignmentLikeLayout::BreakAfterOperator;
    }

    if type_declaration_has_complex_generic_head(context, declaration) {
        return AssignmentLikeLayout::BreakLeftHandSide;
    }

    if !left_may_break && is_left_short {
        return AssignmentLikeLayout::NeverBreakAfterOperator;
    }

    AssignmentLikeLayout::Fluid
}

/// Return the wrapper expression when one declaration appears in expression position.
fn declaration_expression_id(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
) -> Option<LocalNodeId<Expression>> {
    let (parent_id, parent_type) = context.parent(node_id)?;

    if parent_type != NodeType::Expression {
        return None;
    }

    let expression_id = LocalNodeId::<Expression>::new(parent_id);

    match context.tree.get(expression_id) {
        Expression::Declaration(parent_declaration_id) if *parent_declaration_id == node_id => {
            Some(expression_id)
        }
        _ => None,
    }
}

/// Write one body made from statement expressions.
fn write_expression_declaration_body<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    expressions: &[LocalNodeId<Expression>],
) -> FormatResult<()> {
    // empty body
    if expressions.is_empty() {
        let span = f.context().span(node_id);
        let comments = f.context().comments().comments_before(span.end);
        if !comments.is_empty() {
            write!(
                f,
                [
                    space(),
                    token("{"),
                    FormatDanglingComments::Comments {
                        comments,
                        indent: DanglingIndentMode::Block,
                    },
                    token("}")
                ]
            )?;
            return Ok(());
        }

        write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
        return Ok(());
    }

    // statement body
    write!(f, [space(), token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&block_indent(&format_with(move |f| {
            format_block_statement_sequence(f, expressions, false)?;

            let comments = f
                .context()
                .comments()
                .comments_before(f.context().span(node_id).end);
            if let Some(first_comment) = comments.first() {
                let lines_before = f
                    .context()
                    .source_text()
                    .get_lines_before(first_comment.span, f.context().comments());

                if lines_before > 1 {
                    write!(f, [empty_line()])?;
                } else {
                    write!(f, [hard_line_break()])?;
                }

                write!(
                    f,
                    [FormatDanglingComments::Comments {
                        comments,
                        indent: DanglingIndentMode::None,
                    }]
                )?;
            }

            Ok(())
        })))]
    )?;
    write!(f, [hard_line_break(), token("}")])
}

/// Write one declaration member block without its leading separator.
pub(crate) fn write_member_block<'ast, T, F>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    members: &[LocalNodeId<T>],
    write_members: F,
) -> FormatResult<()>
where
    T: Node,
    F: Copy + Fn(&mut TsppFormatter<'ast, '_>, &[LocalNodeId<T>]) -> FormatResult<()>,
{
    // empty body
    if members.is_empty() {
        let node_span = f.context().span(node_id);

        if f.context().comments().has_comment_in_span(node_span) {
            return write!(
                f,
                [
                    token("{"),
                    format_dangling_comments(node_span).with_block_indent(),
                    token("}")
                ]
            );
        }

        return write!(f, [empty_block_with_infix_annotations(node_id)]);
    }

    // member body
    write!(f, [token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&block_indent(&format_with(move |f| {
            write_members(f, members)
        })))]
    )?;
    write!(f, [hard_line_break(), token("}")])
}

/// Write the separator between one grouped declaration header and its body.
pub(crate) fn write_declaration_body_separator<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    header_group_id: GroupId,
) -> FormatResult<()> {
    write!(
        f,
        [
            if_group_fits_on_line(&space()).with_group_id(Some(header_group_id)),
            if_group_breaks(&hard_line_break()).with_group_id(Some(header_group_id))
        ]
    )
}

/// Format one super-type clause.
pub(crate) fn format_super_type_clause<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    keyword: Keyword,
    types: &[LocalNodeId<TypeExpression>],
) -> FormatResult<()> {
    // empty clause
    if types.is_empty() {
        return Ok(());
    }

    let entries = format_with(move |f| {
        f.join_with(&format_args![token(","), soft_line_break_or_space()])
            .entries(types.iter().copied().map(|type_id| {
                format_with(move |f| {
                    write_type_expression_with_inline_prefix_annotations(f, type_id)
                })
            }))
            .finish()
    });

    write!(
        f,
        [indent(&format_args![
            soft_line_break_or_space(),
            keyword,
            group(&indent(&format_args![soft_line_break_or_space(), entries]))
        ])]
    )
}

/// Format one `let` or `const` statement.
pub(crate) fn format_let_statement_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    kind: LetKind,
    export: Option<ExportKind>,
    is_ambient: bool,
    is_shared: bool,
    declarators: &[LocalNodeId<Declarator>],
) -> FormatResult<()> {
    let tree = f.context().tree;
    let has_any_initializer = declarators
        .iter()
        .copied()
        .any(|declarator_id| tree.get(declarator_id).value.is_some());

    let format_declarators = format_with(move |f| {
        let Some((first_declarator, trailing_declarators)) = declarators.split_first() else {
            return Ok(());
        };

        // first declarator
        write!(f, [space()])?;
        format_declarator(f, tree, *first_declarator)?;

        // trailing declarators
        for declarator_id in trailing_declarators {
            write!(f, [token(",")])?;

            if has_any_initializer {
                write!(f, [hard_line_break()])?;
            } else {
                write!(f, [soft_line_break_or_space()])?;
            }

            format_declarator(f, tree, *declarator_id)?;
        }

        Ok(())
    });

    write!(
        f,
        [group(&format_with(|f| {
            // prefixes
            match export {
                Some(ExportKind::Named) => write!(f, [Keyword::Export, space()])?,
                Some(ExportKind::Default) => {
                    write!(f, [Keyword::Export, space(), Keyword::Default, space()])?;
                }
                None => {}
            }

            write_keyword_prefix(f, Keyword::Declare, is_ambient)?;
            write_keyword_prefix(f, Keyword::Shared, is_shared)?;

            // binding keyword
            match kind {
                LetKind::Let => write!(f, [Keyword::Let])?,
                LetKind::Const => write!(f, [Keyword::Const])?,
            }

            // declarators
            if declarators.len() > 1 {
                write!(f, [indent(&format_declarators)])
            } else {
                write!(f, [format_declarators])
            }
        }))]
    )
}

/// Format one `let else` statement.
pub(crate) fn format_let_else_statement_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    kind: LetKind,
    declarator: LocalNodeId<Declarator>,
    else_branch: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let value = tree.get(declarator).value.ok_or(FormatError::SyntaxError {
        message: "let-else declarator requires a value",
    })?;
    let (parent_id, parent_type) =
        f.context()
            .parent(declarator)
            .ok_or(FormatError::SyntaxError {
                message: "let-else declarator requires an expression parent",
            })?;
    if parent_type != NodeType::Expression {
        return Err(FormatError::SyntaxError {
            message: "let-else declarator requires an expression parent",
        });
    }

    let let_else_id = LocalNodeId::<Expression>::new(parent_id);
    let else_span = tree
        .get_side_span(let_else_id, NodeSpanType::Region(NodeSpanRegion::Else))
        .ok_or(FormatError::SyntaxError {
            message: "let-else expression requires else span",
        })?;
    let else_comments = {
        let value_span = f.context().span(value);

        f.context()
            .comments()
            .comments_in_range(value_span.end, else_span.start)
            .to_vec()
    };

    write!(
        f,
        [group(&format_with(|f| {
            let head = format_with(|f| {
                // binding head
                match kind {
                    LetKind::Let => write!(f, [Keyword::Let])?,
                    LetKind::Const => write!(f, [Keyword::Const])?,
                }

                write!(f, [space()])?;
                format_declarator(f, tree, declarator)?;

                if else_comments.is_empty() {
                    write!(f, [space(), Keyword::Else])?;
                } else {
                    write!(
                        f,
                        [indent(&format_args![
                            hard_line_break(),
                            FormatLeadingComments::Comments(&else_comments),
                            Keyword::Else,
                            format_with(|f| write_control_branch_after_head(f, else_branch))
                        ])]
                    )?;
                }

                Ok(())
            });

            if else_comments.is_empty() {
                write!(
                    f,
                    [
                        head,
                        format_with(|f| write_control_branch_after_head(f, else_branch))
                    ]
                )?;
            } else {
                write!(f, [head])?;
            }

            Ok(())
        }))]
    )
}

/// Format one `using` statement.
pub(crate) fn format_using_statement_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    asynchrony: Asynchrony,
    export: Option<ExportKind>,
    is_ambient: bool,
    declarators: &[LocalNodeId<Declarator>],
) -> FormatResult<()> {
    let tree = f.context().tree;

    let format_declarators = format_with(move |f| {
        for (index, declarator_id) in declarators.iter().copied().enumerate() {
            if index > 0 {
                write!(f, [token(",")])?;
            }

            write!(f, [space()])?;
            format_declarator(f, tree, declarator_id)?;
        }

        Ok(())
    });

    write!(
        f,
        [group(&format_with(|f| {
            // prefixes
            match export {
                Some(ExportKind::Named) => write!(f, [Keyword::Export, space()])?,
                Some(ExportKind::Default) => {
                    write!(f, [Keyword::Export, space(), Keyword::Default, space()])?;
                }
                None => {}
            }

            write_keyword_prefix(f, Keyword::Declare, is_ambient)?;

            if asynchrony == Asynchrony::Async {
                write!(f, [Keyword::Await, space()])?;
            }

            // head
            write!(f, [Keyword::Using, format_declarators])
        }))]
    )
}

/// Format one global augmentation declaration.
fn format_global_declaration<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &GlobalDeclaration,
) -> FormatResult<()> {
    // prefixes
    format_declaration_export_modifier(f, node_id, None)?;
    let should_declare =
        declaration.is_ambient && !f.context().options.language_type.is_declaration();
    if should_declare {
        write_keyword_prefix(f, Keyword::Declare, true)?;
    }

    // head
    write!(f, [token("global")])?;

    // body
    write_expression_declaration_body(f, node_id, &declaration.expressions)?;

    // postfix annotations
    write!(f, [postfix_annotations(f.context(), node_id)])
}

/// Format one module declaration.
fn format_module_declaration<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &ModuleDeclaration,
) -> FormatResult<()> {
    // head
    write!(f, [token("module")])?;

    // body
    write_expression_declaration_body(f, node_id, &declaration.expressions)?;

    // postfix annotations
    write!(f, [postfix_annotations(f.context(), node_id)])
}

/// Format one type alias declaration.
fn format_type_declaration<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &TypeDeclaration,
) -> FormatResult<()> {
    let (left_instructions, is_left_short, left_may_break) =
        capture_type_declaration_left(f, node_id, declaration)?;
    let layout = type_declaration_layout(f.context(), declaration, is_left_short, left_may_break);

    let left = left_instructions.collapse();
    let left = format_with(move |f: &mut TsppFormatter<'ast, '_>| {
        if let Some(left) = &left {
            f.write_element(*left);
        }

        Ok(())
    });

    let right = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        if matches!(
            f.context().tree.get(declaration.value),
            TypeExpression::Union { .. }
        ) {
            write!(f, [FormatNodeWithoutTrailingComments(declaration.value)])?;
            write!(
                f,
                [infix_or_postfix_annotations(f.context(), declaration.value)]
            )
        } else {
            write_type_expression_with_inline_prefix_annotations(f, declaration.value)
        }
    });

    let inner_content = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        // left side
        if left_may_break || layout == AssignmentLikeLayout::BreakLeftHandSide {
            write!(f, [left])?;
        } else {
            write!(f, [group(&left)])?;
        }

        // operator
        write!(f, [space(), token("=")])?;

        // right side
        write_assignment_like_right(f, layout, &right)
    });

    write!(f, [group(&inner_content)])?;

    // postfix annotations
    write!(f, [postfix_annotations(f.context(), node_id)])?;

    // terminator
    write_statement_terminator_after_anchor(f, f.context().span(declaration.value).end)
}

/// Format one extension declaration.
fn format_extension_declaration<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &ExtensionDeclaration,
) -> FormatResult<()> {
    let header = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        // prefixes
        format_declaration_export_modifier(f, node_id, declaration.export)?;
        write_keyword_prefix(f, Keyword::Declare, declaration.is_ambient)?;

        // head
        write!(f, [Keyword::Extension])?;

        if let Some(name) = declaration.name {
            write!(f, [space(), name])?;
        }

        // target
        write_declaration_generic_parameters(f, &declaration.generic_parameters)?;
        write!(f, [space(), Keyword::Of])?;

        let target_type = format_with(|f: &mut TsppFormatter<'ast, '_>| {
            write_type_expression_with_inline_prefix_annotations(f, declaration.target_type)
        });
        write!(
            f,
            [group(&indent(&format_args![
                soft_line_break_or_space(),
                group(&target_type)
            ]))]
        )?;

        // implements clause
        format_super_type_clause(f, Keyword::Implements, &declaration.implements_types)?;

        // where clause
        write_declaration_where_clauses(f, &declaration.where_clauses)
    });
    let header_group_id = f.group_id();
    write!(f, [group(&header).with_id(Some(header_group_id))])?;

    // body separator
    write_declaration_body_separator(f, header_group_id)?;

    // body
    write_member_block(f, node_id, &declaration.members, format_block_of_members)?;

    // postfix annotations
    write!(f, [postfix_annotations(f.context(), node_id)])
}

impl<'ast> FormatNode<'ast, Declaration> for Declaration {
    fn format_node(
        &self,
        node_id: LocalNodeId<Declaration>,
        f: &mut TsppFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // class declarations own their decorator placement relative to `export`
        if !matches!(self, Declaration::Class(_)) {
            write!(f, [prefix_annotations(f.context(), node_id)])?;
        }

        // main declaration body
        match self {
            Declaration::Global(declaration) => {
                format_global_declaration(f, node_id, declaration)?;
            }
            Declaration::Module(declaration) => {
                format_module_declaration(f, node_id, declaration)?;
            }
            Declaration::Type(declaration) => {
                format_type_declaration(f, node_id, declaration)?;
            }
            Declaration::Struct(declaration) => {
                format_struct_declaration(f, node_id, declaration)?;
            }
            Declaration::Class(declaration) => {
                let declaration_expression_id = declaration_expression_id(f.context(), node_id);
                format_class_declaration(f, node_id, declaration_expression_id, declaration)?;
            }
            Declaration::Enum(declaration) => {
                format_enum_declaration(f, node_id, declaration)?;
            }
            Declaration::Interface(declaration) => {
                format_interface_declaration(f, node_id, declaration)?;
            }
            Declaration::Extension(declaration) => {
                format_extension_declaration(f, node_id, declaration)?;
            }
            Declaration::Function(FunctionDeclaration {
                name,
                export,
                is_ambient,
                signature,
                body,
            }) => {
                if signature.form == FunctionForm::Lambda {
                    format_lambda_declaration(
                        f,
                        node_id,
                        *export,
                        *is_ambient,
                        *name,
                        signature,
                        body,
                    )?;
                } else {
                    format_function_declaration(
                        f,
                        node_id,
                        *export,
                        *is_ambient,
                        *name,
                        signature,
                        body,
                        FunctionCacheMode::default(),
                    )?;
                }
            }
        };

        Ok(())
    }
}

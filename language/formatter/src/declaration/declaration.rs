use crate::annotation::{
    DanglingIndentMode, FormatDanglingComments, FormatLeadingComments, format_comment,
    infix_or_postfix_annotations, postfix_annotations, prefix_annotations,
};
use crate::collection::member::format_block_of_members;
use crate::context::FormatNodeWithoutTrailingComments;
use crate::declaration::function::format_function_declaration;
use crate::declaration::sequence::format_block_statement_sequence;
use crate::declaration::signature::{
    default_generic_parameter_trailing_separator, format_where_clause_with_break,
    function_grouping_generic_parameter_is_plain, write_generic_parameter_list,
};
use crate::declaration::r#type::{
    format_class_declaration, format_enum_declaration, format_interface_declaration,
    format_struct_declaration,
};
use crate::declaration::{
    FunctionCacheMode, empty_block_with_infix_annotations, format_lambda_declaration,
    write_statement_terminator_after_anchor,
};
use crate::expression::{format_declarator, write_control_branch_after_head};
use crate::operator::{
    AssignmentLikeLayout, write_assignment_like_right,
    write_type_expression_with_inline_prefix_annotations,
};
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_dir::{
    Asynchrony, Comment, Declaration, Declarator, ExportKind, Expression, ExtensionDeclaration,
    FunctionDeclaration, FunctionForm, GenericParameter, GlobalDeclaration, Keyword, LetKind,
    LocalNodeId, Member, ModuleDeclaration, Mutability, NodeType, TokenSpan, TokenType,
    TypeDeclaration, TypeExpression, WhereClause,
};
use destack_fir::format::{
    FormatError, FormatNode as FirNode, FormatNodes, FormatResult, Formatter as FirFormatter,
    VecBuffer,
};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::{NodeSpanRegion, NodeSpanType};

const MIN_OVERLAP_FOR_BREAK: u32 = 3;

/// Return the `export` token for one declaration, if present.
pub(crate) fn declaration_export_token(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
) -> Option<TokenSpan> {
    let declaration_span = context.span(node_id);

    context
        .tokens
        .iter()
        .copied()
        .chain(context.side_tokens.iter().copied())
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
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
    export: ExportKind,
) -> Vec<Comment> {
    let declaration_span = context.span(node_id);
    let export_token = declaration_export_token(context, node_id);
    let Some(export_token) = export_token else {
        return Vec::new();
    };

    let mut comment_ids: Vec<Comment> = Vec::new();

    // export separator
    if let Some(next_token) = context.next_non_whitespace_token_after_span(export_token.span)
        && next_token.span.file == export_token.span.file
        && next_token.span.start > export_token.span.end
    {
        let comments = context.comments();
        comment_ids
            .extend(comments.comments_in_range(export_token.span.end, next_token.span.start));
    }

    // default separator
    if export == ExportKind::Default {
        let default_token = context
            .tokens
            .iter()
            .copied()
            .chain(context.side_tokens.iter().copied())
            .filter(|token| {
                token.span.file == declaration_span.file
                    && token.span.start >= export_token.span.end
                    && token.span.end <= declaration_span.end
                    && context.token_keyword(*token) == Some(Keyword::Default)
            })
            .min_by_key(|token| token.span.start);

        if let Some(default_token) = default_token
            && let Some(next_token) =
                context.next_non_whitespace_token_after_span(default_token.span)
            && next_token.span.file == default_token.span.file
            && next_token.span.start > default_token.span.end
        {
            let comments = context.comments();
            comment_ids
                .extend(comments.comments_in_range(default_token.span.end, next_token.span.start));
        }
    }

    comment_ids.sort_by_key(|comment| comment.span.start);
    comment_ids.dedup();

    comment_ids
}

/// Write comments between `export` and the declaration head.
pub(crate) fn write_declaration_export_head_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    export: ExportKind,
) -> FormatResult<()> {
    let comment_ids = declaration_export_head_comments(f.context(), node_id, export);

    // empty separator
    if comment_ids.is_empty() {
        return Ok(());
    }

    // comment sequence
    for comment_id in comment_ids {
        format_comment(f, comment_id)?;

        let is_line_comment = f
            .context()
            .comment_token_type_at_span(comment_id.span)
            .is_some_and(|token_type| {
                matches!(
                    token_type,
                    TokenType::LineComment | TokenType::DocLineComment
                )
            });

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
    f: &mut DestackFormatter<'ast, '_>,
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

/// Write one is_ambient prefix.
fn write_ambient_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    is_ambient: bool,
) -> FormatResult<()> {
    // is_ambient
    if is_ambient {
        write!(f, [Keyword::Declare, space()])?;
    }

    Ok(())
}

/// Write one declaration generic parameter list.
fn write_declaration_generic_parameters<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    generic_parameters: &[LocalNodeId<GenericParameter>],
) -> FormatResult<()> {
    // generic parameters
    if !generic_parameters.is_empty() {
        write_generic_parameter_list(
            f,
            generic_parameters,
            default_generic_parameter_trailing_separator(f),
        )?;
    }

    Ok(())
}

/// Write one declaration where clause list.
fn write_declaration_where_clauses<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    where_clauses: &[LocalNodeId<WhereClause>],
) -> FormatResult<()> {
    // where clauses
    if !where_clauses.is_empty() {
        format_where_clause_with_break(f, where_clauses)?;
    }

    Ok(())
}

/// Buffer one type declaration head so layout can inspect the formatted left side first.
fn buffer_type_declaration_left<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &TypeDeclaration,
) -> FormatResult<(Vec<FirNode>, bool, bool)> {
    let mut buffer = VecBuffer::new(f.state_mut());
    let formatter = &mut FirFormatter::new(&mut buffer);

    // prefixes
    format_declaration_export_modifier(formatter, node_id, declaration.export)?;
    write_ambient_prefix(formatter, declaration.is_ambient)?;

    // modifiers
    if declaration.is_nominal {
        write!(formatter, [Keyword::Newtype, space()])?;
    } else if declaration.mutability == Some(Mutability::Immutable) {
        write!(formatter, [Keyword::Readonly, space()])?;
    }

    // head
    if !declaration.is_nominal {
        write!(formatter, [Keyword::Type, space()])?;
    }

    write!(formatter, [declaration.name])?;

    // generic parameters
    write_declaration_generic_parameters(formatter, &declaration.generic_parameters)?;

    // where clauses
    write_declaration_where_clauses(formatter, &declaration.where_clauses)?;

    let nodes = buffer.into_vec();
    let is_left_short = nodes.single_line_width().is_some_and(|width| {
        width < (u32::from(f.context().options.indent_width) + MIN_OVERLAP_FOR_BREAK)
    });
    let left_may_break = nodes.may_directly_break();

    Ok((nodes, is_left_short, left_may_break))
}

/// Return whether one type expression counts as generic in one conditional head.
fn type_expression_is_assignment_like_generic_condition(
    context: &DestackFormatContext<'_>,
    type_id: LocalNodeId<TypeExpression>,
) -> bool {
    match context.tree.get(type_id) {
        TypeExpression::Reference {
            generic_arguments, ..
        }
        | TypeExpression::Member {
            generic_arguments, ..
        } => !generic_arguments.is_empty(),

        TypeExpression::FunctionTypeDeclaration(function) => {
            !function.generic_parameters.is_empty()
        }
        TypeExpression::ConstructorTypeDeclaration(function) => {
            !function.generic_parameters.is_empty()
        }

        _ => false,
    }
}

/// Return whether one type declaration rhs should break after `=`.
fn type_declaration_should_break_after_operator(
    context: &DestackFormatContext<'_>,
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

        // unions own their indentation logic
        TypeExpression::Union { .. } => false,

        _ => comments.has_comment_before(value_start),
    }
}

/// Return whether one type alias has a complex generic head.
fn type_declaration_has_complex_generic_head(
    context: &DestackFormatContext<'_>,
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
    context: &DestackFormatContext<'_>,
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
    context: &DestackFormatContext<'_>,
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
    f: &mut DestackFormatter<'ast, '_>,
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

/// Write one body made from declaration members.
fn write_member_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    members: &[LocalNodeId<Member>],
    break_before_body: bool,
) -> FormatResult<()> {
    // empty body
    if members.is_empty() {
        if break_before_body {
            write!(
                f,
                [
                    hard_line_break(),
                    empty_block_with_infix_annotations(node_id)
                ]
            )?;
        } else {
            write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
        }

        return Ok(());
    }

    // member body
    if break_before_body {
        write!(f, [hard_line_break(), token("{"), hard_line_break()])?;
    } else {
        write!(f, [space(), token("{"), hard_line_break()])?;
    }

    write!(
        f,
        [group(&block_indent(&format_with(move |f| {
            format_block_of_members(f, members)
        })))]
    )?;
    write!(f, [hard_line_break(), token("}")])
}

/// Format one super-type clause.
pub(crate) fn format_super_type_clause<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    keyword: Keyword,
    types: &[LocalNodeId<TypeExpression>],
) -> FormatResult<()> {
    format_super_type_clause_with_expand(f, keyword, types, false, false)
}

/// Format one super-type clause and optionally force expansion.
pub(crate) fn format_super_type_clause_with_expand<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    keyword: Keyword,
    types: &[LocalNodeId<TypeExpression>],
    force_expand: bool,
    start_on_new_line: bool,
) -> FormatResult<()> {
    // empty clause
    if types.is_empty() {
        return Ok(());
    }

    let entries = format_with(move |f| {
        // entries
        if start_on_new_line || force_expand {
            f.join_with(&format_args![token(","), hard_line_break()])
                .entries(types.iter().copied().map(|type_id| {
                    format_with(move |f| {
                        write_type_expression_with_inline_prefix_annotations(f, type_id)
                    })
                }))
                .finish()
        } else {
            f.join_with(&format_args![token(","), soft_line_break_or_space()])
                .entries(types.iter().copied().map(|type_id| {
                    format_with(move |f| {
                        write_type_expression_with_inline_prefix_annotations(f, type_id)
                    })
                }))
                .finish()
        }
    });
    let clause = format_with(move |f| {
        // separator
        if start_on_new_line {
            write!(f, [hard_line_break()])?;
        } else if force_expand {
            write!(f, [soft_line_break_or_space()])?;
        } else {
            write!(f, [space()])?;
        }

        // expanded head
        if start_on_new_line || force_expand {
            write!(f, [keyword, hard_line_break(), entries])
        } else {
            write!(
                f,
                [
                    keyword,
                    group(&indent(&format_args![soft_line_break_or_space(), entries]))
                ]
            )
        }
    });

    if start_on_new_line || force_expand {
        write!(f, [group(&indent(&clause))])
    } else {
        write!(f, [group(&clause)])
    }
}

/// Format one `let` or `const` statement.
pub(crate) fn format_let_statement_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
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

            write_ambient_prefix(f, is_ambient)?;

            if is_shared {
                write!(f, [token("shared"), space()])?;
            }

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
    f: &mut DestackFormatter<'ast, '_>,
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
        .get_side_span(let_else_id, NodeSpanType::Region(NodeSpanRegion::Clause))
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
    f: &mut DestackFormatter<'ast, '_>,
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

            write_ambient_prefix(f, is_ambient)?;

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
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &GlobalDeclaration,
) -> FormatResult<()> {
    // prefixes
    format_declaration_export_modifier(f, node_id, None)?;
    if f.context()
        .tree
        .get_side_span(node_id, NodeSpanType::Region(NodeSpanRegion::Prelude))
        .is_some()
    {
        write_ambient_prefix(f, declaration.is_ambient)?;
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
    f: &mut DestackFormatter<'ast, '_>,
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
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &TypeDeclaration,
) -> FormatResult<()> {
    let (left_nodes, is_left_short, left_may_break) =
        buffer_type_declaration_left(f, node_id, declaration)?;
    let layout = type_declaration_layout(f.context(), declaration, is_left_short, left_may_break);

    let left = f.intern_vec(left_nodes);
    let left = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        if let Some(left) = &left {
            f.write_node(left.clone());
        }

        Ok(())
    });

    let right = format_with(|f: &mut DestackFormatter<'ast, '_>| {
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

    let inner_content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
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
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &ExtensionDeclaration,
) -> FormatResult<()> {
    // prefixes
    format_declaration_export_modifier(f, node_id, declaration.export)?;
    write_ambient_prefix(f, declaration.is_ambient)?;

    // head
    write!(f, [Keyword::Extension])?;

    if let Some(name) = declaration.name {
        write!(f, [space(), name])?;
    }

    write_declaration_generic_parameters(f, &declaration.generic_parameters)?;
    write!(f, [space(), Keyword::Of, space()])?;
    write_type_expression_with_inline_prefix_annotations(f, declaration.target_type)?;

    // heritage
    format_super_type_clause(f, Keyword::Implements, &declaration.implements_types)?;

    // where clauses
    write_declaration_where_clauses(f, &declaration.where_clauses)?;

    // body
    write_member_body(f, node_id, &declaration.members, false)?;

    // postfix annotations
    write!(f, [postfix_annotations(f.context(), node_id)])
}

impl<'ast> FormatNode<'ast, Declaration> for Declaration {
    fn format_node(
        &self,
        node_id: LocalNodeId<Declaration>,
        f: &mut DestackFormatter<'ast, '_>,
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

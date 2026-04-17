use crate::format::annotation::{
    format_raw_comment, postfix_annotations_without_line_suffix_boundary, prefix_annotations,
};
use crate::format::collection::member::format_block_of_members;
use crate::format::declaration::function::format_function_declaration;
use crate::format::declaration::sequence::format_block_statement_sequence;
use crate::format::declaration::signature::{
    default_generic_parameter_trailing_separator, format_where_clause_with_break,
    write_generic_parameter_list,
};
use crate::format::declaration::r#type::{
    format_class_declaration, format_enum_declaration, format_interface_declaration,
    format_struct_declaration,
};
use crate::format::declaration::write_statement_terminator_after_anchor;
use crate::format::expression::format_declarator;
use crate::format::operator::write_type_expression_with_inline_prefix_annotations;
use crate::{
    DestackFormatContext, DestackFormatter, FormatNode, empty_block_with_infix_annotations,
};
use destack_ast::{
    Ambientness, Asynchrony, Declaration, Declarator, ExportMode, Expression, FunctionDeclaration,
    GlobalDeclaration, ImportAliasDeclaration, ImportAliasTarget, Keyword, LetKind, LocalNodeId,
    NamespaceDeclaration, NamespaceKind, NodeType, TypeDeclaration, TypeExpression,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};

/// Return raw comments between `export` and the declaration head.
fn declaration_export_head_comment_nodes(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
    export: ExportMode,
) -> Vec<destack_ast::Comment> {
    let declaration_span = context.span(node_id);

    let export_token = context
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
        .min_by_key(|token| token.span.start);
    let Some(export_token) = export_token else {
        return Vec::new();
    };

    let mut comment_ids: Vec<destack_ast::Comment> = Vec::new();

    // export boundary
    if let Some(next_token) = context.next_non_whitespace_token_after_span(export_token.span)
        && next_token.span.file == export_token.span.file
        && next_token.span.start > export_token.span.end
    {
        let comments = context.comments();
        comment_ids
            .extend(comments.comments_in_range(export_token.span.end, next_token.span.start));
    }

    // default boundary
    if export == ExportMode::Default {
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

/// Write raw comments between `export` and the declaration head.
fn write_declaration_export_head_boundary_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    export: ExportMode,
) -> FormatResult<()> {
    let comment_ids = declaration_export_head_comment_nodes(f.context(), node_id, export);

    // empty boundary
    if comment_ids.is_empty() {
        return Ok(());
    }

    // comment sequence
    for comment_id in comment_ids {
        format_raw_comment(f, comment_id)?;

        let is_line_comment = f
            .context()
            .comment_token_type_at_span(comment_id.span)
            .is_some_and(|token_type| {
                matches!(
                    token_type,
                    destack_ast::TokenType::LineComment | destack_ast::TokenType::DocLineComment
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
    export: Option<ExportMode>,
) -> FormatResult<()> {
    // export
    match export {
        Some(ExportMode::Named) => {
            write!(f, [Keyword::Export, space()])?;
            write_declaration_export_head_boundary_comments(f, node_id, ExportMode::Named)?;
        }
        Some(ExportMode::Default) => {
            write!(f, [Keyword::Export, space(), Keyword::Default, space()])?;
            write_declaration_export_head_boundary_comments(f, node_id, ExportMode::Default)?;
        }
        None => {}
    }

    Ok(())
}

/// Write one ambient prefix.
fn write_ambient_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    ambient: Ambientness,
) -> FormatResult<()> {
    // ambient
    if ambient.is_ambient() {
        write!(f, [Keyword::Declare, space()])?;
    }

    Ok(())
}

/// Write one declaration generic parameter list.
fn write_declaration_generic_parameters<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    generic_parameters: &[LocalNodeId<destack_ast::GenericParameter>],
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
    where_clauses: &[LocalNodeId<destack_ast::WhereClause>],
) -> FormatResult<()> {
    // where clauses
    if !where_clauses.is_empty() {
        format_where_clause_with_break(f, where_clauses)?;
    }

    Ok(())
}

/// Return the wrapper expression when one declaration appears in expression position.
fn declaration_expression_id(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
) -> Option<LocalNodeId<Expression>> {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return None;
    };
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
        write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
        return Ok(());
    }

    // statement body
    write!(f, [space(), token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&block_indent(&format_with(move |f| {
            format_block_statement_sequence(f, expressions, false)
        })))]
    )?;
    write!(f, [hard_line_break(), token("}")])
}

/// Write one body made from declaration members.
fn write_member_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    members: &[LocalNodeId<destack_ast::Member>],
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

    let clause = format_with(move |f| {
        // clause boundary
        if start_on_new_line {
            write!(f, [hard_line_break()])?;
        } else if force_expand {
            write!(f, [soft_line_break_or_space()])?;
        } else {
            write!(f, [space()])?;
        }

        // clause head
        write!(f, [keyword, space()])?;

        // clause entries
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
    export: Option<ExportMode>,
    ambient: Ambientness,
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
                Some(ExportMode::Named) => write!(f, [Keyword::Export, space()])?,
                Some(ExportMode::Default) => {
                    write!(f, [Keyword::Export, space(), Keyword::Default, space()])?;
                }
                None => {}
            }

            write_ambient_prefix(f, ambient)?;

            // binding keyword
            match kind {
                LetKind::Let => write!(f, [Keyword::Let])?,
                LetKind::Var => write!(f, [Keyword::Var])?,
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

/// Format one `using` statement.
pub(crate) fn format_using_statement_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    asynchrony: Asynchrony,
    export: Option<ExportMode>,
    ambient: Ambientness,
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
                Some(ExportMode::Named) => write!(f, [Keyword::Export, space()])?,
                Some(ExportMode::Default) => {
                    write!(f, [Keyword::Export, space(), Keyword::Default, space()])?;
                }
                None => {}
            }

            write_ambient_prefix(f, ambient)?;

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
    write_ambient_prefix(f, declaration.ambient)?;

    // head
    write!(f, [token("global")])?;

    // body
    write_expression_declaration_body(f, node_id, &declaration.expressions)
}

/// Format one namespace declaration.
fn format_namespace_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &NamespaceDeclaration,
) -> FormatResult<()> {
    // prefixes
    format_declaration_export_modifier(f, node_id, declaration.export)?;
    write_ambient_prefix(f, declaration.ambient)?;

    // keyword
    match declaration.kind {
        NamespaceKind::Namespace => write!(f, [Keyword::Namespace])?,
        NamespaceKind::Module => write!(f, [token("module")])?,
    }

    // name
    write!(f, [space(), declaration.name])?;

    // generic parameters
    write_declaration_generic_parameters(f, &declaration.generic_parameters)?;

    // where clauses
    write_declaration_where_clauses(f, &declaration.where_clauses)?;

    // body
    write_expression_declaration_body(f, node_id, &declaration.expressions)
}

/// Format one type alias declaration.
fn format_type_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &TypeDeclaration,
) -> FormatResult<()> {
    // prefixes
    format_declaration_export_modifier(f, node_id, declaration.export)?;
    write_ambient_prefix(f, declaration.ambient)?;

    if declaration.is_nominal {
        write!(f, [Keyword::Newtype, space()])?;
    } else if declaration.mutability == Some(destack_ast::Mutability::Immutable) {
        write!(f, [Keyword::Readonly, space()])?;
    }

    // head
    write!(f, [Keyword::Type, space(), declaration.name])?;

    // generic parameters
    write_declaration_generic_parameters(f, &declaration.generic_parameters)?;

    // where clauses
    write_declaration_where_clauses(f, &declaration.where_clauses)?;

    // value
    write!(f, [space(), token("="), space()])?;
    write_type_expression_with_inline_prefix_annotations(f, declaration.value)?;
    write_statement_terminator_after_anchor(f, f.context().span(declaration.value).end)
}

/// Format one import alias declaration.
fn format_import_alias_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &ImportAliasDeclaration,
) -> FormatResult<()> {
    // prefixes
    format_declaration_export_modifier(f, node_id, declaration.export)?;
    write_ambient_prefix(f, declaration.ambient)?;

    // head
    write!(f, [Keyword::Import, space()])?;
    if declaration.kind == destack_ast::DependencyKind::Type {
        write!(f, [Keyword::Type, space()])?;
    }

    write!(f, [declaration.name, space(), token("="), space()])?;

    // target
    match &declaration.target {
        ImportAliasTarget::Require { target } => {
            write!(
                f,
                [
                    token("require"),
                    token("("),
                    token("\""),
                    *target,
                    token("\""),
                    token(")")
                ]
            )?;
        }
        ImportAliasTarget::Path { path } => {
            write!(f, [path.clone()])?;
        }
    }

    write_statement_terminator_after_anchor(f, f.context().span(node_id).end)
}

/// Format one extension declaration.
fn format_extension_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &destack_ast::ExtensionDeclaration,
) -> FormatResult<()> {
    // prefixes
    format_declaration_export_modifier(f, node_id, declaration.export)?;
    write_ambient_prefix(f, declaration.ambient)?;

    // head
    write!(f, [Keyword::Extension])?;

    if let Some(name) = declaration.name {
        write!(f, [space(), name])?;
    }

    write_declaration_generic_parameters(f, &declaration.generic_parameters)?;
    write!(f, [space(), Keyword::For, space()])?;
    write_type_expression_with_inline_prefix_annotations(f, declaration.target_type)?;

    // heritage
    format_super_type_clause(f, Keyword::Implements, &declaration.implements_types)?;

    // where clauses
    write_declaration_where_clauses(f, &declaration.where_clauses)?;

    // body
    write_member_body(f, node_id, &declaration.members, false)
}

impl<'ast> FormatNode<'ast, Declaration> for Declaration {
    fn format_node(
        &self,
        node_id: LocalNodeId<Declaration>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [prefix_annotations(f.context(), node_id)])?;

        // main declaration body
        match self {
            Declaration::Global(declaration) => {
                format_global_declaration(f, node_id, declaration)?;
            }
            Declaration::Namespace(declaration) => {
                format_namespace_declaration(f, node_id, declaration)?;
            }
            Declaration::Type(declaration) => {
                format_type_declaration(f, node_id, declaration)?;
            }
            Declaration::ImportAlias(declaration) => {
                format_import_alias_declaration(f, node_id, declaration)?;
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
                ambient,
                signature,
                body,
            }) => {
                format_function_declaration(f, node_id, *export, *ambient, *name, signature, body)?;
            }
        }

        // postfix annotations
        write!(
            f,
            [postfix_annotations_without_line_suffix_boundary(
                f.context(),
                node_id
            )]
        )
    }
}

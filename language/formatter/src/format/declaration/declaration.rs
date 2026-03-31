use crate::format::collection::member::format_block_of_members;
use crate::format::collection::property::format_key_with_quotes;
use crate::format::declaration::assignment::format_type_alias_assignment_like;
use crate::format::declaration::sequence::format_block_statement_sequence;
use crate::format::declaration::signature::format_where_clause_with_break;
use crate::format::expression::{expression_has_static_type_arguments, format_declarator};
use crate::{
    DestackFormatContext, DestackFormatter, FormatNode as AstFormatNode,
    empty_block_with_infix_annotations,
};
use destack_ast::{
    AnnotationPosition, Asynchrony, Declaration, DeclarationDescriptor, DeclarationKind,
    Declarator, DependencyKind, DependencyMode, Expression, FunctionKind, Generics, Heritage,
    ImportAliasTarget, Key, Keyword, LetKind, LocalNodeId, Member, Mutability, Name, NamespaceKind,
    NodeType, Parameter, TypeKind, Visibility,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};

use crate::format::declaration::function::format_function_declaration;
use crate::format::declaration::r#type::{
    format_enum_declaration, format_interface_declaration, format_struct_or_class_declaration,
};

/// Return the raw comment nodes between `export` and the declaration head.
fn declaration_export_head_comment_nodes(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
    export_mode: DependencyMode,
) -> Vec<LocalNodeId<destack_ast::Comment>> {
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

    let mut comment_ids = Vec::new();

    if let Some(next_token_after_export) =
        context.next_non_whitespace_token_after_span(export_token.span)
        && next_token_after_export.span.file == export_token.span.file
        && next_token_after_export.span.start > export_token.span.end
    {
        comment_ids.extend(
            context
                .comment_nodes_in_range(export_token.span.end, next_token_after_export.span.start),
        );
    }

    if export_mode == DependencyMode::Default {
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
            && let Some(next_token_after_default) =
                context.next_non_whitespace_token_after_span(default_token.span)
            && next_token_after_default.span.file == default_token.span.file
            && next_token_after_default.span.start > default_token.span.end
        {
            comment_ids.extend(context.comment_nodes_in_range(
                default_token.span.end,
                next_token_after_default.span.start,
            ));
        }
    }

    comment_ids.sort_by_key(|comment_id| context.span(*comment_id).start);
    comment_ids.dedup();

    comment_ids
}

/// Write raw comment seams between `export` and the declaration head.
fn write_declaration_export_head_comment_seams<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    export_mode: DependencyMode,
) -> FormatResult<()> {
    let comment_ids = declaration_export_head_comment_nodes(f.context(), node_id, export_mode);
    if comment_ids.is_empty() {
        return Ok(());
    }

    for comment_id in comment_ids {
        write!(f, [comment_id])?;

        let comment_span = f.context().span(comment_id);
        let is_line_comment = f
            .context()
            .comment_token_type_at_span(comment_span)
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

/// Format one declaration export modifier and export-head seam comments.
pub(crate) fn format_declaration_export_modifier<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &destack_ast::DeclarationDescriptor,
) -> FormatResult<()> {
    if let Some(export) = descriptor.export {
        write!(f, [export, space()])?;
        write_declaration_export_head_comment_seams(f, node_id, export)?;
    }

    Ok(())
}

/// Format a super type clause.
pub(crate) fn format_super_type_clause<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    keyword: Keyword,
    types: &[LocalNodeId<Expression>],
) -> FormatResult<()> {
    format_super_type_clause_with_expand(f, keyword, types, false, false)
}

/// Format a super type clause and optionally force line breaking.
pub(crate) fn format_super_type_clause_with_expand<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    keyword: Keyword,
    types: &[LocalNodeId<Expression>],
    force_expand: bool,
    start_on_new_line: bool,
) -> FormatResult<()> {
    assert!(!types.is_empty());
    let should_group_clause = !start_on_new_line
        && !force_expand
        && super_type_clause_prefers_group_mode(f.context(), types);

    let format_clause = format_with(|f| {
        if start_on_new_line {
            write!(f, [hard_line_break()])?;
        } else if force_expand || should_group_clause {
            write!(f, [soft_line_break_or_space()])?;
        } else {
            write!(f, [space()])?;
        }

        write!(f, [keyword, space()])?;

        let format_types = types
            .iter()
            .copied()
            .map(|type_id| format_with(move |f| format_super_type_expression(f, keyword, type_id)));
        if start_on_new_line && !force_expand {
            f.join_with(&format_args![&token(","), space()])
                .entries(format_types)
                .finish()
        } else {
            f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                .entries(format_types)
                .finish()
        }
    });

    if start_on_new_line || force_expand || should_group_clause {
        write!(
            f,
            [group(&indent(&format_clause)).should_expand(force_expand)]
        )
    } else {
        write!(f, [group(&format_clause)])
    }
}

/// Return whether one super-type clause should use grouped head layout.
fn super_type_clause_prefers_group_mode(
    context: &DestackFormatContext<'_>,
    types: &[LocalNodeId<Expression>],
) -> bool {
    if types.len() > 1 {
        return true;
    }

    let Some(type_id) = types.first().copied() else {
        return false;
    };
    if expression_has_static_type_arguments(context, type_id) {
        return false;
    }

    let type_id = match context.tree.get(type_id) {
        Expression::Statement(inner_type_id)
        | Expression::Parenthesized {
            expression: inner_type_id,
        } => *inner_type_id,
        _ => type_id,
    };

    matches!(
        context.tree.get(type_id),
        Expression::Path { path, .. } if path.segments.len() > 1
    ) || matches!(
        context.tree.get(type_id),
        Expression::Member { .. } | Expression::PrivateMember { .. }
    )
}

/// Format one super type expression.
fn format_super_type_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    keyword: Keyword,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let should_wrap_class_extends_head = keyword == Keyword::Extends
        && class_extends_expression_requires_parenthesized_head(f.context(), expression_id);
    if should_wrap_class_extends_head {
        write!(f, [token("("), expression_id, token(")")])?;
    } else {
        write!(f, [expression_id])?;
    }

    Ok(())
}

/// Return whether one class extends head requires explicit parenthesized grouping.
fn class_extends_expression_requires_parenthesized_head(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(expression_id) else {
        return false;
    };
    if parent_type != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
    let Declaration::Class { heritage, .. } = context.tree.get(declaration_id) else {
        return false;
    };
    let Some(extends_types) = heritage.extends_types.as_ref() else {
        return false;
    };
    if !extends_types.contains(&expression_id) {
        return false;
    }

    super_type_has_invalid_unparenthesized_head(context.tree, expression_id)
}

/// Return whether one class heritage expression starts with an invalid unparenthesized head.
fn super_type_has_invalid_unparenthesized_head(
    tree: &destack_ast::NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    // unparenthesized lambdas are not valid in class heritage heads
    if matches!(
        tree.get(expression_id),
        Expression::Declaration(declaration_id)
            if matches!(
                tree.get(*declaration_id),
                Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
            )
    ) {
        return true;
    }

    // these forms require explicit parentheses in class heritage expressions
    matches!(
        tree.get(expression_id),
        Expression::ObjectExpression { .. }
            | Expression::Unary { .. }
            | Expression::Binary { .. }
            | Expression::TypeBinary { .. }
            | Expression::TypeConditional { .. }
            | Expression::If { .. }
            | Expression::Assign { .. }
            | Expression::SequenceExpression { .. }
    )
}

/// Format one variable-like `let` statement expression.
pub(crate) fn format_let_statement_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    kind: LetKind,
    descriptor: &DeclarationDescriptor,
    declarators: &[LocalNodeId<Declarator>],
) -> FormatResult<()> {
    let tree = f.context().tree;

    // export import equals
    let handled_export_import_equals =
        crate::format::declaration::dependency::format_export_import_equals_statement(
            f,
            tree,
            descriptor,
            declarators,
        )?;

    // keyword header
    if !handled_export_import_equals {
        if let Some(export) = descriptor.export {
            write!(f, [export, space()])?;
        }

        if descriptor.kind == DeclarationKind::Declaration {
            write!(f, [Keyword::Declare, space()])?;
        }

        match kind {
            LetKind::Let => write!(f, [Keyword::Let])?,
            LetKind::Var => write!(f, [Keyword::Var])?,
            LetKind::Const => write!(f, [Keyword::Const])?,
        }

        for (index, declarator_id) in declarators.iter().enumerate() {
            if index > 0 {
                write!(f, [token(",")])?;
            }

            write!(f, [space()])?;
            format_declarator(f, tree, *declarator_id)?;
        }
    }

    Ok(())
}

/// Format one `using` statement expression.
pub(crate) fn format_using_statement_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    asynchrony: Asynchrony,
    descriptor: &DeclarationDescriptor,
    declarators: &[LocalNodeId<Declarator>],
) -> FormatResult<()> {
    let tree = f.context().tree;

    // keyword header
    if let Some(export) = descriptor.export {
        write!(f, [export, space()])?;
    }

    if descriptor.kind == DeclarationKind::Declaration {
        write!(f, [Keyword::Declare, space()])?;
    }

    if asynchrony == Asynchrony::Async {
        write!(f, [Keyword::Await, space()])?;
    }

    write!(f, [Keyword::Using])?;

    for (index, declarator_id) in declarators.iter().enumerate() {
        if index > 0 {
            write!(f, [token(",")])?;
        }

        write!(f, [space()])?;
        format_declarator(f, tree, *declarator_id)?;
    }

    Ok(())
}

/// Format a global augmentation declaration.
pub(crate) fn format_global_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    expressions: &[LocalNodeId<Expression>],
) -> FormatResult<()> {
    // export
    format_declaration_export_modifier(f, node_id, descriptor)?;

    // kind
    if descriptor.kind == DeclarationKind::Declaration {
        write!(f, [Keyword::Declare, space()])?;
    }

    // keyword
    write!(f, [token("global")])?;

    // body
    write!(f, [space()])?;
    if expressions.is_empty() {
        write!(f, [empty_block_with_infix_annotations(node_id)])?;
        write!(
            f,
            [crate::format::annotation::postfix_annotations(
                f.context(),
                node_id
            )]
        )?;
    } else {
        let expressions = expressions.to_vec();
        write!(f, [token("{"), hard_line_break()])?;
        write!(
            f,
            [group(&block_indent(&format_with(move |_f| {
                format_block_statement_sequence(_f, &expressions, false)
            })))]
        )?;
        write!(
            f,
            [
                crate::format::annotation::block_infix_annotations(f.context(), node_id),
                token("}")
            ]
        )?;
    }

    Ok(())
}

/// Format a namespace declaration.
pub(crate) fn format_namespace_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    kind: NamespaceKind,
    generics: &Generics,
    expressions: &[LocalNodeId<Expression>],
) -> FormatResult<()> {
    // export
    format_declaration_export_modifier(f, node_id, descriptor)?;

    // kind
    if descriptor.kind == DeclarationKind::Declaration {
        write!(f, [Keyword::Declare, space()])?;
    }

    // keyword
    if kind == NamespaceKind::Module {
        write!(f, [token("module")])?;
    } else {
        write!(f, [Keyword::Namespace])?;
    }

    // name / key
    if let Some(name) = descriptor.name {
        write!(f, [space()])?;
        if matches!(name, Name::String(_)) {
            format_key_with_quotes(f, Key::Name(name), true)?;
        } else {
            write!(f, [name])?;
        }
    }

    // where
    if let Some(where_clauses) = generics.where_clauses.as_ref()
        && !where_clauses.is_empty()
    {
        format_where_clause_with_break(f, where_clauses)?;
    }

    // body
    write!(f, [space()])?;
    if expressions.is_empty() {
        write!(f, [empty_block_with_infix_annotations(node_id)])?;
        write!(
            f,
            [crate::format::annotation::postfix_annotations(
                f.context(),
                node_id
            )]
        )?;
    } else {
        let expressions = expressions.to_vec();
        write!(f, [token("{"), hard_line_break()])?;
        write!(
            f,
            [group(&block_indent(&format_with(move |_f| {
                format_block_statement_sequence(_f, &expressions, false)
            })))]
        )?;
        write!(
            f,
            [
                hard_line_break(),
                crate::format::annotation::block_infix_annotations(f.context(), node_id),
                token("}")
            ]
        )?;
    }

    Ok(())
}

/// Format an import alias declaration.
pub(crate) fn format_import_alias_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    kind: DependencyKind,
    target: &ImportAliasTarget,
) -> FormatResult<()> {
    // export
    format_declaration_export_modifier(f, node_id, descriptor)?;

    // keyword
    write!(f, [Keyword::Import])?;
    if kind == DependencyKind::Type {
        write!(f, [space(), Keyword::Type])?;
    }

    // name
    if let Some(name) = descriptor.name {
        write!(f, [space(), name])?;
    }

    // target
    write!(f, [space(), token("="), space()])?;
    match target {
        ImportAliasTarget::Require { target } => {
            write!(
                f,
                [
                    token("require"),
                    token("("),
                    token("\""),
                    target,
                    token("\""),
                    token(")")
                ]
            )?;
        }
        ImportAliasTarget::Path { value } => {
            write!(f, [*value])?;
        }
    }

    write!(f, [token(";")])?;
    write!(
        f,
        [crate::format::annotation::line_postfix_boundary_annotations(f.context(), node_id)]
    )?;
    Ok(())
}

/// Format an extension declaration.
pub(crate) fn format_extension_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    generics: &Generics,
    target_type: LocalNodeId<Expression>,
    heritage: &Heritage,
    members: &[LocalNodeId<Member>],
) -> FormatResult<()> {
    // export
    format_declaration_export_modifier(f, node_id, descriptor)?;

    // kind
    if descriptor.kind == DeclarationKind::Declaration {
        write!(f, [Keyword::Declare, space()])?;
    }

    // keyword
    write!(f, [Keyword::Extension])?;

    // named extensions: `extension Name<T> of Target`
    // anonymous extensions: `extension<T> of Target`
    if let Some(name) = descriptor.name {
        // named: name first, then generics
        write!(f, [space(), name])?;

        if let Some(static_arguments) = generics.static_parameters.as_ref()
            && !static_arguments.is_empty()
        {
            write!(
                f,
                [group(&format_args![
                    token("<"),
                    soft_block_indent(&format_with(|f| {
                        f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                            .entries(static_arguments)
                            .finish()
                    })),
                    token(">")
                ])]
            )?;
        }
    } else {
        // anonymous: generics first, no name
        if let Some(static_arguments) = generics.static_parameters.as_ref()
            && !static_arguments.is_empty()
        {
            write!(
                f,
                [group(&format_args![
                    token("<"),
                    soft_block_indent(&format_with(|f| {
                        f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                            .entries(static_arguments)
                            .finish()
                    })),
                    token(">")
                ])]
            )?;
        }
    }

    // for keyword + target type
    write!(f, [space(), Keyword::For, space(), target_type])?;

    // implements types
    if let Some(implements_types) = heritage.implements_types.as_ref()
        && !implements_types.is_empty()
    {
        format_super_type_clause(f, Keyword::Implements, implements_types)?;
    }

    // where
    if let Some(where_clauses) = generics.where_clauses.as_ref()
        && !where_clauses.is_empty()
    {
        format_where_clause_with_break(f, where_clauses)?;
    }

    // body
    if members.is_empty() {
        write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
        return Ok(());
    }

    // body
    write!(f, [space(), token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&format_args![block_indent(&format_with(|f| {
            format_block_of_members(f, members)
        })),])]
    )?;
    write!(f, [hard_line_break(), token("}")])?;

    Ok(())
}

/// Format a type alias declaration.
pub(crate) fn format_type_alias_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    kind: TypeKind,
    mutability: Option<Mutability>,
    static_parameters: &Option<Vec<LocalNodeId<Parameter>>>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    format_type_alias_assignment_like(
        f,
        node_id,
        descriptor,
        kind,
        mutability,
        static_parameters,
        value_id,
    )?;

    // type alias declarations need trailing semicolon (like const/let)
    write!(f, [token(";")])?;
    write!(
        f,
        [crate::format::annotation::line_postfix_boundary_annotations(f.context(), node_id)]
    )?;

    Ok(())
}

impl<'ast> Format<DestackFormatContext<'ast>> for Visibility {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Visibility::Public => write!(f, [Keyword::Public]),
            Visibility::Protected => write!(f, [Keyword::Protected]),
            Visibility::Private => write!(f, [Keyword::Private]),
        }
    }
}

impl<'ast> Format<DestackFormatContext<'ast>> for DependencyMode {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            DependencyMode::Item => write!(f, [Keyword::Export]),
            DependencyMode::Default => write!(f, [Keyword::Export, space(), Keyword::Default]),
            DependencyMode::Namespace => write!(f, [Keyword::Export]),
        }
    }
}

impl<'ast> AstFormatNode<'ast, Declaration> for Declaration {
    fn format_node(
        &self,
        node_id: LocalNodeId<Declaration>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let is_lambda_declaration = matches!(
            self,
            Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
        );
        let declaration_expression_id =
            if let Some((parent_id, parent_type)) = f.context().parent(node_id) {
                if parent_type == NodeType::Expression {
                    let expression_id = LocalNodeId::<Expression>::new(parent_id);
                    match f.context().tree.get(expression_id) {
                        Expression::Declaration(parent_declaration_id)
                            if *parent_declaration_id == node_id =>
                        {
                            Some(expression_id)
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            };
        write!(
            f,
            [crate::format::annotation::prefix_annotations(
                f.context(),
                node_id
            )]
        )?;
        let mut declaration_emits_boundary_before_terminator = false;

        match self {
            // global augmentation
            Declaration::Global {
                descriptor,
                expressions,
            } => {
                format_global_declaration(f, node_id, descriptor, expressions)?;
            }

            // module
            Declaration::Namespace {
                descriptor,
                kind,
                generics,
                expressions,
            } => {
                format_namespace_declaration(f, node_id, descriptor, *kind, generics, expressions)?;
            }

            // type alias
            Declaration::Type {
                descriptor,
                kind,
                mutability,
                static_parameters,
                value: value_id,
            } => {
                declaration_emits_boundary_before_terminator = true;
                format_type_alias_declaration(
                    f,
                    node_id,
                    descriptor,
                    *kind,
                    *mutability,
                    static_parameters,
                    *value_id,
                )?;
            }

            // import alias
            Declaration::ImportAlias {
                descriptor,
                kind,
                target,
            } => {
                declaration_emits_boundary_before_terminator = true;
                format_import_alias_declaration(f, node_id, descriptor, *kind, target)?;
            }

            // struct or class
            Declaration::Struct {
                descriptor,
                generics,
                heritage,
                members,
            }
            | Declaration::Class {
                descriptor,
                generics,
                heritage,
                members,
            } => {
                declaration_emits_boundary_before_terminator = true;
                let is_class = matches!(self, Declaration::Class { .. });
                if format_struct_or_class_declaration(
                    f,
                    node_id,
                    declaration_expression_id,
                    descriptor,
                    generics,
                    heritage,
                    members,
                    is_class,
                )? {
                    return Ok(());
                }
            }

            // enum
            Declaration::Enum {
                descriptor,
                kind,
                generics,
                heritage,
                fields,
                members,
            } => {
                declaration_emits_boundary_before_terminator = true;
                if format_enum_declaration(
                    f, node_id, descriptor, *kind, generics, heritage, fields, members,
                )? {
                    return Ok(());
                }
            }

            // interface
            Declaration::Interface {
                descriptor,
                kind,
                generics,
                heritage,
                members,
            } => {
                declaration_emits_boundary_before_terminator = true;
                if format_interface_declaration(
                    f, node_id, descriptor, *kind, generics, heritage, members,
                )? {
                    return Ok(());
                }
            }

            // extension
            Declaration::Extension {
                descriptor,
                generics,
                target_type,
                heritage,
                members,
            } => {
                format_extension_declaration(
                    f,
                    node_id,
                    descriptor,
                    generics,
                    *target_type,
                    heritage,
                    members,
                )?;
            }

            // function
            Declaration::Function {
                descriptor,
                signature,
                body,
            } => {
                declaration_emits_boundary_before_terminator = true;
                format_function_declaration(f, node_id, descriptor, signature, body)?;
            }
        }

        if !declaration_emits_boundary_before_terminator {
            write!(
                f,
                [
                    crate::format::annotation::line_postfix_boundary_annotations(
                        f.context(),
                        node_id
                    )
                ]
            )?;
        }

        let should_skip_blank_postfix_annotations =
            is_lambda_declaration && declaration_expression_id.is_some();
        if should_skip_blank_postfix_annotations {
            let annotation_count = f.context().annotation_ids(node_id).len();
            for annotation_index in 0..annotation_count {
                let annotation_id = f.context().annotation_ids(node_id)[annotation_index];
                let annotation = f.context().annotation(annotation_id);
                if !matches!(
                    annotation.position(),
                    AnnotationPosition::BlockPostfix | AnnotationPosition::LinePostfix
                ) {
                    continue;
                }

                annotation.format_node(annotation_id, f)?;
            }
        } else {
            write!(
                f,
                [
                    crate::format::annotation::postfix_annotations_without_line_postfix_boundary(
                        f.context(),
                        node_id
                    )
                ]
            )?;
        }

        Ok(())
    }
}

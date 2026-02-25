use crate::format::analysis::next_non_whitespace_after_annotation;
use crate::format::collection::list_like;
use crate::format::collection::property::{format_block_of_members, format_key_with_quotes};
use crate::format::declaration::signature::format_where_clause_with_break;
use crate::format::declaration::statement::format_block_of_statements;
use crate::format::directive::{
    FormatterDirective, FormatterDirectiveKind, FormatterDirectivePosition, directive_for_node,
};
use crate::format::expression::{
    ParenthesizedDropMode, format_expression, is_expression_breakable, should_drop_parenthesized,
};
use crate::format::operator::is_type_context;
use crate::{
    Annotation, DestackFormatContext, DestackFormatter, FormatNode,
    empty_block_with_infix_annotations,
};
use destack_ast::{
    AnnotationPosition, Comment, CommentStyle, Declaration, DeclarationDescriptor, DeclarationKind,
    DependencyKind, DependencyMode, Expression, FunctionKind, Generics, Heritage, IfKind,
    ImportAliasTarget, Key, Keyword, LocalNodeId, Member, Mutability, Name, NamespaceKind,
    NodeType, Parameter, TypeKind, Visibility,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::Span;

use crate::format::declaration::function::format_function_declaration;
use crate::format::declaration::r#type::{
    EnumDeclarationFormatData, format_enum_declaration, format_interface_declaration,
    format_struct_or_class_declaration,
};

const TEMPLATE_LITERAL_TYPE_EQUALS_BREAK_WIDTH: u16 = 80;

/// Format one declaration export modifier and export-head seam comments.
pub(crate) fn format_declaration_export_modifier<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &destack_ast::DeclarationDescriptor,
) -> FormatResult<()> {
    if let Some(export) = descriptor.export {
        write!(
            f,
            [
                export,
                space(),
                f.context().declaration_export_head_annotations(node_id)
            ]
        )?;
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

    write!(
        f,
        [group(&indent(&format_args![
            format_with(|f| {
                if start_on_new_line {
                    write!(f, [hard_line_break()])?;
                } else {
                    write!(f, [soft_line_break_or_space()])?;
                }
                Ok(())
            }),
            keyword,
            space(),
            format_with(|f| {
                let format_types = types.iter().copied().map(|type_id| {
                    format_with(move |f| format_super_type_expression(f, keyword, type_id))
                });
                if start_on_new_line && !force_expand {
                    f.join_with(&format_args![&token(","), space()])
                        .entries(format_types)
                        .finish()
                } else {
                    f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                        .entries(format_types)
                        .finish()
                }
            }),
        ]))
        .should_expand(force_expand)]
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
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
    } else {
        write!(f, [token("{"), hard_line_break()])?;
        write!(
            f,
            [group(&block_indent(&format_with(|f| {
                format_block_of_statements(f, expressions, false)
            })))]
        )?;
        write!(
            f,
            [f.context().block_infix_annotations(node_id), token("}")]
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
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
    } else {
        write!(f, [token("{"), hard_line_break()])?;
        write!(
            f,
            [group(&block_indent(&format_with(|f| {
                format_block_of_statements(f, expressions, false)
            })))]
        )?;
        write!(
            f,
            [
                hard_line_break(),
                f.context().block_infix_annotations(node_id),
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

/// Store inline block-prefix comment metadata for type grouping expressions.
#[derive(Debug, Clone)]
struct InlineTypePrefixCommentCluster {
    /// The target expression id whose leading comments are emitted inline.
    expression_id: LocalNodeId<Expression>,
    /// The annotation ids in source order.
    annotation_ids: Vec<LocalNodeId<Annotation>>,
    /// Whether each annotation starts after a source newline relative to the previous one.
    annotation_breaks_before: Vec<bool>,
}

impl InlineTypePrefixCommentCluster {
    /// Return whether the cluster contains source newline breaks between comments.
    fn has_multiline_breaks(&self) -> bool {
        self.annotation_breaks_before
            .iter()
            .copied()
            .skip(1)
            .any(|has_break| has_break)
    }
}

/// Return inline prefix comment cluster metadata for type grouping expressions.
fn single_line_type_grouping_prefix_comment_cluster(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<InlineTypePrefixCommentCluster> {
    if !is_type_context(context, expression_id) {
        return None;
    }

    let mut current_id = expression_id;
    loop {
        let Some(annotations) = context.annotations(current_id) else {
            let next_id = match context.tree.get(current_id) {
                Expression::Parenthesized { expression } | Expression::Statement(expression) => {
                    Some(*expression)
                }
                Expression::Binary { left, .. } | Expression::TypeBinary { left, .. } => {
                    Some(*left)
                }
                _ => None,
            }?;
            current_id = next_id;
            continue;
        };

        let mut cluster = Vec::new();
        let mut annotation_breaks_before = Vec::new();
        for annotation_id in annotations {
            let annotation = context.annotation(annotation_id);
            let Annotation::Comment {
                node: comment_id,
                position,
                ..
            } = annotation
            else {
                break;
            };
            if !matches!(
                position,
                AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
            ) {
                break;
            }

            let comment = context.tree.get::<Comment>(comment_id);
            if comment.style != CommentStyle::Star {
                break;
            }
            let comment_span = context.annotation_span(annotation_id);
            if context.has_newline(comment_span) {
                return None;
            }
            let has_newline_before =
                cluster
                    .last()
                    .copied()
                    .is_some_and(|previous_annotation_id| {
                        let previous_span = context.annotation_span(previous_annotation_id);
                        let current_span = context.annotation_span(annotation_id);
                        let between_span =
                            Span::new(previous_span.file, previous_span.end, current_span.start);
                        context.has_newline(between_span)
                    });

            cluster.push(annotation_id);
            annotation_breaks_before.push(has_newline_before);
        }

        if cluster.is_empty() {
            return None;
        }

        let last_annotation_id = *cluster.last()?;
        let next_character = next_non_whitespace_after_annotation(context, last_annotation_id);
        if !matches!(next_character, Some('|' | '&')) {
            return None;
        }
        return Some(InlineTypePrefixCommentCluster {
            expression_id: current_id,
            annotation_ids: cluster,
            annotation_breaks_before,
        });
    }
}

/// Return whether an expression or its transparent left spine has a prefix annotation.
fn expression_has_prefix_annotation_in_left_spine(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = expression_id;
    loop {
        if context.has_prefix_annotation(current_id) {
            return true;
        }

        let next_id = match context.tree.get(current_id) {
            Expression::Parenthesized { expression } | Expression::Statement(expression) => {
                Some(*expression)
            }
            Expression::Binary { left, .. } | Expression::TypeBinary { left, .. } => Some(*left),
            _ => None,
        };

        let Some(next_id) = next_id else {
            return false;
        };
        current_id = next_id;
    }
}

/// Format an expression while omitting its own prefix annotations.
fn format_expression_without_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let directive = directive_for_node(f.context(), expression_id);
    let expression = f.context().tree.get(expression_id);

    format_expression(f, expression_id, expression, directive)?;

    if !matches!(
        directive,
        Some(FormatterDirective {
            kind: FormatterDirectiveKind::IgnoreFormat,
            position: FormatterDirectivePosition::Postfix { .. },
        })
    ) {
        write!(
            f,
            [f.context().any_infix_or_postfix_annotations(expression_id)]
        )?;
    }

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
    let header = format_with(|f| {
        // export
        format_declaration_export_modifier(f, node_id, descriptor)?;

        // kind
        if descriptor.kind == DeclarationKind::Declaration {
            write!(f, [Keyword::Declare, space()])?;
        }

        // keyword
        if mutability == Some(Mutability::Immutable) {
            // for readonly type expression
            write!(f, [Keyword::Readonly])?;
        } else if kind == TypeKind::Structural {
            write!(f, [Keyword::Type])?;
        } else {
            write!(f, [Keyword::Newtype])?;
        }

        // name
        if let Some(name) = descriptor.name {
            write!(f, [space(), name])?;
        }

        // static parameters
        if let Some(static_parameters) = static_parameters {
            write!(f, [list_like("<", ">", ",", static_parameters)])?;
            write!(
                f,
                [f.context().declaration_generic_head_annotations(node_id)]
            )?;
        }

        Ok(())
    });

    let inline_prefix_comment_cluster =
        single_line_type_grouping_prefix_comment_cluster(f.context(), value_id);

    let tree = f.context().tree;

    // prefer keeping the value on a single line
    let format_inline = format_with(|f| {
        write!(f, [header, space(), token("=")])?;
        if let Some(cluster) = inline_prefix_comment_cluster.as_ref() {
            if cluster.has_multiline_breaks() {
                write!(
                    f,
                    [indent(&format_with(
                        |f: &mut DestackFormatter<'ast, '_>| {
                            write!(f, [hard_line_break()])?;
                            for (index, annotation_id) in
                                cluster.annotation_ids.iter().copied().enumerate()
                            {
                                if index > 0 {
                                    if cluster.annotation_breaks_before[index] {
                                        write!(f, [hard_line_break()])?;
                                    } else {
                                        write!(f, [space()])?;
                                    }
                                }

                                let annotation = f.context().annotation(annotation_id);
                                annotation.format_node(annotation_id, f)?;
                            }

                            write!(f, [space()])?;
                            format_expression_without_prefix_annotations(f, cluster.expression_id)?;
                            Ok(())
                        }
                    ))]
                )?;
            } else {
                write!(f, [space()])?;
                for (index, annotation_id) in cluster.annotation_ids.iter().copied().enumerate() {
                    if index > 0 {
                        write!(f, [space()])?;
                    }
                    let annotation = f.context().annotation(annotation_id);
                    annotation.format_node(annotation_id, f)?;
                }
                write!(f, [space()])?;
                format_expression_without_prefix_annotations(f, cluster.expression_id)?;
            }
        } else {
            write!(f, [space()])?;
            write!(f, [value_id])?;
        }
        Ok(())
    });

    let format_soft_break = format_with(|f| {
        write!(
            f,
            [group(&format_args![
                header,
                space(),
                token("="),
                indent(&format_args![soft_line_break_or_space(), value_id])
            ])]
        )
    });

    // expand inline if breakable (like let x = [\n ... ])
    let format_inline_expanded = format_with(|f| {
        write!(f, [header, space(), token("=")])?;
        write!(f, [space()])?;
        write!(f, [fits_expanded(&group(&value_id).should_expand(true))])
    });
    let value_expression = tree.get(value_id);
    let value_has_prefix_annotation =
        expression_has_prefix_annotation_in_left_spine(f.context(), value_id);
    let line_width = f.context().options.line_width;
    let should_break_template_literal_type_after_equals = match value_expression {
        Expression::TypeTemplateLiteral { spans, .. } => {
            line_width <= TEMPLATE_LITERAL_TYPE_EQUALS_BREAK_WIDTH
                && spans.iter().any(|span_id| {
                    matches!(
                        tree.get(*span_id),
                        Expression::TypeConditional { .. }
                            | Expression::If {
                                kind: IfKind::Ternary,
                                ..
                            }
                    )
                })
        }
        _ => false,
    };
    let should_break_after_equals = match value_expression {
        Expression::TypeConditional { left, .. } => {
            !matches!(tree.get(*left), Expression::Parenthesized { .. })
        }
        _ => false,
    };
    let value_prefers_inline_after_equals = match value_expression {
        Expression::Parenthesized { expression } => !should_drop_parenthesized(
            f.context(),
            value_id,
            *expression,
            ParenthesizedDropMode::ExpressionWrapper,
        ),
        Expression::Index { left, .. } | Expression::TypeIndex { left, .. } => {
            matches!(tree.get(*left), Expression::Parenthesized { .. })
        }
        _ => false,
    };
    if inline_prefix_comment_cluster.is_some() {
        format_inline.format(f)?;
    } else if should_break_after_equals || should_break_template_literal_type_after_equals {
        format_soft_break.format(f)?;
    } else if is_expression_breakable(tree, tree.get(value_id)) {
        if value_prefers_inline_after_equals {
            format_inline.format(f)?;
        } else {
            format_inline_expanded.format(f)?;
        }
    } else if value_has_prefix_annotation || value_prefers_inline_after_equals {
        format_inline.format(f)?;
    } else {
        format_soft_break.format(f)?;
    }

    // type alias declarations need trailing semicolon (like const/let)
    write!(f, [token(";")])?;

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

impl<'ast> FormatNode<'ast, Declaration> for Declaration {
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
        write!(f, [f.context().declaration_prefix_annotations(node_id)])?;

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
                let data = EnumDeclarationFormatData {
                    descriptor,
                    kind: *kind,
                    generics,
                    heritage,
                    fields,
                    members,
                };
                if format_enum_declaration(f, node_id, data)? {
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
                format_function_declaration(f, node_id, descriptor, signature, body)?;
            }
        }

        let should_skip_blank_postfix_annotations =
            is_lambda_declaration && declaration_expression_id.is_some();
        if should_skip_blank_postfix_annotations {
            if let Some(annotation_ids) = f.context().annotations(node_id) {
                for annotation_id in annotation_ids {
                    let annotation = f.context().annotation(annotation_id);
                    if matches!(annotation, Annotation::Blank { .. }) {
                        continue;
                    }
                    if !matches!(
                        annotation.position(),
                        AnnotationPosition::BlockPostfix
                            | AnnotationPosition::LinePostfix
                            | AnnotationPosition::LinePostfixBoundary
                    ) {
                        continue;
                    }
                    annotation.format_node(annotation_id, f)?;
                }
            }
        } else {
            write!(f, [f.context().any_postfix_annotations(node_id)])?;
        }

        Ok(())
    }
}

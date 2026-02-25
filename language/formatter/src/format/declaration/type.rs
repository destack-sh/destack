use crate::format::collection::list_like;
use crate::format::collection::property::format_block_of_members;
use crate::format::declaration::declaration::{
    format_declaration_export_modifier, format_super_type_clause,
    format_super_type_clause_with_expand,
};
use crate::format::declaration::signature::format_where_clause_with_break;
use crate::format::directive::{
    FormatterDirective, FormatterDirectiveKind, FormatterDirectivePosition, directive_for_node,
};
use crate::format::expression::{expression_has_static_type_arguments, format_expression};
use crate::{Annotation, DestackFormatter, FormatNode, empty_block_with_infix_annotations};
use destack_ast::{
    AnnotationPosition, Declaration, DeclarationAbstraction, DeclarationDescriptor,
    DeclarationKind, EnumField, EnumKind, Expression, FunctionKind, Generics, Heritage, Keyword,
    LocalNodeId, Member, NodeType, TypeKind, WhereClause,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};

/// Return whether one super-type list has a line-postfix-boundary comment.
fn super_type_clause_has_line_postfix_boundary_annotation(
    f: &DestackFormatter<'_, '_>,
    types: &[LocalNodeId<Expression>],
) -> bool {
    types.iter().copied().any(|expression_id| {
        let Some(annotation_ids) = f.context().annotations(expression_id) else {
            return false;
        };

        annotation_ids.iter().any(|annotation_id| {
            matches!(
                f.context().annotation(*annotation_id),
                Annotation::Comment {
                    position: AnnotationPosition::LinePostfixBoundary,
                    ..
                }
            )
        })
    })
}

/// Return whether one extends type is a parenthesized class declaration expression.
fn is_parenthesized_class_extends_type(
    f: &DestackFormatter<'_, '_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Parenthesized { expression } = f.context().tree.get(expression_id) else {
        return false;
    };

    let Expression::Declaration(declaration_id) = f.context().tree.get(*expression) else {
        return false;
    };

    matches!(
        f.context().tree.get(*declaration_id),
        Declaration::Class { .. }
    )
}

/// Return whether one class extends head requires explicit parenthesized grouping.
fn class_extends_expression_requires_parentheses(
    f: &DestackFormatter<'_, '_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    // unparenthesized lambdas are not valid in class heritage heads
    if matches!(
        f.context().tree.get(expression_id),
        Expression::Declaration(declaration_id)
            if matches!(
                f.context().tree.get(*declaration_id),
                Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
            )
    ) {
        return true;
    }

    // these forms require explicit parentheses in class heritage expressions
    matches!(
        f.context().tree.get(expression_id),
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

/// Format one class extends expression, adding wrappers for invalid unparenthesized heads.
fn format_class_extends_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if class_extends_expression_requires_parentheses(f, expression_id) {
        write!(f, [token("("), expression_id, token(")")])?;
    } else {
        write!(f, [expression_id])?;
    }

    Ok(())
}

/// Format an expanded implements clause with one type per line.
fn format_expanded_implements_clause<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    implements_types: &[LocalNodeId<Expression>],
    start_on_new_line: bool,
) -> FormatResult<()> {
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
            Keyword::Implements,
            indent(&format_args![
                hard_line_break(),
                format_with(|f| {
                    f.join_with(&format_args![&token(","), hard_line_break()])
                        .entries(implements_types)
                        .finish()
                })
            ])
        ]))]
    )
}

/// Return whether one expression has prefix closure-cast style annotations.
fn expression_has_prefix_closure_cast_annotation(
    f: &DestackFormatter<'_, '_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    f.context()
        .visit_annotations(expression_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    f.context().annotation(*annotation_id),
                    Annotation::Doc {
                        position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                        ..
                    } | Annotation::Comment {
                        position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                        ..
                    }
                )
            })
        })
        .unwrap_or(false)
}

/// Return one extends wrapper annotation/render target pair for closure-cast style wrappers.
fn extends_expression_closure_cast_wrapper_target(
    f: &DestackFormatter<'_, '_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<(LocalNodeId<Expression>, LocalNodeId<Expression>)> {
    if expression_has_prefix_closure_cast_annotation(f, expression_id) {
        return Some((expression_id, expression_id));
    }

    match f.context().tree.get(expression_id) {
        Expression::Call { left, .. } | Expression::New { left, .. } => {
            if expression_has_prefix_closure_cast_annotation(f, *left) {
                return Some((*left, expression_id));
            }
        }
        Expression::Parenthesized { expression } => {
            if expression_has_prefix_closure_cast_annotation(f, *expression) {
                return Some((*expression, *expression));
            }

            if let Expression::Call { left, .. } | Expression::New { left, .. } =
                f.context().tree.get(*expression)
                && expression_has_prefix_closure_cast_annotation(f, *left)
            {
                return Some((*left, *expression));
            }
        }
        _ => {}
    }

    None
}

/// Format one extends expression as `/** doc */ (expr)` while preserving postfix material.
fn format_extends_expression_with_closure_cast_wrapper<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    annotation_expression_id: LocalNodeId<Expression>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let directive = directive_for_node(f.context(), expression_id);
    let expression = f.context().tree.get(expression_id);

    write!(
        f,
        [f.context().any_prefix_annotations(annotation_expression_id)]
    )?;
    write!(f, [token("(")])?;
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
            [f.context()
                .any_infix_or_postfix_annotations(annotation_expression_id)]
        )?;
    }
    write!(f, [token(")")])
}

/// Format shared export and declaration modifiers for declarations.
fn format_declaration_header_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
) -> FormatResult<()> {
    format_declaration_export_modifier(f, node_id, descriptor)?;

    if descriptor.kind == DeclarationKind::Declaration {
        write!(f, [Keyword::Declare, space()])?;
    }

    Ok(())
}

/// Format declaration static parameters when present.
fn format_declaration_static_parameters<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    generics: &Generics,
) -> FormatResult<()> {
    if let Some(static_parameters) = generics.static_parameters.as_ref()
        && !static_parameters.is_empty()
    {
        write!(f, [list_like("<", ">", ",", static_parameters)])?;
        write!(
            f,
            [f.context().declaration_generic_head_annotations(node_id)]
        )?;
    }

    Ok(())
}

/// Format declaration where clauses when present.
fn format_declaration_where_clauses<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    where_clauses: Option<&[LocalNodeId<WhereClause>]>,
) -> FormatResult<()> {
    if let Some(where_clauses) = where_clauses
        && !where_clauses.is_empty()
    {
        format_where_clause_with_break(f, where_clauses)?;
    }

    Ok(())
}

/// Format declaration extends and optional implements clauses.
fn format_declaration_heritage<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    heritage: &Heritage,
    include_implements: bool,
    start_on_new_line: bool,
) -> FormatResult<()> {
    let mut extends_has_boundary_comments = false;
    if let Some(extends_types) = heritage.extends_types.as_ref()
        && !extends_types.is_empty()
    {
        // keep closure-cast extends heads explicit: `extends /** doc */ (expr)`
        let closure_cast_wrapper_target = if include_implements && extends_types.len() == 1 {
            extends_expression_closure_cast_wrapper_target(f, extends_types[0])
        } else {
            None
        };
        if let Some((annotation_expression_id, expression_id)) = closure_cast_wrapper_target {
            write!(f, [space(), Keyword::Extends, space()])?;
            format_extends_expression_with_closure_cast_wrapper(
                f,
                annotation_expression_id,
                expression_id,
            )?;
            return Ok(());
        }

        extends_has_boundary_comments =
            super_type_clause_has_line_postfix_boundary_annotation(f, extends_types);
        let extends_should_stay_inline_with_head = !extends_has_boundary_comments
            && !start_on_new_line
            && extends_types.len() == 1
            && is_parenthesized_class_extends_type(f, extends_types[0]);
        if extends_should_stay_inline_with_head {
            write!(f, [space(), Keyword::Extends, space(), extends_types[0]])?;
        } else {
            format_super_type_clause_with_expand(
                f,
                Keyword::Extends,
                extends_types,
                extends_has_boundary_comments,
                start_on_new_line || extends_has_boundary_comments,
            )?;
        }
    }

    if include_implements
        && let Some(implements_types) = heritage.implements_types.as_ref()
        && !implements_types.is_empty()
    {
        let implements_has_boundary_comments =
            super_type_clause_has_line_postfix_boundary_annotation(f, implements_types);
        let implements_start_on_new_line =
            start_on_new_line || extends_has_boundary_comments || implements_has_boundary_comments;
        if implements_has_boundary_comments {
            format_expanded_implements_clause(f, implements_types, implements_start_on_new_line)?;
        } else {
            format_super_type_clause_with_expand(
                f,
                Keyword::Implements,
                implements_types,
                false,
                implements_start_on_new_line,
            )?;
        }
    }

    Ok(())
}

/// Format anonymous class heritage, preserving oxfmt style for generic extends and implements.
fn format_anonymous_class_heritage<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    heritage: &Heritage,
) -> FormatResult<()> {
    if let Some(extends_types) = heritage.extends_types.as_ref()
        && !extends_types.is_empty()
    {
        let has_generic_extends = extends_types
            .iter()
            .copied()
            .any(|type_id| expression_has_static_type_arguments(f.context(), type_id));
        if has_generic_extends {
            write!(f, [space(), Keyword::Extends, space(), token("(")])?;
            write!(
                f,
                [group(&indent(&format_args![
                    soft_line_break_or_space(),
                    format_with(|f| {
                        f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                            .entries(extends_types.iter().copied().map(|type_id| {
                                format_with(move |f| format_class_extends_expression(f, type_id))
                            }))
                            .finish()
                    })
                ]))]
            )?;
            write!(f, [soft_line_break_or_space(), token(")")])?;
        } else {
            write!(f, [space(), Keyword::Extends, space()])?;
            write!(
                f,
                [format_with(|f| {
                    f.join_with(&format_args![&token(","), space()])
                        .entries(extends_types.iter().copied().map(|type_id| {
                            format_with(move |f| format_class_extends_expression(f, type_id))
                        }))
                        .finish()
                })]
            )?;
        }
    }

    if let Some(implements_types) = heritage.implements_types.as_ref()
        && !implements_types.is_empty()
    {
        let has_generic_implements = implements_types
            .iter()
            .copied()
            .any(|type_id| expression_has_static_type_arguments(f.context(), type_id));
        if has_generic_implements {
            write!(f, [space(), Keyword::Implements, space()])?;
            write!(
                f,
                [format_with(|f| {
                    f.join_with(&format_args![&token(","), space()])
                        .entries(implements_types)
                        .finish()
                })]
            )?;
        } else {
            format_super_type_clause(f, Keyword::Implements, implements_types)?;
        }
    }

    Ok(())
}

/// Format a struct or class declaration and return whether it ended early.
#[allow(clippy::too_many_arguments)]
pub(crate) fn format_struct_or_class_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration_expression_id: Option<LocalNodeId<Expression>>,
    descriptor: &DeclarationDescriptor,
    generics: &Generics,
    heritage: &Heritage,
    members: &[LocalNodeId<Member>],
    is_class: bool,
) -> FormatResult<bool> {
    format_declaration_header_prefix(f, node_id, descriptor)?;

    if descriptor.abstraction == DeclarationAbstraction::Abstract {
        write!(f, [Keyword::Abstract, space()])?;
    }

    if is_class {
        write!(f, [Keyword::Class])?;
    } else {
        write!(f, [Keyword::Struct])?;
    }

    if let Some(name) = descriptor.name {
        write!(f, [space(), name])?;
    }

    let is_assignment_rhs_anonymous_class = is_class
        && descriptor.name.is_none()
        && declaration_expression_id.is_some_and(|expression_id| {
            let Some((parent_id, parent_type)) = f.context().parent(expression_id) else {
                return false;
            };
            if parent_type != NodeType::Expression {
                return false;
            }

            let parent_expression = LocalNodeId::<Expression>::new(parent_id);
            matches!(
                f.context().tree.get(parent_expression),
                Expression::Assign { right, .. } if *right == expression_id
            )
        });
    let has_generic_head_comment = f.context().has_declaration_generic_head_annotation(node_id);
    let has_body_head_comment = f.context().has_declaration_body_head_annotation(node_id);

    format_declaration_static_parameters(f, node_id, generics)?;

    if is_assignment_rhs_anonymous_class {
        format_anonymous_class_heritage(f, heritage)?;
    } else {
        format_declaration_heritage(f, heritage, true, has_generic_head_comment)?;
    }
    format_declaration_where_clauses(f, generics.where_clauses.as_deref())?;
    let implements_has_line_boundary_comment = heritage
        .implements_types
        .as_ref()
        .is_some_and(|types| super_type_clause_has_line_postfix_boundary_annotation(f, types));
    let has_heritage_line_boundary_annotation =
        has_generic_head_comment || implements_has_line_boundary_comment;

    if has_heritage_line_boundary_annotation {
        write!(f, [hard_line_break()])?;
    } else if !has_body_head_comment {
        write!(f, [space()])?;
    } else {
        // body-head annotations emit their own boundary separator.
    }
    write!(f, [f.context().declaration_body_head_annotations(node_id)])?;

    if members.is_empty() {
        write!(f, [empty_block_with_infix_annotations(node_id)])?;
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
        return Ok(true);
    }

    write!(f, [token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&format_args![block_indent(&format_with(|f| {
            format_block_of_members(f, members)
        })),])]
    )?;
    write!(f, [f.context().block_infix_annotations(node_id)])?;
    write!(f, [hard_line_break(), token("}")])?;
    Ok(false)
}

/// Format one enum field entry.
impl<'ast> FormatNode<'ast, EnumField> for EnumField {
    /// Emit the field name, optional value, trailing comma, and attached annotations.
    fn format_node(
        &self,
        node_id: LocalNodeId<EnumField>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // name
        write!(f, [self.name])?;

        // value
        if let Some(value) = self.value {
            write!(f, [space(), token("="), space(), value])?;
        }

        // comma after field
        // NOTE #Cleanup: having commas inside EnumField formatting feels wrong
        write!(f, [token(",")])?;

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
        Ok(())
    }
}

/// Shared enum declaration inputs.
pub(crate) struct EnumDeclarationFormatData<'a> {
    /// The shared declaration descriptor.
    pub descriptor: &'a DeclarationDescriptor,
    /// The enum kind.
    pub kind: EnumKind,
    /// The enum generics.
    pub generics: &'a Generics,
    /// The enum heritage clauses.
    pub heritage: &'a Heritage,
    /// The enum fields.
    pub fields: &'a [LocalNodeId<EnumField>],
    /// The enum members.
    pub members: &'a [LocalNodeId<Member>],
}

/// Format an enum declaration and return whether it ended early.
pub(crate) fn format_enum_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    data: EnumDeclarationFormatData<'_>,
) -> FormatResult<bool> {
    let EnumDeclarationFormatData {
        descriptor,
        kind,
        generics,
        heritage,
        fields,
        members,
    } = data;

    format_declaration_header_prefix(f, node_id, descriptor)?;

    if kind == EnumKind::Const {
        write!(f, [Keyword::Const, space()])?;
    }

    write!(f, [Keyword::Enum])?;

    if let Some(name) = descriptor.name {
        write!(f, [space(), name])?;
    }

    format_declaration_static_parameters(f, node_id, generics)?;
    format_declaration_heritage(f, heritage, true, false)?;
    format_declaration_where_clauses(f, generics.where_clauses.as_deref())?;

    write!(f, [space()])?;

    if fields.is_empty() && members.is_empty() {
        write!(f, [empty_block_with_infix_annotations(node_id)])?;
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
        return Ok(true);
    }

    write!(f, [token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&format_args![block_indent(&format_with(|f| {
            f.join_with(&format_args![&hard_line_break()])
                .entries(fields)
                .finish()
        })),])]
    )?;

    if !fields.is_empty() && !members.is_empty() {
        write!(f, [hard_line_break()])?;
        if !f.context().has_blank_prefix_annotation(members[0]) {
            write!(f, [empty_line()])?;
        }
    }

    write!(
        f,
        [group(&format_args![block_indent(&format_with(|f| {
            format_block_of_members(f, members)
        })),])]
    )?;
    write!(f, [f.context().block_infix_annotations(node_id)])?;
    write!(f, [hard_line_break(), token("}")])?;
    Ok(false)
}

/// Format an interface declaration and return whether it ended early.
pub(crate) fn format_interface_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    kind: TypeKind,
    generics: &Generics,
    heritage: &Heritage,
    members: &[LocalNodeId<Member>],
) -> FormatResult<bool> {
    format_declaration_header_prefix(f, node_id, descriptor)?;

    if kind == TypeKind::Nominal {
        write!(f, [Keyword::Newtype, space()])?;
    }
    write!(f, [Keyword::Interface])?;

    if let Some(name) = descriptor.name {
        write!(f, [space(), name])?;
    }

    format_declaration_static_parameters(f, node_id, generics)?;
    format_declaration_heritage(f, heritage, false, false)?;
    format_declaration_where_clauses(f, generics.where_clauses.as_deref())?;

    write!(f, [space()])?;

    if members.is_empty() {
        write!(f, [empty_block_with_infix_annotations(node_id)])?;
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
        return Ok(true);
    }

    write!(f, [token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&format_args![block_indent(&format_with(|f| {
            format_block_of_members(f, members)
        })),])]
    )?;
    write!(f, [f.context().block_infix_annotations(node_id)])?;
    write!(f, [hard_line_break(), token("}")])?;
    Ok(false)
}

#[cfg(test)]
mod tests {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};
    use destack_ast::{DeclarationDescriptor, EnumKind};

    #[test]
    fn test_format_enum_empty() {
        assert_format!(
            "enum { }",
            "enum {}",
            |p| p.eat_enum(&p.mark(), EnumKind::Enum, DeclarationDescriptor::default()),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_enum_with_simple_fields() {
        assert_format!(
            "enum { A, B }",
            "enum {\n\tA,\n\tB,\n}",
            |p| p.eat_enum(&p.mark(), EnumKind::Enum, DeclarationDescriptor::default()),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_enum_with_annotations() {
        assert_format!(
            "enum { A }",
            "enum {\n\tA,\n}",
            |p| p.eat_enum(&p.mark(), EnumKind::Enum, DeclarationDescriptor::default()),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_enum_with_static_parameters() {
        let source = r"enum Machine<T: int32 = 3, IsSomething: boolean = true> {
    A = 1,
    B = T,
    @if(IsSomething)
    C = 3,
}";
        assert_format!(
            source,
            source,
            |p| p.eat_enum(&p.mark(), EnumKind::Enum, DeclarationDescriptor::default()),
            DestackFormatOptions::default()
        );
    }
}

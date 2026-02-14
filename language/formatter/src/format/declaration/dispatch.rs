use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    Annotation, AnnotationPosition, Argument, Declaration, DependencyMode, Expression,
    FunctionKind, Keyword, LocalNodeId, NodeType, Visibility,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};

use super::function_like::format_function_declaration;
use super::module_like::{
    format_extension_declaration, format_global_declaration, format_import_alias_declaration,
    format_namespace_declaration,
};
use super::type_alias::format_type_alias_declaration;
use super::type_like::{
    EnumDeclarationFormatData, format_enum_declaration, format_interface_declaration,
    format_struct_or_class_declaration,
};

/// Format a super type clause.
pub(super) fn format_super_type_clause<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    keyword: Keyword,
    types: &[LocalNodeId<Expression>],
) -> FormatResult<()> {
    assert!(!types.is_empty());

    write!(
        f,
        [group(&indent(&format_args![
            soft_line_break_or_space(),
            keyword,
            space(),
            format_with(|f| {
                f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                    .entries(types)
                    .finish()
            }),
        ]))]
    )
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
            if let Some((parent_id, parent_type)) = f.context().get_parent(node_id) {
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
        let deferred_lambda_expression = if is_lambda_declaration {
            declaration_expression_id
        } else {
            None
        };
        let deferred_lambda_argument = if let Some(expression_id) = deferred_lambda_expression {
            if let Some((parent_id, parent_type)) = f.context().get_parent(expression_id) {
                if parent_type == NodeType::Argument {
                    Some(LocalNodeId::<Argument>::new(parent_id))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        let lambda_is_nested_in_lambda_body = if is_lambda_declaration {
            if let Some(expression_id) = deferred_lambda_expression {
                if let Some((container_id, container_type)) = f.context().get_parent(expression_id)
                {
                    if container_type == NodeType::Declaration {
                        let container_declaration_id =
                            LocalNodeId::<Declaration>::new(container_id);
                        matches!(
                            f.context().tree.get(container_declaration_id),
                            Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
                        )
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };
        let lambda_has_argument_ancestor = is_lambda_declaration
            && f.context()
                .any_ancestor(node_id, |_, node_type| node_type == NodeType::Argument);
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

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
                format_import_alias_declaration(f, descriptor, *kind, target)?;
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
                format_function_declaration(
                    f,
                    node_id,
                    descriptor,
                    signature,
                    body,
                    is_lambda_declaration,
                    deferred_lambda_expression,
                    deferred_lambda_argument,
                    lambda_is_nested_in_lambda_body,
                    lambda_has_argument_ancestor,
                )?;
            }
        }

        let should_skip_blank_postfix_annotations =
            is_lambda_declaration && declaration_expression_id.is_some();
        if should_skip_blank_postfix_annotations {
            if let Some(annotation_ids) = f.context().get_annotations(node_id) {
                for annotation_id in annotation_ids {
                    let annotation = f.context().tree.get::<Annotation>(annotation_id);
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

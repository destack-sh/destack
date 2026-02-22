use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Declaration, DependencyMode, Expression, FunctionKind, Keyword,
    LocalNodeId, NodeType, Visibility,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};

use crate::format::declaration::function::format_function_declaration;
use crate::format::declaration::module::{
    format_extension_declaration, format_global_declaration, format_import_alias_declaration,
    format_namespace_declaration,
};
use crate::format::declaration::r#type::{
    EnumDeclarationFormatData, format_enum_declaration, format_interface_declaration,
    format_struct_or_class_declaration,
};
use crate::format::declaration::type_alias::format_type_alias_declaration;

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
                if start_on_new_line && !force_expand {
                    f.join_with(&format_args![&token(","), space()])
                        .entries(types)
                        .finish()
                } else {
                    f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                        .entries(types)
                        .finish()
                }
            }),
        ]))
        .should_expand(force_expand)]
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

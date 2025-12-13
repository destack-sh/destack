use crate::argument::list_like;
use crate::block::format_block_of_statements;
use crate::expression::is_expression_breakable;
use crate::property::format_block_of_members;
use crate::r#where::format_where_clause;
use crate::{
    DestackFormatContext, DestackFormatter, FormatNode, empty_block_with_infix_annotations,
};
use destack_ast::{
    Asynchrony, Declaration, DeclarationKind, DependencyMode, EnumKind, Expression,
    FunctionAbstraction, FunctionCardinality, FunctionKind, FunctionMode, Keyword, LocalNodeId,
    Mutability, TypeKind, Visibility,
};
use destack_fir::format::{BestFittingMode, FormatResult};
use destack_fir::prelude::*;
use destack_fir::{best_fitting, format_args, write};

/// Format a super type clause.
pub(crate) fn format_super_type_clause<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    keyword: Keyword,
    types: &[LocalNodeId<Expression>],
) -> FormatResult<()> {
    assert!(!types.is_empty());

    write!(
        f,
        [
            space(),
            keyword,
            space(),
            group(&format_args![
                if_group_breaks(&token("(")),
                soft_block_indent(&format_with(|f| {
                    f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                        .entries(types)
                        .finish()?;
                    write!(f, [if_group_breaks(&token(","))])?;
                    Ok(())
                })),
                if_group_breaks(&token(")")),
            ]),
        ]
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
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            // module
            Declaration::Namespace {
                descriptor,
                generics,
                expressions,
            } => {
                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                write!(f, [Keyword::Namespace])?;

                // name / key
                if let Some(name) = descriptor.name {
                    write!(f, [space(), name])?;
                }

                // where
                if let Some(where_clauses) = generics.where_clauses.as_ref()
                    && !where_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
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
                            format_block_of_statements(f, node_id.into_any(), expressions)
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
            }

            // type alias
            Declaration::Type {
                descriptor,
                kind,
                mutability,
                static_parameters,
                value: value_id,
            } => {
                let header = format_with(|f| {
                    // export
                    if let Some(export) = descriptor.export {
                        write!(f, [export, space()])?;
                    }
                    // keyword
                    if *mutability == Some(Mutability::Immutable) {
                        // for readonly type expression
                        write!(f, [Keyword::Readonly])?;
                    } else if *kind == TypeKind::Structural {
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
                    }
                    Ok(())
                });

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

                let tree = f.context().tree;
                if is_expression_breakable(tree, tree.get(*value_id)) {
                    best_fitting![format_inline, format_inline_expanded, format_indented]
                        .with_mode(BestFittingMode::AllLines)
                        .format(f)?;
                } else {
                    best_fitting![format_inline, format_indented]
                        .with_mode(BestFittingMode::AllLines)
                        .format(f)?;
                }
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

                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                if is_class {
                    write!(f, [Keyword::Class])?;
                } else {
                    write!(f, [Keyword::Struct])?;
                }

                // name / key
                if let Some(name) = descriptor.name {
                    write!(f, [space(), name])?;
                }

                // static parameters
                if let Some(static_parameters) = generics.static_parameters.as_ref()
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // extends types
                if let Some(extends_types) = heritage.extends_types.as_ref()
                    && !extends_types.is_empty()
                {
                    format_super_type_clause(f, Keyword::Extends, extends_types)?;
                }

                // implements types
                if let Some(implements_types) = heritage.implements_types.as_ref()
                    && !implements_types.is_empty()
                {
                    format_super_type_clause(f, Keyword::Implements, implements_types)?;
                }

                // where clauses
                if let Some(where_clauses) = generics.where_clauses.as_ref()
                    && !where_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
                }

                write!(f, [space()])?;

                // empty body
                if members.is_empty() {
                    write!(f, [empty_block_with_infix_annotations(node_id)])?;
                    write!(f, [f.context().any_postfix_annotations(node_id)])?;
                    return Ok(());
                }

                // body
                write!(f, [token("{"), hard_line_break()])?;

                // members
                if !members.is_empty() {
                    write!(
                        f,
                        [group(&format_args![block_indent(&format_with(|f| {
                            format_block_of_members(f, members)
                        })),])]
                    )?;
                }

                write!(f, [f.context().block_infix_annotations(node_id)])?;

                write!(f, [hard_line_break(), token("}")])?;
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
                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // const enum
                if *kind == EnumKind::Const {
                    write!(f, [Keyword::Const, space()])?;
                }

                // header
                write!(f, [Keyword::Enum])?;

                // name / key
                if let Some(name) = descriptor.name {
                    write!(f, [space(), name])?;
                }

                // static parameters
                if let Some(static_parameters) = generics.static_parameters.as_ref()
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // extends types
                if let Some(extends_types) = heritage.extends_types.as_ref()
                    && !extends_types.is_empty()
                {
                    format_super_type_clause(f, Keyword::Extends, extends_types)?;
                }

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
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
                }

                write!(f, [space()])?;

                // empty block
                if fields.is_empty() && members.is_empty() {
                    write!(f, [empty_block_with_infix_annotations(node_id)])?;
                    write!(f, [f.context().any_postfix_annotations(node_id)])?;
                    return Ok(());
                }

                // body
                write!(f, [token("{"), hard_line_break()])?;

                // fields
                write!(
                    f,
                    [group(&format_args![block_indent(&format_with(|f| f
                        .join_with(&format_args![&hard_line_break()])
                        .entries(fields)
                        .finish())),])]
                )?;

                // blank line
                if !fields.is_empty() && !members.is_empty() {
                    write!(f, [hard_line_break()])?;
                    if !f.context().has_blank_prefix_annotation(members[0]) {
                        write!(f, [empty_line()])?;
                    }
                }

                // members
                write!(
                    f,
                    [group(&format_args![block_indent(&format_with(|f| {
                        format_block_of_members(f, members)
                    })),])]
                )?;
                write!(f, [f.context().block_infix_annotations(node_id)])?;
                write!(f, [hard_line_break(), token("}")])?;
            }

            // interface
            Declaration::Interface {
                descriptor,
                generics,
                heritage,
                members,
            } => {
                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                write!(f, [Keyword::Interface])?;

                // name / key
                if let Some(name) = descriptor.name {
                    write!(f, [space(), name])?;
                }

                // static parameters
                if let Some(static_parameters) = generics.static_parameters.as_ref()
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // extends types
                if let Some(extends_types) = heritage.extends_types.as_ref()
                    && !extends_types.is_empty()
                {
                    format_super_type_clause(f, Keyword::Extends, extends_types)?;
                }

                // where clauses
                if let Some(where_clauses) = generics.where_clauses.as_ref()
                    && !where_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
                }

                // space before body
                write!(f, [space()])?;

                // empty body
                if members.is_empty() {
                    write!(f, [empty_block_with_infix_annotations(node_id)])?;
                    write!(f, [f.context().any_postfix_annotations(node_id)])?;
                    return Ok(());
                }

                // body
                write!(f, [token("{"), hard_line_break()])?;

                // members
                if !members.is_empty() {
                    write!(
                        f,
                        [group(&format_args![block_indent(&format_with(|f| {
                            format_block_of_members(f, members)
                        })),])]
                    )?;
                }

                write!(f, [f.context().block_infix_annotations(node_id)])?;

                write!(f, [hard_line_break(), token("}")])?;
            }

            // extension
            Declaration::Extension {
                descriptor,
                generics,
                target_type,
                heritage,
                members,
            } => {
                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                write!(f, [Keyword::Extension])?;

                // static arguments
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

                // name / key
                if let Some(name) = descriptor.name {
                    write!(f, [space(), name, token(":")])?;
                }

                // target type
                write!(f, [space(), target_type])?;

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
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
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
            }

            // function
            Declaration::Function {
                descriptor,
                signature,
                body,
            } => {
                let generics = signature.generics.as_ref();

                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // abstraction
                match signature.abstraction {
                    FunctionAbstraction::Abstract => {
                        write!(f, [Keyword::Abstract, space()])?;
                    }
                    FunctionAbstraction::AbstractOverride => {
                        write!(f, [Keyword::Abstract, space(), Keyword::Override, space()])?;
                    }
                    FunctionAbstraction::ConcreteOverride => {
                        write!(f, [Keyword::Override, space()])?;
                    }
                    FunctionAbstraction::Concrete => {}
                }

                // asynchrony
                if signature.asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Async, space()])?;
                }

                // kind
                if let Some(kind) = signature.mode {
                    write!(f, [kind.to_keyword()])?;
                    if descriptor.name.is_some() || kind == FunctionMode::New {
                        write!(f, [space()])?;
                    }
                }

                // keyword
                if signature.kind == FunctionKind::Function
                    && signature.mode != Some(FunctionMode::Constructor)
                    && signature.mode != Some(FunctionMode::New)
                {
                    // function keyword
                    if signature.cardinality == FunctionCardinality::Generator {
                        write!(f, [Keyword::Function, token("*"), space()])?;
                    } else {
                        write!(f, [Keyword::Function, space()])?;
                    }
                } else {
                    // lambda (no keyword, maybe star)
                    if signature.cardinality == FunctionCardinality::Generator {
                        write!(f, [token("*"), space()])?;
                    }
                }

                // name / key
                if signature.kind == FunctionKind::Function
                    && let Some(name) = descriptor.name
                {
                    write!(f, [name])?;
                }

                // static parameters
                if let Some(static_parameters) =
                    generics.and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // self parameter and dynamic parameters
                write!(
                    f,
                    [group(&format_args![
                        token("("),
                        soft_block_indent(&format_with(|f| {
                            // dynamic parameters
                            f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                                .entries(&signature.dynamic_parameters)
                                .finish()?;

                            // trailing comma
                            write!(f, [if_group_breaks(&token(","))])?;

                            Ok(())
                        })),
                        token(")")
                    ])]
                )?;

                // return type
                if let Some(return_type) = signature.return_type {
                    if signature.kind == FunctionKind::Lambda && body.is_none() {
                        write!(f, [space(), token("=>"), space(), return_type])?;
                    } else {
                        write!(f, [token(":"), space(), return_type])?;
                    }
                }

                // where clause
                if let Some(where_clauses) =
                    generics.and_then(|generics| generics.where_clauses.as_ref())
                    && !where_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
                }

                // body
                if let Some(body) = body {
                    if signature.kind == FunctionKind::Lambda {
                        // arrow is fine since lambdas can only have return type or body
                        write!(f, [space(), token("=>"), space(), body])?;
                    } else {
                        write!(f, [space(), body])?;
                    }
                }
            }
        }

        write!(f, [f.context().any_postfix_annotations(node_id)])?;

        Ok(())
    }
}

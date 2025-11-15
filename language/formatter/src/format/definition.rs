use crate::argument::list_like;
use crate::block::format_block_of_statements;
use crate::property::format_block_of_properties;
use crate::r#where::format_where_clause;
use crate::with::format_with_clause;
use crate::{DystFormatContext, DystFormatter, FormatNode, empty_block_with_infix_annotations};
use dyst_ast::{
    Asynchrony, DeclarationKind, Definition, ExportType, Expression, FunctionAbstraction,
    FunctionCardinality, FunctionKind, FunctionMode, Keyword, NodeId, StructKind, Visibility,
};
use dyst_fir::format::FormatResult;
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

/// Format a super type clause.
pub(crate) fn format_super_type_clause<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    keyword: Keyword,
    types: &[NodeId<Expression>],
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

impl<'ast> Format<DystFormatContext<'ast>> for Visibility {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Visibility::Public => write!(f, [Keyword::Public]),
            Visibility::Protected => write!(f, [Keyword::Protected]),
            Visibility::Private => write!(f, [Keyword::Private]),
        }
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for ExportType {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            ExportType::Item => write!(f, [Keyword::Export]),
            ExportType::Default => write!(f, [Keyword::Export, space(), Keyword::Default]),
            ExportType::Module => write!(f, [Keyword::Export]),
        }
    }
}

impl<'ast> FormatNode<'ast, Definition> for Definition {
    fn format_node(
        &self,
        node_id: NodeId<Definition>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            // module
            Definition::Namespace {
                descriptor,
                generics,
                expressions,
            } => {
                let generics = generics.as_ref();

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

                // with
                if let Some(with_clauses) =
                    generics.and_then(|generics| generics.with_clauses.as_ref())
                    && !with_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_with_clause(f, with_clauses)?;
                }

                // where
                if let Some(where_clauses) =
                    generics.and_then(|generics| generics.where_clauses.as_ref())
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

            // struct
            Definition::Struct {
                descriptor,
                kind,
                generics,
                heritage,
                properties,
            } => {
                let generics = generics.as_ref();
                let heritage = heritage.as_ref();

                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                match kind {
                    StructKind::Struct => write!(f, [Keyword::Struct])?,
                    StructKind::Class => write!(f, [Keyword::Class])?,
                }

                // name / key
                if let Some(name) = descriptor.name {
                    write!(f, [space(), name])?;
                }

                // static parameters
                if let Some(static_parameters) =
                    generics.and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // extends types
                if let Some(extends_types) =
                    heritage.and_then(|heritage| heritage.extends_types.as_ref())
                    && !extends_types.is_empty()
                {
                    format_super_type_clause(f, Keyword::Extends, extends_types)?;
                }

                // implements types
                if let Some(implements_types) =
                    heritage.and_then(|heritage| heritage.implements_types.as_ref())
                    && !implements_types.is_empty()
                {
                    format_super_type_clause(f, Keyword::Implements, implements_types)?;
                }

                // with clauses
                if let Some(with_clauses) =
                    generics.and_then(|generics| generics.with_clauses.as_ref())
                    && !with_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_with_clause(f, with_clauses)?;
                }

                // where clauses
                if let Some(where_clauses) =
                    generics.and_then(|generics| generics.where_clauses.as_ref())
                    && !where_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
                }

                write!(f, [space()])?;

                // empty body
                if properties.is_empty() {
                    write!(f, [empty_block_with_infix_annotations(node_id)])?;
                    write!(f, [f.context().any_postfix_annotations(node_id)])?;
                    return Ok(());
                }

                // body
                write!(f, [token("{"), hard_line_break()])?;

                // properties
                if !properties.is_empty() {
                    write!(
                        f,
                        [group(&format_args![block_indent(&format_with(|f| {
                            format_block_of_properties(f, properties)
                        })),])]
                    )?;
                }

                write!(f, [f.context().block_infix_annotations(node_id)])?;

                write!(f, [hard_line_break(), token("}")])?;
            }

            // enum
            Definition::Enum {
                descriptor,
                generics,
                heritage,
                fields,
                properties,
            } => {
                let generics = generics.as_ref();
                let heritage = heritage.as_ref();

                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // header
                write!(f, [Keyword::Enum])?;

                // name / key
                if let Some(name) = descriptor.name {
                    write!(f, [space(), name])?;
                }

                // static parameters
                if let Some(static_parameters) =
                    generics.and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // extends types
                if let Some(extends_types) =
                    heritage.and_then(|heritage| heritage.extends_types.as_ref())
                    && !extends_types.is_empty()
                {
                    format_super_type_clause(f, Keyword::Extends, extends_types)?;
                }

                // implements types
                if let Some(implements_types) =
                    heritage.and_then(|heritage| heritage.implements_types.as_ref())
                    && !implements_types.is_empty()
                {
                    format_super_type_clause(f, Keyword::Implements, implements_types)?;
                }

                // with
                if let Some(with) = generics.and_then(|generics| generics.with_clauses.as_ref())
                    && !with.is_empty()
                {
                    write!(f, [space()])?;
                    format_with_clause(f, with)?;
                }

                // where
                if let Some(where_clauses) =
                    generics.and_then(|generics| generics.where_clauses.as_ref())
                    && !where_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
                }

                write!(f, [space()])?;

                // empty block
                if fields.is_empty() && properties.is_empty() {
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
                        .join_with(hard_line_break())
                        .entries(fields)
                        .finish())),])]
                )?;

                // blank line
                if !fields.is_empty() && !properties.is_empty() {
                    write!(f, [hard_line_break()])?;
                    if !f.context().has_blank_prefix_annotation(properties[0]) {
                        write!(f, [empty_line()])?;
                    }
                }

                // properties
                write!(
                    f,
                    [group(&format_args![block_indent(&format_with(|f| {
                        format_block_of_properties(f, properties)
                    })),])]
                )?;
                write!(f, [f.context().block_infix_annotations(node_id)])?;
                write!(f, [hard_line_break(), token("}")])?;
            }

            // interface
            Definition::Interface {
                descriptor,
                generics,
                heritage,
                properties,
            } => {
                let generics = generics.as_ref();
                let heritage = heritage.as_ref();

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
                if let Some(static_parameters) =
                    generics.and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // extends types
                if let Some(extends_types) =
                    heritage.and_then(|heritage| heritage.extends_types.as_ref())
                    && !extends_types.is_empty()
                {
                    format_super_type_clause(f, Keyword::Extends, extends_types)?;
                }

                // with clauses
                if let Some(with) = generics.and_then(|generics| generics.with_clauses.as_ref())
                    && !with.is_empty()
                {
                    write!(f, [space()])?;
                    format_with_clause(f, with)?;
                }

                // where clauses
                if let Some(where_clauses) =
                    generics.and_then(|generics| generics.where_clauses.as_ref())
                    && !where_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
                }

                // space before body
                write!(f, [space()])?;

                // empty body
                if properties.is_empty() {
                    write!(f, [empty_block_with_infix_annotations(node_id)])?;
                    write!(f, [f.context().any_postfix_annotations(node_id)])?;
                    return Ok(());
                }

                // body
                write!(f, [token("{"), hard_line_break()])?;

                // expressions
                if !properties.is_empty() {
                    write!(
                        f,
                        [group(&format_args![block_indent(&format_with(|f| {
                            format_block_of_properties(f, properties)
                        })),])]
                    )?;
                }

                write!(f, [f.context().block_infix_annotations(node_id)])?;

                write!(f, [hard_line_break(), token("}")])?;
            }

            // implement
            Definition::Implement {
                descriptor,
                generics,
                target_type,
                heritage,
                properties,
            } => {
                let generics = generics.as_ref();
                let heritage = heritage.as_ref();

                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                write!(f, [Keyword::Implement])?;

                // static arguments
                if let Some(static_arguments) =
                    generics.and_then(|generics| generics.static_parameters.as_ref())
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

                // target type
                write!(f, [space(), target_type])?;

                // implements types
                if let Some(implements_types) =
                    heritage.and_then(|heritage| heritage.implements_types.as_ref())
                    && !implements_types.is_empty()
                {
                    format_super_type_clause(f, Keyword::Implements, implements_types)?;
                }

                // with
                if let Some(with) = generics.and_then(|generics| generics.with_clauses.as_ref())
                    && !with.is_empty()
                {
                    write!(f, [space()])?;
                    format_with_clause(f, with)?;
                }

                // where
                if let Some(where_clauses) =
                    generics.and_then(|generics| generics.where_clauses.as_ref())
                    && !where_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
                }

                // body
                if properties.is_empty() {
                    write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
                    return Ok(());
                }

                // body
                write!(f, [space(), token("{"), hard_line_break()])?;
                write!(
                    f,
                    [group(&format_args![block_indent(&format_with(|f| {
                        format_block_of_properties(f, properties)
                    })),])]
                )?;
                write!(f, [hard_line_break(), token("}")])?;
            }

            // function
            Definition::Function {
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

                // with clause
                if let Some(with) = generics.and_then(|generics| generics.with_clauses.as_ref())
                    && !with.is_empty()
                {
                    write!(f, [space()])?;
                    format_with_clause(f, with)?;
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

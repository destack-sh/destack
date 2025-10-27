use crate::argument::list_like;
use crate::r#let::FormatScopedMutability;
use crate::r#where::format_where_clause;
use crate::with::format_with_clause;
use crate::{
    Definition, DystFormatContext, DystFormatter, FormatNode, Keyword, ModuleFormat, NodeId,
    Runtime, VariantField, VariantStyle, empty_block_with_infix_annotations,
};
use dyst_ast::{
    Asyncness, DeclarationKind, ExportType, FunctionCardinality, FunctionKind, FunctionStyle,
    Visibility,
};
use dyst_fir::format::FormatResult;
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

pub(crate) fn format_super_types<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    super_types: &[NodeId<crate::Expression>],
) -> FormatResult<()> {
    assert!(!super_types.is_empty());

    // colon separator
    write!(f, [token(":"), space()])?;

    // super types
    write!(
        f,
        [group(&format_args![
            if_group_breaks(&token("(")),
            soft_block_indent(&format_with(|f| {
                f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                    .entries(super_types)
                    .finish()?;
                // trailing comma
                write!(f, [if_group_breaks(&token(","))])?;
                Ok(())
            })),
            if_group_breaks(&token(")")),
        ])]
    )
}

impl<'ast> Format<DystFormatContext<'ast>> for Visibility {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Visibility::Public => write!(f, [Keyword::Public])?,
            Visibility::Protected => write!(f, [Keyword::Protected])?,
            Visibility::Private => write!(f, [Keyword::Private])?,
        };
        Ok(())
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for ExportType {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            ExportType::Item => write!(f, [Keyword::Export])?,
            ExportType::Default => write!(f, [Keyword::Export, space(), Keyword::Default])?,
        };
        Ok(())
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
            Definition::Module {
                meta,
                format,
                with_clauses,
                where_clauses,
                expressions,
            } => {
                if format == &ModuleFormat::Source {
                    // print expressions only for source modules (?)
                    if !expressions.is_empty() {
                        write!(
                            f,
                            [format_with(|f| f
                                .join_with(hard_line_break())
                                .entries(expressions)
                                .finish())]
                        )?;
                    }
                    return Ok(());
                }

                // export
                if let Some(export) = meta.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if meta.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // visibility
                if let Some(visibility) = meta.visibility {
                    write!(f, [visibility, space()])?;
                }

                // keyword
                write!(f, [Keyword::Module])?;
                if let Some(name) = meta.name {
                    write!(f, [space(), name])?;
                }

                // with
                if let Some(with_clauses) = with_clauses
                    && !with_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_with_clause(f, with_clauses)?;
                }

                // where
                if let Some(where_clauses) = where_clauses
                    && !where_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
                }

                // body
                match format {
                    ModuleFormat::Forward => {
                        write!(f, [token(";")])?;
                        write!(f, [f.context().any_postfix_annotations(node_id)])?;
                    }
                    ModuleFormat::Inline => {
                        write!(f, [space()])?;
                        if expressions.is_empty() {
                            write!(f, [empty_block_with_infix_annotations(node_id)])?;
                            write!(f, [f.context().any_postfix_annotations(node_id)])?;
                        } else {
                            write!(f, [token("{"), hard_line_break()])?;
                            write!(
                                f,
                                [group(&format_args![block_indent(&format_with(|f| f
                                    .join_with(hard_line_break())
                                    .entries(expressions)
                                    .finish())),])]
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
                    ModuleFormat::Source => unreachable!(),
                }
            }

            // struct
            Definition::Struct {
                meta,
                style,
                super_types,
                representation_type,
                static_parameters,
                with_clauses,
                where_clauses,
                fields,
                expressions,
            } => {
                // split tuple / struct fields
                let tuple_fields: &[NodeId<VariantField>] = if *style == VariantStyle::Tuple {
                    fields
                } else {
                    &[]
                };
                let fields: &[NodeId<VariantField>] = if *style == VariantStyle::Struct {
                    fields
                } else {
                    &[]
                };

                // export
                if let Some(export) = meta.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if meta.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // visibility
                if let Some(visibility) = meta.visibility {
                    write!(f, [visibility, space()])?;
                }

                // keyword
                write!(f, [Keyword::Struct])?;
                if let Some(representation_type) = representation_type {
                    write!(f, [token("("), representation_type, token(")")])?;
                }

                // name
                if let Some(name) = meta.name {
                    write!(f, [space()])?;
                    write!(f, [name])?;

                    // static parameters
                    if let Some(static_parameters) = &static_parameters
                        && !static_parameters.is_empty()
                    {
                        write!(f, [list_like("<", ">", ",", static_parameters)])?;
                    }
                }

                // tuple
                if *style == VariantStyle::Tuple {
                    if tuple_fields.is_empty() {
                        write!(f, [token("("), token(")")])?;
                    } else {
                        write!(
                            f,
                            [group(&format_args![
                                token("("),
                                soft_block_indent(&format_with(|f| f
                                    .join_with(&format_args![
                                        &token(","),
                                        soft_line_break_or_space()
                                    ])
                                    .entries(tuple_fields)
                                    .finish())),
                                token(")")
                            ])]
                        )?;
                    }
                }

                // super types
                if let Some(super_types) = &super_types
                    && !super_types.is_empty()
                {
                    format_super_types(f, super_types)?;
                }

                // with
                if let Some(with) = with_clauses
                    && !with.is_empty()
                {
                    write!(f, [space()])?;
                    format_with_clause(f, with)?;
                }

                if let Some(where_clauses) = where_clauses
                    && !where_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
                }

                write!(f, [space()])?;

                // empty body
                if fields.is_empty() && expressions.is_empty() {
                    write!(f, [empty_block_with_infix_annotations(node_id)])?;
                    write!(f, [f.context().any_postfix_annotations(node_id)])?;
                    return Ok(());
                }

                // body
                write!(f, [token("{"), hard_line_break()])?;

                // fields
                if !fields.is_empty() {
                    write!(
                        f,
                        [group(&format_args![block_indent(&format_with(|f| f
                            .join_with(hard_line_break())
                            .entries(fields)
                            .finish())),])]
                    )?;
                }

                // blank line between fields and statements
                if !fields.is_empty() && !expressions.is_empty() {
                    write!(f, [hard_line_break()])?;
                    if !f.context().has_blank_prefix_annotation(expressions[0]) {
                        write!(f, [empty_line()])?;
                    }
                }

                // statements
                if !expressions.is_empty() {
                    write!(
                        f,
                        [group(&format_args![block_indent(&format_with(|f| f
                            .join_with(hard_line_break())
                            .entries(expressions)
                            .finish())),])]
                    )?;
                }

                write!(f, [f.context().block_infix_annotations(node_id)])?;

                write!(f, [hard_line_break(), token("}")])?;
            }

            // enum
            Definition::Enum {
                meta,
                tag_type,
                static_parameters,
                super_types,
                with_clauses: with,
                where_clauses,
                fields,
                expressions,
            } => {
                // export
                if let Some(export) = meta.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if meta.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // visibility
                if let Some(visibility) = meta.visibility {
                    write!(f, [visibility, space()])?;
                }

                // header
                write!(f, [Keyword::Enum])?;
                // type
                if let Some(ty) = tag_type {
                    write!(f, [token("("), ty, token(")"), space()])?;
                } else {
                    write!(f, [space()])?;
                }
                // name
                if let Some(name) = meta.name {
                    write!(f, [name])?;
                }

                // static parameters
                if let Some(static_parameters) = &static_parameters
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                    write!(f, [space()])?;
                } else if meta.name.is_some() {
                    write!(f, [space()])?;
                }

                // super types
                if let Some(super_types) = &super_types
                    && !super_types.is_empty()
                {
                    format_super_types(f, super_types)?;
                }

                // with
                if let Some(with) = with
                    && !with.is_empty()
                {
                    write!(f, [space()])?;
                    format_with_clause(f, with)?;
                }

                // where
                if let Some(where_clauses) = &where_clauses
                    && !where_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
                }

                // empty block
                if fields.is_empty() && expressions.is_empty() {
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
                if !fields.is_empty() && !expressions.is_empty() {
                    write!(f, [hard_line_break()])?;
                    if !f.context().has_blank_prefix_annotation(expressions[0]) {
                        write!(f, [empty_line()])?;
                    }
                }

                // statements
                write!(
                    f,
                    [group(&format_args![block_indent(&format_with(|f| f
                        .join_with(hard_line_break())
                        .entries(expressions)
                        .finish())),])]
                )?;
                write!(f, [f.context().block_infix_annotations(node_id)])?;
                write!(f, [hard_line_break(), token("}")])?;
            }

            // interface
            Definition::Interface {
                meta,
                static_parameters,
                super_types,
                with_clauses: with,
                where_clauses,
                fields,
                expressions,
            } => {
                // export
                if let Some(export) = meta.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if meta.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // visibility
                if let Some(visibility) = meta.visibility {
                    write!(f, [visibility, space()])?;
                }

                // keyword
                write!(f, [Keyword::Interface])?;

                // name
                if let Some(name) = meta.name {
                    write!(f, [space()])?;
                    write!(f, [name])?;
                }

                // static parameters
                if let Some(static_parameters) = &static_parameters
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // super types
                if let Some(super_types) = &super_types
                    && !super_types.is_empty()
                {
                    format_super_types(f, super_types)?;
                }

                // with clauses
                if let Some(with) = &with
                    && !with.is_empty()
                {
                    write!(f, [space()])?;
                    format_with_clause(f, with)?;
                }

                if let Some(where_clauses) = &where_clauses
                    && !where_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
                }

                // space before body
                write!(f, [space()])?;

                // empty body
                if expressions.is_empty() && fields.is_empty() {
                    write!(f, [empty_block_with_infix_annotations(node_id)])?;
                    write!(f, [f.context().any_postfix_annotations(node_id)])?;
                    return Ok(());
                }

                // body
                write!(f, [token("{"), hard_line_break()])?;

                // fields
                if !fields.is_empty() {
                    write!(
                        f,
                        [group(&format_args![block_indent(&format_with(|f| f
                            .join_with(hard_line_break())
                            .entries(fields)
                            .finish())),])]
                    )?;
                }

                // blank line between fields and statements
                if !fields.is_empty() && !expressions.is_empty() {
                    write!(f, [hard_line_break()])?;
                    if !f.context().has_blank_prefix_annotation(expressions[0]) {
                        write!(f, [empty_line()])?;
                    }
                }

                // statements
                if !expressions.is_empty() {
                    write!(
                        f,
                        [group(&format_args![block_indent(&format_with(|f| f
                            .join_with(hard_line_break())
                            .entries(expressions)
                            .finish())),])]
                    )?;
                }

                write!(f, [f.context().block_infix_annotations(node_id)])?;

                write!(f, [hard_line_break(), token("}")])?;
            }

            // union
            Definition::Union {
                meta,
                tag_type,
                representation_type,
                static_parameters,
                super_types,
                with_clauses: with,
                where_clauses,
                fields,
                expressions,
            } => {
                // export
                if let Some(export) = meta.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if meta.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // visibility
                if let Some(visibility) = meta.visibility {
                    write!(f, [visibility, space()])?;
                }

                // keyword
                write!(f, [Keyword::Union])?;

                // tag and representation type
                if tag_type.is_some() || representation_type.is_some() {
                    write!(f, [token("(")])?;
                    let mut join = f.join_with(token(", "));
                    if let Some(tag_type) = tag_type {
                        join.entry(&tag_type);
                    }
                    if let Some(representation_type) = representation_type {
                        join.entry(&representation_type);
                    }
                    join.finish()?;
                    write!(f, [token(")")])?;
                }

                // name
                if let Some(name) = meta.name {
                    write!(f, [space()])?;
                    write!(f, [name])?;
                }

                // static parameters
                if let Some(static_parameters) = &static_parameters
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // super types
                if let Some(super_types) = &super_types
                    && !super_types.is_empty()
                {
                    format_super_types(f, super_types)?;
                }

                // with
                if let Some(with) = with
                    && !with.is_empty()
                {
                    write!(f, [space()])?;
                    format_with_clause(f, with)?;
                }

                // where
                if let Some(where_clauses) = &where_clauses
                    && !where_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
                }

                // space before body braces
                write!(f, [space()])?;

                // empty body
                if fields.is_empty() && expressions.is_empty() {
                    write!(f, [empty_block_with_infix_annotations(node_id)])?;
                    write!(f, [f.context().any_postfix_annotations(node_id)])?;
                    return Ok(());
                }

                // body
                write!(f, [token("{"), hard_line_break()])?;

                // fields
                if !fields.is_empty() {
                    write!(
                        f,
                        [group(&format_args![block_indent(&format_with(|f| f
                            .join_with(hard_line_break())
                            .entries(fields)
                            .finish())),])]
                    )?;
                }

                // blank line between fields and statements
                if !fields.is_empty() && !expressions.is_empty() {
                    write!(f, [hard_line_break()])?;
                    if !f.context().has_blank_prefix_annotation(expressions[0]) {
                        write!(f, [empty_line()])?;
                    }
                }

                // statements
                if !expressions.is_empty() {
                    write!(
                        f,
                        [group(&format_args![block_indent(&format_with(|f| f
                            .join_with(hard_line_break())
                            .entries(expressions)
                            .finish())),])]
                    )?;
                }
                write!(f, [f.context().block_infix_annotations(node_id)])?;

                // body closing braces
                write!(f, [hard_line_break(), token("}")])?;
            }

            // implement
            Definition::Implement {
                meta,
                static_parameters: static_arguments,
                target_type,
                super_types,
                with_clauses: with,
                where_clauses,
                expressions,
            } => {
                // export
                if let Some(export) = meta.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if meta.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // visibility
                if let Some(visibility) = meta.visibility {
                    write!(f, [visibility, space()])?;
                }

                // keyword
                write!(f, [Keyword::Implement])?;

                // static arguments
                if let Some(static_arguments) = &static_arguments
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

                // super types
                if let Some(super_types) = &super_types
                    && !super_types.is_empty()
                {
                    format_super_types(f, super_types)?;
                }

                // with
                if let Some(with) = with
                    && !with.is_empty()
                {
                    write!(f, [space()])?;
                    format_with_clause(f, with)?;
                }

                // where
                if let Some(where_clauses) = &where_clauses
                    && !where_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
                }

                // body
                if expressions.is_empty() {
                    write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
                    return Ok(());
                }

                // body
                write!(f, [space(), token("{"), hard_line_break()])?;
                write!(
                    f,
                    [group(&format_args![block_indent(&format_with(|f| f
                        .join_with(hard_line_break())
                        .entries(expressions)
                        .finish())),])]
                )?;
                write!(f, [hard_line_break(), token("}")])?;
            }

            // function
            Definition::Function {
                meta,
                runtime,
                asyncness,
                cardinality,
                kind,
                style,
                static_parameters,
                self_parameter,
                dynamic_parameters,
                return_type,
                with_clauses: with,
                where_clauses,
                body,
            } => {
                // export
                if let Some(export) = meta.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if meta.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // visibility
                if let Some(visibility) = meta.visibility {
                    write!(f, [visibility, space()])?;
                }

                // asyncness
                if *asyncness == Asyncness::Async {
                    write!(f, [Keyword::Async, space()])?;
                }

                // kind
                if let Some(kind) = kind {
                    write!(f, [kind.to_keyword()])?;
                    if meta.name.is_some() {
                        write!(f, [space()])?;
                    }
                }

                // keyword
                if *style == FunctionStyle::Function && *kind != Some(FunctionKind::Constructor) {
                    // function keyword
                    if *cardinality == FunctionCardinality::Generator {
                        write!(f, [Keyword::Function, token("*"), space()])?;
                    } else {
                        write!(f, [Keyword::Function, space()])?;
                    }
                } else {
                    // lambda (no keyword, maybe star)
                    if *cardinality == FunctionCardinality::Generator {
                        write!(f, [token("*"), space()])?;
                    }
                }

                if *style == FunctionStyle::Function {
                    // name (with @)
                    if *runtime == Runtime::Static {
                        write!(f, [token("@")])?;
                    }
                    if let Some(name) = meta.name {
                        write!(f, [name])?;
                    }

                    // static parameters
                    if let Some(static_parameters) = &static_parameters
                        && !static_parameters.is_empty()
                    {
                        write!(f, [list_like("<", ">", ",", static_parameters)])?;
                    }
                }

                // self parameter and dynamic parameters
                write!(
                    f,
                    [group(&format_args![
                        token("("),
                        soft_block_indent(&format_with(|f| {
                            // self parameter
                            let separator = format_with(|f| {
                                token(",").format(f)?;
                                soft_line_break_or_space().format(f)
                            });
                            let mut join = f.join_with(&separator);
                            if let Some(self_parameter) = self_parameter.as_ref() {
                                let is_reference = self_parameter.is_reference;
                                let mutability = &self_parameter.mutability;
                                join.entry(&format_with(move |f| {
                                    // pointer
                                    if is_reference {
                                        write!(f, [token("&")])?;
                                    }
                                    // mutability
                                    write!(
                                        f,
                                        [FormatScopedMutability::implicit_const(
                                            mutability.clone()
                                        )]
                                    )?;
                                    if mutability.is_mutable() {
                                        write!(f, [space()])?;
                                    }
                                    // self
                                    write!(f, [self_parameter.keyword])?;
                                    // ty
                                    if let Some(ty) = self_parameter.ty {
                                        write!(f, [token(":"), space(), ty])?;
                                    }
                                    Ok(())
                                }));
                            }
                            // dynamic parameters
                            join.entries(dynamic_parameters);
                            join.finish()?;

                            // trailing comma
                            write!(f, [if_group_breaks(&token(","))])?;

                            Ok(())
                        })),
                        token(")")
                    ])]
                )?;

                // return type
                if let Some(return_type) = return_type {
                    if *style == FunctionStyle::Lambda && body.is_some() {
                        write!(f, [token(":"), space(), return_type])?;
                    } else {
                        write!(f, [space(), token("=>"), space(), return_type])?;
                    }
                }

                // with clause
                if let Some(with) = with
                    && !with.is_empty()
                {
                    write!(f, [space()])?;
                    format_with_clause(f, with)?;
                }

                // where clause
                if let Some(where_clauses) = &where_clauses
                    && !where_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_where_clause(f, where_clauses)?;
                }

                // body
                if let Some(body) = body {
                    if *style == FunctionStyle::Lambda {
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

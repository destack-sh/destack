use crate::argument::list_like;
use crate::r#let::FormatScopedMutability;
use crate::r#where::format_where_clause;
use crate::with::format_with_clause;
use crate::{
    Definition, DystFormatter, FormatNode, Keyword, ModuleFormat, NodeId, Runtime, StructField,
    StructStyle, empty_block_with_infix_annotations,
};
use dyst_fir::format::FormatResult;
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

pub(crate) fn format_super_types<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    super_types: &[NodeId<crate::Expression>],
) -> FormatResult<()> {
    // colon separator
    write!(f, [token(":"), space()])?;

    // super types
    write!(
        f,
        [group(&format_args![
            if_group_breaks(&token("(")),
            soft_block_indent(&format_with(|f| {
                f.join_with(&format_args![
                    if_group_fits_on_line(&token(",")),
                    soft_line_break_or_space()
                ])
                .entries(super_types)
                .finish()
            })),
            if_group_breaks(&token(")")),
        ])]
    )?;

    Ok(())
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
                format,
                name,
                visibility: _,
                expressions,
            } => {
                // implicit module (whole file)
                match format {
                    ModuleFormat::Source => write!(
                        f,
                        [format_with(|f| f
                            .join_with(hard_line_break())
                            .entries(expressions)
                            .finish()),]
                    )?,
                    // declaration module (module x;)
                    ModuleFormat::Forward => write!(f, [Keyword::Module, space(), name])?,
                    // explicit module (module { ... })
                    ModuleFormat::Inline => {
                        // header
                        if let Some(name) = name {
                            write!(f, [Keyword::Module, space(), name, space()])?;
                        } else {
                            write!(f, [Keyword::Module, space()])?;
                        }
                        // empty body
                        if expressions.is_empty() {
                            write!(f, [empty_block_with_infix_annotations(node_id)])?;
                            write!(f, [f.context().any_postfix_annotations(node_id)])?;
                            return Ok(());
                        }
                        // body
                        write!(
                            f,
                            [group(&format_args![
                                token("{"),
                                hard_line_break(),
                                format_with(|f| f
                                    .join_with(hard_line_break())
                                    .entries(expressions)
                                    .finish()),
                                hard_line_break(),
                                f.context().block_infix_annotations(node_id),
                                token("}")
                            ])]
                        )?
                    }
                }
            }

            // struct
            Definition::Struct {
                name,
                visibility: _,
                style,
                super_types,
                representation_type,
                static_parameters,
                with_clauses: with,
                where_clauses,
                fields,
                expressions,
            } => {
                // split tuple / struct fields
                let tuple_fields: &[NodeId<StructField>] = if *style == StructStyle::Tuple {
                    fields
                } else {
                    &[]
                };
                let struct_fields: &[NodeId<StructField>] = if *style == StructStyle::Struct {
                    fields
                } else {
                    &[]
                };

                // keyword
                write!(f, [Keyword::Struct])?;
                if let Some(representation_type) = representation_type {
                    write!(f, [token("("), representation_type, token(")")])?;
                }

                // name
                if let Some(name) = name {
                    write!(f, [space()])?;
                    write!(f, [name])?;

                    // static parameters
                    if let Some(static_parameters) = &static_parameters
                        && !static_parameters.is_empty()
                    {
                        write!(f, [list_like("<", ">", ",", false, static_parameters)])?;
                    }
                }

                // tuple
                if *style == StructStyle::Tuple {
                    if tuple_fields.is_empty() {
                        write!(f, [token("("), token(")")])?;
                    } else {
                        write!(
                            f,
                            [group(&format_args![
                                token("("),
                                soft_block_indent(&format_with(|f| f
                                    .join_with(&format_args![
                                        if_group_fits_on_line(&token(",")),
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
                if let Some(with) = with
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

                write!(f, [space()])?;

                // empty body
                if struct_fields.is_empty() && expressions.is_empty() {
                    write!(f, [empty_block_with_infix_annotations(node_id)])?;
                    write!(f, [f.context().any_postfix_annotations(node_id)])?;
                    return Ok(());
                }

                // body
                write!(f, [token("{"), hard_line_break()])?;

                // fields
                if !struct_fields.is_empty() {
                    write!(
                        f,
                        [group(&format_args![block_indent(&format_with(|f| f
                            .join_with(hard_line_break())
                            .entries(struct_fields)
                            .finish())),])]
                    )?;
                }

                // blank line between fields and statements
                if !struct_fields.is_empty() && !expressions.is_empty() {
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
                name,
                visibility: _,
                tag_type,
                static_parameters,
                super_types,
                with_clauses: with,
                where_clauses,
                fields,
                expressions,
            } => {
                // header
                write!(f, [Keyword::Enum])?;
                // type
                if let Some(ty) = tag_type {
                    write!(f, [token("("), ty, token(")"), space()])?;
                } else {
                    write!(f, [space()])?;
                }
                // name
                if let Some(name) = name {
                    write!(f, [name])?;
                }

                // static parameters
                if let Some(static_parameters) = &static_parameters
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", false, static_parameters)])?;
                    write!(f, [space()])?;
                } else if name.is_some() {
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

            // trait
            Definition::Trait {
                name,
                visibility: _,
                static_parameters,
                super_types,
                with_clauses: with,
                where_clauses,
                expressions,
            } => {
                // keyword
                write!(f, [Keyword::Trait])?;

                // name
                if let Some(name) = name {
                    write!(f, [space()])?;
                    write!(f, [name])?;
                }

                // static parameters
                if let Some(static_parameters) = &static_parameters
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", false, static_parameters)])?;
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

                // space before trait body
                write!(f, [space()])?;

                // empty body
                if expressions.is_empty() {
                    write!(f, [empty_block_with_infix_annotations(node_id)])?;
                    write!(f, [f.context().any_postfix_annotations(node_id)])?;
                    return Ok(());
                }

                // body
                write!(f, [token("{"), hard_line_break()])?;
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

            // union
            Definition::Union {
                name,
                visibility: _,
                tag_type,
                representation_type,
                static_parameters,
                super_types,
                with_clauses: with,
                where_clauses,
                fields,
                expressions,
            } => {
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
                if let Some(name) = name {
                    write!(f, [space()])?;
                    write!(f, [name])?;
                }

                // static parameters
                if let Some(static_parameters) = &static_parameters
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", false, static_parameters)])?;
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
                static_arguments,
                receiver,
                for_trait,
                with_clauses: with,
                where_clauses,
                expressions,
            } => {
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
                                f.join_with(&format_args![
                                    if_group_fits_on_line(&token(",")),
                                    soft_line_break_or_space()
                                ])
                                .entries(static_arguments)
                                .finish()
                            })),
                            token(">")
                        ])]
                    )?;
                }

                // receiver
                write!(f, [space(), receiver])?;

                // for clause
                if let Some(for_trait) = for_trait {
                    write!(f, [space(), Keyword::For, space(), for_trait])?;
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
                name,
                visibility: _,
                runtime,
                style: _,
                static_parameters,
                self_parameter,
                dynamic_parameters,
                return_type,
                with_clauses: with,
                where_clauses,
                body,
            } => {
                // keyword
                write!(f, [Keyword::Function, space()])?;

                // name (with @)
                if *runtime == Runtime::Static {
                    write!(f, [token("@")])?;
                }
                if let Some(name) = name {
                    write!(f, [name])?;
                }

                // static parameters
                if let Some(static_parameters) = &static_parameters
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", false, static_parameters)])?;
                }

                // self parameter and dynamic parameters
                write!(
                    f,
                    [group(&format_args![
                        token("("),
                        soft_block_indent(&format_with(|f| {
                            // self parameter
                            let separator = format_with(|f| {
                                if_group_fits_on_line(&token(",")).format(f)?;
                                soft_line_break_or_space().format(f)
                            });
                            let mut join = f.join_with(&separator);
                            if let Some(self_parameter) = self_parameter.as_ref() {
                                let is_pointer = self_parameter.is_pointer;
                                let mutability = &self_parameter.mutability;
                                join.entry(&format_with(move |f| {
                                    // pointer
                                    if is_pointer {
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
                                    write!(f, [Keyword::Self_])
                                }));
                            }
                            // dynamic parameters
                            join.entries(dynamic_parameters);
                            join.finish()
                        })),
                        token(")")
                    ])]
                )?;

                // return type
                if let Some(return_type) = return_type {
                    write!(f, [space(), token("=>"), space(), return_type])?;
                }

                // with clause
                if let Some(with) = with
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

                // body
                if let Some(body) = body {
                    write!(f, [space(), body])?;
                }
            }
        }

        write!(f, [f.context().any_postfix_annotations(node_id)])?;

        Ok(())
    }
}

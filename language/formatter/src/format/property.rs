use crate::argument::list_like;
use crate::variant::{
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix_maybe,
};
use crate::r#where::format_where_clause;
use crate::with::format_with_clause;
use crate::{DystFormatter, FormatNode};
use dyst_ast::{Asynchrony, FunctionAbstraction, FunctionCardinality, Keyword, NodeId, Property};
use dyst_fir::format::FormatResult;
use dyst_fir::prelude::*;
use dyst_fir::write;

/// Format a block of properties (with appropriate empty annotations)
pub(crate) fn format_block_of_properties<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    properties: &[NodeId<Property>],
) -> FormatResult<()> {
    for (i, &property_id) in properties.iter().enumerate() {
        // blank line between properties
        if i > 0 {
            write!(f, [hard_line_break()])?;
        }
        property_id.format(f)?;
    }
    Ok(())
}

impl<'ast> FormatNode<'ast, Property> for Property {
    fn format_node(
        &self,
        node_id: NodeId<Property>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            Property::Field {
                modifiers,
                key,
                value,
                default,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // key
                write!(f, [key])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // value
                if let Some(value) = value {
                    write!(f, [token(":"), space(), value])?;
                }
                // default
                if let Some(default) = default {
                    write!(f, [space(), token("="), space(), default])?;
                }
            }
            Property::Method {
                modifiers,
                key,
                asynchrony,
                abstraction,
                cardinality,
                mode,
                static_parameters,
                dynamic_parameters,
                return_type,
                with_clauses,
                where_clauses,
                body,
            } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;

                // abstraction
                match *abstraction {
                    FunctionAbstraction::Abstract => {
                        write!(f, [Keyword::Abstract, space()])?;
                    }
                    FunctionAbstraction::AbstractOverride => {
                        write!(f, [Keyword::Abstract, space()])?;
                        write!(f, [Keyword::Override, space()])?;
                    }
                    FunctionAbstraction::ConcreteOverride => {
                        write!(f, [Keyword::Override, space()])?;
                    }
                    FunctionAbstraction::Concrete => {}
                }

                // asynchrony
                if *asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Async, space()])?;
                }

                // mode
                if let Some(mode) = mode {
                    write!(f, [mode.to_keyword()])?;
                    if key.is_some() {
                        write!(f, [space()])?;
                    }
                }

                // cardinality
                if *cardinality == FunctionCardinality::Generator {
                    write!(f, [token("*")])?;
                }

                // key
                write!(f, [key])?;

                // static parameters
                if let Some(static_parameters) = &static_parameters
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // dynamic parameters
                write!(f, [list_like("(", ")", ",", dynamic_parameters)])?;

                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;

                // return type
                if let Some(return_type) = return_type {
                    write!(f, [token(":"), space(), return_type])?;
                }

                // with clauses
                if let Some(with_clauses) = with_clauses
                    && !with_clauses.is_empty()
                {
                    write!(f, [space()])?;
                    format_with_clause(f, with_clauses)?;
                }

                // where clauses
                if let Some(where_clauses) = where_clauses
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
            Property::Spread { modifiers, value } => {
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // keyword
                write!(f, [token("...")])?;
                // value
                write!(f, [value])?;
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

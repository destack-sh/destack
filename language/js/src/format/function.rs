use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};

use crate::format::expression::format_expression_id_with_precedence;
use crate::{
    Asynchrony, Context, FormatNode, Formatter, FunctionSignature, Keyword, LocalNodeId, Parameter,
    Pattern, Precedence,
};

/// Format one function signature parameter list.
pub(crate) fn format_function_signature_parameters<'ast>(
    signature: &FunctionSignature,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    format_function_parameters(&signature.parameters, signature.rest, f)
}

/// Format one JavaScript parameter list.
pub(crate) fn format_function_parameters<'ast>(
    parameters: &[LocalNodeId<Parameter>],
    rest: Option<LocalNodeId<Pattern>>,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    let body = format_with(|f| {
        for (index, parameter) in parameters.iter().enumerate() {
            if index > 0 {
                write!(f, [token(","), soft_line_break_or_space()])?;
            }
            parameter.format(f)?;
        }

        if let Some(rest) = rest {
            if !parameters.is_empty() {
                write!(f, [token(","), soft_line_break_or_space()])?;
            }
            write!(f, [token("..."), rest])?;
        } else if !parameters.is_empty() {
            write!(f, [if_group_breaks(&token(","))])?;
        }

        Ok(())
    });

    write!(
        f,
        [group(&format_args![
            &token("("),
            block_indent(&body),
            &token(")")
        ])]
    )
}

/// Format one ordinary method header.
pub(crate) fn format_method<'ast, T>(
    is_static: bool,
    key: &T,
    signature: &FunctionSignature,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()>
where
    T: Format<'ast, Context<'ast>>,
{
    format_static(is_static, f)?;
    if signature.asynchrony == Asynchrony::Async {
        write!(f, [Keyword::Async, space()])?;
    }
    if signature.is_generator {
        write!(f, [token("*")])?;
    }
    key.format(f)?;
    format_function_signature_parameters(signature, f)
}

/// Format one static modifier.
pub(crate) fn format_static<'ast>(
    is_static: bool,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    if is_static {
        write!(f, [Keyword::Static, space()])?;
    }

    Ok(())
}

impl<'ast> FormatNode<'ast> for Parameter {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Parameter::Named { name, default } => {
                write!(f, [name])?;
                format_default(*default, f)?;
            }
            Parameter::Pattern { pattern, default } => {
                write!(f, [pattern])?;
                format_default(*default, f)?;
            }
        }

        Ok(())
    }
}

/// Format one optional parameter default.
fn format_default<'ast>(
    default: Option<LocalNodeId<crate::Expression>>,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    if let Some(default) = default {
        write!(f, [space(), token("="), space()])?;
        format_expression_id_with_precedence(default, Precedence::Assignment, f)?;
    }

    Ok(())
}

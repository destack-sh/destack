use destack_ast::{self as ast};

use crate::rules::common::expression_path_segments;

/// Regex pattern info extracted from one AST expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AstRegexPatternInfo {
    /// The regex pattern string id.
    pub pattern_id: ast::StringId,
    /// Optional regex flags string id when statically known.
    pub flags_id: Option<ast::StringId>,
    /// Whether constructor flags are present but not statically known.
    pub has_unknown_flags: bool,
}

/// Resolve regex pattern info from a regex literal or `RegExp` constructor call.
pub fn regex_pattern_info(
    tree: &ast::NodeTree,
    expression_id: ast::LocalNodeId<ast::Expression>,
    regexp_name: ast::StringId,
) -> Option<AstRegexPatternInfo> {
    // normalize expression shape
    let expression_id = super::expression_unwrap_parenthesized_syntax(tree, expression_id);
    let expression = tree.get(expression_id);

    // support direct regex literals
    if let ast::Expression::ScalarLiteral(ast::ScalarLiteral::RegexString { content, flags }) =
        expression
    {
        return Some(AstRegexPatternInfo {
            pattern_id: *content,
            flags_id: *flags,
            has_unknown_flags: false,
        });
    }

    // support `RegExp(...)` and `new RegExp(...)`
    let (callee_id, arguments) = match expression {
        ast::Expression::Call {
            left,
            dynamic_arguments,
            ..
        }
        | ast::Expression::New {
            left,
            dynamic_arguments,
            ..
        } => (*left, dynamic_arguments.as_slice()),
        _ => return None,
    };

    // require global RegExp constructor identifier
    let Some(path_segments) = expression_path_segments(tree, callee_id) else {
        return None;
    };
    if path_segments.as_slice() != [regexp_name] {
        return None;
    }

    // require first positional string pattern argument
    let first_argument_id = *arguments.first()?;
    let first_argument = tree.get(first_argument_id);
    let ast::Argument::Positional {
        value: pattern_value,
        ..
    } = first_argument
    else {
        return None;
    };
    let pattern_expression = tree.get(*pattern_value);
    let ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(pattern_id)) = pattern_expression
    else {
        return None;
    };

    // resolve optional second positional flags argument when statically known
    let mut has_unknown_flags = false;
    let flags_id = arguments.get(1).and_then(|flags_argument_id| {
        let flags_argument = tree.get(*flags_argument_id);
        let ast::Argument::Positional {
            value: flags_value, ..
        } = flags_argument
        else {
            has_unknown_flags = true;
            return None;
        };
        let flags_expression = tree.get(*flags_value);
        let ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(flags_id)) = flags_expression
        else {
            has_unknown_flags = true;
            return None;
        };

        Some(*flags_id)
    });

    Some(AstRegexPatternInfo {
        pattern_id: *pattern_id,
        flags_id,
        has_unknown_flags,
    })
}

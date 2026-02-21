/// Enumerate declarator layout outcomes.
#[derive(Clone, Copy, Debug)]
pub(super) enum DeclaratorLayout {
    Inline,
    BreakAfterOperator,
    ValueExpanded,
    HeaderExpanded,
    Indented,
}

/// Store normalized declarator layout selection inputs.
#[derive(Clone, Copy, Debug)]
pub(super) struct DeclaratorLayoutInputs {
    pub(super) pattern_breakable: bool,
    pub(super) value_breakable: bool,
    pub(super) value_is_leading_pipe_type_union: bool,
    pub(super) value_is_inline_closure_cast_type_binary: bool,
    pub(super) is_string_literal: bool,
    pub(super) value_is_long_binary: bool,
    pub(super) value_has_between_comment: bool,
    pub(super) value_handles_its_own_breaking: bool,
    pub(super) value_has_prefix_annotation_that_forces_break: bool,
    pub(super) has_significant_between_comment: bool,
    pub(super) value_has_generic_class_heritage: bool,
    pub(super) value_is_declaration: bool,
    pub(super) is_template_expression: bool,
    pub(super) value_is_await_expression: bool,
    pub(super) value_is_comptime_expression: bool,
    pub(super) value_is_sequence: bool,
    pub(super) value_has_line_comment_between_operands: bool,
    pub(super) value_is_call_like: bool,
    pub(super) value_is_poor_chain: bool,
    pub(super) value_has_static_arguments: bool,
    pub(super) value_has_nested_call_chain: bool,
    pub(super) value_has_instantiation_prefix: bool,
    pub(super) value_has_newline: bool,
    pub(super) pattern_has_newline: bool,
    pub(super) pattern_has_default_assignment: bool,
    pub(super) pattern_has_comments_or_annotations: bool,
    pub(super) value_is_parenthesized: bool,
    pub(super) value_has_block_static_arguments: bool,
    pub(super) value_has_class_heritage: bool,
}

/// Choose the declarator layout from normalized inputs.
pub(super) fn choose_declarator_layout(inputs: DeclaratorLayoutInputs) -> DeclaratorLayout {
    // keep leading pipe unions inline at the declarator level
    if inputs.value_is_leading_pipe_type_union {
        return DeclaratorLayout::Inline;
    }

    // keep closure-cast type binaries inline in declarator rhs
    if inputs.value_is_inline_closure_cast_type_binary {
        return DeclaratorLayout::Inline;
    }

    // template rhs values stay inline in declarators
    if inputs.is_template_expression {
        return DeclaratorLayout::Inline;
    }

    // compact await and comptime rhs values stay inline when trivia-free
    if (inputs.value_is_await_expression || inputs.value_is_comptime_expression)
        && !inputs.pattern_breakable
        && !inputs.value_has_between_comment
        && !inputs.value_has_prefix_annotation_that_forces_break
    {
        return DeclaratorLayout::Inline;
    }

    // keep string rhs mostly inline
    if inputs.is_string_literal {
        if inputs.pattern_breakable {
            return if inputs.pattern_has_newline {
                DeclaratorLayout::HeaderExpanded
            } else {
                DeclaratorLayout::BreakAfterOperator
            };
        }

        return DeclaratorLayout::BreakAfterOperator;
    }

    // declaration rhs values with prefix trivia should break directly after `=`
    if inputs.value_is_declaration && inputs.value_has_prefix_annotation_that_forces_break {
        return DeclaratorLayout::BreakAfterOperator;
    }

    // binary rhs operator break policy
    if inputs.value_is_long_binary {
        return DeclaratorLayout::BreakAfterOperator;
    }

    // chain and binary values that already own their line breaking
    if inputs.value_handles_its_own_breaking {
        let has_forced_operator_break = inputs.value_has_prefix_annotation_that_forces_break
            || inputs.has_significant_between_comment;

        if inputs.pattern_breakable {
            if has_forced_operator_break {
                return DeclaratorLayout::BreakAfterOperator;
            }

            let should_break_after_operator_for_rhs = !inputs.pattern_has_newline
                && ((inputs.value_is_call_like && inputs.pattern_has_default_assignment)
                    || inputs.value_is_sequence
                    || inputs.value_has_line_comment_between_operands);
            if should_break_after_operator_for_rhs {
                return DeclaratorLayout::BreakAfterOperator;
            }

            return DeclaratorLayout::Inline;
        }

        if has_forced_operator_break {
            return DeclaratorLayout::BreakAfterOperator;
        }

        // complex static generic argument blocks already break inside the rhs
        if inputs.value_has_block_static_arguments {
            return DeclaratorLayout::Inline;
        }

        // class heritage wrappers own their internal multiline breaking
        if inputs.value_has_class_heritage && !inputs.value_has_generic_class_heritage {
            return DeclaratorLayout::Inline;
        }

        // poor chains, generic argument calls, and sequence-like rhs shapes prefer operator seams
        let value_is_simple_static_argument_call =
            inputs.value_has_static_arguments && !inputs.value_has_nested_call_chain;
        let value_is_simple_poor_chain = inputs.value_is_poor_chain
            && !inputs.value_has_static_arguments
            && !inputs.value_is_await_expression
            && !inputs.value_is_parenthesized;
        let prefers_operator_seam = inputs.value_has_line_comment_between_operands
            || inputs.value_is_sequence
            || inputs.value_has_generic_class_heritage
            || inputs.value_has_instantiation_prefix
            || value_is_simple_poor_chain
            || value_is_simple_static_argument_call;
        if prefers_operator_seam {
            return DeclaratorLayout::BreakAfterOperator;
        }

        return DeclaratorLayout::Inline;
    }

    // layout matrix for non self breaking values
    match (inputs.pattern_breakable, inputs.value_breakable) {
        (true, true) => {
            if inputs.value_has_newline || inputs.pattern_has_newline {
                DeclaratorLayout::ValueExpanded
            } else {
                DeclaratorLayout::Inline
            }
        }
        (true, false) => {
            if inputs.pattern_has_newline {
                DeclaratorLayout::HeaderExpanded
            } else if inputs.pattern_has_comments_or_annotations {
                DeclaratorLayout::BreakAfterOperator
            } else {
                DeclaratorLayout::Inline
            }
        }
        (false, true) => {
            let prefers_operator_break = inputs.value_has_between_comment;
            let prefer_declaration_operator_break =
                inputs.value_is_declaration && inputs.value_has_newline;

            if prefers_operator_break || prefer_declaration_operator_break {
                if !inputs.value_has_newline
                    && !inputs.value_has_between_comment
                    && !inputs.value_has_prefix_annotation_that_forces_break
                {
                    DeclaratorLayout::Inline
                } else {
                    DeclaratorLayout::Indented
                }
            } else if inputs.value_is_parenthesized && inputs.value_has_newline {
                DeclaratorLayout::Inline
            } else if inputs.value_has_newline
                || inputs.value_has_prefix_annotation_that_forces_break
            {
                DeclaratorLayout::ValueExpanded
            } else {
                DeclaratorLayout::Inline
            }
        }
        (false, false) => {
            if inputs.value_has_generic_class_heritage && inputs.value_has_newline {
                DeclaratorLayout::Indented
            } else if inputs.value_is_parenthesized && inputs.value_has_newline {
                DeclaratorLayout::Inline
            } else {
                DeclaratorLayout::BreakAfterOperator
            }
        }
    }
}

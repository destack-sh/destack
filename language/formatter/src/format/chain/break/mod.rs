mod analysis;
mod annotation;
mod head_split;
mod intervening;
mod overflow;
mod path;

pub(crate) use self::analysis::{
    ChainBreakAnalysis, analyze_chain_break, chain_has_nonhead_nonlambda_function_call_argument,
    should_break_chain,
};
pub(crate) use self::annotation::{
    chain_line_starts_with_block_prefix_annotation, chain_node_has_breaking_annotation,
    chain_node_has_non_inline_annotation,
};
pub(crate) use self::head_split::split_chain_head_operations;
pub(crate) use self::path::{
    expression_is_in_conditional_branch, expression_is_in_template_literal_interpolation,
    is_assignment_chain_tail_lambda, should_split_chain_root_path_segments,
};

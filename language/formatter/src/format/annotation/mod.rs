mod decorator;
mod sequence;
mod trivia;

pub(crate) use self::sequence::{
    block_infix_annotations, infix_or_postfix_annotations,
    infix_or_postfix_annotations_without_line_postfix_boundary, line_postfix_boundary_annotations,
    postfix_annotations, postfix_annotations_without_line_postfix_boundary, prefix_annotations,
    write_annotation_sequence, write_annotation_sequence_without_trailing_break,
    write_inline_prefix_annotations,
};

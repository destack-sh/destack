mod annotation;
mod attachment;
mod blank;
mod boundary;
mod declaration;
mod expression;
mod operator;
mod ownership;
mod render;
mod semicolon;
mod statement;

pub(crate) use attachment::formatter_annotation_projection;
pub use render::{AnnotationCapture, Annotations};
pub(crate) use semicolon::{
    SemicolonGuardCommentSeam, classify_semicolon_guard_comment_seam,
    expression_needs_statement_terminator, statement_wrapper_needs_semicolon,
    token_type_is_semicolon_guard_head,
};

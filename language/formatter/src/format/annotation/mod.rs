mod attachment;
mod facts;
mod ownership;
mod render;
mod semantic;
mod semicolon;
mod terminator;

pub(crate) use attachment::annotation_projection;
#[cfg(test)]
pub(crate) use ownership::find_smallest_owner_enclosing_range;
pub(crate) use render::{annotation_render_items_matching, write_annotation_render_items};
pub(crate) use terminator::{
    expression_needs_statement_terminator, statement_wrapper_needs_semicolon,
};

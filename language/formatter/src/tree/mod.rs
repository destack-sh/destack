mod attribute;
mod child;
mod element;
mod literal;
mod node;
mod text;
mod whitespace;

pub(crate) use self::attribute::should_force_break_tree_attributes;
pub(crate) use self::child::{tree_child_breaks_element, tree_control_child_should_expand};
pub(crate) use self::element::FormatTreeOpeningElement;
pub(crate) use self::literal::{format_tree_literal_expression, tree_literal_should_break};
pub(crate) use self::node::{has_multiline_tree_argument, write_tree_attribute, write_tree_child};
pub(crate) use self::whitespace::{
    is_tree_whitespace_char, tree_children_have_blank_line_between, tree_text_child_text,
    tree_text_is_whitespace_only,
};

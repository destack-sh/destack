mod declaration;
mod dispatch;
mod external;
mod interface;
mod lower;
mod name;
mod root;
mod string;
mod symbol;

pub(crate) use lower::*;
pub(crate) use name::{static_key_to_field_name, string_literal_global_name_for_content};
pub(crate) use string::collect_expression_string_literals;

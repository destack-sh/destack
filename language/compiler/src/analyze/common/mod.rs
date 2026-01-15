mod canonical;
mod conditional;
mod json;
mod key;
mod literal;
mod mapped;
mod normalize;
mod shape;
mod r#type;

pub(crate) use destack_dir::NormalizationMode;
pub(crate) use json::json_value_to_type;
pub(crate) use literal::evaluate_numeric_literal;
pub(crate) use shape::{ObjectShape, ObjectShapeSet};

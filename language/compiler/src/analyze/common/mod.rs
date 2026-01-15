mod canonical;
mod json;
mod key;
mod mapped;
mod normalize;
mod shape;
mod r#type;

pub(crate) use destack_dir::NormalizationMode;
pub(crate) use json::json_value_to_type;
pub(crate) use shape::{ObjectShapeSet, ObjectShape};

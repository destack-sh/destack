mod no_blank_target;
mod no_script_url;

use crate::{BoxedLintRule, boxed};

pub use no_blank_target::*;
pub use no_script_url::*;

/// Get all security rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![boxed(NoBlankTarget), boxed(NoScriptUrl)]
}

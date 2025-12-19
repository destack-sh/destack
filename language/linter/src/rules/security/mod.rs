mod no_blank_target;
mod no_hardcoded_ip;
mod no_script_url;
mod no_secrets;

use crate::{BoxedLintRule, boxed};

pub use no_blank_target::*;
pub use no_hardcoded_ip::*;
pub use no_script_url::*;
pub use no_secrets::*;

/// Get all security rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(NoBlankTarget),
        boxed(NoHardcodedIp),
        boxed(NoScriptUrl),
        boxed(NoSecrets),
    ]
}

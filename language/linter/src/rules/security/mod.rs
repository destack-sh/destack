mod no_blank_target;
mod no_hardcoded_ip;
mod no_implied_eval;
mod no_insecure_random;
mod no_open_redirect;
mod no_prototype_pollution;
mod no_regex_injection;
mod no_script_url;
mod no_secrets;
mod no_tainted_sink;
mod no_weak_crypto;

use crate::{BoxedLintRule, boxed};

pub use no_blank_target::*;
pub use no_hardcoded_ip::*;
pub use no_implied_eval::*;
pub use no_insecure_random::*;
pub use no_open_redirect::*;
pub use no_prototype_pollution::*;
pub use no_regex_injection::*;
pub use no_script_url::*;
pub use no_secrets::*;
pub use no_tainted_sink::*;
pub use no_weak_crypto::*;

/// Get all security rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(NoBlankTarget),
        boxed(NoHardcodedIp),
        boxed(NoImpliedEval),
        boxed(NoInsecureRandom),
        boxed(NoOpenRedirect),
        boxed(NoPrototypePollution),
        boxed(NoRegexInjection),
        boxed(NoScriptUrl),
        boxed(NoSecrets),
        boxed(NoTaintedSink),
        boxed(NoWeakCrypto),
    ]
}

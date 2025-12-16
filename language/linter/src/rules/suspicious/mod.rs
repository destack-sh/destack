mod no_cond_assign;
mod no_debugger;
mod no_empty;
mod no_useless_catch;
mod no_useless_computed_key;
mod no_useless_concat;
mod no_useless_constructor;
mod no_useless_escape;
mod no_useless_rename;
mod no_useless_return;

use crate::{BoxedLintRule, boxed};

pub use no_cond_assign::*;
pub use no_debugger::*;
pub use no_empty::*;
pub use no_useless_catch::*;
pub use no_useless_computed_key::*;
pub use no_useless_concat::*;
pub use no_useless_constructor::*;
pub use no_useless_escape::*;
pub use no_useless_rename::*;
pub use no_useless_return::*;

/// Get all suspicious rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(NoCondAssign),
        boxed(NoDebugger),
        boxed(NoEmpty),
        boxed(NoUselessCatch),
        boxed(NoUselessComputedKey),
        boxed(NoUselessConcat),
        boxed(NoUselessConstructor),
        boxed(NoUselessEscape),
        boxed(NoUselessRename),
        boxed(NoUselessReturn),
    ]
}

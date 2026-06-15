use destack_artifact::{ConditionSet, ProfileKey};
use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::Module;

/// A static evaluator over the load-known compiler environment.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StaticContext<'a> {
    /// The DIR view being evaluated.
    pub(crate) view: dir::View<'a>,
    /// The current module.
    pub(crate) module: &'a Module,
    /// The active profile key.
    pub(crate) profile: &'a ProfileKey,
    /// The active profile conditions.
    pub(crate) conditions: &'a ConditionSet,
    /// The shared string pool.
    pub(crate) strings: &'a StringPool,
}

impl<'a> StaticContext<'a> {
    /// Create a static context.
    pub(crate) fn new(
        view: dir::View<'a>,
        module: &'a Module,
        profile: &'a ProfileKey,
        conditions: &'a ConditionSet,
        strings: &'a StringPool,
    ) -> Self {
        Self {
            view,
            module,
            profile,
            conditions,
            strings,
        }
    }
}

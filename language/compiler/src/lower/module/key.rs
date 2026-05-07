use destack_core::StringId;
use destack_dir::{self as dir};

use crate::{LowerError, LowerResult};

use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Resolve a dispatchable member name from a key and signature role.
    pub(crate) fn member_dispatch_name_or_error(
        &self,
        key: Option<&dir::Key>,
        role: Option<dir::FunctionRole>,
        node: dir::LocalNodeIdAny,
    ) -> LowerResult<StringId> {
        if let Some(dir::Key::Name(name)) = key {
            return Ok(name.string());
        }

        match role {
            Some(dir::FunctionRole::Call) => Ok(self.dispatch_call_name),
            Some(dir::FunctionRole::Constructor | dir::FunctionRole::New) => {
                Ok(self.dispatch_construct_name)
            }
            _ => Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    node.into_global(self.module_id)
                        .into_anchored(Some(self.profile)),
                ),
                message: "method must have a static name".to_string(),
            }
            .into()),
        }
    }
}

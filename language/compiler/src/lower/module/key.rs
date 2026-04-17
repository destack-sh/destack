use destack_core::StringId;
use destack_dir::{self as dir};

use crate::{LowerError, LowerResult};

use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Resolve a dispatchable member name from a key and signature mode.
    pub(crate) fn member_dispatch_name_or_error(
        &self,
        key: Option<&dir::Key>,
        mode: Option<dir::FunctionMode>,
        node: dir::LocalNodeIdAny,
    ) -> LowerResult<StringId> {
        if let Some(dir::Key::Name(name)) = key {
            return Ok(name.string());
        }

        match mode {
            Some(dir::FunctionMode::Call) => Ok(self.dispatch_call_name),
            Some(dir::FunctionMode::Constructor | dir::FunctionMode::New) => {
                Ok(self.dispatch_construct_name)
            }
            _ => Err(LowerError::UnsupportedConstruct {
                node: node
                    .into_global(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "method must have a static name".to_string(),
            }),
        }
    }
}

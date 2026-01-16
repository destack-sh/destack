use destack_base::StringId;
use destack_dir::{DynamicKey, LocalNodeId, Member};

use crate::{LowerError, LowerResult};

use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Resolve a static member name from a dynamic key.
    pub(crate) fn member_name_or_error(
        &self,
        key: Option<&DynamicKey>,
        node: LocalNodeId<Member>,
    ) -> LowerResult<StringId> {
        // resolve the static member name
        let name = match key {
            Some(DynamicKey::Name(name)) => *name,
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: node
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "method must have a static name".to_string(),
                });
            }
        };

        // return the resolved name
        Ok(name)
    }
}

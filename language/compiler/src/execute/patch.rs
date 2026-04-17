use crate::{Compiler, ExecuteError};

use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::ProfileId;

use super::ComptimeOutput;

/// Patch information for a single comptime slot.
#[derive(Debug, Clone)]
pub(crate) struct ComptimePatch {
    /// The expression to replace.
    pub expression_id: dir::LocalNodeIdAny,
    /// The computed result.
    pub result: Option<ComptimeOutput>,
}

impl Compiler {
    /// Apply comptime results by patching the DIR in place.
    pub(crate) fn apply_comptime_patch(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        tree: &mut dir::NodeTree,
        types: &dir::TypeTable,
        patch: ComptimePatch,
    ) {
        // resolve the expression to patch
        let expression_id = patch
            .expression_id
            .try_into_typed::<dir::Expression>()
            .map_err(|_| ExecuteError::UnsupportedConstruct {
                node: patch
                    .expression_id
                    .into_global(module_id)
                    .into_anchored(Some(profile_id)),
            });
        let expression_id = match expression_id {
            Ok(expression_id) => expression_id,
            Err(error) => {
                self.error(error);
                return;
            }
        };

        // skip missing results
        let Some(output) = patch.result.as_ref() else {
            return;
        };

        // skip results that lack a dir value
        let Some(static_value) = output.dir.as_ref() else {
            self.error(ExecuteError::UnsupportedConstruct {
                node: patch
                    .expression_id
                    .into_global(module_id)
                    .into_anchored(Some(profile_id)),
            });
            return;
        };

        // build the replacement expression from the static value
        let scope = tree.get_scope(expression_id);
        let replacement = match self.static_expression_to_expression(
            tree,
            types,
            module_id,
            profile_id,
            expression_id.into_any(),
            expression_id.into_any(),
            scope,
            static_value,
        ) {
            Ok(replacement) => replacement,
            Err(error) => {
                self.error(error);
                return;
            }
        };
        tree.replace(expression_id, replacement);

        // attach a comment with the original comptime source
        if self.options.retain_comptime_as_comment {
            self.attach_comptime_comment(module_id, tree, expression_id);
        }
    }

    /// Attach a postfix comment with the original comptime source.
    fn attach_comptime_comment(
        &self,
        module_id: ModuleId,
        tree: &mut dir::NodeTree,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) {
        // comments are no longer representable as DIR annotations
        let _ = (module_id, tree, expression_id);
    }
}

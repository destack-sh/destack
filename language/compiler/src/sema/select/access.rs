use destack_dir as dir;

use crate::sema::CheckState;
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Commit one selected stable storage access.
    pub(in crate::sema) fn commit_access(
        &mut self,
        node: dir::GlobalNodeIdAny,
        path: dir::AccessPath,
    ) -> CompilerResult<()> {
        let resolution = dir::AccessResolution::new(path);
        let module = self.module(node.module_id);

        // accept identical selections and reject conflicting access identities
        if let Some(previous) = module.decisions.access_resolution(node) {
            if previous == &resolution {
                return Ok(());
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "check node {} selected conflicting value accesses: previous = {previous:?}, new = {resolution:?}",
                    self.node_label(node),
                ),
            });
        }

        self.module_mut(node.module_id)
            .decisions
            .set_access_resolution(node, resolution);

        Ok(())
    }

    /// Record one use of a selected stable storage access.
    pub(in crate::sema) fn record_access_use(
        &mut self,
        node: dir::GlobalNodeIdAny,
        uses: dir::BindingUse,
    ) {
        let Some(access) = self
            .module(node.module_id)
            .decisions
            .access_resolution(node)
        else {
            return;
        };
        let path = access.path().clone();
        let root = path.root();
        let root_uses = if path.keys().is_empty() {
            uses
        } else {
            uses.without(dir::BindingUse::WRITTEN)
        };
        let flows = &mut self.module_mut(node.module_id).flows;

        // record the exact access independently from its root binding
        flows.record_access_use(node.local_id, path, uses);

        // record symbol root uses without treating projected writes as reassignment
        if let dir::AccessRoot::Symbol(symbol) = root
            && !root_uses.is_empty()
        {
            flows.record_binding_use(node.local_id, symbol, root_uses);
        }
    }

    /// Commit one selected projection from a stable receiver access.
    pub(in crate::sema) fn commit_projected_access(
        &mut self,
        node: dir::GlobalNodeIdAny,
        receiver: dir::GlobalNodeIdAny,
        key: dir::StaticKey,
    ) -> CompilerResult<()> {
        let Some(receiver) = self
            .module(receiver.module_id)
            .decisions
            .access_resolution(receiver)
        else {
            return Ok(());
        };
        let mut path = receiver.path().clone();

        path.push(key);

        self.commit_access(node, path)
    }

    /// Commit one chain expression's selected access.
    pub(in crate::sema) fn commit_chain_access(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        let module = self.module(node.module_id);
        let expression = node.into_typed::<dir::Expression>();
        let dir::Expression::Chain { expression } = module.view().get(expression.local_id) else {
            return Err(CompilerError::Internal {
                message: format!("access chain node {node:?} is not a chain expression"),
            });
        };
        let resolution = module
            .decisions
            .access_resolution(expression.into_global_any(node.module_id))
            .cloned();

        // copy a stable operand access to the transparent chain node
        if let Some(resolution) = resolution {
            self.commit_access(node, resolution.path().clone())?;
        }

        Ok(())
    }
}

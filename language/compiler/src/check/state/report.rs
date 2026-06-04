use destack_artifact::{ArtifactEvent, ArtifactEventLog};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerError;
use crate::check::{CheckState, Dump, DumpContext, Origin, StaticOperand, TypeOperand};

impl CheckState<'_> {
    /// Return an internal error for one node type operand that cannot be committed.
    pub(in crate::check) fn unresolved_node_type_error(
        &self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
        operand: TypeOperand,
    ) -> CompilerError {
        self.unresolved_commit_operand_error(
            module,
            "node_type",
            "node",
            Origin::Node(node),
            &operand,
        )
    }

    /// Return an internal error for one symbol type operand that cannot be committed.
    pub(in crate::check) fn unresolved_symbol_type_error(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        operand: TypeOperand,
    ) -> CompilerError {
        self.unresolved_commit_operand_error(
            module,
            "symbol_type",
            "symbol",
            Origin::Symbol(symbol),
            &operand,
        )
    }

    /// Return an internal error for one symbol static operand that cannot be committed.
    pub(in crate::check) fn unresolved_symbol_static_error(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        operand: StaticOperand,
    ) -> CompilerError {
        self.unresolved_commit_operand_error(
            module,
            "symbol_static",
            "symbol",
            Origin::Symbol(symbol),
            &operand,
        )
    }

    /// Return an internal error for one operand that cannot be committed.
    fn unresolved_commit_operand_error<T: Dump + ?Sized>(
        &self,
        module: ModuleId,
        kind: &'static str,
        target_key: &'static str,
        target: Origin,
        operand: &T,
    ) -> CompilerError {
        let mut log = ArtifactEventLog::new();

        // record failing commit operand
        log.push(
            ArtifactEvent::new("commit.unresolved")
                .error()
                .text("kind", kind)
                .text("module", DumpContext::new(self).module_label(module))
                .text(target_key, self.dump_in_module(module, &target))
                .text("operand", self.dump_in_module(module, operand)),
        );

        let report = log.render_raw();
        let trace = self.trace.render_dump(self);

        CompilerError::Internal {
            message: format!(
                "check internal error
---------------
{report}
---------------
check trace
---------------
{trace}
---------------"
            ),
        }
    }
}

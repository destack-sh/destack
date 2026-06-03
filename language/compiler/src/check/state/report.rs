use destack_artifact::{ArtifactEvent, ArtifactEventLog};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, Dump, DumpContext, Origin, StaticOperand, TypeOperand};

impl CheckState<'_> {
    /// Panic for one node type operand that cannot be committed.
    #[track_caller]
    pub(in crate::check) fn panic_unresolved_node_type(
        &self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
        operand: TypeOperand,
    ) -> ! {
        self.panic_unresolved_commit_operand(
            module,
            "node_type",
            "node",
            Origin::Node(node),
            &operand,
        )
    }

    /// Panic for one symbol type operand that cannot be committed.
    #[track_caller]
    pub(in crate::check) fn panic_unresolved_symbol_type(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        operand: TypeOperand,
    ) -> ! {
        self.panic_unresolved_commit_operand(
            module,
            "symbol_type",
            "symbol",
            Origin::Symbol(symbol),
            &operand,
        )
    }

    /// Panic for one symbol static operand that cannot be committed.
    #[track_caller]
    pub(in crate::check) fn panic_unresolved_symbol_static(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        operand: StaticOperand,
    ) -> ! {
        self.panic_unresolved_commit_operand(
            module,
            "symbol_static",
            "symbol",
            Origin::Symbol(symbol),
            &operand,
        )
    }

    /// Panic for one operand that cannot be committed.
    #[track_caller]
    fn panic_unresolved_commit_operand<T: Dump + ?Sized>(
        &self,
        module: ModuleId,
        kind: &'static str,
        target_key: &'static str,
        target: Origin,
        operand: &T,
    ) -> ! {
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

        self.panic_check_report(log);
    }

    /// Panic with one structured check report and the current trace.
    #[track_caller]
    fn panic_check_report(&self, log: ArtifactEventLog) -> ! {
        let report = log.render_raw();
        let trace = self.trace.render_dump(self);

        panic!(
            "check crash
---------------
{report}
---------------
check trace
---------------
{trace}
---------------"
        );
    }
}

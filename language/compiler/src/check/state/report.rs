use destack_artifact::{ArtifactEvent, ArtifactEventLog};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, Dump, DumpContext, Origin, StaticOperand, TypeOperand};

impl CheckState<'_> {
    /// Panic for one checked node type that cannot be committed.
    #[track_caller]
    pub(in crate::check) fn panic_unresolved_checked_node_type(
        &self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
        operand: TypeOperand,
    ) -> ! {
        self.panic_unresolved_checked_output(
            module,
            "node_type",
            "node",
            Origin::Node(node),
            &operand,
        )
    }

    /// Panic for one checked symbol type that cannot be committed.
    #[track_caller]
    pub(in crate::check) fn panic_unresolved_checked_symbol_type(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        operand: TypeOperand,
    ) -> ! {
        self.panic_unresolved_checked_output(
            module,
            "symbol_type",
            "symbol",
            Origin::Symbol(symbol),
            &operand,
        )
    }

    /// Panic for one checked symbol static that cannot be committed.
    #[track_caller]
    pub(in crate::check) fn panic_unresolved_checked_symbol_static(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        operand: StaticOperand,
    ) -> ! {
        self.panic_unresolved_checked_output(
            module,
            "symbol_static",
            "symbol",
            Origin::Symbol(symbol),
            &operand,
        )
    }

    /// Panic for one checked output that cannot be committed.
    #[track_caller]
    fn panic_unresolved_checked_output<T: Dump + ?Sized>(
        &self,
        module: ModuleId,
        kind: &'static str,
        target_key: &'static str,
        target: Origin,
        operand: &T,
    ) -> ! {
        let mut log = ArtifactEventLog::new();

        // record failing output
        log.push(
            ArtifactEvent::new("output.fail")
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
            "check crash (oh no)
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

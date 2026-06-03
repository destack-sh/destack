use destack_mir as mir;
use destack_source::{DiffOptions, print_diff};

use crate::tests::TestProgram;
use crate::verify::{BorrowObligationRecord, DropInsert, OwnershipCheck, VerifyError, VerifyState};

impl TestProgram {
    /// Run ownership verification.
    pub(super) fn run_ownership(&mut self) -> Vec<VerifyError> {
        let mut state = VerifyState::new(
            self.module_id(),
            self.profile_id(),
            self.target_id(),
            &self.provider,
        );
        OwnershipCheck.run(&mut self.tree, &mut state);

        collect_errors(&state)
    }

    /// Run ownership verification and return borrow obligations.
    pub(super) fn run_ownership_obligations(
        &mut self,
    ) -> (Vec<VerifyError>, Vec<BorrowObligationRecord>) {
        let mut state = VerifyState::new(
            self.module_id(),
            self.profile_id(),
            self.target_id(),
            &self.provider,
        );
        OwnershipCheck.run(&mut self.tree, &mut state);

        (collect_errors(&state), state.borrow_obligations().to_vec())
    }

    /// Run drop insertion.
    pub(super) fn run_drop(&mut self) -> String {
        let mut state = VerifyState::new(
            self.module_id(),
            self.profile_id(),
            self.target_id(),
            &self.provider,
        );
        DropInsert.run(&mut self.tree, &mut state);

        mir::format_mir(&self.tree, &self.strings, mir::MirFormatOptions::default())
            .expect("format MIR")
    }

    /// Assert the MIR produced by drop insertion.
    #[track_caller]
    pub(super) fn assert_dropped_mir(&mut self, expected: &str) {
        let actual = self.run_drop();

        if actual != expected {
            print_diff(
                expected,
                &actual,
                &DiffOptions::new().with_path("verify/drop.mir"),
            );
            panic!("dropped MIR mismatch");
        }
    }

    /// Return the type id with the given display name.
    #[track_caller]
    pub(super) fn type_by_name(&self, name: &str) -> mir::LocalNodeId<mir::Type> {
        self.tree
            .iter_nodes::<mir::Type>()
            .find_map(|(id, _)| {
                let display_name = self.tree.type_display_name(id)?;
                (self.strings.get(display_name) == name).then_some(id)
            })
            .unwrap_or_else(|| panic!("missing MIR type {name}"))
    }

    /// Mark one type as having dynamic drop glue.
    pub(super) fn mark_dynamic_drop(&mut self, name: &str) {
        let ty = self.type_by_name(name);
        let glue = mir::DropGlue::Dynamic {
            slot: mir::DispatchSlot::new(0),
        };

        self.tree.metadata.drop.set_drop_glue(ty, glue);
    }
}

/// Collect typed verify errors.
fn collect_errors(state: &VerifyState<'_>) -> Vec<VerifyError> {
    state
        .errors()
        .iter()
        .map(|diagnostic| diagnostic.diagnostic().clone())
        .collect()
}

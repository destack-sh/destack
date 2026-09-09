use std::sync::Arc;

use destack_mir as mir;

use crate::elaborate::ElaborateState;
use crate::instantiate::Instantiated;
use crate::tests::{TestProgram, assert_snapshot};
use crate::verify::VerifyState;

impl TestProgram {
    /// Run MIR elaboration.
    pub(in crate::elaborate::tests) fn elaborate(&mut self) {
        let mut state = VerifyState::new(&self.lowered);
        state.verify();

        let errors = state.take_errors();
        assert!(
            errors.is_empty(),
            "elaboration input has ownership errors: {errors:?}"
        );
        let retention = state.take_retention();
        let safepoints = state.take_safepoints();
        let mut state = ElaborateState::new(
            Instantiated {
                module: self.module_id(),
                layout: self.lowered.target,
                tree: mir::Tree::clone(&self.lowered.tree),
                layouts: self.lowered.layouts.clone(),
                drops: self.lowered.drops.clone(),
                accesses: self.lowered.accesses.clone(),
                effects: self.lowered.effects.clone(),
                specializations: Vec::new(),
            },
            &self.strings,
        );
        state
            .elaborate(&retention, &safepoints)
            .expect("elaboration layouts should build");
        let elaborated = state.finish();
        self.lowered.tree = Arc::new(elaborated.tree);
        self.lowered.layouts = elaborated.layouts;
        self.lowered.drops = elaborated.drops;
        self.lowered.effects = elaborated.effects;
    }

    /// Assert the MIR produced by elaboration.
    #[track_caller]
    pub(in crate::elaborate::tests) fn assert_elaborated(&mut self, expected: &str) {
        self.elaborate();
        let actual = mir::Formatter::new(
            &self.lowered.tree,
            self.lowered.target,
            &self.strings,
            mir::FormatOptions::default(),
        )
        .format()
        .expect("format MIR");

        assert_snapshot(actual, expected);
    }
}

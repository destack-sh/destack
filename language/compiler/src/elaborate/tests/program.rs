use std::sync::Arc;

use destack_mir as mir;

use crate::elaborate::ElaborateState;
use crate::optimize::optimize;
use crate::tests::{TestProgram, assert_snapshot};
use crate::verify::VerifyState;
use destack_artifact::MirInstantiated;

impl TestProgram {
    /// Elaborate MIR and run the optimization pipeline.
    pub(in crate::elaborate::tests) fn optimize(&mut self) {
        let mut state = VerifyState::new(&self.lowered);
        state.verify();

        let errors = state.take_errors();
        assert!(
            errors.is_empty(),
            "elaboration input has ownership errors: {errors:?}"
        );
        let retention = state.take_retention();
        let mut state = ElaborateState::new(
            self.module_id(),
            &MirInstantiated {
                target: self.lowered.target,
                initializer: self.lowered.initializer,
                tree: self.lowered.tree.clone(),
                layouts: self.lowered.layouts.clone(),
                dispatch: self.lowered.dispatch.clone(),
                drops: self.lowered.drops.clone(),
                effects: self.lowered.effects.clone(),
                profile: self.lowered.profile.clone(),
                retention: retention.clone(),
            },
            &self.strings,
        );
        state
            .elaborate(&retention)
            .expect("elaboration layouts should build");
        let elaborated = state.finish();
        let optimized = optimize(&elaborated).expect("runtime operations should be inserted");
        self.lowered.tree = Arc::new(optimized.tree);
        self.lowered.layouts = optimized.layouts;
        self.lowered.drops = optimized.drops;
        self.lowered.effects = optimized.effects;
    }

    /// Assert the MIR produced by elaboration and optimization.
    #[track_caller]
    pub(in crate::elaborate::tests) fn assert_optimized(&mut self, expected: &str) {
        self.optimize();
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

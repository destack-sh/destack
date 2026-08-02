use destack_artifact::{MirElaborated, MirLowered};
use destack_mir as mir;

use crate::elaborate::ElaborateState;
use crate::tests::{TestProgram, assert_snapshot};

impl TestProgram {
    /// Run MIR elaboration.
    pub(in crate::elaborate::tests) fn run_elaborate(&mut self) -> String {
        let lowered = std::mem::take(&mut self.lowered);
        let mut state = ElaborateState::new(lowered, &self.strings);
        state.generate_destructors();
        state.insert_drops();
        self.store_elaborated(state.finish());

        mir::format_mir(
            &self.lowered.tree,
            self.lowered.target,
            &self.strings,
            mir::MirFormatOptions::default(),
        )
        .expect("format MIR")
    }

    /// Assert the MIR produced by elaboration.
    #[track_caller]
    pub(in crate::elaborate::tests) fn assert_elaborated_mir(&mut self, expected: &str) {
        let actual = self.run_elaborate();

        assert_snapshot(actual, expected);
    }

    /// Assert the MIR produced by drop elaboration.
    #[track_caller]
    pub(in crate::elaborate::tests) fn assert_drop_mir(&mut self, expected: &str) {
        self.assert_elaborated_mir(expected);
    }

    /// Store elaborated MIR as the next lowered test artifact.
    fn store_elaborated(&mut self, elaborated: MirElaborated) {
        self.lowered = MirLowered {
            tree: elaborated.tree,
            target: elaborated.target,
            types: elaborated.types,
            layouts: elaborated.layouts,
            dispatch: elaborated.dispatch,
            drops: elaborated.drops,
            memory: elaborated.memory,
            effects: elaborated.effects,
            profile: elaborated.profile,
            initializer: None,
        };
    }
}

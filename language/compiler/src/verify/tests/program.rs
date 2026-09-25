use std::sync::{Arc, Mutex};

use destack_artifact::{DiagnosticBuilder, ToDiagnostic};
use destack_source::{DiagnosticCollection, FileId, PrintOptions, print_diagnostics};

use crate::CompilerError;
use crate::tests::TestProgram;
use crate::tests::snapshot::assert_snapshot;
use crate::verify::{VerifyError, VerifyState};

impl TestProgram {
    /// Run MIR verification.
    pub(in crate::verify) fn verify(&mut self) -> Vec<DiagnosticBuilder<VerifyError>> {
        let mut state = VerifyState::new(&self.lowered, &self.strings);
        state.verify();
        if let Some(error) = state.take_invalid_mir() {
            panic!("verified MIR is invalid: {error:?}");
        }

        state.take_errors()
    }

    /// Assert the complete MIR invariant violation verification reports.
    #[track_caller]
    pub(in crate::verify) fn assert_invalid_mir(&mut self, expected: &str) {
        let mut state = VerifyState::new(&self.lowered, &self.strings);
        state.verify();
        let Some(CompilerError::Internal { message }) = state.take_invalid_mir() else {
            panic!("verified MIR is valid");
        };

        assert_snapshot(message, expected);
    }

    /// Assert that MIR verification succeeds.
    #[track_caller]
    pub(in crate::verify) fn assert_verified(&mut self) {
        let diagnostics = self.verify();

        if !diagnostics.is_empty() {
            panic!("{}", self.render_verify_diagnostics(&diagnostics));
        }
    }

    /// Assert the complete MIR verification diagnostics.
    #[track_caller]
    pub(in crate::verify) fn assert_verify_errors(&mut self, expected: &str) {
        assert!(
            !expected.is_empty(),
            "an empty expectation is written as the blessable placeholder r#\"\\n\"#"
        );
        let diagnostics = self.verify();
        let actual = self.render_verify_diagnostics(&diagnostics);

        assert_snapshot(actual, expected);
    }

    /// Render verify diagnostics through the standard source printer.
    fn render_verify_diagnostics(&self, diagnostics: &[DiagnosticBuilder<VerifyError>]) -> String {
        // finalize provider diagnostics
        let diagnostics = diagnostics
            .iter()
            .map(|diagnostic| {
                diagnostic
                    .to_diagnostic(&self.provider)
                    .expect("render verify diagnostic")
            })
            .collect();
        let diagnostics = DiagnosticCollection::from_diagnostics(diagnostics);

        // capture printer output
        let lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let output = lines.clone();
        let writer = Arc::new(move |line: &str| {
            output
                .lock()
                .expect("lock diagnostic output")
                .push(line.to_string());
        });

        // resolve raw MIR file ids
        let file = self.provider.file.clone();
        let file_for_id = move |file_id: FileId| {
            if file_id == file.id {
                Some(file.clone())
            } else {
                None
            }
        };

        // print exactly like normal diagnostics
        let options = PrintOptions::new()
            .with_color(false)
            .with_skip_summary(true)
            .with_line_writer(writer);
        print_diagnostics(&file_for_id, &diagnostics, options).expect("print verify diagnostics");

        lines.lock().expect("lock diagnostic output").join("\n")
    }
}

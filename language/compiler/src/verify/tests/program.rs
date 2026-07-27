use std::sync::{Arc, Mutex};

use destack_artifact::{DiagnosticBuilder, ToDiagnostic};
use destack_source::{DiagnosticCollection, FileId, PrintOptions, print_diagnostics};

use crate::DiagnosticAnchor;
use crate::tests::TestProgram;
use crate::verify::{VerifyError, VerifyState};

impl TestProgram {
    /// Run ownership verification.
    pub(in crate::verify::tests) fn run_ownership(
        &mut self,
    ) -> Vec<DiagnosticBuilder<VerifyError>> {
        let mut state = VerifyState::new(
            self.module_id(),
            self.profile_id(),
            self.target_id(),
            &self.provider,
            &self.lowered,
        );
        state.check_ownership();

        state.errors().to_vec()
    }

    /// Assert no ownership errors.
    pub(in crate::verify::tests) fn assert_no_ownership_errors(&mut self) {
        let diagnostics = self.run_ownership();

        if !diagnostics.is_empty() {
            panic!("{}", self.render_verify_diagnostics(&diagnostics));
        }
    }

    /// Assert one definite use after move error.
    pub(in crate::verify::tests) fn assert_error_use_after_move(&mut self) {
        let diagnostics = self.run_ownership();
        let VerifyError::UseAfterMove { anchor, moved_at } = self.one_ownership_error(&diagnostics)
        else {
            panic!("{}", self.render_verify_diagnostics(&diagnostics));
        };

        self.assert_anchor_before(moved_at, anchor);
    }

    /// Assert one maybe use after move error.
    pub(in crate::verify::tests) fn assert_error_maybe_use_after_move(&mut self) {
        let diagnostics = self.run_ownership();
        let VerifyError::MaybeUseAfterMove { anchor, moved_at } =
            self.one_ownership_error(&diagnostics)
        else {
            panic!("{}", self.render_verify_diagnostics(&diagnostics));
        };

        self.assert_anchor_before(moved_at, anchor);
    }

    /// Assert one borrow conflict error.
    pub(in crate::verify::tests) fn assert_error_borrow_conflict(&mut self) {
        let diagnostics = self.run_ownership();
        let VerifyError::BorrowConflict {
            anchor,
            active_borrow,
        } = self.one_ownership_error(&diagnostics)
        else {
            panic!("{}", self.render_verify_diagnostics(&diagnostics));
        };

        self.assert_anchor_before(active_borrow, anchor);
    }

    /// Assert one borrowed-place invalidation error.
    pub(in crate::verify::tests) fn assert_error_invalidation_of_borrowed_place(&mut self) {
        let diagnostics = self.run_ownership();
        let VerifyError::InvalidationOfBorrowedPlace {
            anchor,
            borrowed_at,
        } = self.one_ownership_error(&diagnostics)
        else {
            panic!("{}", self.render_verify_diagnostics(&diagnostics));
        };

        self.assert_anchor_before(borrowed_at, anchor);
    }

    /// Assert one write through readonly reference error.
    pub(in crate::verify::tests) fn assert_error_write_through_readonly_reference(&mut self) {
        let diagnostics = self.run_ownership();
        let VerifyError::WriteThroughReadonlyReference { anchor } =
            self.one_ownership_error(&diagnostics)
        else {
            panic!("{}", self.render_verify_diagnostics(&diagnostics));
        };

        assert!(matches!(anchor, DiagnosticAnchor::Span(_)), "{anchor:#?}");
    }

    /// Assert one exclusive shared managed borrow error.
    pub(in crate::verify::tests) fn assert_error_exclusive_borrow_from_shared_managed(&mut self) {
        let diagnostics = self.run_ownership();
        let VerifyError::ExclusiveBorrowFromSharedManaged { anchor } =
            self.one_ownership_error(&diagnostics)
        else {
            panic!("{}", self.render_verify_diagnostics(&diagnostics));
        };

        assert!(matches!(anchor, DiagnosticAnchor::Span(_)), "{anchor:#?}");
    }

    /// Assert one borrow outlives origin error.
    pub(in crate::verify::tests) fn assert_error_borrow_outlives_origin(&mut self) {
        let diagnostics = self.run_ownership();
        let VerifyError::BorrowOutlivesOrigin { anchor } = self.one_ownership_error(&diagnostics)
        else {
            panic!("{}", self.render_verify_diagnostics(&diagnostics));
        };

        assert!(matches!(anchor, DiagnosticAnchor::Span(_)), "{anchor:#?}");
    }

    /// Assert one incomplete aggregate move error.
    pub(in crate::verify::tests) fn assert_error_partial_move(&mut self) {
        let diagnostics = self.run_ownership();
        let VerifyError::PartialMove { anchor, moved_at } = self.one_ownership_error(&diagnostics)
        else {
            panic!("{}", self.render_verify_diagnostics(&diagnostics));
        };

        assert!(matches!(anchor, DiagnosticAnchor::Span(_)), "{anchor:#?}");
        assert!(
            matches!(moved_at, DiagnosticAnchor::Span(_)),
            "{moved_at:#?}"
        );
    }

    /// Assert one move out of a type with a Drop hook.
    pub(in crate::verify::tests) fn assert_error_move_out_of_drop(&mut self) {
        let diagnostics = self.run_ownership();
        let VerifyError::MoveOutOfDrop { anchor } = self.one_ownership_error(&diagnostics) else {
            panic!("{}", self.render_verify_diagnostics(&diagnostics));
        };

        assert!(matches!(anchor, DiagnosticAnchor::Span(_)), "{anchor:#?}");
    }

    /// Return the only ownership error.
    fn one_ownership_error<'a>(
        &self,
        diagnostics: &'a [DiagnosticBuilder<VerifyError>],
    ) -> &'a VerifyError {
        if diagnostics.is_empty() {
            panic!("expected one ownership diagnostic, got none");
        }

        if diagnostics.len() != 1 {
            panic!("{}", self.render_verify_diagnostics(diagnostics));
        }

        diagnostics[0].diagnostic()
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
        let output = Arc::clone(&lines);
        let writer = Arc::new(move |line: &str| {
            output
                .lock()
                .expect("lock diagnostic output")
                .push(line.to_string());
        });

        // resolve raw MIR file ids
        let file = Arc::clone(&self.provider.file);
        let file_for_id = move |file_id: FileId| {
            if file_id == file.id {
                Some(Arc::clone(&file))
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

    /// Return the byte start for one diagnostic anchor.
    fn anchor_start(&self, anchor: &DiagnosticAnchor) -> u32 {
        let DiagnosticAnchor::Span(span) = anchor else {
            panic!("expected span anchor, got {anchor:#?}");
        };

        span.start
    }

    /// Assert that `first` points before `second`.
    fn assert_anchor_before(&self, first: &DiagnosticAnchor, second: &DiagnosticAnchor) {
        assert!(
            self.anchor_start(first) < self.anchor_start(second),
            "expected {first:#?} before {second:#?}",
        );
    }
}

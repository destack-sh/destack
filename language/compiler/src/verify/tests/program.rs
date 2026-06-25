use std::sync::{Arc, Mutex};

use destack_artifact::{DiagnosticBuilder, ToDiagnostic};
use destack_mir as mir;
use destack_source::{
    DiagnosticCollection, DiffOptions, FileId, PrintOptions, print_diagnostics, print_diff,
};

use crate::DiagnosticAnchor;
use crate::tests::TestProgram;
use crate::verify::{VerifyError, VerifyState};

impl TestProgram {
    /// Run ownership verification.
    pub(in crate::verify::tests) fn run_ownership(
        &mut self,
    ) -> Vec<DiagnosticBuilder<VerifyError>> {
        let tree = std::mem::take(&mut self.tree);
        let mut state = VerifyState::new(
            self.module_id(),
            self.profile_id(),
            self.target_id(),
            &self.provider,
            tree,
            &self.strings,
        );
        state.check_ownership();
        let diagnostics = state.errors().to_vec();
        self.tree = state.tree;

        diagnostics
    }

    /// Run drop insertion.
    pub(in crate::verify::tests) fn run_drop_phase(&mut self) -> String {
        let tree = std::mem::take(&mut self.tree);
        let mut state = VerifyState::new(
            self.module_id(),
            self.profile_id(),
            self.target_id(),
            &self.provider,
            tree,
            &self.strings,
        );
        if !state.has_errors() {
            state.generate_drop_glue();
            state.insert_drops();
        }
        self.tree = state.tree;

        mir::format_mir(&self.tree, &self.strings, mir::MirFormatOptions::default())
            .expect("format MIR")
    }

    /// Run full MIR verification.
    pub(in crate::verify::tests) fn run_verify(&mut self) -> String {
        let tree = std::mem::take(&mut self.tree);
        let mut state = VerifyState::new(
            self.module_id(),
            self.profile_id(),
            self.target_id(),
            &self.provider,
            tree,
            &self.strings,
        );
        state.check_ownership();
        if !state.has_errors() {
            state.generate_drop_glue();
            state.insert_drops();
        }
        self.tree = state.tree;

        mir::format_mir(&self.tree, &self.strings, mir::MirFormatOptions::default())
            .expect("format MIR")
    }

    /// Assert the MIR produced by drop insertion.
    #[track_caller]
    pub(in crate::verify::tests) fn assert_drop_mir(&mut self, expected: &str) {
        let actual = self.run_drop_phase();
        let expected = expected.trim();
        let actual = actual.trim();

        if actual != expected {
            print_diff(
                expected,
                actual,
                &DiffOptions::new().with_path("verify/drop.mir"),
            );
            panic!("dropped MIR mismatch");
        }
    }

    /// Assert the MIR produced by full verification.
    #[track_caller]
    pub(in crate::verify::tests) fn assert_verified_mir(&mut self, expected: &str) {
        let actual = self.run_verify();
        let expected = expected.trim();
        let actual = actual.trim();

        if actual != expected {
            print_diff(
                expected,
                actual,
                &DiffOptions::new().with_path("verify/verified.mir"),
            );
            panic!("verified MIR mismatch");
        }
    }

    /// Return the type id with the given display name.
    #[track_caller]
    pub(in crate::verify::tests) fn type_by_name(&self, name: &str) -> mir::LocalNodeId<mir::Type> {
        self.tree
            .iter_nodes::<mir::Type>()
            .find_map(|(id, _)| {
                let display_name = self.tree.type_display_name(id)?;
                (self.strings.get(display_name) == name).then_some(id)
            })
            .unwrap_or_else(|| panic!("missing MIR type {name}"))
    }

    /// Mark a dynamic value type as having dynamic drop glue.
    #[track_caller]
    pub(in crate::verify::tests) fn mark_dynamic_drop_for_constraint(&mut self, name: &str) {
        let constraint = self.type_by_name(name);
        let ty = self
            .tree
            .iter_nodes::<mir::Type>()
            .find_map(|(id, ty)| match ty {
                mir::Type::Dynamic { constraint: found } if *found == constraint => Some(id),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing dynamic MIR type for {name}"));
        let glue = mir::DropGlue::Dynamic {
            slot: mir::DispatchSlot::new(0),
        };

        self.tree.metadata.drops.set_drop_glue(ty, glue);
    }

    /// Mark one type as having a custom drop hook.
    #[track_caller]
    pub(in crate::verify::tests) fn mark_drop_hook(&mut self, name: &str, function_name: &str) {
        let ty = self.type_by_name(name);
        let function = self
            .tree
            .iter_nodes::<mir::Function>()
            .find_map(|(id, function)| {
                (self.strings.get(function.name) == function_name).then_some(id)
            })
            .unwrap_or_else(|| panic!("missing MIR function {function_name}"));
        let hook = mir::DropHook { function };

        self.tree.metadata.drops.set_drop_hook(ty, hook);
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

    /// Assert one managed borrow across suspension error.
    pub(in crate::verify::tests) fn assert_error_managed_borrow_across_suspension(&mut self) {
        let diagnostics = self.run_ownership();
        let VerifyError::ManagedBorrowAcrossSuspension {
            anchor,
            borrowed_at,
        } = self.one_ownership_error(&diagnostics)
        else {
            panic!("{}", self.render_verify_diagnostics(&diagnostics));
        };

        assert!(matches!(anchor, DiagnosticAnchor::Span(_)), "{anchor:#?}");
        assert!(
            matches!(borrowed_at, DiagnosticAnchor::Span(_)),
            "{borrowed_at:#?}"
        );
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

    /// Assert one undeclared borrow obligation error.
    pub(in crate::verify::tests) fn assert_error_undeclared_borrow_obligation(&mut self) {
        let diagnostics = self.run_ownership();
        let VerifyError::UndeclaredBorrowObligation { anchor } =
            self.one_ownership_error(&diagnostics)
        else {
            panic!("{}", self.render_verify_diagnostics(&diagnostics));
        };

        assert!(matches!(anchor, DiagnosticAnchor::Span(_)), "{anchor:#?}");
    }

    /// Assert one partial move of custom drop type error.
    pub(in crate::verify::tests) fn assert_error_partial_move_of_custom_drop(&mut self) {
        let diagnostics = self.run_ownership();
        let VerifyError::PartialMoveOfCustomDrop { anchor } =
            self.one_ownership_error(&diagnostics)
        else {
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

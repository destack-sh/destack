use destack_dir as dir;
use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Replace element-by-element copy loops with a bulk copy operation.
    pub MANUAL_COPY {
        id: "manual-copy",
        summary: "Replace element-by-element copy loops with a bulk copy operation",
        explanation: r#"
Copying corresponding elements one at a time repeats bounds checks and hides a contiguous copy.
Instead, you SHOULD use the collection's bulk copy operation.
"#,
        example: {
            reported: r#"
function copy(target: &exclusive int32[], source: &readonly int32[]): void {
    for (let index: isize = 0; index < source.length; index++) {
        target[index] = source[index];
    }
}
"#,
            accepted: r#"
function copy(target: &exclusive int32[], source: &readonly int32[]): void {
    target.view(0, source.length).copyFrom(source.view(0));
}
"#,
        },
        provenance: [Clippy("manual_memcpy")],
        category: Performance,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// One increasing bounded index loop.
#[derive(Debug, Clone, Copy)]
struct IndexLoop {
    /// The index binding.
    index: dir::GlobalSymbolId,
    /// The first yielded index.
    start: i64,
    /// The loop body.
    body: dir::LocalNodeId<dir::Block>,
}

/// One collection access at a constant offset from a loop index.
#[derive(Debug, Clone, Copy)]
struct IndexedAccess {
    /// The indexed collection.
    collection: dir::LocalNodeId<dir::Expression>,
    /// The constant added to the loop index.
    offset: i64,
}

impl IndexLoop {
    /// Select one increasing bounded index loop.
    fn select(
        module: &DirModule<'_>,
        expression: dir::LocalNodeId<dir::Expression>,
        occurrences: &[dir::BindingOccurrence],
    ) -> Result<Option<Self>, ProviderError> {
        let Some(iteration) = module.counted_iteration(expression)? else {
            return Ok(None);
        };
        let Some(index) = iteration.binding else {
            return Ok(None);
        };
        if iteration.start < 0 {
            return Ok(None);
        }

        // require the upper bound to remain independent from the counter
        let end_uses = module.binding_uses_within(index, iteration.end.into_any(), occurrences);
        if !end_uses.is_empty() {
            return Ok(None);
        }

        // require repeatedly evaluated bounds to remain stable
        if iteration.is_end_rechecked {
            let is_length = module.length_receiver(iteration.end)?.is_some();
            if !is_length && !module.is_speculatable_expression(iteration.end)? {
                return Ok(None);
            }
        }

        Ok(Some(Self {
            index,
            start: iteration.start,
            body: iteration.body,
        }))
    }

    /// Return whether every body expression copies corresponding elements.
    fn copies_only(self, module: &DirModule<'_>) -> Result<bool, ProviderError> {
        let mut actions = module.view().get(self.body).iter_expressions();
        let Some(first) = actions.next() else {
            return Ok(false);
        };
        if !self.copies(module, first)? {
            return Ok(false);
        }

        // reject any remaining noncopying action
        for action in actions {
            if !self.copies(module, action)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return whether one action copies corresponding elements.
    fn copies(
        self,
        module: &DirModule<'_>,
        action: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        // require one plain assignment between indexed elements
        let Some(assignment) = module.place_assignment(action) else {
            return Ok(false);
        };
        if assignment.operator != dir::AssignOperator::Assign {
            return Ok(false);
        }
        let Some(target) = IndexedAccess::select(module, assignment.target, self.index)? else {
            return Ok(false);
        };
        let Some(source) = IndexedAccess::select(module, assignment.value, self.index)? else {
            return Ok(false);
        };
        let Some(target_start) = self.start.checked_add(target.offset) else {
            return Ok(false);
        };
        let Some(source_start) = self.start.checked_add(source.offset) else {
            return Ok(false);
        };
        if target_start < 0
            || source_start < 0
            || module.is_same_computation(target.collection, source.collection)?
        {
            return Ok(false);
        }

        // require equal elements in canonical contiguous collections
        let target_type = module.node_type_id(assignment.target.into_any())?;
        let source_type = module.node_type_id(assignment.value.into_any())?;
        let is_target_contiguous = is_contiguous_collection(module, target.collection)?;
        let is_source_contiguous = is_contiguous_collection(module, source.collection)?;
        if !is_target_contiguous
            || !is_source_contiguous
            || !module.dir.types_match(target_type, source_type)?
        {
            return Ok(false);
        }

        // require ownership or exclusive borrowing to rule out target aliases
        let target_type = module.node_type_id(target.collection.into_any())?;
        let ownership = module.dir.default_ownership(target_type)?;
        let has_unique_target = ownership == Some(dir::Ownership::Owned)
            || ownership == Some(dir::Ownership::Borrowed)
                && module.dir.borrow_access(target_type)? == Some(dir::Access::Exclusive);

        Ok(has_unique_target)
    }
}

impl IndexedAccess {
    /// Select one collection indexed at a constant offset from a binding.
    fn select(
        module: &DirModule<'_>,
        expression: dir::LocalNodeId<dir::Expression>,
        index: dir::GlobalSymbolId,
    ) -> Result<Option<Self>, ProviderError> {
        let dir::Expression::Index {
            left,
            index: Some(selected),
            is_optional: false,
            ..
        } = module.view().get(expression)
        else {
            return Ok(None);
        };
        let Some(offset) = Self::offset(module, *selected, index)? else {
            return Ok(None);
        };

        Ok(Some(Self {
            collection: *left,
            offset,
        }))
    }

    /// Return the constant offset from one selected loop index.
    fn offset(
        module: &DirModule<'_>,
        expression: dir::LocalNodeId<dir::Expression>,
        index: dir::GlobalSymbolId,
    ) -> Result<Option<i64>, ProviderError> {
        // recognize the direct loop index
        if module.selected_symbol(expression)? == Some(index) {
            return Ok(Some(0));
        }

        // recognize checked addition with one constant operand
        let Some((operator, [left, right])) = module.integral_binary(expression)? else {
            return Ok(None);
        };
        let offset = match operator {
            dir::BinaryOperator::Add if module.selected_symbol(left)? == Some(index) => {
                module.integral_constant(right)?
            }
            dir::BinaryOperator::Add if module.selected_symbol(right)? == Some(index) => {
                module.integral_constant(left)?
            }
            dir::BinaryOperator::Subtract if module.selected_symbol(left)? == Some(index) => {
                module.integral_constant(right)?.map(|offset| -offset)
            }
            _ => None,
        };

        Ok(offset)
    }
}

/// Report complete index loops that copy corresponding collection elements.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect increasing bounded index loops
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some(index_loop) = IndexLoop::select(module, expression, &occurrences)? else {
            continue;
        };
        if !index_loop.copies_only(module)? {
            continue;
        }

        // report the complete copy loop
        let span = module.source_extent(expression.into_any())?;
        let diagnostic =
            lint.diagnostic("index loop copies corresponding collection elements", span);
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return whether one expression denotes an array or slice.
fn is_contiguous_collection(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, ProviderError> {
    if !module.is_speculatable_expression(expression)? {
        return Ok(false);
    }
    let ty = module.node_type_id(expression.into_any())?;
    let item = module.dir.representation_item(ty)?;

    Ok(matches!(
        item,
        Some(dir::LanguageItem::Array | dir::LanguageItem::Slice)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a complete element copy loop.
    #[test]
    fn test_reports_array_copy_loop() {
        let session = TestSession::dir(
            &MANUAL_COPY,
            r#"
function copy(target: &exclusive int32[], source: &readonly int32[]): void {
    for (let index: isize = 0; index < source.length; index++) {
        target[index] = source[index];
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-copy]: index loop copies corresponding collection elements
 ──▶ main.ds:2:5
  │
1 │ function copy(target: &exclusive int32[], source: &readonly int32[]): void {
2 │     for (let index: isize = 0; index < source.length; index++) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         target[index] = source[index];
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
4 │     }
  │     ^
5 │ }
  │
"#,
        );
    }

    /// Report an element copy over an authored index range.
    #[test]
    fn test_reports_range_copy_loop() {
        let session = TestSession::dir(
            &MANUAL_COPY,
            r#"
function copy(target: &exclusive int32[], source: &readonly int32[]): void {
    for (const index of 0..source.length) {
        target[index] = source[index];
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-copy]: index loop copies corresponding collection elements
 ──▶ main.ds:2:5
  │
1 │ function copy(target: &exclusive int32[], source: &readonly int32[]): void {
2 │     for (const index of 0..source.length) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         target[index] = source[index];
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
4 │     }
  │     ^
5 │ }
  │
"#,
        );
    }

    /// Report a copy into a shifted target range.
    #[test]
    fn test_reports_shifted_target() {
        let session = TestSession::dir(
            &MANUAL_COPY,
            r#"
function copy(target: &exclusive int32[], source: &readonly int32[]): void {
    for (let index: isize = 0; index < source.length; index++) {
        target[index + 1] = source[index];
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-copy]: index loop copies corresponding collection elements
 ──▶ main.ds:2:5
  │
1 │ function copy(target: &exclusive int32[], source: &readonly int32[]): void {
2 │     for (let index: isize = 0; index < source.length; index++) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         target[index + 1] = source[index];
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
4 │     }
  │     ^
5 │ }
  │
"#,
        );
    }

    /// Report copying the remainder of a source after a nonzero index.
    #[test]
    fn test_reports_nonzero_start() {
        let session = TestSession::dir(
            &MANUAL_COPY,
            r#"
function copy(target: &exclusive int32[], source: &readonly int32[]): void {
    for (let index: isize = 2; index < source.length; index++) {
        target[index] = source[index];
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-copy]: index loop copies corresponding collection elements
 ──▶ main.ds:2:5
  │
1 │ function copy(target: &exclusive int32[], source: &readonly int32[]): void {
2 │     for (let index: isize = 2; index < source.length; index++) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         target[index] = source[index];
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
4 │     }
  │     ^
5 │ }
  │
"#,
        );
    }

    /// Report offset copies over an inclusive range.
    #[test]
    fn test_reports_inclusive_offset_copy() {
        let session = TestSession::dir(
            &MANUAL_COPY,
            r#"
function copy(target: &exclusive int32[], source: &readonly int32[]): void {
    for (const index of 2..=source.length - 2) {
        target[index - 2] = source[index + 1];
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-copy]: index loop copies corresponding collection elements
 ──▶ main.ds:2:5
  │
1 │ function copy(target: &exclusive int32[], source: &readonly int32[]): void {
2 │     for (const index of 2..=source.length - 2) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         target[index - 2] = source[index + 1];
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
4 │     }
  │     ^
5 │ }
  │
"#,
        );
    }

    /// Report parallel copies performed by the same index loop.
    #[test]
    fn test_reports_parallel_copies() {
        let session = TestSession::dir(
            &MANUAL_COPY,
            r#"
function copy(
    left: &exclusive int32[],
    right: &exclusive int32[],
    source: &readonly int32[],
): void {
    for (let index: isize = 0; index < source.length; index++) {
        left[index] = source[index];
        right[index] = source[index];
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-copy]: index loop copies corresponding collection elements
  ──▶ main.ds:6:5
   │
 4 │     source: &readonly int32[],
 5 │ ): void {
 6 │     for (let index: isize = 0; index < source.length; index++) {
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 7 │         left[index] = source[index];
   │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 8 │         right[index] = source[index];
   │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
 9 │     }
   │     ^
10 │ }
   │
"#,
        );
    }

    /// Accept a loop that transforms each source element.
    #[test]
    fn test_accepts_transformed_value() {
        let session = TestSession::dir(
            &MANUAL_COPY,
            r#"
function double(target: &exclusive int32[], source: &readonly int32[]): void {
    for (let index: isize = 0; index < source.length; index++) {
        target[index] = source[index] * 2;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a loop with additional work.
    #[test]
    fn test_accepts_additional_action() {
        let session = TestSession::dir(
            &MANUAL_COPY,
            r#"
function copy(target: &exclusive int32[], source: &readonly int32[]): void {
    for (let index: isize = 0; index < source.length; index++) {
        target[index] = source[index];
        target[index] += 1;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept potentially aliased managed arrays.
    #[test]
    fn test_accepts_managed_collections() {
        let session = TestSession::dir(
            &MANUAL_COPY,
            r#"
function copy(target: int32[], source: int32[]): void {
    for (let index: isize = 0; index < source.length; index++) {
        target[index] = source[index];
    }
}

function copyAliased(target: &int32[], source: &readonly int32[]): void {
    for (let index: isize = 0; index < source.length; index++) {
        target[index] = source[index];
    }
}

"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept element assignments that perform a conversion.
    #[test]
    fn test_accepts_converting_copy() {
        let session = TestSession::dir(
            &MANUAL_COPY,
            r#"
function copy(target: &unknown[], source: &readonly int32[]): void {
    for (let index: isize = 0; index < source.length; index++) {
        target[index] = source[index];
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}

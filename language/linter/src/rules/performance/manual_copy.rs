use destack_dir as dir;
use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, IntegerStep, Lint, LintOutput, LintResult};

const COLLECTION_LENGTH_OWNERS: &[dir::LanguageItem] = &[
    dir::LanguageItem::Sequence,
    dir::LanguageItem::Array,
    dir::LanguageItem::Slice,
];

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
function copy(target: int32[], source: int32[]): void {
    for (let index: isize = 0; index < source.length; index++) {
        target[index] = source[index];
    }
}
"#,
            accepted: r#"
function copy(target: int32[], source: int32[]): void {
    target.view(0, source.length).copyFrom(source.view(0));
}
"#,
        },
        category: Performance,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// One increasing index loop bounded by a collection length.
#[derive(Debug, Clone, Copy)]
struct IndexLoop {
    /// The index binding.
    index: dir::GlobalSymbolId,
    /// The first yielded index.
    start: i64,
    /// The indexed collection.
    collection: dir::LocalNodeId<dir::Expression>,
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
    /// Select one increasing index loop bounded by a collection length.
    fn select(
        module: &DirModule<'_>,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<Self>, ProviderError> {
        match module.view().get(expression) {
            // select classic counter loops
            dir::Expression::For {
                initialization: Some(initialization),
                condition: Some(condition),
                increment: Some(increment),
                body,
                ..
            } => Self::select_counter(module, *initialization, *condition, *increment, *body),

            // select authored range loops
            dir::Expression::ForEach {
                asynchrony: dir::Asynchrony::Sync,
                operator: dir::ForEachOperator::Of,
                binding,
                iterator,
                body,
                ..
            } => Self::select_range(module, binding, *iterator, *body),
            _ => Ok(None),
        }
    }

    /// Select one classic increasing counter loop.
    fn select_counter(
        module: &DirModule<'_>,
        initialization: dir::LocalNodeId<dir::Expression>,
        condition: dir::LocalNodeId<dir::Expression>,
        increment: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> Result<Option<Self>, ProviderError> {
        // require one integral index binding
        let Some((dir::Mutability::Mutable, declarator)) =
            module.binding_declarator(initialization)
        else {
            return Ok(None);
        };
        let Some(initializer) = declarator.value else {
            return Ok(None);
        };
        let Some(start) = module.integral_constant(initializer)? else {
            return Ok(None);
        };
        if start < 0 {
            return Ok(None);
        }
        let index = module.declaration_symbol(declarator.pattern)?;

        // require one increasing unit step over the same binding
        let Some(IntegerStep::Increment(target)) = module.integer_update(increment)? else {
            return Ok(None);
        };
        if module.selected_symbol(target)? != Some(index) {
            return Ok(None);
        }

        // select the exclusive collection length upper bound
        let Some((operator, [left, right])) = module.builtin_binary(condition)? else {
            return Ok(None);
        };
        let length = match operator {
            dir::BinaryOperator::LessThan
                if module.selected_symbol(left.source.local_id)? == Some(index) =>
            {
                right.source.local_id
            }
            dir::BinaryOperator::GreaterThan
                if module.selected_symbol(right.source.local_id)? == Some(index) =>
            {
                left.source.local_id
            }
            _ => return Ok(None),
        };
        let Some(collection) = length_receiver(module, length)? else {
            return Ok(None);
        };

        Ok(Some(Self {
            index,
            start,
            collection,
            body,
        }))
    }

    /// Select one exclusive integral range ending at a collection length.
    fn select_range(
        module: &DirModule<'_>,
        binding: &dir::ForEachBinding,
        iterator: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) -> Result<Option<Self>, ProviderError> {
        // require one direct declared binding
        let dir::ForEachBinding::Pattern {
            pattern,
            keyword: Some(_),
        } = binding
        else {
            return Ok(None);
        };
        if !matches!(
            module.view().get(*pattern),
            dir::Pattern::Binding { pattern: None, .. }
        ) {
            return Ok(None);
        }

        // require one constant start and collection length end
        let dir::Expression::RangeExpression {
            start: Some(start),
            end: Some(end),
            end_kind: dir::RangeEnd::Open,
        } = module.view().get(iterator)
        else {
            return Ok(None);
        };
        let Some(start) = module.integral_constant(*start)? else {
            return Ok(None);
        };
        if start < 0 {
            return Ok(None);
        }
        let Some(collection) = length_receiver(module, *end)? else {
            return Ok(None);
        };

        Ok(Some(Self {
            index: module.declaration_symbol(*pattern)?,
            start,
            collection,
            body,
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
        let copies_bound_collection =
            source.offset == 0 && module.is_same_computation(source.collection, self.collection)?;
        let has_distinct_target =
            !module.is_same_computation(target.collection, source.collection)?;
        if target_start < 0 || !copies_bound_collection || !has_distinct_target {
            return Ok(false);
        }

        // require the assigned element to satisfy the bulk operation's Copy bound
        let element = module.node_type_id(assignment.value.into_any())?;
        let element = module.dir.strip_form(element)?;
        if !module.auto.conforms(element, dir::AutoInterface::Copy) {
            return Ok(false);
        }

        // require canonical contiguous source and target collections
        let is_target_contiguous = is_contiguous_collection(module, target.collection)?;
        let is_source_contiguous = is_contiguous_collection(module, source.collection)?;

        Ok(is_target_contiguous && is_source_contiguous)
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
    let mut output = LintOutput::default();

    // inspect increasing index loops bounded by one collection length
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some(index_loop) = IndexLoop::select(module, expression)? else {
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

/// Return the receiver of one canonical collection length property.
fn length_receiver(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let dir::Expression::Member {
        left: collection,
        is_optional: false,
        ..
    } = module.view().get(expression)
    else {
        return Ok(None);
    };
    let Some(member) = module.language_member(expression)? else {
        return Ok(None);
    };
    let is_length = COLLECTION_LENGTH_OWNERS
        .iter()
        .any(|owner| member == owner.member("length"));

    Ok(is_length.then_some(*collection))
}

/// Return whether one expression denotes an array or slice.
fn is_contiguous_collection(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, ProviderError> {
    if !module.is_repeatable_expression(expression)? {
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
function copy(target: int32[], source: int32[]): void {
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
1 │ function copy(target: int32[], source: int32[]): void {
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
function copy(target: int32[], source: int32[]): void {
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
1 │ function copy(target: int32[], source: int32[]): void {
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
function copy(target: int32[], source: int32[]): void {
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
1 │ function copy(target: int32[], source: int32[]): void {
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
function copy(target: int32[], source: int32[]): void {
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
1 │ function copy(target: int32[], source: int32[]): void {
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

    /// Report parallel copies performed by the same index loop.
    #[test]
    fn test_reports_parallel_copies() {
        let session = TestSession::dir(
            &MANUAL_COPY,
            r#"
function copy(left: int32[], right: int32[], source: int32[]): void {
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
 ──▶ main.ds:2:5
  │
1 │ function copy(left: int32[], right: int32[], source: int32[]): void {
2 │     for (let index: isize = 0; index < source.length; index++) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         left[index] = source[index];
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
4 │         right[index] = source[index];
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
5 │     }
  │     ^
6 │ }
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
function double(target: int32[], source: int32[]): void {
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
function copy(target: int32[], source: int32[]): void {
    for (let index: isize = 0; index < source.length; index++) {
        target[index] = source[index];
        target[index] += 1;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}

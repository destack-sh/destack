use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, FilePatch, NodeSpanRegion};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

const CAPACITY_TYPES: &[dir::LanguageItem] = &[
    dir::LanguageItem::Array,
    dir::LanguageItem::BinaryHeap,
    dir::LanguageItem::ByteBuffer,
    dir::LanguageItem::Deque,
    dir::LanguageItem::LinkedList,
    dir::LanguageItem::Map,
    dir::LanguageItem::OsStringBuilder,
    dir::LanguageItem::PathBuilder,
    dir::LanguageItem::Set,
    dir::LanguageItem::Slab,
    dir::LanguageItem::SmallArray,
    dir::LanguageItem::SortedMap,
    dir::LanguageItem::SortedSet,
    dir::LanguageItem::StringBuilder,
];

declare_lint! {
    /// Prefer withCapacity over immediate reservation after construction.
    pub PREFER_WITH_CAPACITY {
        id: "prefer-with-capacity",
        summary: "Prefer withCapacity over immediate reservation after construction",
        explanation: r#"
Constructing an empty collection and immediately reserving capacity splits one initialization across two operations.
Instead, you SHOULD construct the collection with `withCapacity`.
"#,
        example: {
            reported: r#"
function collect(capacity: usize): ^int32[] {
    let values: ^int32[] = Array.new();
    values.reserve(capacity);

    return values;
}
"#,
            accepted: r#"
function collect(capacity: usize): ^int32[] {
    let values: ^int32[] = Array.withCapacity(capacity);

    return values;
}
"#,
        },
        provenance: [Clippy("reserve_after_initialization")],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One empty construction followed by reservation.
#[derive(Debug, Clone, Copy)]
struct ReservedConstruction {
    /// The empty constructor call.
    constructor: dir::LocalNodeId<dir::Expression>,
    /// The constructor member expression.
    constructor_callee: dir::LocalNodeId<dir::Expression>,
    /// The reservation statement.
    reservation: dir::LocalNodeId<dir::Expression>,
    /// The requested capacity.
    capacity: dir::LocalNodeId<dir::Expression>,
}

/// Report empty collections reserved immediately after construction.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect adjacent statements within each block
    for (_, block) in view.iter_nodes::<dir::Block>() {
        let expressions = block.iter_expressions().collect::<Vec<_>>();
        for statements in expressions.windows(2) {
            let Some(construction) =
                select_reserved_construction(module, statements[0], statements[1], &occurrences)?
            else {
                continue;
            };

            // combine construction and reservation
            let span = module.source_extent(construction.reservation.into_any())?;
            let mut diagnostic = lint.diagnostic("empty collection is immediately reserved", span);
            if let Some(suggestion) = suggestion(module, lint, construction)? {
                diagnostic = diagnostic.suggestion(suggestion);
            }
            output.report(diagnostic);
        }
    }

    Ok(output)
}

/// Select one direct binding construction followed by reservation.
fn select_reserved_construction(
    module: &DirModule<'_>,
    declaration: dir::LocalNodeId<dir::Expression>,
    reservation: dir::LocalNodeId<dir::Expression>,
    occurrences: &[dir::BindingOccurrence],
) -> Result<Option<ReservedConstruction>, ProviderError> {
    // require one direct binding initialized by an argument-free new call
    let Some((_, declarator)) = module.binding_declarator(declaration) else {
        return Ok(None);
    };
    let Some(constructor) = declarator.value else {
        return Ok(None);
    };
    let Some(constructor_call) = module.member_call(constructor) else {
        return Ok(None);
    };
    if constructor_call.is_optional()
        || !constructor_call.generic_arguments.is_empty()
        || !constructor_call.arguments.is_empty()
    {
        return Ok(None);
    }
    let Some(constructor_member) = module.language_member(constructor)? else {
        return Ok(None);
    };
    let is_capacity_type = CAPACITY_TYPES.contains(&constructor_member.owner);
    if !is_capacity_type || constructor_member != constructor_member.owner.member("new") {
        return Ok(None);
    }

    // require reserve on the declared binding and the same canonical owner
    let Some(reserve_call) = module.member_call(reservation) else {
        return Ok(None);
    };
    if reserve_call.is_optional() || !reserve_call.generic_arguments.is_empty() {
        return Ok(None);
    }
    let [argument] = reserve_call.arguments else {
        return Ok(None);
    };
    let Some(capacity) = module.view().get(*argument).value() else {
        return Ok(None);
    };
    let Some(reserve_member) = module.language_member(reservation)? else {
        return Ok(None);
    };
    if reserve_member != constructor_member.owner.member("reserve") {
        return Ok(None);
    }
    let symbol = module.declaration_symbol(declarator.pattern)?;
    if module.selected_symbol(reserve_call.receiver)? != Some(symbol) {
        return Ok(None);
    }
    if !module.is_speculatable_expression(capacity)?
        || !module
            .binding_uses_within(symbol, capacity.into_any(), occurrences)
            .is_empty()
    {
        return Ok(None);
    }

    Ok(Some(ReservedConstruction {
        constructor,
        constructor_callee: constructor_call.callee,
        reservation,
        capacity,
    }))
}

/// Combine one empty construction and immediate reservation.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    construction: ReservedConstruction,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let constructor = module.source_extent(construction.constructor.into_any())?;
    let capacity = module.source_extent(construction.capacity.into_any())?;
    let reservation = module.statement_span(construction.reservation)?;
    if module.has_unretained_comment(constructor, &[])?
        || module.has_unretained_comment(reservation, &[capacity])?
    {
        return Ok(None);
    }

    // move the capacity into the constructor and remove reserve
    let arguments = module.source_region(
        construction.constructor.into_any(),
        NodeSpanRegion::Arguments,
    )?;
    let capacity = module.source(capacity)?;
    let mut patch = FilePatch::new(constructor.file);
    patch.replace(
        module.main_span(construction.constructor_callee.into_any())?,
        "withCapacity",
    );
    patch.replace(arguments, format!("({capacity})"));
    patch.delete(module.line_removal_span(reservation)?);
    patch.sort();
    let suggestion = lint.suggestion("construct with the required capacity", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Combine Array.new and an immediate reserve call.
    #[test]
    fn test_combines_array_reservation() {
        let session = TestSession::dir(
            &PREFER_WITH_CAPACITY,
            r#"
function collect(capacity: usize): ^int32[] {
    let values: ^int32[] = Array.new();
    values.reserve(capacity);

    return values;
}
"#,
        );

        session.assert_suggestions(
            r#"
function collect(capacity: usize): ^int32[] {
    let values: ^int32[] = Array.withCapacity(capacity);

    return values;
}
"#,
        );
    }

    /// Combine Map.new and an immediate reserve call.
    #[test]
    fn test_combines_map_reservation() {
        let session = TestSession::dir(
            &PREFER_WITH_CAPACITY,
            r#"
function collect(capacity: usize): ^Map<string, int32> {
    let values: ^Map<string, int32> = Map.new();
    values.reserve(capacity);

    return values;
}
"#,
        );

        session.assert_suggestions(
            r#"
function collect(capacity: usize): ^Map<string, int32> {
    let values: ^Map<string, int32> = Map.withCapacity(capacity);

    return values;
}
"#,
        );
    }

    /// Combine Set.new and an immediate reserve call.
    #[test]
    fn test_combines_set_reservation() {
        let session = TestSession::dir(
            &PREFER_WITH_CAPACITY,
            r#"
function collect(capacity: usize): ^Set<int32> {
    let values: ^Set<int32> = Set.new();
    values.reserve(capacity);

    return values;
}
"#,
        );

        session.assert_suggestions(
            r#"
function collect(capacity: usize): ^Set<int32> {
    let values: ^Set<int32> = Set.withCapacity(capacity);

    return values;
}
"#,
        );
    }

    /// Combine ByteBuffer.new and an immediate reserve call.
    #[test]
    fn test_combines_byte_buffer_reservation() {
        let session = TestSession::dir(
            &PREFER_WITH_CAPACITY,
            r#"
import { ByteBuffer } from "tspp:bytes";

function collect(capacity: usize): ^ByteBuffer {
    let value: ^ByteBuffer = ByteBuffer.new();
    value.reserve(capacity);

    return value;
}
"#,
        );

        session.assert_suggestions(
            r#"
import { ByteBuffer } from "tspp:bytes";

function collect(capacity: usize): ^ByteBuffer {
    let value: ^ByteBuffer = ByteBuffer.withCapacity(capacity);

    return value;
}
"#,
        );
    }

    /// Combine PathBuilder.new and an immediate reserve call.
    #[test]
    fn test_combines_path_builder_reservation() {
        let session = TestSession::dir(
            &PREFER_WITH_CAPACITY,
            r#"
import { PathBuilder } from "tspp:fs";

function collect(capacity: usize): ^PathBuilder {
    let value: ^PathBuilder = PathBuilder.new();
    value.reserve(capacity);

    return value;
}
"#,
        );

        session.assert_suggestions(
            r#"
import { PathBuilder } from "tspp:fs";

function collect(capacity: usize): ^PathBuilder {
    let value: ^PathBuilder = PathBuilder.withCapacity(capacity);

    return value;
}
"#,
        );
    }

    /// Combine Deque.new and an immediate reserve call.
    #[test]
    fn test_combines_deque_reservation() {
        let session = TestSession::dir(
            &PREFER_WITH_CAPACITY,
            r#"
function collect(capacity: usize): ^Deque<int32> {
    let values: ^Deque<int32> = Deque.new();
    values.reserve(capacity);

    return values;
}
"#,
        );

        session.assert_suggestions(
            r#"
function collect(capacity: usize): ^Deque<int32> {
    let values: ^Deque<int32> = Deque.withCapacity(capacity);

    return values;
}
"#,
        );
    }

    /// Combine StringBuilder.new and an immediate reserve call.
    #[test]
    fn test_combines_string_builder_reservation() {
        let session = TestSession::dir(
            &PREFER_WITH_CAPACITY,
            r#"
import { StringBuilder } from "tspp:string";

function collect(capacity: usize): ^StringBuilder {
    let value: ^StringBuilder = StringBuilder.new();
    value.reserve(capacity);

    return value;
}
"#,
        );

        session.assert_suggestions(
            r#"
import { StringBuilder } from "tspp:string";

function collect(capacity: usize): ^StringBuilder {
    let value: ^StringBuilder = StringBuilder.withCapacity(capacity);

    return value;
}
"#,
        );
    }

    /// Accept reservation after another statement.
    #[test]
    fn test_accepts_delayed_reservation() {
        let session = TestSession::dir(
            &PREFER_WITH_CAPACITY,
            r#"
function collect(capacity: usize): ^int32[] {
    let values: ^int32[] = Array.new();
    capacity;
    values.reserve(capacity);

    return values;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a capacity whose evaluation has effects.
    #[test]
    fn test_accepts_effectful_capacity() {
        let session = TestSession::dir(
            &PREFER_WITH_CAPACITY,
            r#"
declare function nextCapacity(): usize;

function collect(): ^int32[] {
    let values: ^int32[] = Array.new();
    values.reserve(nextCapacity());

    return values;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a capacity that reads the newly constructed collection.
    #[test]
    fn test_accepts_self_referential_capacity() {
        let session = TestSession::dir(
            &PREFER_WITH_CAPACITY,
            r#"
function collect(): ^int32[] {
    let values: ^int32[] = Array.new();
    values.reserve(values.capacity);

    return values;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept reserve on another collection.
    #[test]
    fn test_accepts_other_receiver() {
        let session = TestSession::dir(
            &PREFER_WITH_CAPACITY,
            r#"
function collect(other: int32[], capacity: usize): ^int32[] {
    let values: ^int32[] = Array.new();
    other.reserve(capacity);

    return values;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}

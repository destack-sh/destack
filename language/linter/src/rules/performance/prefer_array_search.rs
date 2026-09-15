use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, NodeSpanRegion, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult, MemberCall};

declare_lint! {
    /// Prefer the direct Array operation for the result consumed by a search.
    pub PREFER_ARRAY_SEARCH {
        id: "prefer-array-search",
        summary: "Prefer direct Array search operations",
        explanation: r#"
Indirect searches allocate arrays, discard indices, or invoke callbacks for builtin equality.
Instead, you SHOULD use the direct Array search matching that result.

`find` and `findLast` return an element.
`some` and `includes` return membership.
`indexOf` and `lastIndexOf` return a position.
Reverse searches can change predicate or comparison order and require review.
"#,
        example: {
            reported: r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values.filter((value) => value > 0).at(0);
}
"#,
            accepted: r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values.find((value) => value > 0);
}
"#,
        },
        provenance: [
            TypeScriptEslint("prefer-find"),
            TypeScriptEslint("prefer-includes"),
            Unicorn("prefer-array-index-of"),
            Unicorn("prefer-array-some"),
        ],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One Array operation and observation replaced by a direct search.
#[derive(Debug, Clone, Copy)]
struct ArraySearch<'a> {
    /// The complete expression reported and replaced.
    expression: dir::LocalNodeId<dir::Expression>,
    /// The Array call retained by the replacement.
    call: MemberCall<'a>,
    /// The equality target replacing the authored predicate, if any.
    target: Option<dir::LocalNodeId<dir::Expression>>,
    /// The direct Array method.
    method: SearchMethod,
    /// Whether the replacement result is negated.
    is_negated: bool,
    /// Whether the correction preserves observable behavior.
    is_automatic: bool,
}

/// One canonical Array search method.
#[derive(Debug, Clone, Copy)]
enum SearchMethod {
    /// Find the first matching element.
    Find,
    /// Find the last matching element.
    FindLast,
    /// Test whether a predicate matches.
    Some,
    /// Test whether a value exists.
    Includes,
    /// Find the first index of a value.
    IndexOf,
    /// Find the last index of a value.
    LastIndexOf,
}

impl SearchMethod {
    /// Return the authored method name.
    fn name(self) -> &'static str {
        match self {
            Self::Find => "find",
            Self::FindLast => "findLast",
            Self::Some => "some",
            Self::Includes => "includes",
            Self::IndexOf => "indexOf",
            Self::LastIndexOf => "lastIndexOf",
        }
    }

    /// Return the diagnostic message.
    fn message(self) -> &'static str {
        match self {
            Self::Find => "filtered array is used only for its first element",
            Self::FindLast => "filtered array is used only for its last element",
            Self::Some => "array search is used only to test for a match",
            Self::Includes => "array index is only used for membership",
            Self::IndexOf | Self::LastIndexOf => "index predicate only compares one value",
        }
    }

    /// Return the correction message.
    fn correction(self) -> &'static str {
        match self {
            Self::Find | Self::FindLast | Self::IndexOf | Self::LastIndexOf => {
                "search the array directly"
            }
            Self::Some => "test matching elements directly",
            Self::Includes => "query array membership directly",
        }
    }
}

/// Report Array searches whose observed result has a direct operation.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect Array calls once and select their complete observation
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        let Some(search) = array_search(module, call, &occurrences)? else {
            continue;
        };

        // report the indirect search and offer its canonical operation
        let span = module.source_extent(search.expression.into_any())?;
        let mut diagnostic = lint.diagnostic(search.method.message(), span);
        if let Some(suggestion) = suggestion(module, lint, search)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select the direct operation for one Array call.
fn array_search<'a>(
    module: &DirModule<'_>,
    call: MemberCall<'a>,
    occurrences: &[dir::BindingOccurrence],
) -> Result<Option<ArraySearch<'a>>, ProviderError> {
    let member = module.language_member(call.expression)?;

    // dispatch by the canonical operation producing the indirect result
    if member == Some(dir::LanguageItem::Array.member("filter")) {
        filtered_search(module, call)
    } else if member == Some(dir::LanguageItem::Array.member("findIndex")) {
        predicate_index_search(module, call, false, occurrences)
    } else if member == Some(dir::LanguageItem::Array.member("findLastIndex")) {
        predicate_index_search(module, call, true, occurrences)
    } else if member == Some(dir::LanguageItem::Array.member("indexOf")) {
        value_index_search(module, call, false)
    } else if member == Some(dir::LanguageItem::Array.member("lastIndexOf")) {
        value_index_search(module, call, true)
    } else {
        Ok(None)
    }
}

/// Select a direct element or membership search over one filtered Array.
fn filtered_search<'a>(
    module: &DirModule<'_>,
    filter: MemberCall<'a>,
) -> Result<Option<ArraySearch<'a>>, ProviderError> {
    if filter.arguments.len() != 1 {
        return Ok(None);
    }

    // prefer find or findLast when an endpoint consumes the filtered Array
    if let Some(consumer) = module.receiver_call(filter.expression)
        && let Some(search) = filtered_endpoint(module, filter, consumer)?
    {
        return Ok(Some(search));
    }

    // preserve an optional filter result because absence remains observable
    if filter.is_optional() {
        return Ok(None);
    }

    // require a direct property observation over the filtered Array
    let view = module.view();
    let Some(observation) = view
        .get_parent_for(filter.expression)
        .and_then(|parent| parent.try_into_typed::<dir::Expression>().ok())
    else {
        return Ok(None);
    };
    let dir::Expression::Member {
        left,
        is_optional: false,
        ..
    } = view.get(observation)
    else {
        return Ok(None);
    };
    if *left != filter.expression {
        return Ok(None);
    }

    // select the complete empty or nonempty observation
    let observed_member = module.language_member(observation)?;
    let (tested, is_empty) = if observed_member == Some(dir::LanguageItem::Array.member("isEmpty"))
    {
        (observation, true)
    } else if observed_member == Some(dir::LanguageItem::Array.member("length")) {
        let Some(comparison) = view
            .get_parent_for(observation)
            .and_then(|parent| parent.try_into_typed::<dir::Expression>().ok())
        else {
            return Ok(None);
        };
        let Some(test) = module.emptiness_test(comparison)? else {
            return Ok(None);
        };
        if test.receiver != filter.expression
            || test.measurement != dir::LanguageItem::Array.member("length")
        {
            return Ok(None);
        }

        (comparison, test.is_empty)
    } else {
        return Ok(None);
    };

    // include one direct boolean negation around the observation
    let negation = module.direct_negation(tested)?;
    let expression = negation.unwrap_or(tested);
    let is_inverted = negation.is_some();

    Ok(Some(ArraySearch {
        expression,
        call: filter,
        target: None,
        method: SearchMethod::Some,
        is_negated: is_empty ^ is_inverted,
        is_automatic: false,
    }))
}

/// Select find or findLast for one filtered endpoint read.
fn filtered_endpoint<'a>(
    module: &DirModule<'_>,
    filter: MemberCall<'a>,
    consumer: MemberCall<'_>,
) -> Result<Option<ArraySearch<'a>>, ProviderError> {
    let view = module.view();
    let member = module.language_member(consumer.expression)?;

    // map each optional endpoint operation to its direct search
    let method = if member == Some(dir::LanguageItem::Array.member("at")) {
        let [argument] = consumer.arguments else {
            return Ok(None);
        };
        let Some(index) = view.get(*argument).value() else {
            return Ok(None);
        };

        match module.scalar_constant(index)? {
            Some(dir::Literal::Integer(0)) => SearchMethod::Find,
            Some(dir::Literal::Integer(-1)) => SearchMethod::FindLast,
            _ => return Ok(None),
        }
    } else if member == Some(dir::LanguageItem::Array.member("first"))
        && consumer.arguments.is_empty()
    {
        SearchMethod::Find
    } else if member == Some(dir::LanguageItem::Array.member("last"))
        && consumer.arguments.is_empty()
    {
        SearchMethod::FindLast
    } else {
        return Ok(None);
    };

    Ok(Some(ArraySearch {
        expression: consumer.expression,
        call: filter,
        target: None,
        method,
        is_negated: false,
        is_automatic: false,
    }))
}

/// Select some, includes, or an index operation for one predicate index search.
fn predicate_index_search<'a>(
    module: &DirModule<'_>,
    call: MemberCall<'a>,
    is_reverse: bool,
    occurrences: &[dir::BindingOccurrence],
) -> Result<Option<ArraySearch<'a>>, ProviderError> {
    let equality = equality_target(module, call, occurrences)?;
    let presence = presence_test(module, call.expression)?;

    // prefer includes when only equality membership is observed
    if let (Some(equality), Some((expression, is_negated))) = (equality, presence)
        && !call.is_optional()
    {
        return Ok(Some(ArraySearch {
            expression,
            call,
            target: Some(equality),
            method: SearchMethod::Includes,
            is_negated,
            is_automatic: false,
        }));
    }

    // prefer some when an arbitrary predicate index is observed for presence
    if let Some((expression, is_negated)) = presence
        && !call.is_optional()
    {
        return Ok(Some(ArraySearch {
            expression,
            call,
            target: None,
            method: SearchMethod::Some,
            is_negated,
            is_automatic: !is_reverse,
        }));
    }

    // prefer a value index operation when the numeric result remains observable
    let Some(equality) = equality else {
        return Ok(None);
    };
    let method = if is_reverse {
        SearchMethod::LastIndexOf
    } else {
        SearchMethod::IndexOf
    };

    Ok(Some(ArraySearch {
        expression: call.expression,
        call,
        target: Some(equality),
        method,
        is_negated: false,
        is_automatic: false,
    }))
}

/// Select includes for one direct value index observed only for membership.
fn value_index_search<'a>(
    module: &DirModule<'_>,
    call: MemberCall<'a>,
    is_reverse: bool,
) -> Result<Option<ArraySearch<'a>>, ProviderError> {
    if call.is_optional() || call.arguments.is_empty() || is_reverse && call.arguments.len() != 1 {
        return Ok(None);
    }
    let Some((expression, is_negated)) = presence_test(module, call.expression)? else {
        return Ok(None);
    };

    Ok(Some(ArraySearch {
        expression,
        call,
        target: None,
        method: SearchMethod::Includes,
        is_negated,
        is_automatic: !is_reverse,
    }))
}

/// Select a stable value compared directly by one predicate index search.
fn equality_target(
    module: &DirModule<'_>,
    call: MemberCall<'_>,
    occurrences: &[dir::BindingOccurrence],
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let [argument] = call.arguments else {
        return Ok(None);
    };
    let dir::Argument::Positional { value: callback } = module.view().get(*argument) else {
        return Ok(None);
    };

    // require one synchronous lambda with one direct parameter
    let Some(lambda) = module.lambda(*callback) else {
        return Ok(None);
    };
    if lambda.signature.asynchrony != dir::Asynchrony::Sync || lambda.signature.is_generator {
        return Ok(None);
    }
    let [parameter] = lambda.signature.parameters.as_slice() else {
        return Ok(None);
    };
    if !matches!(
        module.view().get(*parameter),
        dir::Parameter::Named {
            default: None,
            is_optional: false,
            ..
        }
    ) {
        return Ok(None);
    }
    let Some(body) = lambda
        .body
        .and_then(|body| module.sole_value_expression(body))
    else {
        return Ok(None);
    };

    // normalize the element parameter and stable target from strict equality
    let Some((dir::BinaryOperator::EqualStrict, [left, right])) = module.builtin_binary(body)?
    else {
        return Ok(None);
    };
    let parameter = module.declaration_symbol(*parameter)?;
    let left = left.source.local_id;
    let right = right.source.local_id;
    let target = if module.selected_symbol(left)? == Some(parameter) {
        right
    } else if module.selected_symbol(right)? == Some(parameter) {
        left
    } else {
        return Ok(None);
    };
    if !module.is_speculatable_expression(target)?
        || !module
            .binding_uses_within(parameter, target.into_any(), occurrences)
            .is_empty()
    {
        return Ok(None);
    }

    Ok(Some(target))
}

/// Select one complete nullish presence test over an Array index search.
fn presence_test(
    module: &DirModule<'_>,
    search: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<(dir::LocalNodeId<dir::Expression>, bool)>, ProviderError> {
    let Some(tested) = module
        .view()
        .get_parent_for(search)
        .and_then(|parent| parent.try_into_typed::<dir::Expression>().ok())
    else {
        return Ok(None);
    };
    let Some(test) = module.nullish_test(tested)? else {
        return Ok(None);
    };
    if test.value != search {
        return Ok(None);
    }

    // include one direct boolean negation around the presence test
    let negation = module.direct_negation(tested)?;
    let expression = negation.unwrap_or(tested);
    let is_inverted = negation.is_some();
    let is_present = test.is_defined ^ is_inverted;

    Ok(Some((expression, !is_present)))
}

/// Build the direct Array search while retaining authored source fragments.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    search: ArraySearch<'_>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(search.expression.into_any())?;
    let extent = match module.view().get(search.expression) {
        dir::Expression::Unary { right, .. } => module
            .source_parentheses(right.into_any())
            .map_or(extent, |parentheses| extent.merge(parentheses)),
        _ => extent,
    };
    let call = module.source_extent(search.call.expression.into_any())?;
    if !extent.contains_span(call) {
        return Err(ProviderError::internal(
            "array search extent does not contain its retained call",
        ));
    }
    if module.has_unretained_comment(extent, &[call])? {
        return Ok(None);
    }

    // retain only the selected target when replacing a predicate argument
    let replacement = if let Some(target) = search.target {
        let target = module.source_extent(target.into_any())?;
        let arguments =
            module.source_region(search.call.expression.into_any(), NodeSpanRegion::Arguments)?;
        if module.has_unretained_comment(arguments, &[target])? {
            return Ok(None);
        }

        Some((arguments, format!("({})", module.source(target)?)))
    } else {
        None
    };

    // remove the observation and rename the retained operation
    let prefix = Span::new(extent.file, extent.start, call.start);
    let suffix = Span::new(extent.file, call.end, extent.end);
    let mut file = FilePatch::new(extent.file);
    if !prefix.is_empty() {
        file.delete(prefix);
    }
    if search.is_negated {
        file.insert(call.start, "!");
    }
    file.replace(
        module.main_span(search.call.callee.into_any())?,
        search.method.name(),
    );
    if let Some((arguments, replacement)) = replacement {
        file.replace(arguments, replacement);
    }
    if !suffix.is_empty() {
        file.delete(suffix);
    }
    file.sort();

    // apply behavior-preserving corrections automatically
    let suggestion = if search.is_automatic {
        lint.fix(search.method.correction(), file)?
    } else {
        lint.suggestion(search.method.correction(), file)?
    };

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a filtered first-element read.
    #[test]
    fn test_replaces_filtered_first() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values.filter((value) => value > 0).at(0);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-array-search]: filtered array is used only for its first element
 ──▶ main.ds:2:12
  │
1 │ function firstPositive(values: int32[]): int32 | undefined {
2 │     return values.filter((value) => value > 0).at(0);
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = suggestion: search the array directly (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function firstPositive(values: int32[]): int32 | undefined {
-   2│     return values.filter((value) => value > 0).at(0);
+   2│     return values.find((value) => value > 0);
    3│ }
"#,
        );
        session.assert_suggestions(
            r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values.find((value) => value > 0);
}
"#,
        );
    }

    /// Replace a filtered last-element read.
    #[test]
    fn test_replaces_filtered_last() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function lastPositive(values: int32[]): int32 | undefined {
    return values.filter((value) => value > 0).last();
}
"#,
        );

        session.assert_suggestions(
            r#"
function lastPositive(values: int32[]): int32 | undefined {
    return values.findLast((value) => value > 0);
}
"#,
        );
    }

    /// Recognize a constant endpoint index.
    #[test]
    fn test_replaces_constant_endpoint() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
const end: isize = -1;

function lastPositive(values: int32[]): int32 | undefined {
    return values.filter((value) => value > 0).at(end);
}
"#,
        );

        session.assert_suggestions(
            r#"
const end: isize = -1;

function lastPositive(values: int32[]): int32 | undefined {
    return values.findLast((value) => value > 0);
}
"#,
        );
    }

    /// Preserve an optional receiver while replacing a filtered endpoint.
    #[test]
    fn test_replaces_optional_endpoint() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function lastPositive(values: int32[] | undefined): int32 | undefined {
    return values?.filter((value) => value > 0).last();
}
"#,
        );

        session.assert_suggestions(
            r#"
function lastPositive(values: int32[] | undefined): int32 | undefined {
    return values?.findLast((value) => value > 0);
}
"#,
        );
    }

    /// Replace filtered emptiness observations in each boolean sense.
    #[test]
    fn test_replaces_filtered_existence() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
const zero: isize = 0;

function hasPositive(values: int32[]): boolean {
    return !values.filter((value) => value > 0).isEmpty;
}
function hasNoPositive(values: int32[]): boolean {
    return values.filter((value) => value > 0).length === zero;
}
"#,
        );

        session.assert_suggestions(
            r#"
const zero: isize = 0;

function hasPositive(values: int32[]): boolean {
    return values.some((value) => value > 0);
}
function hasNoPositive(values: int32[]): boolean {
    return !values.some((value) => value > 0);
}
"#,
        );
    }

    /// Replace a forward index predicate observed only for presence.
    #[test]
    fn test_replaces_predicate_presence() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function hasPositive(values: int32[]): boolean {
    return values.findIndex((value) => value > 0) !== undefined;
}
"#,
        );

        session.assert_fixes(
            r#"
function hasPositive(values: int32[]): boolean {
    return values.some((value) => value > 0);
}
"#,
        );
    }

    /// Suggest replacing a reverse index predicate observed only for presence.
    #[test]
    fn test_replaces_reverse_predicate_presence() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function hasNoPositive(values: int32[]): boolean {
    return !(values.findLastIndex((value) => value > 0) !== undefined);
}
"#,
        );

        session.assert_suggestions(
            r#"
function hasNoPositive(values: int32[]): boolean {
    return !values.some((value) => value > 0);
}
"#,
        );
    }

    /// Suggest replacing a forward equality index predicate used for membership.
    #[test]
    fn test_replaces_equality_membership() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function contains(values: int32[], target: int32): boolean {
    return values.findIndex((value) => value === target) !== undefined;
}
"#,
        );

        session.assert_suggestions(
            r#"
function contains(values: int32[], target: int32): boolean {
    return values.includes(target);
}
"#,
        );
    }

    /// Suggest replacing a reverse equality index predicate used for membership.
    #[test]
    fn test_replaces_reverse_equality_membership() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function excludes(values: int32[], target: int32): boolean {
    return undefined === values.findLastIndex((value) => target === value);
}
"#,
        );

        session.assert_suggestions(
            r#"
function excludes(values: int32[], target: int32): boolean {
    return !values.includes(target);
}
"#,
        );
    }

    /// Replace equality predicates whose numeric index remains observable.
    #[test]
    fn test_replaces_equality_index_searches() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function locate(values: int32[], target: int32): isize | undefined {
    return values.findIndex((value) => value === target);
}
function locateLast(values: int32[], target: int32): isize | undefined {
    return values.findLastIndex((value) => target === value);
}
"#,
        );

        session.assert_suggestions(
            r#"
function locate(values: int32[], target: int32): isize | undefined {
    return values.indexOf(target);
}
function locateLast(values: int32[], target: int32): isize | undefined {
    return values.lastIndexOf(target);
}
"#,
        );
    }

    /// Replace a forward value index observed only for membership.
    #[test]
    fn test_replaces_value_index_membership() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
const absent: undefined = undefined;

function contains(values: int32[], target: int32): boolean {
    return values.indexOf(target) !== absent;
}
"#,
        );

        session.assert_fixes(
            r#"
const absent: undefined = undefined;

function contains(values: int32[], target: int32): boolean {
    return values.includes(target);
}
"#,
        );
    }

    /// Suggest replacing a reverse value index observed only for membership.
    #[test]
    fn test_replaces_reverse_value_index_membership() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function excludes(values: int32[], target: int32): boolean {
    return undefined === values.lastIndexOf(target);
}
"#,
        );

        session.assert_suggestions(
            r#"
function excludes(values: int32[], target: int32): boolean {
    return !values.includes(target);
}
"#,
        );
    }

    /// Preserve an optional equality index search and its presence comparison.
    #[test]
    fn test_replaces_optional_equality_index() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function contains(values: int32[] | undefined, target: int32): boolean {
    return values?.findIndex((value) => value === target) !== undefined;
}
"#,
        );

        session.assert_suggestions(
            r#"
function contains(values: int32[] | undefined, target: int32): boolean {
    return values?.indexOf(target) !== undefined;
}
"#,
        );
    }

    /// Accept optional filtered existence because absence remains observable.
    #[test]
    fn test_accepts_optional_filtered_existence() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function hasPositive(values: int32[] | undefined): boolean | undefined {
    return values?.filter((value) => value > 0)?.isEmpty;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a filtered array retained as the returned value.
    #[test]
    fn test_accepts_filtered_values() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function positives(values: int32[]): int32[] {
    return values.filter((value) => value > 0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept trapping access to the first filtered element.
    #[test]
    fn test_accepts_filtered_subscript() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function firstPositive(values: int32[]): int32 {
    return values.filter((value) => value > 0)[0];
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept optional access to a nonendpoint filtered element.
    #[test]
    fn test_accepts_nonendpoint_filtered_access() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function secondPositive(values: int32[]): int32 | undefined {
    return values.filter((value) => value > 0).at(1);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept find presence because a matching element may be undefined.
    #[test]
    fn test_accepts_find_presence() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function hasUndefined(values: (int32 | undefined)[]): boolean {
    return values.find((value) => value === undefined) !== undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an equality predicate that transforms each element.
    #[test]
    fn test_accepts_transformed_element() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function locate(values: int32[], target: int32): isize | undefined {
    return values.findIndex((value) => value + 1 === target);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an equality predicate that reads its index parameter.
    #[test]
    fn test_accepts_index_parameter() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function locate(values: isize[]): isize | undefined {
    return values.findIndex((value, index) => value === index);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an equality target that depends on the searched element.
    #[test]
    fn test_accepts_element_dependent_target() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function locate(values: int32[]): isize | undefined {
    return values.findIndex((value) => value === value + 1);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an effectful equality target whose evaluation count would change.
    #[test]
    fn test_accepts_effectful_target() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function locate(values: int32[]): isize | undefined {
    return values.findIndex((value) => value === values.pop());
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept bounded reverse value search because includes scans another range.
    #[test]
    fn test_accepts_bounded_reverse_search() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function containsBefore(values: int32[], target: int32, end: isize): boolean {
    return values.lastIndexOf(target, end) !== -1;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve a forward starting index in the direct membership query.
    #[test]
    fn test_replaces_bounded_forward_search() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function containsAfter(values: int32[], target: int32, start: isize): boolean {
    return values.indexOf(target, start) !== undefined;
}
"#,
        );

        session.assert_fixes(
            r#"
function containsAfter(values: int32[], target: int32, start: isize): boolean {
    return values.includes(target, start);
}
"#,
        );
    }

    /// Accept optional value membership because absence remains observable.
    #[test]
    fn test_accepts_optional_value_index() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function contains(values: int32[] | undefined, target: int32): boolean {
    return values?.indexOf(target) !== undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept user-defined methods with matching names.
    #[test]
    fn test_accepts_user_methods() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
class Values {
    findIndex(predicate: (value: int32) => boolean): isize | undefined {
        return undefined;
    }
}

function locate(values: Values, target: int32): isize | undefined {
    return values.findIndex((value) => value === target);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user extension whose method shares an Array member name.
    #[test]
    fn test_accepts_user_array_extension() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
class Bag {
    items: int32[] = [];
}

extension of Bag {
    filter(predicate: (value: int32) => boolean, trace: boolean): int32[] {
        return this.items;
    }
}

function firstPositive(bag: Bag): int32 | undefined {
    return bag.filter((value) => value > 0, true).at(0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Ignore call-shaped newtype construction.
    #[test]
    fn test_accepts_newtype_construction() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
newtype Status = { kind: "ready"; value: int32 } | { kind: "pending" };

function ready(value: int32): Status {
    return Status({ kind: "ready", value });
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve comments outside a retained filtered call by omitting the suggestion.
    #[test]
    fn test_reports_commented_filtered_observation_without_suggestion() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function hasPositive(values: int32[]): boolean {
    return values.filter((value) => value > 0) /* retain */ .length > 0;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-array-search]: array search is used only to test for a match
 ──▶ main.ds:2:12
  │
1 │ function hasPositive(values: int32[]): boolean {
2 │     return values.filter((value) => value > 0) /* retain */ .length > 0;
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Preserve comments inside a replaced equality predicate by omitting the suggestion.
    #[test]
    fn test_reports_commented_equality_without_suggestion() {
        let session = TestSession::dir(
            &PREFER_ARRAY_SEARCH,
            r#"
function locate(values: int32[], target: int32): isize | undefined {
    return values.findIndex((value) => value /* retain */ === target);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-array-search]: index predicate only compares one value
 ──▶ main.ds:2:12
  │
1 │ function locate(values: int32[], target: int32): isize | undefined {
2 │     return values.findIndex((value) => value /* retain */ === target);
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }
}

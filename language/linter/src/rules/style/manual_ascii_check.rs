use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{ComparisonRange, DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer character predicates over ASCII range checks.
    pub MANUAL_ASCII_CHECK {
        id: "manual-ascii-check",
        summary: "Prefer character predicates over ASCII range checks",
        explanation: r#"
Standard ASCII ranges written as comparisons, range membership, or patterns obscure their character class.
Instead, you SHOULD call the corresponding `isAscii` character predicate.
"#,
        example: {
            reported: r#"
function isLowercase(character: char): boolean {
    return character >= 'a' && character <= 'z';
}
"#,
            accepted: r#"
function isLowercase(character: char): boolean {
    return character.isAsciiLowercase();
}
"#,
        },
        provenance: [Clippy("manual_is_ascii_check")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// One standard ASCII character interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AsciiRange {
    /// Lowercase ASCII letters.
    Lowercase,
    /// Uppercase ASCII letters.
    Uppercase,
    /// ASCII decimal digits.
    Digit,
    /// Lowercase ASCII hexadecimal letters.
    LowerHex,
    /// Uppercase ASCII hexadecimal letters.
    UpperHex,
}

/// One character predicate replacing one or more range checks.
struct AsciiCheck {
    /// The tested character.
    value: dir::LocalNodeId<dir::Expression>,
    /// The predicate method.
    method: &'static str,
    /// Whether the predicate result is negated.
    is_negated: bool,
}

impl AsciiCheck {
    /// Report one manual ASCII check.
    fn report(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        module: &DirModule<'_>,
        lint: &Lint,
        output: &mut LintOutput,
    ) -> Result<(), ProviderError> {
        let span = module.source_extent(expression.into_any())?;
        let message = format!("ASCII range manually implements `{}`", self.method);
        let mut diagnostic = lint.diagnostic(message, span);
        if let Some(fix) = fix(module, lint, expression, self)? {
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);

        Ok(())
    }
}

/// One character interval.
#[derive(Debug, Clone, Copy)]
struct CharacterRange {
    /// The tested character.
    value: dir::LocalNodeId<dir::Expression>,
    /// The standard ASCII interval.
    range: AsciiRange,
    /// Whether the interval membership is negated.
    is_negated: bool,
}

/// Report standard ASCII character classes written as ranges.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect maximal runtime logical expressions
    for expression in module.operator_expressions() {
        let expression = expression?;
        if module.has_builtin_binary_parent(expression, dir::BinaryOperator::Or)? {
            continue;
        }
        let Some(ascii) = logical_ascii_check(module, expression)? else {
            continue;
        };

        ascii.report(expression, module, lint, &mut output)?;
    }

    // inspect maximal range membership calls
    for expression in module.call_expressions() {
        let expression = expression?;
        if module.has_builtin_binary_parent(expression, dir::BinaryOperator::Or)? {
            continue;
        }
        let Some(ascii) = range_call(module, expression)?.and_then(ascii_check) else {
            continue;
        };

        ascii.report(expression, module, lint, &mut output)?;
    }

    // inspect boolean range matches
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        if !matches!(node, dir::Expression::Match { .. }) {
            continue;
        }
        let Some(ascii) = ascii_match(module, node)? else {
            continue;
        };

        ascii.report(expression, module, lint, &mut output)?;
    }

    Ok(output)
}

/// Select one predicate from a logical expression of ASCII ranges.
fn logical_ascii_check(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<AsciiCheck>, ProviderError> {
    // retain one complete positive or negative range before flattening unions
    if let Some(range) = module.comparison_range(expression)? {
        let range = comparison_character_range(module, range)?;

        return Ok(range.and_then(ascii_check));
    }

    // combine positive ranges that form a larger standard character class
    let terms = module.short_circuit_operands(expression, dir::BinaryOperator::Or)?;
    let mut ranges = Vec::with_capacity(terms.len());
    for term in terms {
        let range = if let Some(range) = module.comparison_range(term)? {
            comparison_character_range(module, range)?
        } else {
            range_call(module, term)?
        };
        let Some(range) = range else {
            return Ok(None);
        };

        ranges.push(range);
    }
    if ranges.len() > 1 && ranges.iter().any(|range| range.is_negated) {
        return Ok(None);
    }

    ascii_ranges(module, &ranges)
}

/// Select one character interval from a comparison range.
fn comparison_character_range(
    module: &DirModule<'_>,
    range: ComparisonRange,
) -> Result<Option<CharacterRange>, ProviderError> {
    if range.end_kind != dir::RangeEnd::Inclusive {
        return Ok(None);
    }
    let Some(range_kind) = ascii_range(module, range.start, range.end)? else {
        return Ok(None);
    };

    Ok(Some(CharacterRange {
        value: range.value,
        range: range_kind,
        is_negated: range.is_negated,
    }))
}

/// Select one character interval from a canonical inclusive range call.
fn range_call(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<CharacterRange>, ProviderError> {
    let Some(call) = module.member_call(expression) else {
        return Ok(None);
    };
    let [argument] = call.arguments else {
        return Ok(None);
    };
    let Some(value) = module.view().get(*argument).value() else {
        return Ok(None);
    };
    if call.is_optional()
        || !call.generic_arguments.is_empty()
        || module.language_member(expression)?
            != Some(dir::LanguageItem::RangeBounds.member("contains"))
    {
        return Ok(None);
    }
    let dir::Expression::RangeExpression {
        start: Some(start),
        end: Some(end),
        end_kind: dir::RangeEnd::Inclusive,
    } = module.view().get(call.receiver)
    else {
        return Ok(None);
    };
    let Some(range) = ascii_range(module, *start, *end)? else {
        return Ok(None);
    };
    if module.primitive_type(value.into_any())? != Some(dir::PrimitiveType::Character) {
        return Ok(None);
    }

    Ok(Some(CharacterRange {
        value,
        range,
        is_negated: false,
    }))
}

/// Select one character predicate from a boolean range match.
fn ascii_match(
    module: &DirModule<'_>,
    node: &dir::Expression,
) -> Result<Option<AsciiCheck>, ProviderError> {
    let dir::Expression::Match { value, arms } = node else {
        return Ok(None);
    };
    let [first, second] = arms.as_slice() else {
        return Ok(None);
    };
    let Some(first) = boolean_pattern(module, *first)? else {
        return Ok(None);
    };
    let Some(second) = boolean_pattern(module, *second)? else {
        return Ok(None);
    };

    // select one range pattern and an opposite wildcard result
    let (patterns, is_negated) = match (first, second) {
        (BooleanPattern::Ranges(patterns, result), BooleanPattern::Wildcard(fallback))
            if result != fallback =>
        {
            (patterns, !result)
        }
        (BooleanPattern::Wildcard(fallback), BooleanPattern::Ranges(patterns, result))
            if result != fallback =>
        {
            (patterns, !result)
        }
        _ => return Ok(None),
    };
    if module.primitive_type(value.into_any())? != Some(dir::PrimitiveType::Character) {
        return Ok(None);
    }

    // classify every range pattern against the same match value
    let mut ranges = Vec::with_capacity(patterns.len());
    for (start, end) in patterns {
        let Some(range) = ascii_range(module, start, end)? else {
            return Ok(None);
        };
        ranges.push(CharacterRange {
            value: *value,
            range,
            is_negated,
        });
    }

    ascii_ranges(module, &ranges)
}

/// One boolean match arm relevant to ASCII classification.
enum BooleanPattern {
    /// One or more inclusive range patterns and their result.
    Ranges(
        Vec<(
            dir::LocalNodeId<dir::Expression>,
            dir::LocalNodeId<dir::Expression>,
        )>,
        bool,
    ),
    /// The wildcard result.
    Wildcard(bool),
}

/// Return one unguarded boolean range or wildcard arm.
fn boolean_pattern(
    module: &DirModule<'_>,
    arm: dir::LocalNodeId<dir::MatchArm>,
) -> Result<Option<BooleanPattern>, ProviderError> {
    let Some((pattern, body)) = module.match_arm_value(arm) else {
        return Ok(None);
    };
    let Some(result) = module.view().get(body).as_boolean() else {
        return Ok(None);
    };

    // accept the exhaustive wildcard or collect inclusive range alternatives
    if matches!(module.view().get(pattern), dir::Pattern::Wildcard)
        && matches!(
            module.pattern_decision(pattern)?,
            dir::PatternDecision::Ignore
        )
    {
        return Ok(Some(BooleanPattern::Wildcard(result)));
    }
    let mut patterns = Vec::new();
    if !collect_range_patterns(module, pattern, &mut patterns)? {
        return Ok(None);
    }

    Ok(Some(BooleanPattern::Ranges(patterns, result)))
}

/// Append every inclusive range in one range or union pattern.
fn collect_range_patterns(
    module: &DirModule<'_>,
    pattern: dir::LocalNodeId<dir::Pattern>,
    ranges: &mut Vec<(
        dir::LocalNodeId<dir::Expression>,
        dir::LocalNodeId<dir::Expression>,
    )>,
) -> Result<bool, ProviderError> {
    // collect one complete inclusive interval
    match module.view().get(pattern) {
        dir::Pattern::Range {
            start: Some(start),
            end: Some(end),
            end_kind: dir::RangeEnd::Inclusive,
        } if matches!(
            module.pattern_decision(pattern)?,
            dir::PatternDecision::Test(_)
        ) =>
        {
            ranges.push((*start, *end))
        }

        // flatten range alternatives from one union pattern
        dir::Pattern::Union { patterns }
            if matches!(
                module.pattern_decision(pattern)?,
                dir::PatternDecision::Or(_)
            ) =>
        {
            for pattern in patterns {
                if !collect_range_patterns(module, *pattern, ranges)? {
                    return Ok(false);
                }
            }
        }
        _ => return Ok(false),
    }

    Ok(true)
}

/// Return the standard ASCII interval represented by two character literals.
fn ascii_range(
    module: &DirModule<'_>,
    start: dir::LocalNodeId<dir::Expression>,
    end: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<AsciiRange>, ProviderError> {
    let bounds = (module.scalar_constant(start)?, module.scalar_constant(end)?);
    let range = match bounds {
        (Some(dir::Literal::Character('a')), Some(dir::Literal::Character('z'))) => {
            Some(AsciiRange::Lowercase)
        }
        (Some(dir::Literal::Character('A')), Some(dir::Literal::Character('Z'))) => {
            Some(AsciiRange::Uppercase)
        }
        (Some(dir::Literal::Character('0')), Some(dir::Literal::Character('9'))) => {
            Some(AsciiRange::Digit)
        }
        (Some(dir::Literal::Character('a')), Some(dir::Literal::Character('f'))) => {
            Some(AsciiRange::LowerHex)
        }
        (Some(dir::Literal::Character('A')), Some(dir::Literal::Character('F'))) => {
            Some(AsciiRange::UpperHex)
        }
        _ => None,
    };

    Ok(range)
}

/// Select the character predicate represented by exact ASCII intervals.
fn ascii_ranges(
    module: &DirModule<'_>,
    ranges: &[CharacterRange],
) -> Result<Option<AsciiCheck>, ProviderError> {
    let Some(first) = ranges.first() else {
        return Ok(None);
    };
    if ranges
        .iter()
        .any(|range| range.is_negated != first.is_negated)
    {
        return Ok(None);
    }
    for range in &ranges[1..] {
        if !module.is_same_computation(first.value, range.value)? {
            return Ok(None);
        }
    }
    if ranges.len() > 1 && !module.is_duplicable_expression(first.value)? {
        return Ok(None);
    }

    // collect the represented standard character intervals
    let has_lowercase = ranges
        .iter()
        .any(|range| range.range == AsciiRange::Lowercase);
    let has_uppercase = ranges
        .iter()
        .any(|range| range.range == AsciiRange::Uppercase);
    let has_digit = ranges.iter().any(|range| range.range == AsciiRange::Digit);
    let has_lower_hex = ranges
        .iter()
        .any(|range| range.range == AsciiRange::LowerHex);
    let has_upper_hex = ranges
        .iter()
        .any(|range| range.range == AsciiRange::UpperHex);

    // match one complete standard character class
    let method = match ranges.len() {
        1 if has_lowercase => "isAsciiLowercase",
        1 if has_uppercase => "isAsciiUppercase",
        1 if has_digit => "isAsciiDigit",
        2 if has_lowercase && has_uppercase => "isAsciiAlphabetic",
        3 if has_digit && has_lowercase && has_uppercase => "isAsciiAlphanumeric",
        3 if has_digit && has_lower_hex && has_upper_hex => "isAsciiHexDigit",
        _ => return Ok(None),
    };

    Ok(Some(AsciiCheck {
        value: first.value,
        method,
        is_negated: first.is_negated,
    }))
}

/// Select one predicate for a single character range.
fn ascii_check(range: CharacterRange) -> Option<AsciiCheck> {
    let method = match range.range {
        AsciiRange::Lowercase => "isAsciiLowercase",
        AsciiRange::Uppercase => "isAsciiUppercase",
        AsciiRange::Digit => "isAsciiDigit",
        AsciiRange::LowerHex | AsciiRange::UpperHex => return None,
    };

    Some(AsciiCheck {
        value: range.value,
        method,
        is_negated: range.is_negated,
    })
}

/// Build one named ASCII character predicate.
fn fix(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    ascii: &AsciiCheck,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let value_span = module.source_extent(ascii.value.into_any())?;
    if module.has_unretained_comment(span, &[value_span])? {
        return Ok(None);
    }

    // retain the character value with postfix-safe grouping
    let value = module.expression_source(ascii.value, dir::OperatorPrecedence::Postfix)?;
    let negation = if ascii.is_negated { "!" } else { "" };
    let patch = Patch::replace(span, format!("{negation}{value}.{}()", ascii.method));
    let fix = lint.fix(format!("call `.{}()`", ascii.method), patch)?;

    Ok(Some(fix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace lowercase, uppercase, and digit comparisons.
    #[test]
    fn test_replaces_ascii_comparisons() {
        let session = TestSession::dir(
            &MANUAL_ASCII_CHECK,
            r#"
function classes(character: char): boolean {
    const lowercase = character >= 'a' && character <= 'z';
    const uppercase = 'A' <= character && 'Z' >= character;
    const digit = character < '0' || character > '9';

    return lowercase || uppercase || digit;
}
"#,
        );

        session.assert_fixes(
            r#"
function classes(character: char): boolean {
    const lowercase = character.isAsciiLowercase();
    const uppercase = character.isAsciiUppercase();
    const digit = !character.isAsciiDigit();

    return lowercase || uppercase || digit;
}
"#,
        );
    }

    /// Replace alphabetic and hexadecimal range unions.
    #[test]
    fn test_replaces_ascii_range_unions() {
        let session = TestSession::dir(
            &MANUAL_ASCII_CHECK,
            r#"
function classes(character: char): boolean {
    const alphabetic =
        (character >= 'a' && character <= 'z') ||
        (character >= 'A' && character <= 'Z');
    const alphanumeric =
        ('0'..='9').contains(character) ||
        ('a'..='z').contains(character) ||
        ('A'..='Z').contains(character);
    const hexadecimal =
        ('0'..='9').contains(character) ||
        ('a'..='f').contains(character) ||
        ('A'..='F').contains(character);

    return alphabetic || alphanumeric || hexadecimal;
}
"#,
        );

        session.assert_fixes(
            r#"
function classes(character: char): boolean {
    const alphabetic =
        character.isAsciiAlphabetic();
    const alphanumeric =
        character.isAsciiAlphanumeric();
    const hexadecimal =
        character.isAsciiHexDigit();

    return alphabetic || alphanumeric || hexadecimal;
}
"#,
        );
    }

    /// Replace boolean range matches in either result direction.
    #[test]
    fn test_replaces_ascii_range_matches() {
        let session = TestSession::dir(
            &MANUAL_ASCII_CHECK,
            r#"
function classes(character: char): boolean {
    const alphabetic = match (character) {
        'a'..='z' | 'A'..='Z' => true
        _ => false
    };
    const digit = match (character) {
        '0'..='9' => false
        _ => true
    };

    return alphabetic || digit;
}
"#,
        );

        session.assert_fixes(
            r#"
function classes(character: char): boolean {
    const alphabetic = character.isAsciiAlphabetic();
    const digit = !character.isAsciiDigit();

    return alphabetic || digit;
}
"#,
        );
    }

    /// Accept partial ranges, noncharacters, and effectful range unions.
    #[test]
    fn test_accepts_other_ranges() {
        let session = TestSession::dir(
            &MANUAL_ASCII_CHECK,
            r#"
declare function next(): char;

function accepted(character: char, number: int32): boolean {
    const partial = character >= 'a' && character <= 'f';
    const numberRange = number >= 0 && number <= 9;
    const effects =
        (next() >= 'a' && next() <= 'z') ||
        (next() >= 'A' && next() <= 'Z');

    return partial || numberRange || effects;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a disjunction of separately negated character ranges.
    #[test]
    fn test_accepts_negated_range_disjunction() {
        let session = TestSession::dir(
            &MANUAL_ASCII_CHECK,
            r#"
function accepted(character: char): boolean {
    return (character < 'a' || character > 'z')
        || (character < 'A' || character > 'Z');
}
"#,
        );

        session.assert_no_diagnostics();
    }
}

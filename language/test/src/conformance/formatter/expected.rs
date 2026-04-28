use std::path::Path;

use destack_source::{IndentStyle, LineEnding};
use destack_workspace::{
    ArrowParentheses, FormatterOptions, JsdocCommentLineStrategy, JsdocLineWrappingStyle,
    JsdocOptions, QuoteProperty, QuoteStyle, TrailingComma,
};

use crate::conformance::ExpectedOutput;

/// Parsed oxfmt expectation data.
#[derive(Debug, Clone)]
pub(super) struct OxfmtExpectedCase {
    pub output: String,
    pub formatter_options: FormatterOptions,
}

/// Load expected output content for a test case.
pub(super) fn load_expected_output(
    root: &Path,
    expected_output: &ExpectedOutput,
) -> Option<String> {
    match expected_output {
        ExpectedOutput::None => None,
        ExpectedOutput::PlainFile(path) => {
            let absolute_path = root.join(path);
            std::fs::read_to_string(absolute_path).ok()
        }
        ExpectedOutput::OxfmtSnapshot(path) => {
            let absolute_path = root.join(path);
            let snapshot = std::fs::read_to_string(absolute_path).ok()?;
            parse_oxfmt_snapshot_output(&snapshot)
        }
    }
}

/// Load expected output and formatter options from an oxfmt snapshot.
pub(super) fn load_oxfmt_expected_case(
    root: &Path,
    path: &Path,
    default_options: FormatterOptions,
) -> Option<OxfmtExpectedCase> {
    let absolute_path = root.join(path);
    let snapshot = std::fs::read_to_string(absolute_path).ok()?;
    let (output, options_line) = parse_oxfmt_snapshot_variant(&snapshot)?;

    let mut formatter_options = default_options;
    if let Some(options_line) = options_line {
        apply_oxfmt_options_line(&options_line, &mut formatter_options);
    }

    Some(OxfmtExpectedCase {
        output,
        formatter_options,
    })
}

/// Parse the output section from an oxfmt snapshot fixture.
fn parse_oxfmt_snapshot_output(snapshot: &str) -> Option<String> {
    let (output, _) = parse_oxfmt_snapshot_variant(snapshot)?;
    Some(output)
}

#[derive(Debug, Clone)]
struct OxfmtSnapshotVariant {
    output: String,
    options_line: Option<String>,
}

/// Parse the selected output variant and its option line from an oxfmt snapshot fixture.
fn parse_oxfmt_snapshot_variant(snapshot: &str) -> Option<(String, Option<String>)> {
    let variants = parse_oxfmt_snapshot_variants(snapshot);
    let mut first_variant: Option<OxfmtSnapshotVariant> = None;

    for variant in variants {
        if first_variant.is_none() {
            first_variant = Some(variant.clone());
        }
        let unsupported = variant
            .options_line
            .as_deref()
            .is_some_and(has_unsupported_options);
        if !unsupported {
            return Some((variant.output, variant.options_line));
        }
    }

    first_variant.map(|variant| (variant.output, variant.options_line))
}

/// Parse all output variants and option lines from an oxfmt snapshot fixture.
fn parse_oxfmt_snapshot_variants(snapshot: &str) -> Vec<OxfmtSnapshotVariant> {
    let marker = "==================== Output ====================";
    let Some(marker_index) = snapshot.find(marker) else {
        return Vec::new();
    };
    let output = snapshot[marker_index + marker.len()..].trim_start_matches('\n');
    let lines: Vec<&str> = output.lines().collect();
    let mut index = 0;
    let mut variants = Vec::new();

    while index < lines.len() {
        while index < lines.len() && lines[index].trim().is_empty() {
            index += 1;
        }

        if index >= lines.len() || lines[index].starts_with("=====================") {
            break;
        }

        let mut options_line = None;

        // options block between dashed separators
        if is_separator_line(lines[index]) {
            index += 1;
            let mut option_lines = Vec::new();
            while index < lines.len() && !is_separator_line(lines[index]) {
                if !lines[index].trim().is_empty() {
                    option_lines.push(lines[index].trim());
                }
                index += 1;
            }
            if index < lines.len() && is_separator_line(lines[index]) {
                index += 1;
            }
            if !option_lines.is_empty() {
                options_line = Some(option_lines.join(" "));
            }
        }

        let output_start = index;
        while index < lines.len()
            && !lines[index].starts_with("=====================")
            && !is_separator_line(lines[index])
        {
            index += 1;
        }

        let mut output_lines = lines[output_start..index].to_vec();
        while output_lines.last().is_some_and(|line| line.is_empty()) {
            output_lines.pop();
        }

        let mut variant_output = output_lines.join("\n");
        if !variant_output.is_empty() {
            variant_output.push('\n');
            variants.push(OxfmtSnapshotVariant {
                output: variant_output,
                options_line,
            });
        }
    }

    variants
}

/// Apply a single oxfmt options object line to formatter options.
fn apply_oxfmt_options_line(options_line: &str, options: &mut FormatterOptions) {
    let options_line = options_line.trim();
    let Some(inner) = options_line
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
    else {
        return;
    };

    for entry in split_option_entries(inner) {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let Some((raw_key, raw_value)) = entry.split_once(':') else {
            continue;
        };

        let key = raw_key.trim();
        let value = raw_value.trim();
        let value_string = strip_quotes(value);

        match key {
            "printWidth" => {
                if let Ok(width) = value.parse::<u16>() {
                    options.line_width = width;
                }
            }
            "tabWidth" => {
                if let Ok(width) = value.parse::<u8>() {
                    options.indent_width = width;
                }
            }
            "useTabs" => {
                if let Some(use_tabs) = parse_bool(value) {
                    options.indent_style = if use_tabs {
                        IndentStyle::Tab
                    } else {
                        IndentStyle::Space
                    };
                }
            }
            "singleQuote" => {
                if let Some(single_quote) = parse_bool(value) {
                    options.quote_style = if single_quote {
                        QuoteStyle::Single
                    } else {
                        QuoteStyle::Double
                    };
                }
            }
            "trailingComma" => {
                options.trailing_comma = match value_string {
                    "all" => TrailingComma::All,
                    "es5" => TrailingComma::Es5,
                    "none" => TrailingComma::None,
                    _ => options.trailing_comma,
                };
            }
            "bracketSpacing" => {
                if let Some(bracket_spacing) = parse_bool(value) {
                    options.bracket_spacing = bracket_spacing;
                }
            }
            "arrowParens" => {
                options.arrow_parentheses = match value_string {
                    "always" => ArrowParentheses::Always,
                    "avoid" => ArrowParentheses::Avoid,
                    _ => options.arrow_parentheses,
                };
            }
            "quoteProps" => {
                options.quote_property = match value_string {
                    "as-needed" => QuoteProperty::AsNeeded,
                    "consistent" => QuoteProperty::Consistent,
                    "preserve" => QuoteProperty::Preserve,
                    _ => options.quote_property,
                };
            }
            "endOfLine" => {
                options.line_ending = match value_string {
                    "lf" => LineEnding::LineFeed,
                    "crlf" => LineEnding::CarriageReturnLineFeed,
                    "cr" => LineEnding::CarriageReturn,
                    _ => options.line_ending,
                };
            }
            "bracketSameLine" => {
                if let Some(bracket_same_line) = parse_bool(value) {
                    options.bracket_same_line = bracket_same_line;
                }
            }
            "singleAttributePerLine" => {
                if let Some(single_attribute_per_line) = parse_bool(value) {
                    options.single_attribute_per_line = single_attribute_per_line;
                }
            }
            "jsdoc" => {
                let mut jsdoc_options = options.jsdoc.unwrap_or_default();
                apply_jsdoc_options_value(value, &mut jsdoc_options);
                options.jsdoc = Some(jsdoc_options);
            }
            "semi" => {}
            _ => {}
        }
    }
}

/// Apply a JSDoc options object value.
fn apply_jsdoc_options_value(value: &str, options: &mut JsdocOptions) {
    let value = value.trim();
    let Some(inner) = value
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
    else {
        return;
    };

    for entry in split_option_entries(inner) {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        let Some((raw_key, raw_value)) = entry.split_once(':') else {
            continue;
        };

        let key = raw_key.trim();
        let value = raw_value.trim();
        let value_string = strip_quotes(value);

        match key {
            "capitalizeDescriptions" | "capitalize_descriptions" => {
                if let Some(capitalize_descriptions) = parse_bool(value) {
                    options.capitalize_descriptions = capitalize_descriptions;
                }
            }
            "commentLineStrategy" | "comment_line_strategy" => {
                options.comment_line_strategy = match value_string {
                    "single-line" | "singleLine" | "single_line" => {
                        JsdocCommentLineStrategy::SingleLine
                    }
                    "multiline" => JsdocCommentLineStrategy::Multiline,
                    "keep" => JsdocCommentLineStrategy::Keep,
                    _ => options.comment_line_strategy,
                };
            }
            "separateTagGroups" | "separate_tag_groups" => {
                if let Some(separate_tag_groups) = parse_bool(value) {
                    options.separate_tag_groups = separate_tag_groups;
                }
            }
            "separateReturnsFromParam" | "separate_returns_from_param" => {
                if let Some(separate_returns_from_param) = parse_bool(value) {
                    options.separate_returns_from_param = separate_returns_from_param;
                }
            }
            "descriptionWithDot" | "description_with_dot" => {
                if let Some(description_with_dot) = parse_bool(value) {
                    options.description_with_dot = description_with_dot;
                }
            }
            "addDefaultToDescription" | "add_default_to_description" => {
                if let Some(add_default_to_description) = parse_bool(value) {
                    options.add_default_to_description = add_default_to_description;
                }
            }
            "preferCodeFences" | "prefer_code_fences" => {
                if let Some(prefer_code_fences) = parse_bool(value) {
                    options.prefer_code_fences = prefer_code_fences;
                }
            }
            "lineWrappingStyle" | "line_wrapping_style" => {
                options.line_wrapping_style = match value_string {
                    "greedy" => JsdocLineWrappingStyle::Greedy,
                    "balance" => JsdocLineWrappingStyle::Balance,
                    _ => options.line_wrapping_style,
                };
            }
            "descriptionTag" | "description_tag" => {
                if let Some(description_tag) = parse_bool(value) {
                    options.description_tag = description_tag;
                }
            }
            "keepUnparsableExampleIndent" | "keep_unparsable_example_indent" => {
                if let Some(keep_unparsable_example_indent) = parse_bool(value) {
                    options.keep_unparsable_example_indent = keep_unparsable_example_indent;
                }
            }
            _ => {}
        }
    }
}

/// Return whether an oxfmt options line uses unsupported formatter behavior.
fn has_unsupported_options(options_line: &str) -> bool {
    let options_line = options_line.trim();
    let Some(inner) = options_line
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
    else {
        return false;
    };

    for entry in split_option_entries(inner) {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        let Some((raw_key, raw_value)) = entry.split_once(':') else {
            continue;
        };

        let key = raw_key.trim();
        let value = raw_value.trim();
        if key == "semi" && parse_bool(value) == Some(false) {
            return true;
        }
    }

    false
}

/// Split one option object body into top-level entries.
fn split_option_entries(value: &str) -> Vec<&str> {
    let mut entries = Vec::new();
    let mut start = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut quote = None;
    let mut is_escaped = false;

    for (index, character) in value.char_indices() {
        if let Some(quote_character) = quote {
            if is_escaped {
                is_escaped = false;
            } else if character == '\\' {
                is_escaped = true;
            } else if character == quote_character {
                quote = None;
            }

            continue;
        }

        match character {
            '"' | '\'' => quote = Some(character),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ',' if brace_depth == 0 && bracket_depth == 0 && paren_depth == 0 => {
                entries.push(&value[start..index]);
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }

    entries.push(&value[start..]);

    entries
}

/// Parse a boolean option value.
fn parse_bool(value: &str) -> Option<bool> {
    match value.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// Strip single or double quotes around an option value.
fn strip_quotes(value: &str) -> &str {
    let value = value.trim();
    if let Some(stripped) = value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
    {
        return stripped;
    }
    if let Some(stripped) = value
        .strip_prefix('\'')
        .and_then(|inner| inner.strip_suffix('\''))
    {
        return stripped;
    }
    value
}

/// Return whether a line is a dashed separator.
fn is_separator_line(line: &str) -> bool {
    line.len() >= 3 && line.chars().all(|character| character == '-')
}

#[cfg(test)]
mod tests {
    use super::{
        apply_oxfmt_options_line, parse_oxfmt_snapshot_output, parse_oxfmt_snapshot_variant,
    };
    use destack_source::IndentStyle;
    use destack_workspace::{
        ArrowParentheses, FormatterOptions, JsdocCommentLineStrategy, JsdocLineWrappingStyle,
        JsdocOptions, QuoteProperty, QuoteStyle, TrailingComma,
    };

    #[test]
    fn test_parse_oxfmt_snapshot_output_with_options_block() {
        let snapshot = "==================== Output ====================\n------------------\n{ printWidth: 80 }\n------------------\nconst answer = 42;\n";
        let output = parse_oxfmt_snapshot_output(snapshot).expect("output should parse");
        assert_eq!(output, "const answer = 42;\n");
    }

    #[test]
    fn test_parse_oxfmt_snapshot_output_without_options_block() {
        let snapshot = "==================== Output ====================\nconst answer = 42;\n";
        let output = parse_oxfmt_snapshot_output(snapshot).expect("output should parse");
        assert_eq!(output, "const answer = 42;\n");
    }

    #[test]
    fn test_parse_oxfmt_snapshot_output_with_multiple_options_blocks() {
        let snapshot = "==================== Output ====================\n------------------\n{ printWidth: 80 }\n------------------\nconst answer = 42;\n\n-------------------\n{ printWidth: 100 }\n-------------------\nconst answer = 42;\n\n===================== End =====================\n";
        let output = parse_oxfmt_snapshot_output(snapshot).expect("output should parse");
        assert_eq!(output, "const answer = 42;\n");
    }

    #[test]
    fn test_parse_oxfmt_snapshot_variant_extracts_option_line() {
        let snapshot = "==================== Output ====================\n------------------\n{ printWidth: 80, singleQuote: true }\n------------------\nconst answer = 42;\n";
        let (_, options_line) =
            parse_oxfmt_snapshot_variant(snapshot).expect("output should parse");
        assert_eq!(
            options_line.as_deref(),
            Some("{ printWidth: 80, singleQuote: true }")
        );
    }

    #[test]
    fn test_apply_oxfmt_options_line() {
        let mut options = FormatterOptions::default();
        apply_oxfmt_options_line(
            "{ printWidth: 120, tabWidth: 3, useTabs: true, singleQuote: true, trailingComma: 'none', bracketSpacing: false, arrowParens: 'avoid', quoteProps: 'consistent' }",
            &mut options,
        );

        assert_eq!(options.line_width, 120);
        assert_eq!(options.indent_width, 3);
        assert_eq!(options.indent_style, IndentStyle::Tab);
        assert_eq!(options.quote_style, QuoteStyle::Single);
        assert!(!options.bracket_spacing);
        assert_eq!(options.arrow_parentheses, ArrowParentheses::Avoid);
        assert_eq!(options.quote_property, QuoteProperty::Consistent);
    }

    #[test]
    fn test_apply_oxfmt_options_line_with_nested_jsdoc_options() {
        let mut options = FormatterOptions::default();
        apply_oxfmt_options_line(
            "{ printWidth: 72, jsdoc: { commentLineStrategy: 'multiline', lineWrappingStyle: 'balance', separateTagGroups: true, keepUnparsableExampleIndent: true }, trailingComma: 'all' }",
            &mut options,
        );

        let expected_jsdoc = JsdocOptions {
            comment_line_strategy: JsdocCommentLineStrategy::Multiline,
            line_wrapping_style: JsdocLineWrappingStyle::Balance,
            separate_tag_groups: true,
            keep_unparsable_example_indent: true,
            ..JsdocOptions::default()
        };

        assert_eq!(options.line_width, 72);
        assert_eq!(options.trailing_comma, TrailingComma::All);
        assert_eq!(options.jsdoc, Some(expected_jsdoc));
    }

    #[test]
    fn test_parse_oxfmt_snapshot_variant_skips_unsupported_semi_false_variant() {
        let snapshot = "==================== Output ====================\n-------------------------------\n{ printWidth: 80, semi: false }\n-------------------------------\nconst x = 1\n\n------------------------------\n{ printWidth: 80, semi: true }\n------------------------------\nconst x = 1;\n";
        let (output, options_line) =
            parse_oxfmt_snapshot_variant(snapshot).expect("output should parse");
        assert_eq!(output, "const x = 1;\n");
        assert_eq!(
            options_line.as_deref(),
            Some("{ printWidth: 80, semi: true }")
        );
    }
}

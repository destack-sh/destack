use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::{Case, CaseResult, format_diagnostics};
use destack_ast::{NodeParentIndex, TokenSpan};
use destack_core::StringPool;
use destack_fir::format as fir_format;
use destack_formatter::{
    DestackFormatContext, DestackFormatOptions, format_file_source, statement_list,
};
use destack_parser::{Parser, ParserOptions, source_colorizer};
use destack_source::{
    DiagnosticCollection, DiagnosticSeverity, DiffOptions, File, FileId, FileType, IndentStyle,
    LanguageType, PrintOptions, Uri, print_diff,
};
use destack_workspace::{
    ArrowParentheses, FormatterOptions, QuoteProperty, QuoteStyle, TrailingComma,
};
use serde::Deserialize;

/// Formatter smoke category prefix.
pub(super) const SMOKE_CATEGORY: &str = "destack_test::formatter::smoke";

/// A formatter smoke case discovered from fixture files.
#[derive(Debug, Clone)]
pub(super) struct FormatterSmokeCase {
    /// The source input file.
    pub input_path: PathBuf,
    /// The expected output file when present.
    pub expected_path: Option<PathBuf>,
    /// The optional formatter options file.
    pub options_path: Option<PathBuf>,
}

/// Discover formatter smoke cases in a fixture directory.
pub(super) fn discover_cases(base_dir: &Path) -> Vec<(Case, FormatterSmokeCase)> {
    let mut cases = Vec::new();
    discover_cases_in_dir(base_dir, base_dir, &mut cases);
    cases.sort_by(|left, right| left.0.name.cmp(&right.0.name));
    cases
}

/// Recursively discover formatter smoke files in a directory.
fn discover_cases_in_dir(
    base_dir: &Path,
    directory: &Path,
    cases: &mut Vec<(Case, FormatterSmokeCase)>,
) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            if name.starts_with('.') || name == "staging" || name == "node_modules" {
                continue;
            }
            discover_cases_in_dir(base_dir, &path, cases);
            continue;
        }
        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !file_name.starts_with("input.") {
            continue;
        }

        let Some(file_type) = FileType::from_path(&path) else {
            continue;
        };
        if !is_smoke_file_type(file_type) {
            continue;
        }

        let Some(parent) = path.parent() else {
            continue;
        };
        let relative_parent = parent.strip_prefix(base_dir).unwrap_or(parent);
        let name = relative_parent.to_string_lossy().replace('\\', "/");
        if name.is_empty() {
            continue;
        }

        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let expected_path = parent.join(format!("expected.{extension}"));
        let expected_path = if expected_path.is_file() {
            Some(expected_path)
        } else {
            None
        };
        let options_path = parent.join("formatter.toml");
        let options_path = if options_path.is_file() {
            Some(options_path)
        } else {
            None
        };

        let is_skipped = relative_parent
            .components()
            .any(|component| component.as_os_str().to_string_lossy().starts_with('_'));
        let test = Case::file(name, path.clone(), SMOKE_CATEGORY).with_skipped(is_skipped);
        let case = FormatterSmokeCase {
            input_path: path,
            expected_path,
            options_path,
        };
        cases.push((test, case));
    }
}

/// Run one formatter smoke case.
pub(super) fn run(test: &Case, case: &FormatterSmokeCase) -> CaseResult {
    // load formatter options
    let formatter_options = match load_smoke_formatter_options(case.options_path.as_deref()) {
        Ok(options) => options,
        Err(message) => {
            return CaseResult::Failed {
                message: format!("invalid formatter options for '{}': {message}", test.name),
            };
        }
    };

    // load the source input
    let original = match std::fs::read_to_string(&case.input_path) {
        Ok(content) => content,
        Err(error) => {
            return CaseResult::Failed {
                message: format!(
                    "failed to read input '{}': {error}",
                    case.input_path.display()
                ),
            };
        }
    };

    // format the original source
    let first_pass = match format_source(&case.input_path, &original, formatter_options) {
        Ok(formatted) => formatted,
        Err(message) => {
            return CaseResult::Failed { message };
        }
    };

    // compare against an explicit expected output when present
    if let Some(expected_path) = &case.expected_path {
        let expected = match std::fs::read_to_string(expected_path) {
            Ok(content) => content,
            Err(error) => {
                return CaseResult::Failed {
                    message: format!(
                        "failed to read expected '{}': {error}",
                        expected_path.display()
                    ),
                };
            }
        };

        let expected = normalize_output(&expected);
        let actual = normalize_output(&first_pass);
        if actual == expected {
            return CaseResult::Passed;
        }

        print_diff(&expected, &actual, &DiffOptions::new());
        return CaseResult::Failed {
            message: format!("formatted output differs from expected for '{}'", test.name),
        };
    }

    // otherwise require idempotence
    let second_pass = match format_source(&case.input_path, &first_pass, formatter_options) {
        Ok(formatted) => formatted,
        Err(message) => {
            return CaseResult::Failed { message };
        }
    };

    let first_pass = normalize_output(&first_pass);
    let second_pass = normalize_output(&second_pass);
    if first_pass == second_pass {
        return CaseResult::Passed;
    }

    print_diff(&first_pass, &second_pass, &DiffOptions::new());
    CaseResult::Failed {
        message: format!("formatter is not idempotent for '{}'", test.name),
    }
}

/// Format a single source file with formatter defaults.
fn format_source(path: &Path, source: &str, formatter: FormatterOptions) -> Result<String, String> {
    // materialize the source file
    let file_type = FileType::from_path(path)
        .ok_or_else(|| format!("unsupported file type: {}", path.display()))?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("input")
        .to_string();
    let uri = Uri::from_path(path);
    let file_id = FileId::from_logical_path(path);
    let file = Arc::new(File::from_text(
        file_id,
        name,
        uri,
        Some(path.to_path_buf()),
        file_type,
        source.to_string(),
    ));
    let file_for_id = |current_file_id| {
        if current_file_id == file_id {
            Some(file.clone())
        } else {
            None
        }
    };

    // dispatch css and html through the shared formatter path directly
    if matches!(file_type, FileType::Css | FileType::Html) {
        return format_file_source(&file, source, formatter).map_err(|error| error.to_string());
    }

    // parse parser driven languages for diagnostic-rich failures
    let language_type = LanguageType::try_from(file_type).expect("file type has no parser language");
    let mut parser = Parser::lex_file_with_options(
        file.clone(),
        language_type,
        ParserOptions {
            preserve_parenthesized_wrappers: false,
            ..ParserOptions::default()
        },
        Arc::new(StringPool::new()),
    );
    let expressions = parser.parse();

    let has_errors = parser
        .diagnostics
        .to_vec()
        .into_iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);
    if has_errors {
        let mut diagnostics = DiagnosticCollection::new();
        for diagnostic in parser.diagnostics.to_vec() {
            diagnostics.insert(diagnostic);
        }
        let options = PrintOptions::new().with_colorizer(source_colorizer());
        let rendered = format_diagnostics(&file_for_id, &diagnostics, options);
        return Err(format!(
            "parse errors in '{}':\n\n{rendered}",
            path.display()
        ));
    }

    let (tokens, side_tokens) = parser.take_tokens();
    Ok(format_expressions(
        &parser,
        &tokens,
        &side_tokens,
        &expressions,
        &file,
        language_type,
        formatter,
    ))
}

/// Format parsed expressions into source code.
fn format_expressions(
    parser: &Parser,
    tokens: &Vec<TokenSpan>,
    side_tokens: &Vec<TokenSpan>,
    expressions: &[destack_ast::LocalNodeId<destack_ast::Expression>],
    file: &File,
    language_type: LanguageType,
    formatter: FormatterOptions,
) -> String {
    // build formatter context
    let side_span = parser.compute_side_span();
    let strings = parser.strings.as_ref();
    let parents = NodeParentIndex::from_expression_roots(&parser.tree, expressions);

    // convert options and format
    let format_options = DestackFormatOptions::from_formatter_options(formatter, language_type);
    let context = DestackFormatContext::new(
        format_options,
        file,
        &parser.tree,
        tokens,
        side_tokens,
        &side_span,
        strings,
        parents,
    );

    // ensure a trailing newline
    let mut result = if expressions.is_empty() {
        String::new()
    } else {
        let formatted = fir_format!(context.clone(), [statement_list(expressions)]).unwrap();
        let printed = formatted.print().unwrap();
        printed.as_str().to_string()
    };

    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    result
}

/// Normalize output for stable comparisons.
fn normalize_output(content: &str) -> String {
    // trim trailing whitespace and normalize the final newline
    let lines: Vec<&str> = content.lines().map(|line| line.trim_end()).collect();
    let mut result = lines.join("\n");

    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Return whether this file type is supported by formatter smoke cases.
fn is_smoke_file_type(file_type: FileType) -> bool {
    matches!(
        file_type,
        FileType::Destack
            | FileType::DestackDeclaration
            | FileType::JavaScript
            | FileType::JavaScriptXml
            | FileType::Css
            | FileType::Html
            | FileType::TypeScript
            | FileType::TypeScriptXml
            | FileType::TypeScriptDeclaration
    )
}

/// Load formatter options for one smoke fixture directory.
fn load_smoke_formatter_options(path: Option<&Path>) -> Result<FormatterOptions, String> {
    let Some(path) = path else {
        return Ok(FormatterOptions::default());
    };

    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    let options_file = toml::from_str::<SmokeFormatterOptionsFile>(&content)
        .map_err(|error| format!("failed to parse '{}': {error}", path.display()))?;
    let mut options = FormatterOptions::default();

    if let Some(indent_width) = options_file.indent_width {
        options = options.with_indent_width(indent_width);
    }

    if let Some(line_width) = options_file.line_width {
        options = options.with_line_width(line_width);
    }

    if let Some(indent_style) = options_file.indent_style {
        let indent_style = match indent_style.as_str() {
            "space" => IndentStyle::Space,
            "tab" => IndentStyle::Tab,
            _ => {
                return Err(format!("unsupported indent-style `{indent_style}`"));
            }
        };
        options = options.with_indent_style(indent_style);
    }

    if let Some(quote_style) = options_file.quote_style {
        let quote_style = match quote_style.as_str() {
            "double" => QuoteStyle::Double,
            "single" => QuoteStyle::Single,
            "semantic" => QuoteStyle::Semantic,
            _ => {
                return Err(format!("unsupported quote-style `{quote_style}`"));
            }
        };
        options = options.with_quote_style(quote_style);
    }

    if let Some(trailing_comma) = options_file.trailing_comma {
        let trailing_comma = match trailing_comma.as_str() {
            "all" => TrailingComma::All,
            "es5" => TrailingComma::Es5,
            "none" => TrailingComma::None,
            _ => {
                return Err(format!("unsupported trailing-comma `{trailing_comma}`"));
            }
        };
        options = options.with_trailing_comma(trailing_comma);
    }

    if let Some(bracket_spacing) = options_file.bracket_spacing {
        options = options.with_bracket_spacing(bracket_spacing);
    }

    if let Some(arrow_parentheses) = options_file.arrow_parentheses {
        let arrow_parentheses = match arrow_parentheses.as_str() {
            "always" => ArrowParentheses::Always,
            "avoid" => ArrowParentheses::Avoid,
            _ => {
                return Err(format!(
                    "unsupported arrow-parentheses `{arrow_parentheses}`"
                ));
            }
        };
        options = options.with_arrow_parens(arrow_parentheses);
    }

    if let Some(quote_property) = options_file.quote_property {
        let quote_property = match quote_property.as_str() {
            "as-needed" => QuoteProperty::AsNeeded,
            "consistent" => QuoteProperty::Consistent,
            "preserve" => QuoteProperty::Preserve,
            _ => {
                return Err(format!("unsupported quote-property `{quote_property}`"));
            }
        };
        options = options.with_quote_props(quote_property);
    }

    Ok(options)
}

/// The formatter options supported by smoke fixtures.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct SmokeFormatterOptionsFile {
    /// The optional indent width.
    indent_width: Option<u8>,
    /// The optional line width.
    line_width: Option<u16>,
    /// The optional indent style string.
    indent_style: Option<String>,
    /// The optional quote style string.
    quote_style: Option<String>,
    /// The optional trailing comma string.
    trailing_comma: Option<String>,
    /// The optional bracket spacing flag.
    bracket_spacing: Option<bool>,
    /// The optional arrow parentheses string.
    arrow_parentheses: Option<String>,
    /// The optional quote property string.
    quote_property: Option<String>,
}

use std::path::Path;

use libtest_mimic::{Failed, Trial};
use tspp_repository::FormatterOptions;
use tspp_source::{DiagnosticSeverity, DiffOptions, IndentStyle, format_diff};

use crate::format::format_source;
use crate::markdown::{Transform, markdown_files, parse_transforms};

/// Discover every formatter transformation as a test trial.
pub(super) fn trials(directory: &Path) -> Result<Vec<Trial>, String> {
    let mut trials = Vec::new();

    // parse every document before executing any transformation
    for path in markdown_files(directory)? {
        for transform in parse_transforms(&path, directory)? {
            let name = transform.name.clone();
            let trial = Trial::test(name, move || run(transform));
            trials.push(trial);
        }
    }

    Ok(trials)
}

/// Format one transformation and compare its complete output.
fn run(transform: Transform) -> Result<(), Failed> {
    let options = formatter_options(&transform).map_err(Failed::from)?;
    let formatted = format_source(
        &transform.path,
        None,
        transform.source,
        options,
        DiagnosticSeverity::Error,
    )
    .map_err(Failed::from)?;
    let expected = normalize(&transform.expected);
    let formatted = normalize(&formatted);
    if formatted == expected {
        return Ok(());
    }
    let difference = format_diff(&expected, &formatted, &DiffOptions::new());

    Err(Failed::from(format!(
        "formatted output differs\n\n{difference}"
    )))
}

/// Parse the formatter options declared by one transformation.
fn formatter_options(transform: &Transform) -> Result<FormatterOptions, String> {
    let mut options = FormatterOptions::default();

    // apply each supported option exactly once
    for (name, value) in &transform.options {
        options = match name.as_str() {
            "line-width" => options.with_line_width(value.parse().map_err(|error| {
                format!(
                    "invalid line width '{value}' in '{}': {error}",
                    transform.name
                )
            })?),
            "indent-width" => options.with_indent_width(value.parse().map_err(|error| {
                format!(
                    "invalid indent width '{value}' in '{}': {error}",
                    transform.name
                )
            })?),
            "indent-style" => options.with_indent_style(match value.as_str() {
                "space" => IndentStyle::Space,
                "tab" => IndentStyle::Tab,
                _ => {
                    return Err(format!(
                        "invalid indent style '{value}' in '{}'",
                        transform.name
                    ));
                }
            }),
            _ => {
                return Err(format!(
                    "unknown formatter option '{name}' in '{}'",
                    transform.name
                ));
            }
        };
    }

    Ok(options)
}

/// Normalize trailing whitespace and the final newline.
fn normalize(source: &str) -> String {
    let mut output = source
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    if !output.is_empty() {
        output.push('\n');
    }

    output
}

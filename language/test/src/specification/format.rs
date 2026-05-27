use std::collections::HashMap;
use std::path::{Path, PathBuf};

use destack_formatter::format_file_source;
use destack_source::{DiffOptions, File, FileId, FileType, IndentStyle, Uri, print_diff};
use destack_workspace::FormatterOptions;

use crate::core::{Case, CaseResult, RunContext, RunOptions, Suite, fixtures_dir};
use crate::mdtest::{MdTestCase, discover_md_files, parse_mdtest_file, slug};

/// The case category for specification format checks.
const CATEGORY: &str = "destack_test::specification::format";

/// One source block extracted from a specification markdown file.
#[derive(Debug, Clone)]
struct SpecificationSourceBlock {
    /// The markdown fixture path.
    markdown_path: PathBuf,
    /// The source filename from the code block.
    source_path: PathBuf,
    /// The source text.
    source: String,
    /// Formatter options from the code block.
    options: FormatterOptions,
}

/// Specification source block formatter check suite.
#[derive(Debug, Default)]
pub struct SpecificationFormatSuite {
    blocks: HashMap<String, SpecificationSourceBlock>,
    cases: Vec<Case>,
}

impl SpecificationFormatSuite {
    /// Load source blocks from specification markdown fixtures.
    pub fn load() -> Result<Self, String> {
        let mut suite = Self::default();
        let spec_dir = fixtures_dir().join("specification");

        for markdown_path in discover_md_files(&spec_dir).map_err(|error| error.to_string())? {
            let tests = parse_mdtest_file(&markdown_path)
                .map_err(|error| format!("failed to parse {}: {error}", markdown_path.display()))?;
            suite.add_file(&spec_dir, &markdown_path, tests)?;
        }

        Ok(suite)
    }

    /// Add source blocks from one markdown file.
    fn add_file(
        &mut self,
        spec_dir: &Path,
        markdown_path: &Path,
        tests: Vec<MdTestCase>,
    ) -> Result<(), String> {
        let relative_path = markdown_path
            .strip_prefix(spec_dir)
            .unwrap_or(markdown_path);
        let relative_name = relative_path.to_string_lossy();

        for test in tests {
            if test.skip {
                continue;
            }

            for (index, file) in test.files.into_iter().enumerate() {
                let source_path = PathBuf::from(&file.path);
                let Some(file_type) = FileType::from_path(&source_path) else {
                    continue;
                };
                if !is_format_file_type(file_type) {
                    continue;
                }

                let options = formatter_options_from_block(&file.options)?;
                let name = format!(
                    "{relative_name}/{}/{}/{}:{index}",
                    slug(&test.section),
                    slug(&test.name),
                    file.path,
                );
                let case = Case::file(name, markdown_path.to_path_buf(), CATEGORY);
                let block = SpecificationSourceBlock {
                    markdown_path: markdown_path.to_path_buf(),
                    source_path,
                    source: file.content,
                    options,
                };

                self.blocks.insert(case.full_name(), block);
                self.cases.push(case);
            }
        }

        Ok(())
    }
}

impl Suite for SpecificationFormatSuite {
    fn name(&self) -> &'static str {
        "specification-format"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        self.cases.clone()
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        let Some(block) = self.blocks.get(&case.full_name()) else {
            return CaseResult::Failed {
                message: "source block not found".to_string(),
            };
        };

        match check_source_block(block) {
            Ok(()) => CaseResult::Passed,
            Err(message) => CaseResult::Failed { message },
        }
    }
}

/// Format specification source blocks in place.
pub fn format_specification_fixtures() -> Result<usize, String> {
    let spec_dir = fixtures_dir().join("specification");
    let mut formatted_files = Vec::new();

    for markdown_path in discover_md_files(&spec_dir).map_err(|error| error.to_string())? {
        let source = std::fs::read_to_string(&markdown_path)
            .map_err(|error| format!("failed to read {}: {error}", markdown_path.display()))?;
        let formatted = format_markdown_source_blocks(&markdown_path, &source)?;

        if formatted != source {
            formatted_files.push((markdown_path, formatted));
        }
    }

    let changed = formatted_files.len();
    for (markdown_path, formatted) in formatted_files {
        std::fs::write(&markdown_path, formatted)
            .map_err(|error| format!("failed to write {}: {error}", markdown_path.display()))?;
    }

    Ok(changed)
}

/// Return whether one source block parses, formats, and formats idempotently.
fn check_source_block(block: &SpecificationSourceBlock) -> Result<(), String> {
    let first = format_source(
        &block.markdown_path,
        &block.source_path,
        &block.source,
        block.options,
    )?;
    let second = format_source(
        &block.markdown_path,
        &block.source_path,
        &first,
        block.options,
    )?;

    if block.source != first {
        print_diff(&block.source, &first, &DiffOptions::new());

        return Err(format!(
            "{} / {}: source block is not formatted",
            block.markdown_path.display(),
            block.source_path.display()
        ));
    }

    if first != second {
        print_diff(&first, &second, &DiffOptions::new());

        return Err(format!(
            "{} / {}: formatter is not idempotent",
            block.markdown_path.display(),
            block.source_path.display()
        ));
    }

    Ok(())
}

/// Format all source code blocks in one markdown document.
fn format_markdown_source_blocks(markdown_path: &Path, source: &str) -> Result<String, String> {
    let mut result = String::with_capacity(source.len());
    let mut lines = source.split_inclusive('\n').peekable();

    while let Some(line) = lines.next() {
        let Some(tag) = opening_fence_tag(line) else {
            result.push_str(line);
            continue;
        };

        let mut content = String::new();
        let mut closing = None;
        for line in lines.by_ref() {
            if is_closing_fence(line) {
                closing = Some(line);
                break;
            }
            content.push_str(line);
        }

        let Some(closing) = closing else {
            return Err(format!(
                "unterminated code fence in {}",
                markdown_path.display()
            ));
        };

        result.push_str(line);
        if let Some(block) = parse_fence_block(tag)? {
            let content = trim_fence_content(&content);
            let formatted = format_source(markdown_path, &block.path, content, block.options)?;
            push_formatted_fence_content(&mut result, &formatted);
        } else {
            result.push_str(&content);
        }
        result.push_str(closing);
    }

    Ok(result)
}

/// Format one source block.
fn format_source(
    markdown_path: &Path,
    source_path: &Path,
    source: &str,
    options: FormatterOptions,
) -> Result<String, String> {
    let file_type = FileType::from_path(source_path)
        .ok_or_else(|| format!("unsupported source block: {}", source_path.display()))?;
    let file_id = FileId::from_logical_path(source_path);
    let file = File::from_text(
        file_id,
        source_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("main.ds")
            .to_string(),
        Uri::from_path(source_path),
        Some(source_path.to_path_buf()),
        file_type,
        source.to_string(),
    );

    format_file_source(&file, source, options).map_err(|error| {
        format!(
            "{} / {}: {}",
            markdown_path.display(),
            source_path.display(),
            error.message
        )
    })
}

/// One parsed source code fence.
#[derive(Debug)]
struct FenceBlock {
    /// The synthetic source path.
    path: PathBuf,
    /// Formatter options from the fence tag.
    options: FormatterOptions,
}

/// Parse one markdown fence tag as a source code block.
fn parse_fence_block(tag: &str) -> Result<Option<FenceBlock>, String> {
    let parsed = parse_fence_tag(tag);
    if parsed.markers.iter().any(|marker| *marker == "expected") {
        return Ok(None);
    }

    let Some(extension) = extension_for_language(parsed.base) else {
        return Ok(None);
    };
    let path = parsed
        .filename
        .map_or_else(|| PathBuf::from(format!("main.{extension}")), PathBuf::from);
    let options = formatter_options_from_block(&parsed.options)?;

    Ok(Some(FenceBlock { path, options }))
}

/// Parsed markdown fence tag.
struct FenceTag<'a> {
    /// The source language.
    base: &'a str,
    /// The optional filename after `:`.
    filename: Option<&'a str>,
    /// Marker words after the language.
    markers: Vec<&'a str>,
    /// Key-value options after the language.
    options: HashMap<String, String>,
}

/// Parse a markdown fence tag.
fn parse_fence_tag(tag: &str) -> FenceTag<'_> {
    let mut parts = tag.split_whitespace();
    let first = parts.next().unwrap_or("");
    let (base, filename) = first
        .split_once(':')
        .map_or((first, None), |(base, filename)| (base, Some(filename)));
    let mut markers = Vec::new();
    let mut options = HashMap::new();

    for part in parts {
        if let Some((key, value)) = part.split_once('=') {
            options.insert(key.to_string(), value.to_string());
        } else {
            markers.push(part);
        }
    }

    FenceTag {
        base,
        filename,
        markers,
        options,
    }
}

/// Build formatter options for one source block.
fn formatter_options_from_block(
    options: &HashMap<String, String>,
) -> Result<FormatterOptions, String> {
    let mut formatter = FormatterOptions::default();

    if let Some(line_width) = option_value(options, &["line-width", "line_width", "printWidth"]) {
        let line_width = line_width
            .parse::<u16>()
            .map_err(|error| format!("invalid line-width '{line_width}': {error}"))?;
        formatter = formatter.with_line_width(line_width);
    }

    if let Some(indent_width) = option_value(options, &["indent-width", "indent_width", "tabWidth"])
    {
        let indent_width = indent_width
            .parse::<u8>()
            .map_err(|error| format!("invalid indent-width '{indent_width}': {error}"))?;
        formatter = formatter.with_indent_width(indent_width);
    }

    if let Some(indent_style) = option_value(options, &["indent-style", "indent_style"]) {
        let indent_style = match indent_style {
            "tab" => IndentStyle::Tab,
            "space" => IndentStyle::Space,
            _ => return Err(format!("invalid indent-style '{indent_style}'")),
        };
        formatter = formatter.with_indent_style(indent_style);
    }

    Ok(formatter)
}

/// Return the first present option value.
fn option_value<'a>(options: &'a HashMap<String, String>, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        if let Some(value) = options.get(*key) {
            return Some(value);
        }
    }

    None
}

/// Return whether one file type can be formatted.
fn is_format_file_type(file_type: FileType) -> bool {
    matches!(
        file_type,
        FileType::Destack
            | FileType::DestackDeclaration
            | FileType::JavaScript
            | FileType::JavaScriptXml
            | FileType::TypeScript
            | FileType::TypeScriptXml
            | FileType::TypeScriptDeclaration
            | FileType::Css
            | FileType::Html
    )
}

/// Return the default extension for one fence language.
fn extension_for_language(language: &str) -> Option<&'static str> {
    match language.to_ascii_lowercase().as_str() {
        "ds" | "destack" => Some("ds"),
        "d.ds" | "destack-declaration" => Some("d.ds"),
        "ts" | "typescript" => Some("ts"),
        "d.ts" | "typescript-declaration" => Some("d.ts"),
        "tsx" => Some("tsx"),
        "js" | "javascript" => Some("js"),
        "jsx" => Some("jsx"),
        "css" => Some("css"),
        "html" => Some("html"),
        _ => None,
    }
}

/// Return a code fence tag from one opening fence line.
fn opening_fence_tag(line: &str) -> Option<&str> {
    let line = line.trim_end_matches(['\r', '\n']);
    let line = line.trim_start();
    let tag = line.strip_prefix("```")?;
    (!tag.starts_with('`')).then_some(tag.trim())
}

/// Return whether one line closes a code fence.
fn is_closing_fence(line: &str) -> bool {
    let line = line.trim();
    line == "```"
}

/// Trim fence syntax newline from source content.
fn trim_fence_content(content: &str) -> &str {
    content
        .strip_suffix("\r\n")
        .or_else(|| content.strip_suffix('\n'))
        .unwrap_or(content)
}

/// Push formatted source content before a closing fence.
fn push_formatted_fence_content(result: &mut String, formatted: &str) {
    result.push_str(formatted);
    if !formatted.ends_with('\n') {
        result.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use destack_source::IndentStyle;

    use super::*;

    #[test]
    fn test_parse_fence_block_uses_source_aliases() {
        let block = parse_fence_block("typescript:example.ts printWidth=90")
            .expect("parse fence")
            .expect("source block");

        assert_eq!(block.path, PathBuf::from("example.ts"));
        assert_eq!(block.options.line_width, 90);
    }

    #[test]
    fn test_parse_fence_block_skips_expected_blocks() {
        let block = parse_fence_block("ds expected").expect("parse fence");

        assert!(block.is_none());
    }

    #[test]
    fn test_parse_fence_block_accepts_tab_options() {
        let block = parse_fence_block("ds indent-style=tab")
            .expect("parse fence")
            .expect("source block");

        assert_eq!(block.options.indent_style, IndentStyle::Tab);
    }

    #[test]
    fn test_trim_fence_content_removes_one_line_ending() {
        assert_eq!(
            trim_fence_content("const value = 1;\r\n"),
            "const value = 1;"
        );
        assert_eq!(trim_fence_content("const value = 1;\n"), "const value = 1;");
    }
}

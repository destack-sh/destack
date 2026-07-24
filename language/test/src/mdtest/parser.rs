use std::collections::HashMap;
use std::ops::Range;
use std::path::Path;

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

/// One source or data file in a Markdown test case.
#[derive(Debug, Clone)]
pub struct MdTestFile {
    /// The workspace-relative file path.
    pub path: String,
    /// The exact file contents.
    pub content: String,
    /// The options declared by the fenced block language tag.
    pub options: HashMap<String, String>,
}

/// One fenced code block retained for suite-specific interpretation.
#[derive(Debug, Clone)]
pub struct RawCodeBlock {
    /// The complete fenced code block language tag.
    pub language: String,
    /// The block content.
    pub content: String,
    /// The exact content range in the Markdown source.
    pub content_range: Option<Range<usize>>,
}

/// One test case extracted from Markdown.
#[derive(Debug, Clone)]
pub struct MdTestCase {
    /// The test name from its H3 or H4 heading.
    pub name: String,
    /// The containing H2 section.
    pub section: String,
    /// The options declared for the complete test.
    pub options: HashMap<String, String>,
    /// The source and data files.
    pub files: Vec<MdTestFile>,
    /// The expected diagnostic bullet items.
    pub bullet_items: Vec<String>,
    /// The fenced blocks retained for suite-specific interpretation.
    pub extra_blocks: Vec<RawCodeBlock>,
    /// The Markdown source line where this test starts.
    pub line: usize,
    /// Whether the test heading begins with `_`.
    pub skip: bool,
}

/// Parse one Markdown file into test cases.
pub fn parse_mdtest_file(path: &Path) -> std::io::Result<Vec<MdTestCase>> {
    let content = std::fs::read_to_string(path)?;
    parse_mdtest(&content)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

/// One parsed language tag.
struct ParsedLanguageTag<'a> {
    /// The base language.
    base: &'a str,
    /// The optional positional or colon-delimited file path.
    filename: Option<&'a str>,
    /// The space-separated markers.
    markers: Vec<&'a str>,
    /// The key-value options.
    options: HashMap<String, String>,
}

/// Parse one fenced code block language tag.
///
/// A file path may follow the language as `ds main.ds` or `ds:main.ds`.
fn parse_language_tag(language: &str) -> Result<ParsedLanguageTag<'_>, String> {
    let mut parts = language.split_whitespace();
    let first = parts.next().unwrap_or("");

    // split a colon-delimited file path from the base language
    let (base, mut filename) = if let Some(colon_pos) = first.find(':') {
        (&first[..colon_pos], Some(&first[colon_pos + 1..]))
    } else {
        (first, None)
    };

    let mut markers = Vec::new();
    let mut options = HashMap::new();

    for part in parts {
        if let Some(eq_pos) = part.find('=') {
            let key = &part[..eq_pos];
            let value = &part[eq_pos + 1..];
            if key.is_empty() {
                return Err(format!("code block option '{part}' has no name"));
            }
            if options.insert(key.to_string(), value.to_string()).is_some() {
                return Err(format!("code block repeats option '{key}'"));
            }
        } else if matches!(part, "expected" | "after") {
            if markers.contains(&part) {
                return Err(format!("code block repeats marker '{part}'"));
            }
            markers.push(part);
        } else if filename.is_none() {
            filename = Some(part);
        } else {
            markers.push(part);
        }
    }

    Ok(ParsedLanguageTag {
        base,
        filename,
        markers,
        options,
    })
}

/// Normalize a test option key for profile selection.
fn normalize_test_option_key(key: &str) -> Option<&'static str> {
    match key {
        "lib" | "libs" => Some("libs"),
        "emit" | "format" => Some("emit"),
        "runtime" => Some("runtime"),
        "runtime_version" | "runtime-version" | "runtimeVersion" => Some("runtime_version"),
        "platform" => Some("platform"),
        "native" => Some("native"),
        "debug" => Some("debug"),
        _ => None,
    }
}

/// Insert one test option.
fn insert_test_option(
    test_name: Option<&str>,
    options: &mut HashMap<String, String>,
    key: String,
    value: String,
) -> Result<(), String> {
    if let Some(existing) = options.get(&key)
        && existing != &value
    {
        let name = test_name.unwrap_or("<unknown>");
        return Err(format!(
            "conflicting test option '{key}' for '{name}': '{existing}' vs '{value}'"
        ));
    }

    options.insert(key, value);

    Ok(())
}

/// Apply test options from a `test` block.
fn apply_test_options(
    test_name: Option<&str>,
    options: HashMap<String, String>,
    test_options: &mut HashMap<String, String>,
) -> Result<(), String> {
    for (key, value) in options {
        let normalized = normalize_test_option_key(&key).unwrap_or(&key);
        insert_test_option(test_name, test_options, normalized.to_string(), value)?;
    }

    Ok(())
}

/// Split test options from per file options.
fn split_test_options(
    test_name: Option<&str>,
    options: HashMap<String, String>,
    test_options: &mut HashMap<String, String>,
) -> Result<HashMap<String, String>, String> {
    let mut file_options = HashMap::new();

    for (key, value) in options {
        if let Some(normalized) = normalize_test_option_key(&key) {
            insert_test_option(test_name, test_options, normalized.to_string(), value)?;
        } else {
            file_options.insert(key, value);
        }
    }

    Ok(file_options)
}

/// Check if a language tag is a supported source code language.
fn is_code_language(language: &str) -> bool {
    let lower = language.to_lowercase();
    matches!(lower.as_str(), "ds" | "destack")
}

/// Check if a language tag is a data/text file type that can be imported.
fn is_data_language(language: &str) -> bool {
    let lower = language.to_lowercase();
    matches!(
        lower.as_str(),
        "json" | "toml" | "yaml" | "yml" | "text" | "txt" | "env"
    )
}

/// Parse Markdown source into test cases.
pub fn parse_mdtest(content: &str) -> Result<Vec<MdTestCase>, String> {
    let parser = Parser::new(content).into_offset_iter();

    let mut tests = Vec::new();
    let mut current_section: Option<String> = None;
    let mut current_test_name: Option<String> = None;
    let mut current_test_skip = false;
    let mut current_options: HashMap<String, String> = HashMap::new();
    let mut current_files: Vec<MdTestFile> = Vec::new();
    let mut current_bullets: Vec<String> = Vec::new();
    let mut current_extra_blocks: Vec<RawCodeBlock> = Vec::new();
    let mut in_heading = false;
    let mut heading_level: Option<HeadingLevel> = None;
    let mut heading_text = String::new();
    let mut in_code_block = false;
    let mut code_block_language = String::new();
    let mut code_block_content = String::new();
    let mut code_block_content_range = None;
    let mut in_list_item = false;
    let mut list_item_text = String::new();
    let mut current_line = 1;
    let mut errors = Vec::new();

    for (event, event_range) in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                // finalize the previous test before a new heading
                finalize_pending_test(
                    &mut tests,
                    current_section.as_ref(),
                    &mut current_test_name,
                    &mut current_test_skip,
                    &mut current_options,
                    &mut current_files,
                    &mut current_bullets,
                    &mut current_extra_blocks,
                    current_line,
                    &mut errors,
                );

                in_heading = true;
                heading_level = Some(level);
                heading_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                let text = heading_text.trim().to_string();

                match heading_level {
                    Some(HeadingLevel::H2) => {
                        current_section = Some(text);
                        current_test_name = None;
                        current_test_skip = false;
                        current_options.clear();
                        current_files.clear();
                        current_bullets.clear();
                        current_extra_blocks.clear();
                    }
                    Some(HeadingLevel::H3 | HeadingLevel::H4) => {
                        if current_section.is_none() {
                            errors.push(format!("test '{text}' appears before a section heading"));
                        }
                        let (name, skip) = if let Some(stripped) = text.strip_prefix('_') {
                            (stripped.to_string(), true)
                        } else {
                            (text, false)
                        };
                        current_test_name = Some(name);
                        current_test_skip = skip;
                        current_options.clear();
                        current_files.clear();
                        current_bullets.clear();
                        current_extra_blocks.clear();
                    }
                    _ => {}
                }
                heading_level = None;
            }
            Event::Text(text) => {
                if in_heading {
                    heading_text.push_str(&text);
                } else if in_code_block {
                    code_block_content.push_str(&text);
                    extend_range(&mut code_block_content_range, event_range);
                } else if in_list_item {
                    list_item_text.push_str(&text);
                }
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                if in_heading {
                    heading_text.push_str(&html);
                } else if in_list_item {
                    list_item_text.push_str(&html);
                }
            }
            Event::Code(code) => {
                if in_heading {
                    heading_text.push_str(&code);
                } else if in_list_item {
                    list_item_text.push_str(&code);
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_block_content.clear();
                code_block_content_range = None;
                code_block_language = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(language) => language.to_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                if let Some(range) = &code_block_content_range
                    && content.get(range.clone()) != Some(code_block_content.as_str())
                {
                    errors.push(format!(
                        "code block '{code_block_language}' content range does not match its text"
                    ));
                }

                // classify the fenced block
                let parsed = match parse_language_tag(&code_block_language) {
                    Ok(parsed) => parsed,
                    Err(error) => {
                        errors.push(error);

                        continue;
                    }
                };
                let is_expected = parsed.markers.contains(&"expected");
                let is_after = parsed.markers.contains(&"after");
                if is_expected && is_after {
                    errors.push(format!(
                        "code block '{code_block_language}' cannot be both expected and after"
                    ));

                    continue;
                }

                if current_test_name.is_none() && !code_block_language.is_empty() {
                    errors.push(format!(
                        "code block '{code_block_language}' appears outside a test heading"
                    ));
                }

                if parsed.base.eq_ignore_ascii_case("test") {
                    if current_test_name.is_some() {
                        let result = apply_test_options(
                            current_test_name.as_deref(),
                            parsed.options,
                            &mut current_options,
                        );
                        if let Err(error) = result {
                            errors.push(error);
                        }
                    }
                } else if is_code_language(parsed.base) && !is_expected && !is_after {
                    if !parsed.markers.is_empty() {
                        errors.push(format!(
                            "source block '{code_block_language}' has unknown markers"
                        ));

                        continue;
                    }

                    // record a source file
                    let path = parsed.filename.unwrap_or("main.ds").to_string();
                    let options = split_test_options(
                        current_test_name.as_deref(),
                        parsed.options,
                        &mut current_options,
                    );
                    let options = match options {
                        Ok(options) => options,
                        Err(error) => {
                            errors.push(error);

                            continue;
                        }
                    };
                    current_files.push(MdTestFile {
                        path,
                        content: code_block_content.clone(),
                        options,
                    });
                } else if !is_expected
                    && !is_after
                    && let Some(filename) =
                        parsed.filename.filter(|_| is_data_language(parsed.base))
                {
                    if !parsed.markers.is_empty() {
                        errors.push(format!(
                            "data block '{code_block_language}' has unknown markers"
                        ));

                        continue;
                    }

                    // record an explicitly named data file
                    let path = filename.to_string();
                    current_files.push(MdTestFile {
                        path,
                        content: code_block_content.clone(),
                        options: HashMap::new(),
                    });
                } else if !code_block_language.is_empty() {
                    // retain a suite-specific block
                    current_extra_blocks.push(RawCodeBlock {
                        language: code_block_language.clone(),
                        content: code_block_content.clone(),
                        content_range: code_block_content_range.clone(),
                    });
                }
            }
            Event::Start(Tag::Item) => {
                in_list_item = true;
                list_item_text.clear();
            }
            Event::End(TagEnd::Item) => {
                in_list_item = false;
                let text = list_item_text.trim().to_string();

                if current_test_name.is_some() && !current_files.is_empty() && !text.is_empty() {
                    current_bullets.push(text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                current_line += 1;
                if in_code_block {
                    code_block_content.push('\n');
                    extend_range(&mut code_block_content_range, event_range);
                } else if in_list_item {
                    list_item_text.push(' ');
                }
            }
            _ => {}
        }
    }

    // finalize remaining test
    finalize_pending_test(
        &mut tests,
        current_section.as_ref(),
        &mut current_test_name,
        &mut current_test_skip,
        &mut current_options,
        &mut current_files,
        &mut current_bullets,
        &mut current_extra_blocks,
        current_line,
        &mut errors,
    );

    if errors.is_empty() {
        Ok(tests)
    } else {
        Err(errors.join("; "))
    }
}

/// Extend one optional source range to include an event range.
fn extend_range(range: &mut Option<Range<usize>>, event: Range<usize>) {
    match range {
        Some(range) => range.end = event.end,
        None => *range = Some(event),
    }
}

/// Finalize one pending Markdown test.
#[allow(clippy::too_many_arguments)]
fn finalize_pending_test(
    tests: &mut Vec<MdTestCase>,
    current_section: Option<&String>,
    current_test_name: &mut Option<String>,
    current_test_skip: &mut bool,
    current_options: &mut HashMap<String, String>,
    current_files: &mut Vec<MdTestFile>,
    current_bullets: &mut Vec<String>,
    current_extra_blocks: &mut Vec<RawCodeBlock>,
    current_line: usize,
    errors: &mut Vec<String>,
) {
    // skip when there is no pending test
    let Some(name) = current_test_name.take() else {
        return;
    };

    // require a section heading
    let Some(section) = current_section else {
        errors.push(format!("test '{name}' appears before a section heading"));
        return;
    };

    // require at least one code block
    if current_files.is_empty() {
        errors.push(format!("test '{name}' is missing a code block"));
        return;
    }

    // materialize the parsed test case
    tests.push(MdTestCase {
        name,
        section: section.clone(),
        options: std::mem::take(current_options),
        files: std::mem::take(current_files),
        bullet_items: std::mem::take(current_bullets),
        extra_blocks: std::mem::take(current_extra_blocks),
        line: current_line,
        skip: *current_test_skip,
    });
    *current_test_skip = false;
}

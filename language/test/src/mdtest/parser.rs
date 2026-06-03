use std::collections::HashMap;
use std::path::Path;

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

/// A single file within a test case.
#[derive(Debug, Clone)]
pub struct MdTestFile {
    /// The filename (e.g., "main.ds" or "lib.ds").
    pub path: String,
    /// The source code content.
    pub content: String,
    /// Options from the language tag (e.g., "line-width=40").
    pub options: HashMap<String, String>,
}

/// A raw code block from markdown (before interpretation).
#[derive(Debug, Clone)]
pub struct RawCodeBlock {
    /// The language tag (e.g., "query completion $0" or "expected:main").
    pub language: String,
    /// The block content.
    pub content: String,
}

/// A single test case extracted from markdown.
/// Contains raw parsed data that spec/query interpret differently.
#[derive(Debug, Clone)]
pub struct MdTestCase {
    /// Name of the test (from H3/H4 heading).
    pub name: String,
    /// Section the test belongs to (from H2 heading).
    pub section: String,
    /// Per-test options parsed from `test` blocks.
    pub options: HashMap<String, String>,
    /// Source files (ds/ts code blocks).
    pub files: Vec<MdTestFile>,
    /// Bullet list items (used as expected errors in spec tests).
    pub bullet_items: Vec<String>,
    /// Non-source code blocks like `query` and `expected:` blocks.
    pub extra_blocks: Vec<RawCodeBlock>,
    /// Line number in the markdown file where this test starts.
    pub line: usize,
    /// Whether this test is marked as skipped (prefixed with `_`).
    pub skip: bool,
}

/// Parse a markdown file and extract test cases.
pub fn parse_mdtest_file(path: &Path) -> std::io::Result<Vec<MdTestCase>> {
    let content = std::fs::read_to_string(path)?;
    parse_mdtest(&content)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

/// Parsed language tag with base language, optional filename, markers, and options.
struct ParsedLanguageTag<'a> {
    /// Base language (e.g., "ds", "ts").
    base: &'a str,
    /// Optional filename after colon (e.g., "main.ds" from "ds:main.ds").
    filename: Option<&'a str>,
    /// Space-separated markers (e.g., ["expected"] from "ds expected").
    markers: Vec<&'a str>,
    /// Key-value options (e.g., {"line-width": "40"} from "ds line-width=40").
    options: HashMap<String, String>,
}

/// Parse the language tag to extract base language, optional filename, markers, and options.
///
/// Supports formats like:
/// - `ds` -> base "ds"
/// - `ds:main.ds` -> base "ds", filename "main.ds"
/// - `ds expected` -> base "ds", marker "expected"
/// - `ds line-width=40` -> base "ds", option line-width=40
/// - `ds:main.ds expected line-width=40` -> all combined
fn parse_language_tag(language: &str) -> ParsedLanguageTag<'_> {
    let mut parts = language.split_whitespace();
    let first = parts.next().unwrap_or("");

    // first part may have colon for filename
    let (base, filename) = if let Some(colon_pos) = first.find(':') {
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
            options.insert(key.to_string(), value.to_string());
        } else {
            markers.push(part);
        }
    }

    ParsedLanguageTag {
        base,
        filename,
        markers,
        options,
    }
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

/// Insert a test option, panicking on conflicting values.
fn insert_test_option(
    test_name: Option<&str>,
    options: &mut HashMap<String, String>,
    key: String,
    value: String,
) {
    if let Some(existing) = options.get(&key)
        && existing != &value
    {
        let name = test_name.unwrap_or("<unknown>");
        panic!("conflicting test option '{key}' for '{name}': '{existing}' vs '{value}'");
    }

    options.insert(key, value);
}

/// Apply test options from a `test` block.
fn apply_test_options(
    test_name: Option<&str>,
    options: HashMap<String, String>,
    test_options: &mut HashMap<String, String>,
) {
    for (key, value) in options {
        let normalized = normalize_test_option_key(&key).unwrap_or(&key);
        insert_test_option(test_name, test_options, normalized.to_string(), value);
    }
}

/// Split test options from per file options.
fn split_test_options(
    test_name: Option<&str>,
    options: HashMap<String, String>,
    test_options: &mut HashMap<String, String>,
) -> HashMap<String, String> {
    let mut file_options = HashMap::new();

    for (key, value) in options {
        if let Some(normalized) = normalize_test_option_key(&key) {
            insert_test_option(test_name, test_options, normalized.to_string(), value);
        } else {
            file_options.insert(key, value);
        }
    }

    file_options
}

/// Check if a language tag is a supported source code language.
fn is_code_language(language: &str) -> bool {
    let lower = language.to_lowercase();
    matches!(
        lower.as_str(),
        "ts" | "tsx" | "typescript" | "js" | "jsx" | "javascript" | "ds" | "destack"
    )
}

/// Check if a language tag is a data/text file type that can be imported.
fn is_data_language(language: &str) -> bool {
    let lower = language.to_lowercase();
    matches!(
        lower.as_str(),
        "json" | "toml" | "yaml" | "yml" | "text" | "txt" | "env"
    )
}

/// Parse markdown content and extract test cases.
pub fn parse_mdtest(content: &str) -> Result<Vec<MdTestCase>, String> {
    let parser = Parser::new(content);

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
    let mut in_list_item = false;
    let mut list_item_text = String::new();
    let mut current_line = 1;
    let mut errors = Vec::new();

    for event in parser {
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
                code_block_language = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(language) => language.to_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;

                // check if it's a source code block
                let parsed = parse_language_tag(&code_block_language);
                let is_expected = parsed.markers.contains(&"expected");

                if current_test_name.is_none()
                    && (is_code_language(parsed.base) || is_data_language(parsed.base))
                {
                    errors.push(format!(
                        "code block '{code_block_language}' appears outside a test heading"
                    ));
                }

                if parsed.base.eq_ignore_ascii_case("test") {
                    if current_test_name.is_some() {
                        apply_test_options(
                            current_test_name.as_deref(),
                            parsed.options,
                            &mut current_options,
                        );
                    }
                } else if is_code_language(parsed.base) && !is_expected {
                    // regular source code file
                    let path = parsed.filename.unwrap_or("main.ds").to_string();
                    let options = split_test_options(
                        current_test_name.as_deref(),
                        parsed.options,
                        &mut current_options,
                    );
                    current_files.push(MdTestFile {
                        path,
                        content: code_block_content.clone(),
                        options,
                    });
                } else if let Some(filename) =
                    parsed.filename.filter(|_| is_data_language(parsed.base))
                {
                    // data/text file with explicit filename (e.g., `json:data.json`)
                    let path = filename.to_string();
                    current_files.push(MdTestFile {
                        path,
                        content: code_block_content.clone(),
                        options: HashMap::new(),
                    });
                } else if !code_block_language.is_empty() {
                    // non-source block (query, expected, etc.)
                    current_extra_blocks.push(RawCodeBlock {
                        language: code_block_language.clone(),
                        content: code_block_content.clone(),
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
    // bail if no pending test
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let md = r#"
## Variables

### Variable type mismatch

```ds
const x: string = 5
```

- type 5 is not assignable to type string
"#;

        let tests = parse_mdtest(md).expect("parse");
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "Variable type mismatch");
        assert_eq!(tests[0].section, "Variables");
        assert!(tests[0].options.is_empty());
        assert_eq!(tests[0].files.len(), 1);
        assert_eq!(tests[0].files[0].path, "main.ds");
        assert_eq!(tests[0].files[0].content.trim(), "const x: string = 5");
        assert_eq!(tests[0].bullet_items.len(), 1);
        assert!(tests[0].bullet_items[0].contains("not assignable"));
    }

    #[test]
    fn test_parse_multiple_errors() {
        let md = r#"
## Types

### Multiple type errors

```ds
const x: string = 5
const y: boolean = "hello"
```

- type 5 is not assignable to type string
- type "hello" is not assignable to type boolean
"#;

        let tests = parse_mdtest(md).expect("parse");
        assert_eq!(tests.len(), 1);
        assert!(tests[0].options.is_empty());
        assert_eq!(tests[0].bullet_items.len(), 2);
    }

    #[test]
    fn test_parse_no_errors() {
        let md = r#"
## Variables

### Valid declaration

```ds
const x: number = 5
```
"#;

        let tests = parse_mdtest(md).expect("parse");
        assert_eq!(tests.len(), 1);
        assert!(tests[0].options.is_empty());
        assert_eq!(tests[0].bullet_items.len(), 0);
    }

    #[test]
    fn test_parse_multiple_tests() {
        let md = r#"
## Section One

### Test A

```ds
let a = 1
```

### Test B

```ds
let b = 2
```

- some error

## Section Two

### Test C

```ds
let c = 3
```
"#;

        let tests = parse_mdtest(md).expect("parse");
        assert_eq!(tests.len(), 3);
        assert!(tests[0].options.is_empty());
        assert_eq!(tests[0].name, "Test A");
        assert_eq!(tests[0].section, "Section One");
        assert!(tests[1].options.is_empty());
        assert_eq!(tests[1].name, "Test B");
        assert_eq!(tests[1].section, "Section One");
        assert!(tests[2].options.is_empty());
        assert_eq!(tests[2].name, "Test C");
        assert_eq!(tests[2].section, "Section Two");
    }

    #[test]
    fn test_ignore_non_ts_code_blocks() {
        let md = r#"
## Example

### My test

```json
{"not": "code"}
```

```ds
const x = 1
```

- expected error
"#;

        let tests = parse_mdtest(md).expect("parse");
        assert_eq!(tests.len(), 1);
        assert!(tests[0].options.is_empty());
        assert_eq!(tests[0].files.len(), 1);
        assert_eq!(tests[0].files[0].content.trim(), "const x = 1");
        // json block goes into extra_blocks
        assert_eq!(tests[0].extra_blocks.len(), 1);
        assert_eq!(tests[0].extra_blocks[0].language, "json");
    }

    #[test]
    fn test_parse_multi_file() {
        let md = r#"
## Extensions

### Multi-file test

```ds:types.ds
export struct Foo {}
```

```ds:main.ds
import { Foo } from "./types.ds"
const f: Foo = Foo {}
```

- some error
"#;

        let tests = parse_mdtest(md).expect("parse");
        assert_eq!(tests.len(), 1);
        assert!(tests[0].options.is_empty());
        assert_eq!(tests[0].name, "Multi-file test");
        assert_eq!(tests[0].files.len(), 2);
        assert_eq!(tests[0].files[0].path, "types.ds");
        assert!(tests[0].files[0].content.contains("export struct Foo"));
        assert_eq!(tests[0].files[1].path, "main.ds");
        assert!(tests[0].files[1].content.contains("import { Foo }"));
    }

    #[test]
    fn test_parse_language_tag() {
        let tag = parse_language_tag("ds");
        assert_eq!(tag.base, "ds");
        assert_eq!(tag.filename, None);
        assert!(tag.markers.is_empty());

        let tag = parse_language_tag("ds:main.ds");
        assert_eq!(tag.base, "ds");
        assert_eq!(tag.filename, Some("main.ds"));
        assert!(tag.markers.is_empty());

        let tag = parse_language_tag("typescript:utils.ts");
        assert_eq!(tag.base, "typescript");
        assert_eq!(tag.filename, Some("utils.ts"));

        let tag = parse_language_tag("ds expected");
        assert_eq!(tag.base, "ds");
        assert_eq!(tag.filename, None);
        assert_eq!(tag.markers, vec!["expected"]);

        let tag = parse_language_tag("ds:out.ds expected");
        assert_eq!(tag.base, "ds");
        assert_eq!(tag.filename, Some("out.ds"));
        assert_eq!(tag.markers, vec!["expected"]);

        let tag = parse_language_tag("ds line-width=40");
        assert_eq!(tag.base, "ds");
        assert_eq!(tag.options.get("line-width"), Some(&"40".to_string()));

        let tag = parse_language_tag("ds expected line-width=40 indent-width=2");
        assert_eq!(tag.base, "ds");
        assert_eq!(tag.markers, vec!["expected"]);
        assert_eq!(tag.options.get("line-width"), Some(&"40".to_string()));
        assert_eq!(tag.options.get("indent-width"), Some(&"2".to_string()));
    }

    #[test]
    fn test_parse_expected_block() {
        let md = r#"
## Formatter

### spacing test

```ds
const   x   =   1
```

```ds expected
const x = 1;
```
"#;

        let tests = parse_mdtest(md).expect("parse");
        assert_eq!(tests.len(), 1);
        assert!(tests[0].options.is_empty());
        assert_eq!(tests[0].files.len(), 1);
        assert_eq!(tests[0].files[0].content.trim(), "const   x   =   1");
        assert_eq!(tests[0].extra_blocks.len(), 1);
        assert_eq!(tests[0].extra_blocks[0].language, "ds expected");
        assert_eq!(tests[0].extra_blocks[0].content.trim(), "const x = 1;");
    }

    #[test]
    fn test_parse_query_block() {
        let md = r#"
## Assist

### Completion test

```ds
struct Point { x: f32, y: f32 }
const p = Point { x: 1, y: 2 };
p.$0
```

```query_completion $0
- x: field
- y: field
```
"#;

        let tests = parse_mdtest(md).expect("parse");
        assert_eq!(tests.len(), 1);
        assert!(tests[0].options.is_empty());
        assert_eq!(tests[0].extra_blocks.len(), 1);
        assert_eq!(tests[0].extra_blocks[0].language, "query_completion $0");
        assert!(tests[0].extra_blocks[0].content.contains("x: field"));
    }

    /// Parse test options from a code block.
    #[test]
    fn test_parse_code_block_test_options() {
        let md = r#"
## Section

### Case

```ds libs=es2024,dom emit=native line-width=40
const x = 1
```
"#;

        let tests = parse_mdtest(md).expect("parse");
        assert_eq!(tests.len(), 1);
        assert_eq!(
            tests[0].options.get("libs"),
            Some(&"es2024,dom".to_string())
        );
        assert_eq!(tests[0].options.get("emit"), Some(&"native".to_string()));
        assert_eq!(
            tests[0].files[0].options.get("line-width"),
            Some(&"40".to_string())
        );
        assert!(!tests[0].files[0].options.contains_key("libs"));
        assert!(!tests[0].files[0].options.contains_key("emit"));
    }

    /// Reject conflicting test options across code blocks.
    #[test]
    #[should_panic(expected = "conflicting test option 'libs'")]
    fn test_parse_conflicting_test_options() {
        let md = r#"
## Section

### Case

```ds:main.ds libs=es2024
const x = 1
```

```ds:other.ds libs=es2015
const y = 2
```
"#;

        let _tests = parse_mdtest(md).expect("parse");
    }
}

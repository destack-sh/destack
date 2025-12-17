use std::path::Path;

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

/// A single file within a test case.
#[derive(Debug, Clone)]
pub struct MdTestFile {
    /// The filename (e.g., "main.ds" or "lib.ds").
    pub path: String,
    /// The source code content.
    pub content: String,
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
    /// Source files (ds/ts code blocks).
    pub files: Vec<MdTestFile>,
    /// Bullet list items (used as expected errors in spec tests).
    pub bullet_items: Vec<String>,
    /// Non-source code blocks like `query` and `expected:` blocks.
    pub extra_blocks: Vec<RawCodeBlock>,
    /// Line number in the markdown file where this test starts.
    pub line: usize,
}

/// Parse a markdown file and extract test cases.
pub fn parse_mdtest_file(path: &Path) -> std::io::Result<Vec<MdTestCase>> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_mdtest(&content))
}

/// Parsed language tag with base language, optional filename, and markers.
struct ParsedLanguageTag<'a> {
    /// Base language (e.g., "ds", "ts").
    base: &'a str,
    /// Optional filename after colon (e.g., "main.ds" from "ds:main.ds").
    filename: Option<&'a str>,
    /// Space-separated markers (e.g., ["expected"] from "ds expected").
    markers: Vec<&'a str>,
}

/// Parse the language tag to extract base language, optional filename, and markers.
///
/// Supports formats like:
/// - `ds` -> base "ds"
/// - `ds:main.ds` -> base "ds", filename "main.ds"
/// - `ds expected` -> base "ds", marker "expected"
/// - `ds:main.ds expected` -> base "ds", filename "main.ds", marker "expected"
fn parse_language_tag(language: &str) -> ParsedLanguageTag<'_> {
    let mut parts = language.split_whitespace();
    let first = parts.next().unwrap_or("");

    // first part may have colon for filename
    let (base, filename) = if let Some(colon_pos) = first.find(':') {
        (&first[..colon_pos], Some(&first[colon_pos + 1..]))
    } else {
        (first, None)
    };

    let markers: Vec<&str> = parts.collect();

    ParsedLanguageTag {
        base,
        filename,
        markers,
    }
}

/// Check if a language tag is a supported source code language.
fn is_code_language(language: &str) -> bool {
    let lower = language.to_lowercase();
    lower == "ts" || lower == "typescript" || lower == "ds" || lower == "destack"
}

/// Parse markdown content and extract test cases.
pub fn parse_mdtest(content: &str) -> Vec<MdTestCase> {
    let parser = Parser::new(content);

    let mut tests = Vec::new();
    let mut current_section = String::new();
    let mut current_test_name: Option<String> = None;
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

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                // finalize pending test before starting a new heading
                if let Some(name) = current_test_name.take()
                    && !current_files.is_empty()
                {
                    tests.push(MdTestCase {
                        name,
                        section: current_section.clone(),
                        files: std::mem::take(&mut current_files),
                        bullet_items: std::mem::take(&mut current_bullets),
                        extra_blocks: std::mem::take(&mut current_extra_blocks),
                        line: current_line,
                    });
                }

                in_heading = true;
                heading_level = Some(level);
                heading_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                let text = heading_text.trim().to_string();

                match heading_level {
                    Some(HeadingLevel::H2) => {
                        current_section = text;
                    }
                    Some(HeadingLevel::H3 | HeadingLevel::H4) => {
                        current_test_name = Some(text);
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

                if current_test_name.is_none() {
                    continue;
                }

                // check if it's a source code block
                let parsed = parse_language_tag(&code_block_language);
                let is_expected = parsed.markers.contains(&"expected");

                if is_code_language(parsed.base) && !is_expected {
                    // regular source code file
                    let path = parsed.filename.unwrap_or("main.ds").to_string();
                    current_files.push(MdTestFile {
                        path,
                        content: code_block_content.clone(),
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
    if let Some(name) = current_test_name
        && !current_files.is_empty()
    {
        tests.push(MdTestCase {
            name,
            section: current_section,
            files: current_files,
            bullet_items: current_bullets,
            extra_blocks: current_extra_blocks,
            line: current_line,
        });
    }

    tests
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

        let tests = parse_mdtest(md);
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "Variable type mismatch");
        assert_eq!(tests[0].section, "Variables");
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

        let tests = parse_mdtest(md);
        assert_eq!(tests.len(), 1);
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

        let tests = parse_mdtest(md);
        assert_eq!(tests.len(), 1);
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

        let tests = parse_mdtest(md);
        assert_eq!(tests.len(), 3);
        assert_eq!(tests[0].name, "Test A");
        assert_eq!(tests[0].section, "Section One");
        assert_eq!(tests[1].name, "Test B");
        assert_eq!(tests[1].section, "Section One");
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

        let tests = parse_mdtest(md);
        assert_eq!(tests.len(), 1);
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

        let tests = parse_mdtest(md);
        assert_eq!(tests.len(), 1);
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

        let tests = parse_mdtest(md);
        assert_eq!(tests.len(), 1);
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

        let tests = parse_mdtest(md);
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].extra_blocks.len(), 1);
        assert_eq!(tests[0].extra_blocks[0].language, "query_completion $0");
        assert!(tests[0].extra_blocks[0].content.contains("x: field"));
    }
}

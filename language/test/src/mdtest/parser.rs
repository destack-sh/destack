use std::path::Path;

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

/// A single file within a test case.
#[derive(Debug, Clone)]
pub struct MdTestFile {
    /// The filename (e.g., "main.ds" or "types.ds").
    pub path: String,
    /// The source code content.
    pub content: String,
}

/// A single test case extracted from markdown.
#[derive(Debug, Clone)]
pub struct MdTestCase {
    /// Name of the test (from heading).
    pub name: String,
    /// Section/category the test belongs to.
    pub section: String,
    /// The source files to compile.
    /// For single-file tests, this contains one file with path "main.ds".
    /// For multi-file tests, this contains multiple files with their specified paths.
    pub files: Vec<MdTestFile>,
    /// Expected error messages (from bullet list).
    pub expected_errors: Vec<String>,
    /// Line number in the markdown file where this test starts.
    pub line: usize,
}

/// Parse a markdown file and extract test cases.
pub fn parse_mdtest_file(path: &Path) -> std::io::Result<Vec<MdTestCase>> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_mdtest(&content))
}

/// Parse the language tag to extract the base language and optional filename.
///
/// Examples:
/// - "ds" -> ("ds", None)
/// - "ds:main.ds" -> ("ds", Some("main.ds"))
/// - "typescript:utils.ts" -> ("typescript", Some("utils.ts"))
fn parse_lang_tag(lang: &str) -> (&str, Option<&str>) {
    if let Some(colon_pos) = lang.find(':') {
        let base_lang = &lang[..colon_pos];
        let filename = &lang[colon_pos + 1..];
        (base_lang, Some(filename))
    } else {
        (lang, None)
    }
}

/// Check if a language tag is a supported code language.
fn is_code_language(lang: &str) -> bool {
    let lower = lang.to_lowercase();
    lower == "ts" || lower == "typescript" || lower == "ds" || lower == "destack"
}

/// Parse markdown content and extract test cases.
pub fn parse_mdtest(content: &str) -> Vec<MdTestCase> {
    let parser = Parser::new(content);

    let mut tests = Vec::new();
    let mut current_section = String::new();
    let mut current_test_name: Option<String> = None;
    let mut current_files: Vec<MdTestFile> = Vec::new();
    let mut current_errors: Vec<String> = Vec::new();
    let mut in_heading = false;
    let mut heading_level: Option<HeadingLevel> = None;
    let mut heading_text = String::new();
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_block_content = String::new();
    let mut in_list_item = false;
    let mut list_item_text = String::new();

    // track line numbers (approximate based on newlines before current position)
    let mut current_line = 1;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                // before starting a new heading, finalize any pending test
                if let Some(name) = current_test_name.take()
                    && !current_files.is_empty()
                {
                    tests.push(MdTestCase {
                        name,
                        section: current_section.clone(),
                        files: std::mem::take(&mut current_files),
                        expected_errors: std::mem::take(&mut current_errors),
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
                    // H2 = section
                    Some(HeadingLevel::H2) => {
                        current_section = text;
                    }
                    // H3 or H4 = test case
                    Some(HeadingLevel::H3 | HeadingLevel::H4) => {
                        current_test_name = Some(text);
                        current_files.clear();
                        current_errors.clear();
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
                code_block_lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;

                // parse lang:filename syntax
                let (base_lang, filename) = parse_lang_tag(&code_block_lang);

                // only capture supported code blocks
                if is_code_language(base_lang) && current_test_name.is_some() {
                    let path = filename.unwrap_or("main.ds").to_string();
                    current_files.push(MdTestFile {
                        path,
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

                // only collect list items if we have a test with files
                if current_test_name.is_some() && !current_files.is_empty() && !text.is_empty() {
                    current_errors.push(text);
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

    // finalize any remaining test
    if let Some(name) = current_test_name
        && !current_files.is_empty()
    {
        tests.push(MdTestCase {
            name,
            section: current_section,
            files: current_files,
            expected_errors: current_errors,
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
        assert_eq!(tests[0].expected_errors.len(), 1);
        assert!(tests[0].expected_errors[0].contains("not assignable"));
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
        assert_eq!(tests[0].expected_errors.len(), 2);
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
        assert_eq!(tests[0].expected_errors.len(), 0);
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
    fn test_parse_lang_tag() {
        assert_eq!(parse_lang_tag("ds"), ("ds", None));
        assert_eq!(parse_lang_tag("ds:main.ds"), ("ds", Some("main.ds")));
        assert_eq!(
            parse_lang_tag("typescript:utils.ts"),
            ("typescript", Some("utils.ts"))
        );
    }
}

use std::ops::Range;
use std::path::Path;

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

/// One initial workspace file in a query case.
#[derive(Debug, Clone)]
pub(crate) struct QuerySource {
    /// The workspace-relative file path.
    pub path: String,
    /// The exact file contents.
    pub content: String,
}

/// One query, response, or workspace change block.
#[derive(Debug, Clone)]
pub(crate) struct QueryBlock {
    /// The complete fenced code block language tag.
    pub language: String,
    /// The block content.
    pub content: String,
    /// The exact content range in the Markdown source.
    pub content_range: Option<Range<usize>>,
}

/// One query case extracted from a Markdown document.
#[derive(Debug, Clone)]
pub(crate) struct MarkdownCase {
    /// The case name from its H3 or H4 heading.
    pub name: String,
    /// The containing H2 section.
    pub section: String,
    /// The initial workspace files.
    pub files: Vec<QuerySource>,
    /// The query, response, and workspace change blocks.
    pub blocks: Vec<QueryBlock>,
}

/// Parse one Markdown document into query cases.
pub(crate) fn parse_query_document(path: &Path) -> std::io::Result<Vec<MarkdownCase>> {
    let content = std::fs::read_to_string(path)?;
    QueryDocument::parse(&content)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

/// One heading being read.
#[derive(Debug)]
struct Heading {
    /// The heading level.
    level: HeadingLevel,
    /// The accumulated heading text.
    text: String,
}

/// One fenced block being read.
#[derive(Debug, Default)]
struct Fence {
    /// The complete fenced block language tag.
    language: String,
    /// The accumulated block content.
    content: String,
    /// The exact content range in the Markdown source.
    content_range: Option<Range<usize>>,
}

/// One parsed initial file block.
struct InitialFile<'a> {
    /// The workspace-relative path.
    path: &'a str,
}

/// The state required while reading one query document.
#[derive(Debug)]
struct QueryDocument<'a> {
    /// The complete Markdown source.
    source: &'a str,
    /// The completed query cases.
    cases: Vec<MarkdownCase>,
    /// The active H2 section.
    section: Option<String>,
    /// The active H3 or H4 case.
    case: Option<MarkdownCase>,
    /// The heading currently being read.
    heading: Option<Heading>,
    /// The fenced block currently being read.
    fence: Option<Fence>,
    /// Every structural error encountered while parsing.
    errors: Vec<String>,
}

impl<'a> QueryDocument<'a> {
    /// Parse complete Markdown source.
    fn parse(source: &'a str) -> Result<Vec<MarkdownCase>, String> {
        let mut document = Self {
            source,
            cases: Vec::new(),
            section: None,
            case: None,
            heading: None,
            fence: None,
            errors: Vec::new(),
        };

        // consume the Markdown event stream in source order
        for (event, range) in Parser::new(source).into_offset_iter() {
            document.event(event, range);
        }
        document.finish_case();

        if document.errors.is_empty() {
            Ok(document.cases)
        } else {
            Err(document.errors.join("; "))
        }
    }

    /// Consume one Markdown event.
    fn event(&mut self, event: Event<'_>, range: Range<usize>) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => self.start_heading(level),
            Event::End(TagEnd::Heading(_)) => self.finish_heading(),
            Event::Start(Tag::CodeBlock(kind)) => self.start_fence(kind),
            Event::End(TagEnd::CodeBlock) => self.finish_fence(),
            Event::Text(text) => self.push_text(&text, range),
            Event::Code(text) | Event::Html(text) | Event::InlineHtml(text) => {
                self.push_heading_text(&text);
            }
            Event::SoftBreak | Event::HardBreak => self.push_break(range),
            _ => {}
        }
    }

    /// Start reading one heading.
    fn start_heading(&mut self, level: HeadingLevel) {
        self.finish_case();
        self.heading = Some(Heading {
            level,
            text: String::new(),
        });
    }

    /// Finish the active heading and select its section or case.
    fn finish_heading(&mut self) {
        let Some(heading) = self.heading.take() else {
            self.errors
                .push("Markdown closes a heading that was not opened".to_string());

            return;
        };
        let text = heading.text.trim().to_string();

        match heading.level {
            HeadingLevel::H2 => self.section = Some(text),
            HeadingLevel::H3 | HeadingLevel::H4 => {
                let Some(section) = &self.section else {
                    self.errors
                        .push(format!("query case '{text}' appears before an H2 section"));

                    return;
                };
                self.case = Some(MarkdownCase {
                    name: text,
                    section: section.clone(),
                    files: Vec::new(),
                    blocks: Vec::new(),
                });
            }
            _ => {}
        }
    }

    /// Start reading one fenced block.
    fn start_fence(&mut self, kind: pulldown_cmark::CodeBlockKind<'_>) {
        let language = match kind {
            pulldown_cmark::CodeBlockKind::Fenced(language) => language.to_string(),
            pulldown_cmark::CodeBlockKind::Indented => String::new(),
        };
        self.fence = Some(Fence {
            language,
            ..Fence::default()
        });
    }

    /// Finish and classify the active fenced block.
    fn finish_fence(&mut self) {
        let Some(fence) = self.fence.take() else {
            self.errors
                .push("Markdown closes a fenced block that was not opened".to_string());

            return;
        };

        // verify pulldown-cmark's source range before retaining it for blessing
        if let Some(range) = &fence.content_range
            && self.source.get(range.clone()) != Some(fence.content.as_str())
        {
            self.errors.push(format!(
                "code block '{}' content range does not match its text",
                fence.language
            ));
        }

        // require every meaningful block to belong to a case
        let Some(case) = &mut self.case else {
            if !fence.language.is_empty() {
                self.errors.push(format!(
                    "code block '{}' appears outside a query case",
                    fence.language
                ));
            }

            return;
        };

        // retain initial files separately from executable blocks
        match parse_initial_file(&fence.language) {
            Ok(Some(file)) => case.files.push(QuerySource {
                path: file.path.to_string(),
                content: fence.content,
            }),
            Ok(None) => case.blocks.push(QueryBlock {
                language: fence.language,
                content: fence.content,
                content_range: fence.content_range,
            }),
            Err(error) => self.errors.push(error),
        }
    }

    /// Append text to the active heading or fence.
    fn push_text(&mut self, text: &str, range: Range<usize>) {
        if let Some(heading) = &mut self.heading {
            heading.text.push_str(text);
        } else if let Some(fence) = &mut self.fence {
            fence.content.push_str(text);
            extend_range(&mut fence.content_range, range);
        }
    }

    /// Append inline source to the active heading.
    fn push_heading_text(&mut self, text: &str) {
        if let Some(heading) = &mut self.heading {
            heading.text.push_str(text);
        }
    }

    /// Append one source break to the active fence.
    fn push_break(&mut self, range: Range<usize>) {
        if let Some(fence) = &mut self.fence {
            fence.content.push('\n');
            extend_range(&mut fence.content_range, range);
        }
    }

    /// Finish the active case.
    fn finish_case(&mut self) {
        let Some(case) = self.case.take() else {
            return;
        };
        if case.files.is_empty() {
            self.errors
                .push(format!("query case '{}' has no initial files", case.name));

            return;
        }

        self.cases.push(case);
    }
}

/// Parse an initial source or data file block.
fn parse_initial_file(language: &str) -> Result<Option<InitialFile<'_>>, String> {
    let mut words = language.split_whitespace();
    let first = words.next().unwrap_or("");
    let (language, inline_path) = first.split_once(':').unwrap_or((first, ""));
    let is_source = matches!(language.to_ascii_lowercase().as_str(), "tspp");
    let is_data = matches!(
        language.to_ascii_lowercase().as_str(),
        "json" | "toml" | "yaml" | "yml" | "text" | "txt" | "env"
    );
    if !is_source && !is_data {
        return Ok(None);
    }

    // read the optional path and change marker
    let path = if inline_path.is_empty() {
        words.next()
    } else {
        Some(inline_path)
    };
    let marker = words.next();
    if words.next().is_some() {
        return Err(format!("file block '{language}' has too many arguments"));
    }
    if let Some(marker) = marker
        && !matches!(
            marker,
            "expected" | "after" | "change" | "type" | "backspace" | "delete" | "add"
        )
    {
        return Err(format!(
            "file block '{language}' has unknown marker '{marker}'"
        ));
    }
    if marker.is_some() {
        return Ok(None);
    }

    // source blocks default to main.tspp while data blocks require a path
    let path = match (path, is_source) {
        (Some(path), _) => path,
        (None, true) => "main.tspp",
        (None, false) => return Err(format!("data block '{language}' requires a file path")),
    };

    Ok(Some(InitialFile { path }))
}

/// Extend one optional source range to include an event range.
fn extend_range(range: &mut Option<Range<usize>>, event: Range<usize>) {
    match range {
        Some(range) => range.end = event.end,
        None => *range = Some(event),
    }
}

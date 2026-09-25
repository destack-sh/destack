use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
use tspp_source::DiagnosticSeverity;

use super::{Example, ExampleError, ExampleFile, Expectation, Probe};

/// The fence language of a TS++ example block.
const EXAMPLE_LANGUAGE: &str = "tspp";
/// The comment that hides the lines above it from the rendered example.
const CUT_MARKER: &str = "---cut---";
/// The trailing comment that opens a value probe.
const VALUE_MARKER: &str = "// => ";

impl Example {
    /// Parse every example of one Markdown document, one per heading section.
    pub fn parse_document(document: &str) -> Result<Vec<Self>, ExampleError> {
        let mut reader = DocumentReader {
            document,
            examples: Vec::new(),
            headings: Vec::new(),
            heading: None,
            fence: None,
            files: Vec::new(),
        };

        // read the Markdown events in source order
        for (event, range) in Parser::new(document).into_offset_iter() {
            reader.event(event, range)?;
        }
        reader.finish_section();

        Ok(reader.examples)
    }
}

impl ExampleFile {
    /// Parse one annotated block into its annotation-free source and probes.
    pub fn parse(path: String, content: &str, block: Range<usize>) -> Result<Self, ExampleError> {
        if u32::try_from(content.len()).is_err() {
            return Err(ExampleError::new(
                block.start,
                "example block exceeds the offset width",
            ));
        }
        let mut source = String::with_capacity(content.len());
        let mut shown = 0;
        let mut probes = Vec::new();
        let mut previous: Option<Range<u32>> = None;
        let mut offset = block.start;

        // offsets below fit in u32 because the source never outgrows its block
        for line in content.split_inclusive('\n') {
            let error = |message: &str| ExampleError::new(offset, message);
            match Line::read(line.trim_end_matches(['\n', '\r'])) {
                // hide the lines above the cut
                Line::Cut => shown = source.len() as u32,
                // place a probe on the source line above
                Line::Probe { column, annotation } => {
                    let line = previous
                        .clone()
                        .ok_or_else(|| error("probe has no source line above it"))?;
                    probes.push(
                        Probe::parse(annotation, column, line)
                            .map_err(|message| error(&message))?,
                    );
                }
                // keep a source line and its trailing value probe
                Line::Source { code, value } => {
                    let start = source.len() as u32;
                    let end = start + code.len() as u32;
                    if let Some(value) = value {
                        if code.trim().is_empty() {
                            return Err(error("value probe has no expression on its line"));
                        }
                        let indent = (code.len() - code.trim_start().len()) as u32;
                        probes.push(Probe {
                            range: start + indent..end,
                            expectation: Expectation::Value(value.to_string()),
                        });
                    }
                    source.push_str(code);
                    source.push('\n');
                    previous = Some(start..end);
                }
            }

            offset += line.len();
        }

        Ok(Self {
            path,
            source,
            shown,
            probes,
            block,
        })
    }
}

impl Probe {
    /// Parse one caret annotation placed at one column below one source line.
    fn parse(annotation: &str, column: usize, line: Range<u32>) -> Result<Self, String> {
        let width = annotation.bytes().take_while(|byte| *byte == b'^').count();
        let rest = &annotation[width..];
        let length = (line.end - line.start) as usize;
        let start = line.start + column as u32;

        // read a type probe at one position
        if let Some(text) = rest.strip_prefix('?') {
            if width != 1 || column > length {
                return Err("type probe must be one caret within its source line".into());
            }

            Ok(Self {
                range: start..start,
                expectation: Expectation::Type(text.trim().to_string()),
            })
        }
        // read a diagnostic probe over the caret range
        else {
            if column + width > length {
                return Err("diagnostic probe exceeds its source line".into());
            }

            Ok(Self {
                range: start..start + width as u32,
                expectation: Expectation::parse_diagnostic(rest.trim())?,
            })
        }
    }
}

impl Expectation {
    /// Parse one `severity[code]: message` diagnostic expectation.
    fn parse_diagnostic(text: &str) -> Result<Self, String> {
        let invalid =
            || format!("diagnostic probe must read `error[code]: message`, found `{text}`");
        let (severity, rest) = text.split_once('[').ok_or_else(invalid)?;
        let (code, message) = rest.split_once("]:").ok_or_else(invalid)?;
        let severity = match severity {
            "error" => DiagnosticSeverity::Error,
            "warning" => DiagnosticSeverity::Warning,
            "note" => DiagnosticSeverity::Note,
            _ => return Err(invalid()),
        };

        Ok(Self::Diagnostic {
            severity,
            code: code.to_string(),
            message: message.trim().to_string(),
        })
    }
}

/// One line of an annotated example block.
enum Line<'a> {
    /// A `// ---cut---` line.
    Cut,
    /// A caret annotation on the source line above.
    Probe {
        /// The column of the first caret.
        column: usize,
        /// The annotation from its first caret.
        annotation: &'a str,
    },
    /// A source line with an optional trailing `// => value`.
    Source {
        /// The source code without the value probe.
        code: &'a str,
        /// The expected value.
        value: Option<&'a str>,
    },
}

impl<'a> Line<'a> {
    /// Classify one line without its line ending.
    fn read(text: &'a str) -> Self {
        let indent = text.len() - text.trim_start().len();
        let comment = text[indent..].strip_prefix("//").map(str::trim_start);

        // read a cut
        if comment == Some(CUT_MARKER) {
            Self::Cut
        }
        // read a caret annotation
        else if let Some(annotation) = comment.filter(|comment| comment.starts_with('^')) {
            Self::Probe {
                column: text.len() - annotation.len(),
                annotation,
            }
        }
        // read a source line with a value probe
        else if let Some((code, value)) = text.split_once(VALUE_MARKER) {
            Self::Source {
                code: code.trim_end(),
                value: Some(value.trim()),
            }
        }
        // read a plain source line
        else {
            Self::Source {
                code: text,
                value: None,
            }
        }
    }
}

/// The state required while reading one Markdown document.
struct DocumentReader<'a> {
    /// The complete Markdown source.
    document: &'a str,
    /// The completed examples.
    examples: Vec<Example>,
    /// The enclosing headings, outermost first.
    headings: Vec<Heading>,
    /// The heading being read.
    heading: Option<Heading>,
    /// The example block being read.
    fence: Option<Fence>,
    /// The files of the current section.
    files: Vec<ExampleFile>,
}

/// One Markdown heading.
struct Heading {
    /// The heading level.
    level: HeadingLevel,
    /// The heading text.
    text: String,
}

/// One example block being read.
struct Fence {
    /// The package-relative file path.
    path: String,
    /// The content range read so far.
    content: Option<Range<usize>>,
}

impl DocumentReader<'_> {
    /// Consume one Markdown event.
    fn event(&mut self, event: Event<'_>, range: Range<usize>) -> Result<(), ExampleError> {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                self.finish_section();
                self.heading = Some(Heading {
                    level,
                    text: String::new(),
                });
            }
            Event::End(TagEnd::Heading(_)) => self.finish_heading(),
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                self.start_fence(&info, range)?;
            }
            Event::End(TagEnd::CodeBlock) => self.finish_fence(range)?,
            Event::Text(text) | Event::Code(text) => self.push_text(&text, range),
            _ => {}
        }

        Ok(())
    }

    /// Append one text run to the heading or block being read.
    fn push_text(&mut self, text: &str, range: Range<usize>) {
        if let Some(heading) = &mut self.heading {
            heading.text.push_str(text);
        }
        if let Some(fence) = &mut self.fence {
            let start = fence
                .content
                .as_ref()
                .map_or(range.start, |content| content.start);
            fence.content = Some(start..range.end);
        }
    }

    /// Record one completed heading in the heading stack.
    fn finish_heading(&mut self) {
        let Some(heading) = self.heading.take() else {
            return;
        };

        self.headings
            .retain(|enclosing| enclosing.level < heading.level);
        self.headings.push(Heading {
            level: heading.level,
            text: heading.text.trim().to_string(),
        });
    }

    /// Start one example block from its fence info.
    fn start_fence(&mut self, info: &str, range: Range<usize>) -> Result<(), ExampleError> {
        let mut words = info.split_whitespace();
        let language = words.next().unwrap_or_default();

        // reject the retired `tspp:path` spelling
        if let Some(path) = language.strip_prefix("tspp:") {
            return Err(ExampleError::new(
                range.start,
                &format!("write the example path after a space: `tspp {path}`"),
            ));
        }
        if language != EXAMPLE_LANGUAGE {
            return Ok(());
        }

        // name unnamed blocks by their position in the section
        let path = match words.next() {
            Some(path) => path.to_string(),
            None => format!("src/example-{}.tspp", self.files.len() + 1),
        };
        self.fence = Some(Fence {
            path,
            content: None,
        });

        Ok(())
    }

    /// Finish one example block into a file of the current section.
    fn finish_fence(&mut self, range: Range<usize>) -> Result<(), ExampleError> {
        let Some(fence) = self.fence.take() else {
            return Ok(());
        };
        if self.files.iter().any(|file| file.path == fence.path) {
            return Err(ExampleError::new(
                range.start,
                &format!("example declares '{}' more than once", fence.path),
            ));
        }

        let content = fence.content.unwrap_or(range.start..range.start);
        let file = ExampleFile::parse(fence.path, &self.document[content.clone()], content)?;
        self.files.push(file);

        Ok(())
    }

    /// Finish the files of the current section into one example.
    fn finish_section(&mut self) {
        if self.files.is_empty() {
            return;
        }

        let name = self
            .headings
            .iter()
            .map(|heading| heading.text.as_str())
            .collect::<Vec<_>>()
            .join(" > ");
        self.examples.push(Example {
            name,
            files: std::mem::take(&mut self.files),
        });
    }
}

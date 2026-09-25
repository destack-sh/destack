use std::sync::Arc;

use tspp_artifact::DirParsedFile;
use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{File, FilePatch, Span};
use tspp_unicode::UnicodeWidthChar;

use super::DirModule;

const TAB_WIDTH: u32 = 4;

/// One logical authored comment block.
#[derive(Debug, Clone, Copy)]
pub struct CommentBlock<'a> {
    /// The physical source file.
    file: &'a File,
    /// The retained comments in this block.
    comments: &'a [dir::Comment],
    /// The canonical attached documentation span.
    documentation: Option<Span>,
}

/// One physical source line inside a comment block.
#[derive(Debug, Clone, Copy)]
pub struct CommentLine<'a> {
    /// The physical source file.
    file: &'a File,
    /// The retained comment containing this line.
    comment: dir::Comment,
    /// The comment portion of the physical source line.
    pub span: Span,
    /// The authored content after comment decoration.
    pub content: Span,
    /// The authored content text.
    pub text: &'a str,
    /// The horizontal spacing after this line's comment decoration.
    pub spacing: Option<Span>,
    /// The zero-based physical source line.
    pub line: u32,
    /// The visual source width through this comment line.
    pub width: u32,
    /// The visual indentation after the conventional separator.
    pub indentation_width: u32,
}

/// One prose sentence retained from a logical comment block.
#[derive(Debug, Clone, Copy)]
pub struct CommentSentence<'a> {
    /// The complete sentence span.
    pub span: Span,
    /// The physical line where the sentence begins.
    pub start_line: CommentLine<'a>,
    /// The physical line where the sentence ends.
    pub end_line: CommentLine<'a>,
    /// The initial prose character when its casing is conventional.
    pub initial: Option<(Span, char)>,
    /// The authored sentence ending.
    pub ending: CommentSentenceEnding,
    /// Whether an annotation prefix introduces this sentence.
    pub is_annotation: bool,
    /// Whether a colon introduces following structured content.
    pub is_introduction: bool,
}

/// The authored ending of one comment sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentSentenceEnding {
    /// A final period.
    Period(Span),
    /// A final question mark.
    Question(Span),
    /// A final exclamation mark.
    Exclamation(Span),
    /// A final introductory colon.
    Colon(Span),
    /// No authored terminal punctuation.
    Missing(Span),
}

/// One sentence under construction while scanning comment prose.
struct SentenceBuilder<'a> {
    /// The physical line where the sentence begins.
    start_line: CommentLine<'a>,
    /// The physical line containing the latest sentence character.
    end_line: CommentLine<'a>,
    /// The first authored character.
    start: u32,
    /// The final authored character or delimiter.
    end: u32,
    /// The insertion point for missing terminal punctuation.
    punctuation_position: u32,
    /// The initial prose character when its casing is conventional.
    initial: Option<(Span, char)>,
    /// Whether an annotation prefix introduces this sentence.
    is_annotation: bool,
}

/// The prose role of one physical comment line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommentLineContent {
    /// A blank separator.
    Blank,
    /// Prose beginning at this byte offset.
    Prose {
        /// The content-local byte offset.
        start: usize,
        /// Whether an annotation prefix was removed.
        is_annotation: bool,
        /// Whether a Markdown list marker was removed.
        is_list: bool,
    },
    /// Markdown outside prose sentences.
    Markdown,
    /// A code-fence delimiter.
    Fence,
}

impl DirModule<'_> {
    /// Return logical authored comment blocks in physical source order.
    pub fn comment_blocks(&self) -> Result<Vec<CommentBlock<'_>>, ProviderError> {
        CommentBlock::collect(self)
    }
}

impl<'a> CommentBlock<'a> {
    /// Collect logical blocks from retained comments and attached documentation.
    fn collect(module: &'a DirModule<'_>) -> Result<Vec<Self>, ProviderError> {
        let view = module.view();
        let mut documentation = view
            .iter_node_ids()
            .filter_map(|node| view.get_documentation_any(node))
            .map(|documentation| documentation.span)
            .collect::<Vec<_>>();
        documentation.sort_unstable_by_key(|span| (span.file, span.start, span.end));
        documentation.dedup();
        let mut blocks = Vec::new();

        // group every physical file independently
        for (parsed, file) in module.stages.parsed.files.iter().zip(module.files) {
            if parsed.file_id != file.id {
                return Err(ProviderError::internal(format!(
                    "parsed comment file {:?} does not match source file {:?}",
                    parsed.file_id, file.id
                )));
            }
            Self::append_file(parsed, file, &documentation, &mut blocks)?;
        }

        Ok(blocks)
    }

    /// Append logical blocks from one physical source file.
    fn append_file(
        parsed: &'a DirParsedFile,
        file: &'a Arc<File>,
        documentation: &[Span],
        blocks: &mut Vec<CommentBlock<'a>>,
    ) -> Result<(), ProviderError> {
        let file = file.as_ref();
        let mut start = 0;

        // consume one maximal block at a time
        while start < parsed.comments.len() {
            let first = parsed.comments[start];
            let attached = Self::attached_documentation(documentation, first);
            let mut end = start + 1;

            // canonical documentation owns its complete comment group
            if let Some(span) = attached {
                while end < parsed.comments.len() && span.contains_span(parsed.comments[end].span) {
                    end += 1;
                }
            }
            // ordinary and detached line comments group by exact source adjacency
            else if first.is_line() {
                while end < parsed.comments.len() {
                    let previous = parsed.comments[end - 1];
                    let next = parsed.comments[end];
                    if Self::attached_documentation(documentation, next).is_some()
                        || !CommentBlock::continues(file, previous, next)?
                    {
                        break;
                    }
                    end += 1;
                }
            }

            blocks.push(Self {
                file,
                comments: &parsed.comments[start..end],
                documentation: attached,
            });
            start = end;
        }

        Ok(())
    }

    /// Return the attached documentation span containing one retained comment.
    fn attached_documentation(documentation: &[Span], comment: dir::Comment) -> Option<Span> {
        let index = documentation.partition_point(|span| {
            span.file < comment.span.file
                || (span.file == comment.span.file && span.start <= comment.span.start)
        });
        let span = documentation.get(index.checked_sub(1)?)?;

        span.contains_span(comment.span).then_some(*span)
    }

    /// Return whether a following line comment continues one logical block.
    fn continues(
        file: &File,
        previous: dir::Comment,
        next: dir::Comment,
    ) -> Result<bool, ProviderError> {
        if !next.is_line()
            || previous.anchor != next.anchor
            || previous.is_documentation() != next.is_documentation()
            || previous.is_legal() != next.is_legal()
        {
            return Ok(false);
        }
        let gap = previous.span.gap_to(next.span).ok_or_else(|| {
            ProviderError::internal(format!(
                "comments {:?} and {:?} are not in source order",
                previous.span, next.span
            ))
        })?;
        let gap = file.get_span_str(gap).ok_or_else(|| {
            ProviderError::internal(format!("comment gap {gap:?} is outside its source file"))
        })?;
        let Some(indentation) = gap.strip_prefix('\n') else {
            return Ok(false);
        };

        Ok(indentation.bytes().all(|byte| matches!(byte, b' ' | b'\t')))
    }

    /// Return the complete authored span of this block.
    pub fn span(self) -> Span {
        let first = self.comments[0].span;
        let last = self.comments[self.comments.len() - 1].span;

        first.merge(last)
    }

    /// Return a patch that demotes this documentation block to ordinary comments.
    pub fn demote_documentation(self) -> Result<FilePatch, ProviderError> {
        let mut file = FilePatch::new(self.span().file);

        // replace every physical documentation delimiter
        for comment in self.comments.iter().copied() {
            let source = self.file.get_span_str(comment.span).ok_or_else(|| {
                ProviderError::internal(format!(
                    "documentation comment {:?} is outside its source file",
                    comment.span
                ))
            })?;
            let (delimiter, replacement) = if comment.is_line() {
                ("///", "//")
            } else {
                ("/**", "/*")
            };
            if !source.starts_with(delimiter) {
                return Err(ProviderError::internal(format!(
                    "documentation comment {:?} has no documentation delimiter",
                    comment.span
                )));
            }
            let prefix = Span::new(
                comment.span.file,
                comment.span.start,
                comment.span.start + 3,
            );
            file.replace(prefix, replacement);
        }

        Ok(file)
    }

    /// Return whether this block contains documentation syntax.
    pub fn is_documentation(self) -> bool {
        self.comments[0].is_documentation()
    }

    /// Return whether this documentation block has no DIR attachment.
    pub fn is_detached_documentation(self) -> bool {
        self.is_documentation() && self.documentation.is_none()
    }

    /// Return whether this block carries legal or preserve semantics.
    pub fn is_legal(self) -> bool {
        self.comments.iter().copied().any(dir::Comment::is_legal)
    }

    /// Return physical source lines in this block.
    pub fn lines(self) -> Result<Vec<CommentLine<'a>>, ProviderError> {
        let mut lines = Vec::new();

        // expand every retained comment into its physical source lines
        for comment in self.comments.iter().copied() {
            CommentLine::append(self.file, comment, &mut lines)?;
        }

        Ok(lines)
    }
}

impl<'a> CommentSentence<'a> {
    /// Collect prose sentences from physical comment lines.
    pub fn collect(lines: &[CommentLine<'a>]) -> Result<Vec<Self>, ProviderError> {
        let contents = Self::line_contents(lines);
        let mut sentences = Vec::new();
        let mut current = None;

        // scan every prose line while carrying an unfinished sentence
        for (index, line) in lines.iter().copied().enumerate() {
            match contents[index] {
                CommentLineContent::Blank
                | CommentLineContent::Markdown
                | CommentLineContent::Fence => {
                    Self::finish_missing(&mut current, &mut sentences);
                }
                CommentLineContent::Prose {
                    start,
                    is_annotation,
                    ..
                } => {
                    let is_introduction = Self::introduces_markdown(&contents, index);
                    let ends_paragraph = !matches!(
                        contents.get(index + 1),
                        Some(CommentLineContent::Prose { is_list: false, .. })
                    );
                    Self::append_sentences(
                        line,
                        start,
                        is_annotation,
                        is_introduction,
                        ends_paragraph,
                        &mut current,
                        &mut sentences,
                    )?;
                }
            }
        }
        Self::finish_missing(&mut current, &mut sentences);

        Ok(sentences)
    }

    /// Classify physical comment lines as prose or Markdown structure.
    fn line_contents(lines: &[CommentLine<'a>]) -> Vec<CommentLineContent> {
        let mut contents = Vec::with_capacity(lines.len());
        let mut is_fenced = false;

        // preserve fence delimiters while excluding fenced source
        for line in lines.iter().copied() {
            let content = line.content_role();
            let content = if content == CommentLineContent::Fence {
                is_fenced = !is_fenced;
                CommentLineContent::Fence
            } else if is_fenced || line.is_indented_code() {
                CommentLineContent::Markdown
            } else {
                content
            };
            contents.push(content);
        }

        contents
    }

    /// Return whether a prose colon introduces following Markdown.
    fn introduces_markdown(contents: &[CommentLineContent], index: usize) -> bool {
        let following = contents[index + 1..]
            .iter()
            .copied()
            .find(|content| *content != CommentLineContent::Blank);

        matches!(
            following,
            Some(CommentLineContent::Markdown | CommentLineContent::Fence)
        ) || matches!(
            following,
            Some(CommentLineContent::Prose { is_list: true, .. })
        )
    }

    /// Scan one physical prose line into sentences.
    fn append_sentences(
        line: CommentLine<'a>,
        start: usize,
        is_annotation: bool,
        is_introduction: bool,
        ends_paragraph: bool,
        current: &mut Option<SentenceBuilder<'a>>,
        sentences: &mut Vec<CommentSentence<'a>>,
    ) -> Result<(), ProviderError> {
        let text = &line.text[start..];
        let base = line.content.start + start as u32;
        let mut index = 0;
        let mut is_inline_code = false;
        let mut link_destination_depth = 0;
        let mut url_end = None;

        // classify each authored scalar without normalizing source offsets
        while index < text.len() {
            let character = text[index..].chars().next().ok_or_else(|| {
                ProviderError::internal(format!(
                    "comment sentence offset {index} is outside line {:?}",
                    line.span
                ))
            })?;
            let length = character.len_utf8();
            let span = Span::at(line.span.file, base + index as u32, length as u32);

            // begin a sentence at its first non-whitespace scalar
            if current.is_none() && !character.is_whitespace() {
                let remainder = &text[index..];
                url_end = (remainder.starts_with("http://") || remainder.starts_with("https://"))
                    .then(|| {
                        index
                            + remainder
                                .find(char::is_whitespace)
                                .map_or(remainder.len(), |end| end)
                    });
                *current = Some(SentenceBuilder {
                    start_line: line,
                    end_line: line,
                    start: span.start,
                    end: span.end,
                    punctuation_position: span.end,
                    initial: CommentSentence::initial(remainder, span),
                    is_annotation,
                });
            }
            let Some(sentence) = current.as_mut() else {
                index += length;
                continue;
            };
            sentence.end = span.end;
            sentence.end_line = line;

            // ignore casing and punctuation inside inline source
            if character == '`' {
                is_inline_code = !is_inline_code;
                index += length;
                continue;
            }

            // exclude Markdown link destinations from prose punctuation
            if !is_inline_code && character == '(' && text[..index].ends_with(']') {
                link_destination_depth = 1;
            } else if link_destination_depth > 0 && character == '(' {
                link_destination_depth += 1;
            } else if link_destination_depth > 0 && character == ')' {
                link_destination_depth -= 1;
            }
            if !character.is_whitespace() && !Self::is_closing_decoration(character) {
                sentence.punctuation_position = span.end;
            }

            // close sentences only at punctuation followed by whitespace or line end
            let remainder = &text[index + length..];
            let closing = Self::closing_decoration_length(remainder);
            let after_closing = &remainder[closing..];
            let has_boundary = after_closing.chars().next().is_none_or(char::is_whitespace);
            let is_url_interior = url_end.is_some_and(|end| index + length < end);
            let ending =
                if is_inline_code || link_destination_depth > 0 || is_url_interior || !has_boundary
                {
                    None
                } else if character == '.'
                    && (!Self::is_dotted_initialism(text, index)
                        || (remainder.is_empty() && ends_paragraph))
                {
                    Some(CommentSentenceEnding::Period(span))
                } else if character == '?' {
                    Some(CommentSentenceEnding::Question(span))
                } else if character == '!' {
                    Some(CommentSentenceEnding::Exclamation(span))
                } else if character == ':' && remainder.is_empty() {
                    Some(CommentSentenceEnding::Colon(span))
                } else {
                    None
                };
            if let Some(ending) = ending {
                sentence.end += closing as u32;
                let sentence = current.take().ok_or_else(|| {
                    ProviderError::internal(format!(
                        "comment punctuation {span:?} has no active sentence"
                    ))
                })?;
                sentences.push(sentence.finish(ending, is_introduction));
                index += closing;
            }
            index += length;
        }

        Ok(())
    }

    /// Complete an unfinished sentence without terminal punctuation.
    fn finish_missing(
        current: &mut Option<SentenceBuilder<'a>>,
        sentences: &mut Vec<CommentSentence<'a>>,
    ) {
        if let Some(sentence) = current.take() {
            let missing = Span::at(
                sentence.start_line.span.file,
                sentence.punctuation_position,
                0,
            );
            sentences.push(sentence.finish(CommentSentenceEnding::Missing(missing), false));
        }
    }

    /// Return whether a character closes Markdown or quoted prose.
    fn is_closing_decoration(character: char) -> bool {
        matches!(character, ')' | ']' | '}' | '"' | '\'' | '*' | '_' | '~')
    }

    /// Return the byte length of closing Markdown and quote decoration.
    fn closing_decoration_length(source: &str) -> usize {
        source
            .char_indices()
            .take_while(|(_, character)| Self::is_closing_decoration(*character))
            .map(|(index, character)| index + character.len_utf8())
            .last()
            .map_or(0, |length| length)
    }

    /// Return whether a period closes a dotted initialism such as `i.e.`.
    fn is_dotted_initialism(source: &str, period: usize) -> bool {
        let start = source[..period]
            .char_indices()
            .rev()
            .find(|(_, character)| character.is_whitespace())
            .map_or(0, |(index, character)| index + character.len_utf8());
        let token =
            source[start..=period].trim_start_matches(|character: char| !character.is_alphabetic());
        let mut segments = token.split('.');
        let mut count = 0;

        // require two or more one-letter segments and one trailing period
        for segment in segments.by_ref() {
            if segment.is_empty() {
                break;
            }
            if segment.chars().count() != 1 || !segment.chars().all(char::is_alphabetic) {
                return false;
            }
            count += 1;
        }

        count >= 2 && token.ends_with('.')
    }
}

impl<'a> SentenceBuilder<'a> {
    /// Complete this sentence with its authored ending.
    fn finish(self, ending: CommentSentenceEnding, is_introduction: bool) -> CommentSentence<'a> {
        CommentSentence {
            span: Span::new(self.start_line.span.file, self.start, self.end),
            start_line: self.start_line,
            end_line: self.end_line,
            initial: self.initial,
            ending,
            is_annotation: self.is_annotation,
            is_introduction,
        }
    }
}

impl<'a> CommentLine<'a> {
    /// Append the physical source lines covered by one retained comment.
    fn append(
        file: &'a File,
        comment: dir::Comment,
        lines: &mut Vec<Self>,
    ) -> Result<(), ProviderError> {
        let last_byte = comment.span.end.checked_sub(1).ok_or_else(|| {
            ProviderError::internal(format!("comment {:?} has an empty span", comment.span))
        })?;
        let (first_line, first_column) =
            file.get_position(comment.span.start).ok_or_else(|| {
                ProviderError::internal(format!(
                    "comment start {:?} is outside its source file",
                    comment.span
                ))
            })?;
        let (last_line, _) = file.get_position(last_byte).ok_or_else(|| {
            ProviderError::internal(format!(
                "comment end {:?} is outside its source file",
                comment.span
            ))
        })?;

        // retain exact source coordinates for every physical line
        for line in first_line..=last_line {
            let source_line = file.get_line_span(line).ok_or_else(|| {
                ProviderError::internal(format!("source file {:?} has no line {line}", file.id))
            })?;
            let span = Span::new(
                file.id,
                source_line.start.max(comment.span.start),
                source_line.end.min(comment.span.end),
            );
            let content = Self::content_span(file, comment, span, line, first_line, last_line)?;
            let text = file.get_span_str(content).ok_or_else(|| {
                ProviderError::internal(format!(
                    "comment content {content:?} is outside its source file"
                ))
            })?;
            let (content, text, spacing, indentation_width) =
                Self::trim_decoration(content, text, comment, line == first_line, first_column);
            let width = Self::source_width(file, source_line.start, span.end)?;
            lines.push(Self {
                file,
                comment,
                span,
                content,
                text,
                spacing,
                line,
                width,
                indentation_width,
            });
        }

        Ok(())
    }

    /// Return one comment line's range inside its opening and closing delimiters.
    fn content_span(
        file: &File,
        comment: dir::Comment,
        line: Span,
        line_index: u32,
        first_line: u32,
        last_line: u32,
    ) -> Result<Span, ProviderError> {
        let mut start = line.start;
        let mut end = line.end;

        // strip the opening delimiter from the first physical line
        if line_index == first_line {
            let delimiter = match (comment.is_line(), comment.is_documentation()) {
                (true, true) => "///",
                (true, false) => "//",
                (false, true) => "/**",
                (false, false) => "/*",
            };
            let source = file.get_span_str(line).ok_or_else(|| {
                ProviderError::internal(format!("comment line {line:?} is outside its source file"))
            })?;
            if !source.starts_with(delimiter) {
                return Err(ProviderError::internal(format!(
                    "comment {:?} has no {delimiter:?} delimiter",
                    comment.span
                )));
            }
            start += delimiter.len() as u32;
        }

        // strip the closing delimiter from the last block-comment line
        if !comment.is_line() && line_index == last_line {
            let source = file.get_span_str(line).ok_or_else(|| {
                ProviderError::internal(format!("comment line {line:?} is outside its source file"))
            })?;
            if !source.ends_with("*/") {
                return Err(ProviderError::internal(format!(
                    "block comment {:?} has no closing delimiter",
                    comment.span
                )));
            }
            end -= 2;
        }
        if start > end {
            return Err(ProviderError::internal(format!(
                "comment line {line:?} has overlapping delimiters"
            )));
        }

        Ok(Span::new(line.file, start, end))
    }

    /// Remove conventional whitespace and block-comment decoration.
    fn trim_decoration(
        content: Span,
        mut text: &'a str,
        comment: dir::Comment,
        is_first_line: bool,
        block_column: u32,
    ) -> (Span, &'a str, Option<Span>, u32) {
        let mut start = content.start;
        let mut has_decoration = is_first_line || comment.is_line();

        // remove source indentation and an optional block-comment star
        if comment.is_block() && !is_first_line {
            let leading = text.len() - text.trim_start_matches([' ', '\t']).len();
            let source_indentation = leading.min(block_column as usize);
            text = &text[source_indentation..];
            start += source_indentation as u32;
            let star_indentation = text.len() - text.trim_start_matches([' ', '\t']).len();
            if text[star_indentation..].starts_with('*') {
                text = &text[star_indentation + 1..];
                start += star_indentation as u32 + 1;
                has_decoration = true;
            }
        }

        // retain exact decoration spacing and measure additional indentation
        let spacing_start = start;
        let spacing_bytes = text.len() - text.trim_start_matches([' ', '\t']).len();
        let spacing_text = &text[..spacing_bytes];
        let indentation_text = if has_decoration {
            spacing_text.strip_prefix(' ').unwrap_or(spacing_text)
        } else {
            spacing_text
        };
        let indentation_width = Self::display_width(indentation_text);
        let spacing = has_decoration.then(|| {
            Span::new(
                content.file,
                spacing_start,
                spacing_start + spacing_bytes as u32,
            )
        });
        text = &text[spacing_bytes..];
        start += spacing_bytes as u32;

        // exclude trailing horizontal whitespace from prose analysis
        let trimmed = text.trim_end_matches([' ', '\t']);
        let end = start + trimmed.len() as u32;

        (
            Span::new(content.file, start, end),
            trimmed,
            spacing,
            indentation_width,
        )
    }

    /// Return the visual width from a physical line start through a comment.
    fn source_width(file: &File, start: u32, end: u32) -> Result<u32, ProviderError> {
        let span = Span::new(file.id, start, end);
        let source = file.get_span_str(span).ok_or_else(|| {
            ProviderError::internal(format!(
                "comment width span {span:?} is outside its source file"
            ))
        })?;
        let source = source.trim_end_matches([' ', '\t']);

        Ok(Self::display_width(source))
    }

    /// Return the visual width of one source fragment.
    fn display_width(source: &str) -> u32 {
        let mut width = 0;

        // advance tabs to the next standard tab stop
        for character in source.chars() {
            if character == '\t' {
                width += TAB_WIDTH - width % TAB_WIDTH;
            } else {
                width += u32::from(character.terminal_display_width());
            }
        }

        width
    }

    /// Return whether this physical line uses a line-comment delimiter.
    pub fn is_line_comment(self) -> bool {
        self.comment.is_line()
    }

    /// Return whether this comment begins after only horizontal whitespace.
    fn is_leading(self) -> Result<bool, ProviderError> {
        let source_line = self.file.get_line_span(self.line).ok_or_else(|| {
            ProviderError::internal(format!(
                "source file {:?} has no line {}",
                self.file.id, self.line
            ))
        })?;
        let leading_span = Span::new(self.span.file, source_line.start, self.span.start);
        let leading = self.file.get_span_str(leading_span).ok_or_else(|| {
            ProviderError::internal(format!(
                "comment prefix {leading_span:?} is outside its source file"
            ))
        })?;

        Ok(leading.bytes().all(|byte| matches!(byte, b' ' | b'\t')))
    }

    /// Return whether this line uses exactly the requested decoration spacing.
    pub fn has_spacing(self, expected: &str) -> Result<bool, ProviderError> {
        let Some(spacing) = self.spacing else {
            return Ok(true);
        };
        let authored = self.file.get_span_str(spacing).ok_or_else(|| {
            ProviderError::internal(format!(
                "comment spacing {spacing:?} is outside its source file"
            ))
        })?;

        Ok(authored == expected)
    }

    /// Return whether decorated content follows one conventional separator.
    pub fn has_separator(self) -> Result<bool, ProviderError> {
        let Some(spacing) = self.spacing else {
            return Ok(true);
        };
        let authored = self.file.get_span_str(spacing).ok_or_else(|| {
            ProviderError::internal(format!(
                "comment spacing {spacing:?} is outside its source file"
            ))
        })?;

        Ok(self.text.is_empty() || authored.starts_with(' '))
    }

    /// Return a line break before one source position with canonical comment spacing.
    pub fn break_before(
        self,
        position: u32,
        empty_lines: usize,
        spaces: usize,
    ) -> Result<Option<(Span, String)>, ProviderError> {
        if !self.comment.is_line() || !self.is_leading()? {
            return Ok(None);
        }
        if position < self.content.start || position > self.content.end {
            return Err(ProviderError::internal(format!(
                "comment break position {position} is outside {:?}",
                self.content
            )));
        }
        let leading = Span::new(self.content.file, self.content.start, position);
        let leading = self.file.get_span_str(leading).ok_or_else(|| {
            ProviderError::internal(format!(
                "comment break position {position} is outside its source file"
            ))
        })?;
        let whitespace = leading.len() - leading.trim_end_matches([' ', '\t']).len();
        let span = Span::new(self.content.file, position - whitespace as u32, position);
        let prefix = self.prefix(spaces)?;
        let blank = self.prefix(0)?;
        let mut replacement = String::new();

        // insert the requested empty comment lines before the continued prose
        for _ in 0..empty_lines {
            replacement.push('\n');
            replacement.push_str(&blank);
        }
        replacement.push('\n');
        replacement.push_str(&prefix);

        Ok(Some((span, replacement)))
    }

    /// Return an empty comment line inserted before this physical line.
    pub fn blank_before(self) -> Result<Option<(Span, String)>, ProviderError> {
        if !self.comment.is_line() || !self.is_leading()? {
            return Ok(None);
        }
        let source_line = self.file.get_line_span(self.line).ok_or_else(|| {
            ProviderError::internal(format!(
                "source file {:?} has no line {}",
                self.file.id, self.line
            ))
        })?;
        let prefix = self.prefix(0)?;
        let replacement = format!("{prefix}\n");
        let span = Span::at(self.span.file, source_line.start, 0);

        Ok(Some((span, replacement)))
    }

    /// Return source replacements that wrap this line before a visual column limit.
    pub fn wrap(
        self,
        maximum_width: u32,
        first_spacing: usize,
        continuation_spacing: usize,
    ) -> Result<Option<Vec<(Span, String)>>, ProviderError> {
        let is_plain_prose = matches!(
            self.content_role(),
            CommentLineContent::Prose { start: 0, .. }
        );
        if !self.comment.is_line()
            || !is_plain_prose
            || self.text.contains('\t')
            || !self.is_leading()?
        {
            return Ok(None);
        }
        let first_prefix = self.prefix(first_spacing)?;
        let continuation_prefix = self.prefix(continuation_spacing)?;
        let first_width = Self::display_width(&first_prefix);
        let continuation_width = Self::display_width(&continuation_prefix);
        let Some(mut available) = maximum_width.checked_sub(first_width) else {
            return Ok(None);
        };
        let Some(continuation_width) = maximum_width.checked_sub(continuation_width) else {
            return Ok(None);
        };
        let replacement = format!("\n{continuation_prefix}");
        let mut start = 0;
        let mut patches = Vec::new();

        // split every remaining overlong segment at its final fitting whitespace run
        while Self::display_width(&self.text[start..]) > available {
            let Some((break_start, break_end)) =
                Self::wrapping_break(&self.text[start..], available)
            else {
                return Ok(None);
            };
            let break_start = start + break_start;
            let break_end = start + break_end;
            let span = self.content.subspan(break_start..break_end);
            patches.push((span, replacement.clone()));
            start = break_end;
            available = continuation_width;
        }

        Ok((!patches.is_empty()).then_some(patches))
    }

    /// Return the final fitting whitespace run in one prospective comment line.
    fn wrapping_break(source: &str, maximum_width: u32) -> Option<(usize, usize)> {
        let mut run = None;
        let mut candidate = None;
        let mut width = 0;
        let mut is_inline_code = false;
        let mut bracket_depth = 0;
        let mut link_depth = 0;
        let mut previous = None;

        // retain complete whitespace runs outside inline source and links
        for (index, character) in source.char_indices() {
            if character == '`' {
                is_inline_code = !is_inline_code;
            } else if !is_inline_code && link_depth > 0 && character == '(' {
                link_depth += 1;
            } else if !is_inline_code && link_depth > 0 && character == ')' {
                link_depth -= 1;
            } else if !is_inline_code && character == '[' {
                bracket_depth += 1;
            } else if !is_inline_code && bracket_depth > 0 && character == ']' {
                bracket_depth -= 1;
            } else if !is_inline_code && character == '(' && previous == Some(']') {
                link_depth = 1;
            }
            let is_protected = is_inline_code || bracket_depth > 0 || link_depth > 0;

            // begin one eligible whitespace run inside the available width
            if character.is_whitespace() && !is_protected {
                if run.is_none() {
                    if width > maximum_width {
                        break;
                    }
                    run = Some(index);
                }
            }
            // retain the completed run as the latest wrapping candidate
            else if let Some(start) = run.take()
                && start > 0
            {
                candidate = Some((start, index));
            }

            width += u32::from(character.terminal_display_width());
            previous = Some(character);
        }

        candidate
    }

    /// Return the physical source prefix through canonical decoration spacing.
    fn prefix(self, spaces: usize) -> Result<String, ProviderError> {
        let spacing = self.spacing.ok_or_else(|| {
            ProviderError::internal(format!(
                "comment line {:?} has no decorated prefix",
                self.span
            ))
        })?;
        let source_line = self.file.get_line_span(self.line).ok_or_else(|| {
            ProviderError::internal(format!(
                "source file {:?} has no line {}",
                self.file.id, self.line
            ))
        })?;
        let prefix_span = Span::new(self.span.file, source_line.start, spacing.start);
        let prefix = self.file.get_span_str(prefix_span).ok_or_else(|| {
            ProviderError::internal(format!(
                "comment prefix {prefix_span:?} is outside its source file"
            ))
        })?;
        let mut canonical = String::with_capacity(prefix.len() + spaces);
        canonical.push_str(prefix);
        canonical.extend(std::iter::repeat_n(' ', spaces));

        Ok(canonical)
    }

    /// Classify this line as prose or Markdown structure.
    fn content_role(self) -> CommentLineContent {
        let text = self.text;
        if text.is_empty() {
            return CommentLineContent::Blank;
        }
        if self.is_code_fence() {
            return CommentLineContent::Fence;
        }
        if Self::is_markdown_structure(text) {
            return CommentLineContent::Markdown;
        }
        let mut start = 0;

        // remove block-quote prefixes before prose classification
        while let Some(rest) = text[start..].strip_prefix('>') {
            start += 1;
            if rest.starts_with(' ') {
                start += 1;
            }
        }
        if Self::is_markdown_structure(&text[start..]) {
            return CommentLineContent::Markdown;
        }

        // remove one unordered or ordered list marker
        let remainder = &text[start..];
        let list = Self::list_prefix(remainder);
        let is_list = list.is_some();
        if let Some(length) = list {
            start += length;
        }

        // treat annotation headers as structure and lint their prose body
        let remainder = &text[start..];
        let annotation = ["NOTE", "TODO", "FUGU", "SAFETY"]
            .into_iter()
            .find(|keyword| {
                remainder.strip_prefix(keyword).is_some_and(|suffix| {
                    suffix.is_empty()
                        || suffix.starts_with(' ')
                        || suffix.starts_with('#')
                        || suffix.starts_with(':')
                })
            });
        let mut is_annotation = false;
        if annotation.is_some()
            && let Some(colon) = remainder.find(':')
        {
            start += colon + 1;
            if text[start..].starts_with(' ') {
                start += 1;
            }
            is_annotation = true;
        }

        CommentLineContent::Prose {
            start,
            is_annotation,
            is_list,
        }
    }

    /// Return whether a complete line is Markdown structure rather than prose.
    fn is_markdown_structure(text: &str) -> bool {
        let is_heading = text.starts_with("# ")
            || text.starts_with("## ")
            || text.starts_with("### ")
            || text.starts_with("#### ")
            || text.starts_with("##### ")
            || text.starts_with("###### ");
        let is_directive = text.starts_with('@');
        let is_table = text.starts_with('|')
            || (text.contains('|')
                && text
                    .chars()
                    .all(|character| matches!(character, ' ' | '-' | ':' | '|')));
        let is_rule = text.len() >= 3
            && text
                .chars()
                .all(|character| matches!(character, '-' | '_' | '*' | ' '));

        is_heading || is_directive || is_table || is_rule
    }

    /// Return the byte length of one Markdown list marker.
    fn list_prefix(text: &str) -> Option<usize> {
        if text.starts_with("- ") || text.starts_with("* ") || text.starts_with("+ ") {
            return Some(2);
        }
        let digits = text.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 && text[digits..].starts_with(". ") {
            Some(digits + 2)
        } else {
            None
        }
    }

    /// Return whether this line opens or closes a fenced code block.
    pub fn is_code_fence(self) -> bool {
        self.text.starts_with("```") || self.text.starts_with("~~~")
    }

    /// Return whether this line is an indented code block line.
    pub fn is_indented_code(self) -> bool {
        self.indentation_width >= TAB_WIDTH
    }

    /// Return whether this complete line is Markdown outside prose.
    pub fn is_markdown(self) -> bool {
        matches!(
            self.content_role(),
            CommentLineContent::Markdown | CommentLineContent::Fence
        )
    }

    /// Return whether this line contains one unbreakable source token.
    pub fn is_unbreakable(self) -> bool {
        !self.text.is_empty() && self.text.split_whitespace().count() == 1
    }
}

impl CommentSentence<'_> {
    /// Return whether this sentence continues onto the given physical line.
    pub fn continues_on(self, line: CommentLine<'_>) -> bool {
        line.span.file == self.span.file
            && line.line > self.start_line.line
            && line.line <= self.end_line.line
    }

    /// Return the initial prose character when its word uses conventional casing.
    fn initial(source: &str, start: Span) -> Option<(Span, char)> {
        // preserve complete source addresses
        if source.starts_with("http://") || source.starts_with("https://") {
            return None;
        }

        // skip opening prose and Markdown decoration
        let trimmed = source.trim_start_matches(['(', '{', '"', '\'', '*', '~']);
        let offset = source.len() - trimmed.len();
        let source = &source[offset..];

        // require a cased initial token
        let initial = source.chars().next()?;
        if !initial.is_alphabetic() {
            return None;
        }

        // preserve source names and stylized proper names
        let word = source
            .chars()
            .take_while(|character| character.is_alphabetic() || *character == '_');
        let is_stylized = word
            .skip(1)
            .any(|character| character == '_' || character.is_uppercase());
        if is_stylized {
            return None;
        }

        // retain the exact source scalar for a correction
        let span = Span::at(
            start.file,
            start.start + offset as u32,
            initial.len_utf8() as u32,
        );

        Some((span, initial))
    }
}

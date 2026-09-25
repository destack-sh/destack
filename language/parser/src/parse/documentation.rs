use tspp_core::StringId;
use tspp_dir::{
    Comment, Declaration, Documentation, DocumentationTag, GenericParameter, LocalNodeId,
    LocalNodeIdAny, Member, Node, NodeType, Parameter, Property, StaticKey, TypeExpression,
    TypeMember,
};
use tspp_source::Span;

use crate::{Parser, ParserError};

/// One adjacent authored documentation comment group.
pub(crate) struct DocumentationComment {
    /// The complete source span.
    span: Span,
    /// The comment text lines.
    lines: Vec<DocumentationLine>,
}

/// One documentation line and its authored source span.
struct DocumentationLine {
    /// The authored line content span.
    span: Span,
}

impl DocumentationLine {
    /// Return the authored line text.
    fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[self.span.start as usize..self.span.end as usize]
    }
}

/// One parsed documentation tag header.
enum DocumentationTagHeader<'a> {
    /// One `@param` header.
    Parameter {
        /// The declared parameter name.
        name: &'a str,
        /// The Markdown after the parameter name.
        markdown: &'a str,
    },
    /// One `@typeParam` header.
    TypeParameter {
        /// The declared generic parameter name.
        name: &'a str,
        /// The Markdown after the parameter name.
        markdown: &'a str,
    },
    /// One `@example` header.
    Example {
        /// The Markdown after the tag.
        markdown: &'a str,
    },
    /// One general section header kept as authored.
    Section,
}

impl Parser {
    /// Parse documentation before the current token.
    pub(crate) fn parse_documentation(&mut self) -> Option<DocumentationComment> {
        let token_start = self.cursor.peek().start();
        let comments = self.cursor.comments();
        let unowned_start = self.next_documentation_comment;

        // skip comments attached to earlier source boundaries
        while let Some(comment) = comments.get(self.next_documentation_comment) {
            let is_before_current = comment
                .following_token_start()
                .map_or(comment.span.end <= token_start, |start| start < token_start);
            if !is_before_current {
                break;
            }

            self.next_documentation_comment += 1;
        }

        // take comments attached to the current token boundary
        let group_start = self.next_documentation_comment;
        while comments
            .get(self.next_documentation_comment)
            .is_some_and(|comment| comment.following_token_start() == Some(token_start))
        {
            self.next_documentation_comment += 1;
        }
        let group_end = self.next_documentation_comment;
        if group_start == group_end {
            self.report_unowned_documentation(unowned_start, group_end);

            return None;
        }

        // skip over ordinary line comments between the documentation and its owner
        let mut documentation_end = group_end;
        let mut following_start = token_start;
        while documentation_end > group_start {
            let comment = comments[documentation_end - 1];
            if comment.is_documentation()
                || !comment.is_line()
                || !self.documentation_is_adjacent(comment.span.end, following_start)
            {
                break;
            }

            documentation_end -= 1;
            following_start = comment.span.start;
        }

        // select the final contiguous documentation block
        let mut documentation_start = documentation_end;
        while documentation_start > group_start {
            let comment = comments[documentation_start - 1];
            if !comment.is_documentation()
                || !self.documentation_is_adjacent(comment.span.end, following_start)
            {
                break;
            }

            documentation_start -= 1;
            following_start = comment.span.start;
        }
        if documentation_start == documentation_end {
            self.report_unowned_documentation(unowned_start, group_end);

            return None;
        }

        // retain comment text with exact source spans
        let first = comments[documentation_start];
        let last = comments[documentation_end - 1];
        let span = first.span.merge(last.span);
        let mut lines = Vec::new();
        for comment in comments[documentation_start..documentation_end]
            .iter()
            .copied()
        {
            lines.extend(self.documentation_lines(comment));
        }

        self.report_unowned_documentation(unowned_start, documentation_start);

        Some(DocumentationComment { span, lines })
    }

    /// Diagnose documentation comments that were not attached to a node.
    fn report_unowned_documentation(&mut self, start: usize, end: usize) {
        for index in start..end {
            let comment = self.cursor.comments()[index];
            if comment.is_documentation() && !comment.is_legal() {
                self.report_error(ParserError::invalid_documentation_owner(
                    comment.span.range(),
                ));
            }
        }
    }

    /// Diagnose retained documentation left after parsing.
    pub(super) fn finalize_documentation(&mut self) {
        let end = self.cursor.comments().len();
        self.report_unowned_documentation(self.next_documentation_comment, end);
        self.next_documentation_comment = end;
    }

    /// Attach parsed documentation to one node.
    pub(crate) fn attach_documentation<T>(
        &mut self,
        node_id: LocalNodeId<T>,
        documentation: Option<DocumentationComment>,
    ) where
        T: Node,
    {
        let Some(documentation) = documentation else {
            return;
        };
        let node_id = node_id.into_any();
        let file = self.file.clone();
        let documentation = documentation.into_documentation(node_id, file.text(), self);
        let Some(documentation) = documentation else {
            return;
        };

        self.tree.set_documentation(node_id.id, documentation);
    }

    /// Return the callable parameters directly owned by one node.
    fn callable_parameters(&self, node_id: LocalNodeIdAny) -> Option<&[LocalNodeId<Parameter>]> {
        match node_id.ty {
            NodeType::Declaration => match self.tree.get(node_id.into_typed::<Declaration>()) {
                Declaration::Function(declaration) => Some(&declaration.signature.parameters),
                _ => None,
            },
            NodeType::Member => self
                .tree
                .get(node_id.into_typed::<Member>())
                .signature()
                .map(|signature| signature.parameters.as_slice()),
            NodeType::Property => self
                .tree
                .get(node_id.into_typed::<Property>())
                .signature()
                .map(|signature| signature.parameters.as_slice()),
            NodeType::TypeMember => match self.tree.get(node_id.into_typed::<TypeMember>()) {
                TypeMember::Method { signature, .. } => Some(&signature.parameters),
                TypeMember::CallSignature { signature } => Some(&signature.parameters),
                TypeMember::ConstructSignature { signature } => Some(&signature.parameters),
                _ => None,
            },
            NodeType::TypeExpression => {
                match self.tree.get(node_id.into_typed::<TypeExpression>()) {
                    TypeExpression::Function(function) => Some(&function.parameters),
                    TypeExpression::Constructor(constructor) => Some(&constructor.parameters),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Return the generic parameters directly owned by one node.
    fn generic_parameters(
        &self,
        node_id: LocalNodeIdAny,
    ) -> Option<&[LocalNodeId<GenericParameter>]> {
        match node_id.ty {
            NodeType::Declaration => self
                .tree
                .get(node_id.into_typed::<Declaration>())
                .generic_parameters(),
            NodeType::Member => match self.tree.get(node_id.into_typed::<Member>()) {
                Member::AssociatedType {
                    generic_parameters, ..
                } => Some(generic_parameters),
                Member::Method { signature, .. } => Some(&signature.generic_parameters),
                _ => None,
            },
            NodeType::Property => self
                .tree
                .get(node_id.into_typed::<Property>())
                .signature()
                .map(|signature| signature.generic_parameters.as_slice()),
            NodeType::TypeMember => match self.tree.get(node_id.into_typed::<TypeMember>()) {
                TypeMember::AssociatedType {
                    generic_parameters, ..
                } => Some(generic_parameters),
                TypeMember::Method { signature, .. } => Some(&signature.generic_parameters),
                TypeMember::CallSignature { signature } => Some(&signature.generic_parameters),
                TypeMember::ConstructSignature { signature } => Some(&signature.generic_parameters),
                _ => None,
            },
            NodeType::TypeExpression => {
                match self.tree.get(node_id.into_typed::<TypeExpression>()) {
                    TypeExpression::Function(function) => Some(&function.generic_parameters),
                    TypeExpression::Constructor(constructor) => {
                        Some(&constructor.generic_parameters)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Bind one documentation tag to an exact child node.
    fn bind_documentation_tag(
        &mut self,
        owner: LocalNodeIdAny,
        header: &DocumentationTagHeader<'_>,
        markdown: StringId,
        span: Span,
        tags: &[DocumentationTag],
    ) -> Option<DocumentationTag> {
        let (name, is_parameter) = match header {
            DocumentationTagHeader::Parameter { name, .. } => (*name, true),
            DocumentationTagHeader::TypeParameter { name, .. } => (*name, false),
            DocumentationTagHeader::Example { .. } => {
                return Some(DocumentationTag::Example { markdown });
            }
            DocumentationTagHeader::Section => {
                return Some(DocumentationTag::Section { markdown });
            }
        };
        let name = self.strings.intern(name);

        // select the exact named child from the tag
        let target = if is_parameter {
            match self.callable_parameters(owner) {
                Some(parameters) => parameters.iter().find_map(|candidate| {
                    let parameter = self.tree.get(*candidate);
                    (parameter.symbol_key() == Some(StaticKey::Name(name)))
                        .then_some(candidate.into_any())
                }),
                None => {
                    self.report_error(ParserError::invalid_documentation_owner(span.range()));

                    return None;
                }
            }
        } else {
            match self.generic_parameters(owner) {
                Some(parameters) => parameters.iter().find_map(|candidate| {
                    let parameter = self.tree.get(*candidate);
                    (parameter.symbol_key() == Some(StaticKey::Name(name)))
                        .then_some(candidate.into_any())
                }),
                None => {
                    self.report_error(ParserError::invalid_documentation_owner(span.range()));

                    return None;
                }
            }
        };
        let Some(target) = target else {
            self.report_error(ParserError::missing_documentation_target(span.range()));

            return None;
        };

        // reject duplicate tag and direct child documentation
        let is_duplicate = self.tree.has_documentation(target.id)
            || tags.iter().any(|tag| tag.target() == Some(target));
        if is_duplicate {
            self.report_error(ParserError::duplicate_documentation_target(span.range()));

            return None;
        }

        if is_parameter {
            Some(DocumentationTag::Parameter {
                parameter: target.into_typed(),
                markdown,
            })
        } else {
            Some(DocumentationTag::TypeParameter {
                parameter: target.into_typed(),
                markdown,
            })
        }
    }

    /// Return source lines for one documentation comment.
    fn documentation_lines(&self, comment: Comment) -> Vec<DocumentationLine> {
        let closing_width = usize::from(comment.is_block()) * 2;
        let start = comment.span.start as usize + 3;
        let end = comment.span.end as usize - closing_width;
        let source = self.file.text();
        let mut lines = Vec::new();
        let mut line_start = start;

        // remove comment markers and retain exact content spans
        while line_start <= end {
            // find this physical line
            let newline = source[line_start..end].find('\n');
            let line_end = newline.map_or(end, |offset| line_start + offset);
            let mut content_start = line_start;
            let mut content_end = line_end;
            let bytes = source.as_bytes();

            // trim trailing source whitespace
            while content_end > content_start
                && matches!(bytes[content_end - 1], b' ' | b'\t' | b'\r')
            {
                content_end -= 1;
            }

            // remove the line or block leader
            if comment.is_line() {
                content_start += usize::from(bytes.get(content_start) == Some(&b' '));
            } else {
                while content_start < content_end && matches!(bytes[content_start], b' ' | b'\t') {
                    content_start += 1;
                }
                if bytes.get(content_start) == Some(&b'*') {
                    content_start += 1;
                    content_start += usize::from(bytes.get(content_start) == Some(&b' '));
                }
            }

            // retain the resulting text and span
            let span = Span::new(comment.span.file, content_start as u32, content_end as u32);
            lines.push(DocumentationLine { span });

            // advance to the next physical line
            let Some(_) = newline else {
                break;
            };

            line_start = line_end + 1;
        }

        lines
    }

    /// Return whether documentation is adjacent across one source range.
    fn documentation_is_adjacent(&self, start: u32, end: u32) -> bool {
        let bytes = self.file.text().as_bytes();
        let mut offset = start as usize;
        let mut line_breaks = 0;

        // reject intervening comments
        while offset < end as usize {
            if offset + 1 < end as usize
                && bytes[offset] == b'/'
                && matches!(bytes[offset + 1], b'/' | b'*')
            {
                return false;
            } else if bytes[offset] == b'\r' {
                line_breaks += 1;
                offset += usize::from(bytes.get(offset + 1) == Some(&b'\n'));
            } else if bytes[offset] == b'\n' {
                line_breaks += 1;
            }

            // reject blank lines
            if line_breaks > 1 {
                return false;
            }

            offset += 1;
        }

        true
    }
}

impl DocumentationComment {
    /// Parse this comment group for one exact owner.
    fn into_documentation(
        self,
        owner: LocalNodeIdAny,
        source: &str,
        parser: &mut Parser,
    ) -> Option<Documentation> {
        let tag_starts = self.tag_starts(source);
        let markdown_end = tag_starts
            .first()
            .map_or(self.lines.len(), |(start, _)| *start);
        let markdown = self.markdown(0, markdown_end, None, source);
        let markdown = parser.strings.intern(&markdown);
        let mut tags = Vec::with_capacity(tag_starts.len());
        let mut is_complete = true;

        // parse each block tag independently
        for (ordinal, (start, header)) in tag_starts.iter().enumerate() {
            let start = *start;
            let end = tag_starts
                .get(ordinal + 1)
                .map_or(self.lines.len(), |(next_start, _)| *next_start);
            let span = self.lines[start].span.merge(self.lines[end - 1].span);
            let markdown = match header {
                DocumentationTagHeader::Section => self.markdown(start, end, None, source),
                header => self.markdown(start + 1, end, Some(header.markdown()), source),
            };
            let markdown = parser.strings.intern(&markdown);
            let Some(tag) = parser.bind_documentation_tag(owner, header, markdown, span, &tags)
            else {
                is_complete = false;

                continue;
            };
            tags.push(tag);
        }

        is_complete.then_some(Documentation {
            span: self.span,
            markdown,
            tags,
        })
    }

    /// Return every recognized block tag line outside fenced code.
    fn tag_starts<'a>(&self, source: &'a str) -> Vec<(usize, DocumentationTagHeader<'a>)> {
        let mut starts = Vec::new();
        let mut fence = None;

        for (index, line) in self.lines.iter().enumerate() {
            let trimmed = line.text(source).trim_start();

            // track fenced Markdown blocks
            if let Some(marker) = documentation_fence(trimmed) {
                fence = match fence {
                    Some(open) if open == marker => None,
                    None => Some(marker),
                    open => open,
                };
            }

            // collect recognized block tags outside fences and keep other lines as prose
            if fence.is_none()
                && let Some(header) = DocumentationTagHeader::parse(trimmed)
            {
                starts.push((index, header));
            }
        }

        starts
    }

    /// Join one Markdown range.
    fn markdown(&self, start: usize, end: usize, first: Option<&str>, source: &str) -> String {
        let capacity = end.saturating_sub(start) + usize::from(first.is_some());
        let mut lines = Vec::with_capacity(capacity);

        // prepend Markdown from the tag header
        if let Some(first) = first {
            lines.push(first);
        }
        lines.extend(self.lines[start..end].iter().map(|line| line.text(source)));

        // trim only empty boundary lines
        let first = lines.iter().position(|line| !line.trim().is_empty());
        let Some(first) = first else {
            return String::new();
        };
        let last = lines
            .iter()
            .rposition(|line| !line.trim().is_empty())
            .unwrap_or(first);

        lines[first..=last].join("\n")
    }
}

impl<'a> DocumentationTagHeader<'a> {
    /// Return the Markdown following this tag header.
    fn markdown(&self) -> &'a str {
        match self {
            Self::Parameter { markdown, .. }
            | Self::TypeParameter { markdown, .. }
            | Self::Example { markdown } => markdown,
            Self::Section => "",
        }
    }

    /// Parse one complete block tag header.
    fn parse(line: &'a str) -> Option<Self> {
        let line = line.trim_start();

        // require a line-start tag identifier
        let identifier = documentation_tag_identifier(line)?;

        // parse examples without a declared target
        if identifier == "example" {
            if line == "@example" {
                return Some(Self::Example { markdown: "" });
            }

            let markdown = line.strip_prefix("@example ")?;

            return Some(Self::Example {
                markdown: markdown.trim(),
            });
        }

        // keep every non-target tag block as authored
        let is_parameter = identifier == "param";
        if !is_parameter && identifier != "typeParam" {
            return Some(Self::Section);
        }

        // select one named tag keyword
        let body = if is_parameter {
            line.strip_prefix("@param ")?
        } else {
            line.strip_prefix("@typeParam ")?
        };

        // split the declared name from its Markdown
        let name_end = body
            .find(|character: char| character.is_whitespace() || character == ':')
            .unwrap_or(body.len());
        let name = &body[..name_end];
        let markdown = body[name_end..].trim_start();

        // remove an optional dash or colon separator
        let has_separator = matches!(markdown.as_bytes().first(), Some(b'-' | b':'));
        let markdown = if has_separator {
            markdown[1..].trim_start()
        } else {
            markdown
        };

        // require both a target name and description
        if name.is_empty() || markdown.is_empty() {
            return None;
        }

        if is_parameter {
            Some(Self::Parameter { name, markdown })
        } else {
            Some(Self::TypeParameter { name, markdown })
        }
    }
}

/// Return the tag identifier opening one documentation line.
fn documentation_tag_identifier(line: &str) -> Option<&str> {
    let rest = line.strip_prefix('@')?;
    let end = rest
        .find(|character: char| !character.is_alphanumeric() && character != '_')
        .unwrap_or(rest.len());

    (end > 0).then(|| &rest[..end])
}

/// Return the opening or closing Markdown fence marker on one line.
fn documentation_fence(line: &str) -> Option<char> {
    if line.starts_with("```") {
        Some('`')
    } else if line.starts_with("~~~") {
        Some('~')
    } else {
        None
    }
}

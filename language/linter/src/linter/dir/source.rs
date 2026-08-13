use std::borrow::Cow;
use std::slice;

use destack_artifact::DiagnosticAnchor;
use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{File, FileId, NodeSpanBoundary, NodeSpanRegion, NodeSpanType, Span};

use super::DirModule;

impl DirModule<'_> {
    /// Return the required source span for one DIR node.
    pub fn span(&self, node: dir::LocalNodeIdAny) -> Result<Span, ProviderError> {
        self.view()
            .get_span_by_id(node.id)
            .ok_or_else(|| ProviderError::Internal {
                message: format!(
                    "DIR node {} in module {:?} has no source span",
                    node.id, self.id
                ),
            })
    }

    /// Return the main source span for one DIR node.
    pub fn main_span(&self, node: dir::LocalNodeIdAny) -> Result<Span, ProviderError> {
        self.view()
            .get_side_span_by_id(node.id, NodeSpanType::Main)
            .ok_or_else(|| ProviderError::Internal {
                message: format!(
                    "DIR node {} in module {:?} has no main source span",
                    node.id, self.id
                ),
            })
    }

    /// Return the complete authored source span for one DIR node.
    pub fn source_extent(&self, node: dir::LocalNodeIdAny) -> Result<Span, ProviderError> {
        self.view()
            .get_source_extent_by_id(node.id)
            .ok_or_else(|| ProviderError::Internal {
                message: format!(
                    "DIR node {} in module {:?} has no source extent",
                    node.id, self.id
                ),
            })
    }

    /// Return the authored parentheses around one DIR node.
    pub fn source_parentheses(&self, node: dir::LocalNodeIdAny) -> Option<Span> {
        self.view()
            .get_side_span_by_id(node.id, NodeSpanType::Region(NodeSpanRegion::Parentheses))
    }

    /// Return one required named source region for a DIR node.
    pub fn source_region(
        &self,
        node: dir::LocalNodeIdAny,
        region: NodeSpanRegion,
    ) -> Result<Span, ProviderError> {
        self.view()
            .get_side_span_by_id(node.id, NodeSpanType::Region(region))
            .ok_or_else(|| ProviderError::Internal {
                message: format!(
                    "DIR node {} in module {:?} has no {region:?} source region",
                    node.id, self.id
                ),
            })
    }

    /// Return whether one visible DIR node was authored directly.
    pub fn is_authored(&self, node: dir::LocalNodeIdAny) -> bool {
        self.view().get_source_any(node) == node.id
    }

    /// Return the source text covered by one span.
    pub fn source(&self, span: Span) -> Result<&str, ProviderError> {
        let source = self.file(span.file)?.text();
        let range = span.start as usize..span.end as usize;

        source.get(range).ok_or_else(|| ProviderError::Internal {
            message: format!("source span {span:?} is not a valid UTF-8 range"),
        })
    }

    /// Return authored expression text grouped for a minimum operator precedence.
    pub(crate) fn expression_source(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        minimum_precedence: dir::OperatorPrecedence,
    ) -> Result<Cow<'_, str>, ProviderError> {
        let extent = self.source_extent(expression.into_any())?;
        let parentheses = self.source_parentheses(expression.into_any());

        // retain existing grouping or meet the requested precedence
        let source = if let Some(parentheses) = parentheses {
            Cow::Borrowed(self.source(parentheses)?)
        } else if self.view().get(expression).precedence() >= minimum_precedence {
            Cow::Borrowed(self.source(extent)?)
        } else {
            let source = self.source(extent)?;

            Cow::Owned(format!("({source})"))
        };

        Ok(source)
    }

    /// Return whether an extent contains a comment outside the retained spans.
    pub fn has_unretained_comment(
        &self,
        extent: Span,
        retained: &[Span],
    ) -> Result<bool, ProviderError> {
        let parsed_file = self.parsed.file(extent.file).ok_or_else(|| {
            ProviderError::internal(format!(
                "source file {:?} is absent from parsed lint module {:?}",
                extent.file, self.id
            ))
        })?;
        let has_comment = parsed_file.comments.iter().any(|comment| {
            extent.contains_span(comment.span)
                && !retained
                    .iter()
                    .any(|retained| retained.contains_span(comment.span))
        });

        Ok(has_comment)
    }

    /// Return the source span and trailing boundary for one DIR statement.
    pub fn statement_span(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Span, ProviderError> {
        let view = self.view();
        let span = self.span(expression.into_any())?;
        let trailing = view.get_side_span(
            expression,
            NodeSpanType::Boundary(NodeSpanBoundary::Trailing),
        );

        Ok(trailing.map_or(span, |trailing| span.merge(trailing)))
    }

    /// Return the source span removed with one complete authored line.
    pub fn line_removal_span(&self, span: Span) -> Result<Span, ProviderError> {
        let source = self.file(span.file)?.text().as_bytes();
        let mut start = span.start as usize;

        // absorb horizontal indentation before the source span
        while start > 0 && matches!(source[start - 1], b' ' | b'\t') {
            start -= 1;
        }

        // absorb the following line break when the span occupies the line
        let mut end = span.end as usize;
        if (start == 0 || source[start - 1] == b'\n') && source.get(end) == Some(&b'\n') {
            end += 1;
        }

        Ok(Span::new(span.file, start as u32, end as u32))
    }

    /// Return the source span removed with one DIR statement.
    pub fn statement_removal_span(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Span, ProviderError> {
        let span = self.statement_span(expression)?;
        let source = self.file(span.file)?.text().as_bytes();
        let mut start = span.start as usize;

        // absorb the horizontal separator immediately before the statement
        while start > 0 && matches!(source[start - 1], b' ' | b'\t') {
            start -= 1;
        }

        // absorb the line break when only the block close follows
        let mut end = span.end as usize;
        if (start == 0 || source[start - 1] == b'\n') && source.get(end) == Some(&b'\n') {
            let mut next = end + 1;
            while next < source.len() && matches!(source[next], b' ' | b'\t') {
                next += 1;
            }
            if source.get(next) == Some(&b'}') {
                end += 1;
            }
        }

        Ok(Span::new(span.file, start as u32, end as u32))
    }

    /// Return a source anchor for one DIR node.
    pub fn anchor(&self, node: dir::LocalNodeIdAny) -> Result<DiagnosticAnchor, ProviderError> {
        let span = self.span(node)?;

        Ok(DiagnosticAnchor::Span(span))
    }

    /// Return the checked DIR tree.
    pub fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(&self.parsed.tree, slice::from_ref(&self.expanded.patch))
    }

    /// Return one source file.
    pub fn file(&self, file_id: FileId) -> Result<&File, ProviderError> {
        self.files
            .iter()
            .find(|file| file.id == file_id)
            .map(AsRef::as_ref)
            .ok_or_else(|| ProviderError::Internal {
                message: format!("file {file_id:?} is outside lint module {:?}", self.id),
            })
    }
}

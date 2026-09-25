use std::borrow::Cow;

use tspp_artifact::DiagnosticAnchor;
use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{File, FileId, NodeSpanBoundary, NodeSpanRegion, NodeSpanType, Patch, Span};

use super::DirModule;

/// The body kept where a removed statement leaves a required block.
const EMPTY_BODY: &str = "{ /* intentionally empty */ }";

impl DirModule<'_> {
    /// Return the token with exactly this source span.
    pub(crate) fn token(&self, span: Span) -> Result<dir::Token, ProviderError> {
        let parsed = self.stages.parsed.file(span.file).ok_or_else(|| {
            ProviderError::internal(format!(
                "source file {:?} is absent from parsed lint module {:?}",
                span.file, self.id
            ))
        })?;

        parsed.token(span.range()).ok_or_else(|| {
            ProviderError::internal(format!("source span {span:?} does not match one token"))
        })
    }

    /// Return the required source span for one node.
    pub fn span(&self, node: dir::LocalNodeIdAny) -> Result<Span, ProviderError> {
        self.view()
            .get_span_by_id(node.id)
            .ok_or_else(|| ProviderError::Internal {
                message: format!(
                    "node {} in module {:?} has no source span",
                    node.id, self.id
                ),
            })
    }

    /// Return the main source span for one node.
    pub fn main_span(&self, node: dir::LocalNodeIdAny) -> Result<Span, ProviderError> {
        self.view()
            .get_side_span_by_id(node.id, NodeSpanType::Main)
            .ok_or_else(|| ProviderError::Internal {
                message: format!(
                    "node {} in module {:?} has no main source span",
                    node.id, self.id
                ),
            })
    }

    /// Return the complete authored source span for one node.
    pub fn source_extent(&self, node: dir::LocalNodeIdAny) -> Result<Span, ProviderError> {
        self.view()
            .get_source_extent_by_id(node.id)
            .ok_or_else(|| ProviderError::Internal {
                message: format!(
                    "node {} in module {:?} has no source extent",
                    node.id, self.id
                ),
            })
    }

    /// Return the authored parentheses around one node.
    pub fn source_parentheses(&self, node: dir::LocalNodeIdAny) -> Option<Span> {
        self.view()
            .get_side_span_by_id(node.id, NodeSpanType::Region(NodeSpanRegion::Parentheses))
    }

    /// Return one required named source region for a node.
    pub fn source_region(
        &self,
        node: dir::LocalNodeIdAny,
        region: NodeSpanRegion,
    ) -> Result<Span, ProviderError> {
        self.view()
            .get_side_span_by_id(node.id, NodeSpanType::Region(region))
            .ok_or_else(|| ProviderError::Internal {
                message: format!(
                    "node {} in module {:?} has no {region:?} source region",
                    node.id, self.id
                ),
            })
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
        let parsed_file = self.stages.parsed.file(extent.file).ok_or_else(|| {
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

    /// Return source text with indentation removed from continuation lines.
    pub(crate) fn dedent_source(
        &self,
        span: Span,
        indentation: u32,
    ) -> Result<Option<String>, ProviderError> {
        let source = self.source(span)?;
        let prefix = " ".repeat(indentation as usize);
        let mut dedented = String::with_capacity(source.len());

        // remove the exact indentation from every nonempty continuation
        for (index, line) in source.split('\n').enumerate() {
            let line = if index == 0 || line.is_empty() {
                line
            } else {
                let Some(line) = line.strip_prefix(&prefix) else {
                    return Ok(None);
                };

                line
            };
            if index > 0 {
                dedented.push('\n');
            }
            dedented.push_str(line);
        }

        Ok(Some(dedented))
    }

    /// Return the horizontal indentation of the source line containing one span.
    pub(crate) fn source_indentation(&self, span: Span) -> Result<&str, ProviderError> {
        let file = self.file(span.file)?;
        let Some((line, _)) = file.get_position(span.start) else {
            return Err(ProviderError::internal(format!(
                "source span {span:?} is outside its authored source file"
            )));
        };
        let line = file.get_line_span(line).ok_or_else(|| {
            ProviderError::internal(format!("source line for span {span:?} is unavailable"))
        })?;
        let prefix = self.source(Span::new(span.file, line.start, span.start))?;
        let indentation_end = prefix
            .bytes()
            .take_while(|byte| matches!(byte, b' ' | b'\t'))
            .count();

        Ok(&prefix[..indentation_end])
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

    /// Build a patch that removes one statement while preserving required bodies.
    pub fn statement_removal_patch(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Patch, ProviderError> {
        let view = self.view();
        let replacement = match view.get_parent_for(expression) {
            // root statements can disappear completely
            None => None,
            // inspect the structural role of a containing block
            Some(parent) if parent.ty == dir::NodeType::Block => {
                let block_id = dir::LocalNodeId::<dir::Block>::new(parent.id);
                let block = view.get(block_id);
                let is_required = block.form == dir::BlockForm::Implicit
                    && block.only_expression() == Some(expression);

                // delete ordinary block statements
                if !is_required {
                    None
                }
                // preserve the required implicit body
                else {
                    let owner = view.get_parent_for(block_id).ok_or_else(|| {
                        ProviderError::internal(format!(
                            "implicit block {} in module {:?} has no DIR parent",
                            block_id.id, self.id
                        ))
                    })?;

                    // switch cases permit an empty statement list
                    (owner.ty != dir::NodeType::SwitchCase).then_some(EMPTY_BODY)
                }
            }
            // preserve required expression bodies
            Some(parent) if matches!(parent.ty, dir::NodeType::MatchArm | dir::NodeType::Catch) => {
                Some(EMPTY_BODY)
            }
            // preserve a directly authored finally body
            Some(parent) if parent.ty == dir::NodeType::Expression => {
                let parent = dir::LocalNodeId::<dir::Expression>::new(parent.id);
                let is_finally = matches!(
                    view.get(parent),
                    dir::Expression::Try {
                        finally: Some(finally),
                        ..
                    } if *finally == expression
                );
                if !is_finally {
                    return Err(ProviderError::internal(format!(
                        "statement {} in module {:?} has invalid DIR parent {parent:?}",
                        expression.id, self.id
                    )));
                }

                Some(EMPTY_BODY)
            }
            Some(parent) => {
                return Err(ProviderError::internal(format!(
                    "statement {} in module {:?} has invalid DIR parent {parent:?}",
                    expression.id, self.id
                )));
            }
        };

        // replace required bodies and delete ordinary statements
        let patch = match replacement {
            Some(replacement) => Patch::replace(self.statement_span(expression)?, replacement),
            None => Patch::delete(self.statement_removal_span(expression)?),
        };

        Ok(patch)
    }

    /// Return a source anchor for one node.
    pub fn anchor(&self, node: dir::LocalNodeIdAny) -> Result<DiagnosticAnchor, ProviderError> {
        let span = self.span(node)?;

        Ok(DiagnosticAnchor::Span(span))
    }

    /// Return the DIR tree.
    pub fn view(&self) -> dir::View<'_> {
        self.stages.tree()
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

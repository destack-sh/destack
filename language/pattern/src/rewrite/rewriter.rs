use tspp_dir as dir;
use tspp_source::{DiagnosticCollection, File, FilePatch, Patch, Span};

use super::renderer::Renderer;
use crate::{PatternMatch, Rewrite, RewriteError};

/// One rendered source edit selected by a structural match.
struct RewriteEdit {
    /// The complete candidate source replaced by the edit.
    span: Span,
    /// The rendered replacement source.
    text: String,
}

/// A structural rewrite over one candidate DIR and source file.
#[derive(Debug)]
pub struct Rewriter<'rewrite, 'candidate> {
    /// The compiled rewrite.
    rewrite: &'rewrite Rewrite,
    /// The candidate nodes.
    candidate: dir::View<'candidate>,
    /// The authored candidate source.
    source: &'candidate File,
}

impl<'rewrite, 'candidate> Rewriter<'rewrite, 'candidate> {
    /// Create a structural rewriter.
    pub fn new(
        rewrite: &'rewrite Rewrite,
        candidate: dir::View<'candidate>,
        source: &'candidate File,
    ) -> Self {
        Self {
            rewrite,
            candidate,
            source,
        }
    }

    /// Rewrite non-overlapping matches in source order.
    pub fn rewrite(
        &self,
        matches: impl IntoIterator<Item = PatternMatch>,
    ) -> Result<FilePatch, DiagnosticCollection> {
        let matches = matches.into_iter();
        let renderer = Renderer::new(self.rewrite.replacement(), self.candidate, self.source);
        let mut edits = Vec::with_capacity(matches.size_hint().0);

        // render every match against its complete source span
        for pattern_match in matches {
            let span = self
                .candidate
                .get_decorated_span(pattern_match.root)
                .ok_or_else(|| {
                    RewriteError::internal(self.source, "matched candidate has no source span")
                })?;
            let text = renderer
                .render(&pattern_match)
                .map_err(|error| RewriteError::internal(self.source, error))?;
            edits.push(RewriteEdit { span, text });
        }
        edits.sort_unstable_by_key(|edit| (edit.span.start, std::cmp::Reverse(edit.span.end)));

        // reject ambiguous edits instead of selecting an implicit overlap policy
        let mut patches = Vec::with_capacity(edits.len());
        let mut previous: Option<Span> = None;
        for edit in edits {
            let RewriteEdit { span, text } = edit;
            if let Some(previous) = previous
                && previous.intersects(span)
            {
                return Err(RewriteError::overlapping(self.source, previous, span));
            }
            patches.push(Patch::replace(span, text));
            previous = Some(span);
        }

        Ok(FilePatch::with_patches(self.source.id, patches))
    }
}

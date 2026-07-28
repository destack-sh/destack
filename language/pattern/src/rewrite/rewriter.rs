use destack_dir as dir;
use destack_source::{DiagnosticCollection, File, FilePatch, ModuleId, Patch, Span};

use super::renderer::Renderer;
use crate::{ContextError, Matcher, ModuleContext, ProgramContext, Rewrite, RewriteError};

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
    /// The checked module and program required by semantic predicates.
    context: Option<(&'candidate ModuleContext, &'candidate ProgramContext)>,
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
            context: None,
        }
    }

    /// Create a rewriter over one checked program module's parsed DIR.
    pub fn in_module(
        rewrite: &'rewrite Rewrite,
        module: ModuleId,
        program: &'candidate ProgramContext,
        source: &'candidate File,
    ) -> Result<Self, ContextError> {
        let context = program.module(module)?;
        let candidate = dir::View::new(context.tree());

        Ok(Self::with_module(
            rewrite, candidate, context, program, source,
        ))
    }

    /// Create a rewriter over an explicit DIR view and checked module.
    pub fn with_module(
        rewrite: &'rewrite Rewrite,
        candidate: dir::View<'candidate>,
        module: &'candidate ModuleContext,
        program: &'candidate ProgramContext,
        source: &'candidate File,
    ) -> Self {
        Self {
            rewrite,
            candidate,
            source,
            context: Some((module, program)),
        }
    }

    /// Rewrite non-overlapping matches in source order.
    pub fn rewrite(
        &self,
        candidates: impl IntoIterator<Item = dir::LocalNodeIdAny>,
    ) -> Result<FilePatch, DiagnosticCollection> {
        let matcher = match self.context {
            Some((module, program)) => {
                Matcher::with_module(self.rewrite.pattern(), self.candidate, module, program)
            }
            None => Matcher::new(self.rewrite.pattern(), self.candidate),
        };
        let matches = matcher
            .find(candidates)
            .map_err(|error| RewriteError::internal(self.source, error))?;
        let renderer = Renderer::new(self.rewrite.replacement(), self.candidate, self.source);
        let mut edits = Vec::with_capacity(matches.len());

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

use destack_dir as dir;
use destack_source::{DiagnosticCollection, File, FilePatch, ModuleId, Patch, Span};

use super::renderer::Renderer;
use crate::{ContextError, Matcher, ModuleContext, ProgramContext, Rewrite, RewriteError};

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

    /// Create a rewriter over one checked program module.
    pub fn in_module(
        rewrite: &'rewrite Rewrite,
        module: ModuleId,
        program: &'candidate ProgramContext,
        source: &'candidate File,
    ) -> Result<Self, ContextError> {
        let context = program.module(module)?;

        Ok(Self {
            rewrite,
            candidate: context.view(),
            source,
            context: Some((context, program)),
        })
    }

    /// Rewrite non-overlapping matches in source order.
    pub fn rewrite(
        &self,
        candidates: impl IntoIterator<Item = dir::LocalNodeIdAny>,
    ) -> Result<FilePatch, DiagnosticCollection> {
        let matcher = match self.context {
            Some((module, program)) => {
                Matcher::in_module(self.rewrite.pattern(), module.module(), program)
                    .map_err(|error| RewriteError::internal(self.source, error))?
            }
            None => Matcher::new(self.rewrite.pattern(), self.candidate),
        };
        let matches = matcher
            .find(candidates)
            .map_err(|error| RewriteError::internal(self.source, error))?;
        let renderer = Renderer::new(self.rewrite.replacement(), self.candidate, self.source);
        let mut replacements = Vec::with_capacity(matches.len());

        // render matches before selecting a non-overlapping edit set
        for pattern_match in matches {
            let span = self
                .candidate
                .get_span_by_id(pattern_match.root.id)
                .ok_or_else(|| {
                    RewriteError::internal(self.source, "matched candidate has no source span")
                })?;
            let text = renderer
                .render(&pattern_match)
                .map_err(|error| RewriteError::internal(self.source, error))?;
            replacements.push((span, text));
        }
        replacements.sort_unstable_by_key(|(span, _)| (span.start, std::cmp::Reverse(span.end)));

        // reject ambiguous edits instead of selecting an implicit overlap policy
        let mut patches = Vec::with_capacity(replacements.len());
        let mut previous: Option<Span> = None;
        for (span, text) in replacements {
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

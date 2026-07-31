use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, Origin};
use crate::{CheckError, CompilerError, CompilerResult, DiagnosticAnchor};

impl CheckState<'_> {
    /// Return the diagnostic anchor for one source node.
    pub(in crate::check) fn source_anchor(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> (ModuleId, DiagnosticAnchor) {
        let module = source.module_id;
        let anchor = self.diagnostic_anchor(module, source.local_id);

        (module, anchor)
    }

    /// Return the diagnostic anchor for one source node.
    pub(in crate::check) fn diagnostic_anchor(
        &self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) -> DiagnosticAnchor {
        let span = match self.module_maybe(module) {
            Some(state) => state.diagnostic_span(source),
            None => self.external_module(module).diagnostic_span(source),
        };
        let span = match span {
            Some(span) => span,
            None => unreachable!("check node {} has no source span", source.id),
        };

        DiagnosticAnchor::from(span)
    }

    /// Return one source node's full anchor.
    pub(in crate::check) fn node_anchor(
        &self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<DiagnosticAnchor> {
        let span = match self.module_maybe(module) {
            Some(state) => state.source_span(source),
            None => self.external_module(module).diagnostic_span(source),
        };
        let span = span.ok_or_else(|| CompilerError::Internal {
            message: format!("check node {} has no source span", source.id),
        })?;

        Ok(DiagnosticAnchor::from(span))
    }

    /// Return one check origin's diagnostic anchor.
    pub(in crate::check) fn origin_diagnostic_anchor(
        &self,
        origin: Origin,
    ) -> CompilerResult<(ModuleId, DiagnosticAnchor)> {
        let source = match origin {
            Origin::Node(node, _) => node,
            Origin::Symbol(symbol) => self.symbol_source(symbol)?,
        };

        Ok(self.source_anchor(source))
    }

    /// Return one check origin's whole authored node anchor.
    pub(in crate::check) fn origin_node_anchor(
        &self,
        origin: Origin,
    ) -> CompilerResult<(ModuleId, DiagnosticAnchor)> {
        let module = origin.module();
        let source = match origin {
            Origin::Node(node, _) => node.local_id,
            Origin::Symbol(symbol) => self
                .module(symbol.module_id)
                .symbol_declaration_node(symbol.local_id)?,
        };
        let anchor = self.node_anchor(module, source)?;

        Ok((module, anchor))
    }

    /// Return one symbol's declaration node.
    pub(in crate::check) fn symbol_source(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalNodeIdAny> {
        let source = match self.module_maybe(symbol.module_id) {
            Some(module) => module
                .symbol_declaration_node(symbol.local_id)?
                .into_global(symbol.module_id),
            None => {
                let external = self.external_module(symbol.module_id);
                let binding = external.bindings.get_symbol(symbol.local_id);
                let Some(declaration) = binding.declaration else {
                    return Err(CompilerError::Internal {
                        message: format!("external symbol {symbol:?} has no declaration node"),
                    });
                };

                declaration
            }
        };

        Ok(source)
    }

    /// Return a circular type diagnostic for one origin.
    pub(in crate::check) fn circular_type_error(
        &self,
        origin: Origin,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;

        Ok(CheckError::CircularType { anchor, module })
    }
}

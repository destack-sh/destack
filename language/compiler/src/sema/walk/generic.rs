use tspp_dir as dir;

use crate::sema::{GenericTemplateId, Receiver, WalkState};
use crate::{CompilerError, CompilerResult};

/// One declaration that receives induced parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct InducedParameterOwner {
    /// The declaration node that receives induced parameters.
    pub(in crate::sema) declaration: dir::GlobalNodeIdAny,
    /// The enclosing generic template.
    pub(in crate::sema) parent: Option<GenericTemplateId>,
    /// The declaration symbol indexed by the generated template.
    pub(in crate::sema) symbol: Option<dir::GlobalSymbolId>,
}

impl InducedParameterOwner {
    /// Create one induced parameter owner.
    pub(in crate::sema) fn new(
        declaration: dir::GlobalNodeIdAny,
        parent: Option<GenericTemplateId>,
        symbol: Option<dir::GlobalSymbolId>,
    ) -> Self {
        Self {
            declaration,
            parent,
            symbol,
        }
    }
}

impl WalkState<'_, '_> {
    /// Walk with a template's declaration owning the parameters it induces.
    pub(in crate::sema) fn with_template_owner<R>(
        &mut self,
        template: GenericTemplateId,
        walk: impl FnOnce(&mut Self) -> CompilerResult<R>,
    ) -> CompilerResult<R> {
        let previous = self.induced_owner;
        if previous.is_none()
            && let Some(declared) = self.check.generic_template(template)?
        {
            self.induced_owner = Some(InducedParameterOwner::new(
                declared.source,
                None,
                declared.symbol,
            ));
        }
        let result = walk(self);
        self.induced_owner = previous;

        result
    }

    /// Return the generic template enclosing one member declaration.
    pub(in crate::sema) fn enclosing_generic_template(
        &self,
        receiver: Option<Receiver>,
        declaration: Option<InducedParameterOwner>,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        // prefer the declaration of the member
        if let Some(symbol) = declaration.and_then(|declaration| declaration.symbol)
            && let Some(template) = self.check.template_by_symbol(symbol)?
        {
            return Ok(Some(template));
        }

        // keep generated declaration templates nested under their parent
        if let Some(parent) = declaration.and_then(|declaration| declaration.parent) {
            return Ok(Some(parent));
        }

        // use declaration receiver scopes
        match receiver.and_then(|receiver| receiver.declaration) {
            Some(symbol) => self.check.template_by_symbol(symbol),
            None => Ok(None),
        }
    }

    /// Open one generic template header with its parameter identities.
    ///
    /// Example:
    /// ```tspp
    /// class Box<T> {}
    /// ```
    pub(in crate::sema) fn open_generic_template(
        &mut self,
        source: dir::GlobalNodeIdAny,
        parameters: &[dir::LocalNodeId<dir::GenericParameter>],
    ) -> CompilerResult<Option<GenericTemplateId>> {
        if parameters.is_empty() {
            return Ok(None);
        }
        if source.module_id != self.module {
            return Err(CompilerError::Internal {
                message: format!("generic template source {source:?} is outside the walked module"),
            });
        }
        let template = self
            .check
            .open_generic_template(source, self.flow().template_scope())?;

        // open parameter identities before walking any bounds
        for parameter in parameters {
            self.open_generic_parameter(template, *parameter, self.tree.get(*parameter))?;
        }

        Ok(Some(template))
    }

    /// Open one generic template and walk its parameter bounds.
    pub(in crate::sema) fn walk_generic_template(
        &mut self,
        source: dir::GlobalNodeIdAny,
        parameters: &[dir::LocalNodeId<dir::GenericParameter>],
    ) -> CompilerResult<Option<GenericTemplateId>> {
        let Some(template) = self.open_generic_template(source, parameters)? else {
            return Ok(None);
        };

        // walk parameter bounds after all identities exist
        for parameter in parameters {
            self.walk_generic_parameter(template, *parameter, self.tree.get(*parameter))?;
        }

        Ok(Some(template))
    }

    /// Return one declaration's template, opened when its signature induced parameters.
    pub(in crate::sema) fn induced_owner_template(
        &mut self,
        owner: InducedParameterOwner,
        template: Option<GenericTemplateId>,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        Ok(template.or_else(|| self.check.template_by_source(owner.declaration)))
    }
}

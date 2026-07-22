use destack_core::FxIndexMap;
use destack_dir as dir;

use crate::check::{CheckState, GenericTemplateId, InducedParameterSite, Receiver, WalkState};
use crate::{CompilerError, CompilerResult};

/// One declaration that receives induced parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct InducedParameterOwner {
    /// The declaration node that receives induced parameters.
    pub(in crate::check) declaration: dir::GlobalNodeIdAny,
    /// The enclosing generic template.
    pub(in crate::check) parent: Option<GenericTemplateId>,
    /// The declaration symbol indexed by the generated template.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
}

impl InducedParameterOwner {
    /// Create one induced parameter owner.
    pub(in crate::check) fn new(
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
    /// Return the generic template enclosing one member declaration.
    pub(in crate::check) fn enclosing_generic_template(
        &self,
        receiver: Option<Receiver>,
        declaration: Option<InducedParameterOwner>,
    ) -> Option<GenericTemplateId> {
        // prefer the declaration that owns the member
        if let Some(symbol) = declaration.and_then(|declaration| declaration.symbol)
            && let Some(template) = self.check.generics.template_by_symbol(symbol)
        {
            return Some(template);
        }

        // keep generated declaration templates nested under their parent
        if let Some(parent) = declaration.and_then(|declaration| declaration.parent) {
            return Some(parent);
        }

        // use declaration receiver scopes
        receiver
            .and_then(|receiver| receiver.declaration)
            .and_then(|symbol| self.check.generics.template_by_symbol(symbol))
    }

    /// Open one generic template header with its parameter identities.
    ///
    /// Example:
    /// ```ds
    /// class Box<T> {}
    /// ```
    pub(in crate::check) fn open_generic_template(
        &mut self,
        source: dir::GlobalNodeIdAny,
        parent: Option<GenericTemplateId>,
        symbol: Option<dir::GlobalSymbolId>,
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
        let template = self.check.open_generic_template(source, parent, symbol)?;

        // open parameter identities before walking any bounds
        for parameter in parameters {
            self.open_generic_parameter(template, *parameter, self.tree.get(*parameter))?;
        }

        Ok(Some(template))
    }

    /// Open one generic template and walk its parameter bounds.
    pub(in crate::check) fn walk_generic_template(
        &mut self,
        source: dir::GlobalNodeIdAny,
        parent: Option<GenericTemplateId>,
        symbol: Option<dir::GlobalSymbolId>,
        parameters: &[dir::LocalNodeId<dir::GenericParameter>],
    ) -> CompilerResult<Option<GenericTemplateId>> {
        let Some(template) = self.open_generic_template(source, parent, symbol, parameters)? else {
            return Ok(None);
        };

        // walk parameter bounds after all identities exist
        for parameter in parameters {
            self.walk_generic_parameter(template, *parameter, self.tree.get(*parameter))?;
        }

        Ok(Some(template))
    }

    /// Return one declaration's template, opened when its walked types induced memory holes.
    pub(in crate::check) fn induced_owner_template(
        &mut self,
        owner: InducedParameterOwner,
        template: Option<GenericTemplateId>,
    ) -> CompilerResult<Option<GenericTemplateId>> {
        if template.is_some() {
            return Ok(template);
        }

        // open the template only when walked declaration types left induced holes
        let types = self.check.generics.induced_site_types(owner.declaration);
        let mut induced = false;
        for ty in types {
            if !self.check.induced_memory_variables(ty)?.is_empty() {
                induced = true;
                break;
            }
        }
        if !induced {
            return Ok(None);
        }

        self.check
            .open_generic_template(owner.declaration, owner.parent, owner.symbol)
            .map(Some)
    }

    /// Push one declaration type that can contain induced memory holes.
    pub(in crate::check) fn push_induced_parameter_site(
        &mut self,
        declaration: InducedParameterOwner,
        ty: dir::GlobalTypeId,
    ) {
        self.check
            .generics
            .push_induced_parameter_site(InducedParameterSite {
                declaration: declaration.declaration,
                parent: declaration.parent,
                symbol: declaration.symbol,
                ty,
            });
    }
}

impl CheckState<'_> {
    /// Propagate induced memory variables into declaration templates.
    pub(in crate::check) fn propagate_induced_parameters(&mut self) -> CompilerResult<()> {
        let sites = self.generics.drain_induced_parameter_sites();

        // collect induced parameters before mutating generic tables
        let mut parameters = FxIndexMap::default();
        for site in sites {
            for (variable, role) in self.induced_memory_variables(site.ty)? {
                parameters.entry(variable).or_insert((
                    site.declaration,
                    site.parent,
                    site.symbol,
                    role,
                ));
            }
        }

        // insert parameters in allocation order
        let mut parameters = parameters.into_iter().collect::<Vec<_>>();
        parameters.sort_by_key(|(variable, _)| variable.0);

        for (variable, (declaration, parent, symbol, role)) in parameters {
            // cyclic applications already froze this declaration's arity:
            //  report once and poison the unresolvable hole
            if let Some(reference) = self.cyclic_inductions.get(&declaration).copied() {
                self.report_circular_lifetime_induction(declaration, reference, symbol)?;
                let error = self.intern_type(declaration.module_id, dir::Type::Error)?;
                self.commit_solution(variable, error)?;

                continue;
            }

            let template = self.open_generic_template(declaration, parent, symbol)?;
            let parameter = self.push_induced_memory_parameter(template, role)?;
            let solution =
                self.intern_type(declaration.module_id, dir::Type::Parameter(parameter))?;
            self.commit_solution(variable, solution)?;
        }

        Ok(())
    }
}

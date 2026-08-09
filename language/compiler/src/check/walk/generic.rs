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
            && let Some(template) = self.check.template_by_symbol(symbol)
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
            .and_then(|symbol| self.check.template_by_symbol(symbol))
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
        let template = self.check.open_generic_template(source)?;

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
        let types = self.check.induced_site_types(owner.declaration);
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
            .open_generic_template(owner.declaration)
            .map(Some)
    }

    /// Push one declaration type that can contain induced memory holes.
    pub(in crate::check) fn push_induced_parameter_site(
        &mut self,
        declaration: InducedParameterOwner,
        ty: dir::GlobalTypeId,
    ) {
        self.check
            .induced_parameter_sites
            .push(InducedParameterSite {
                declaration: declaration.declaration,
                ty,
            });
    }
}

impl CheckState<'_> {
    /// Propagate induced memory variables into declaration templates.
    pub(in crate::check) fn induce_signature_lifetimes(&mut self) -> CompilerResult<()> {
        let sites = std::mem::take(&mut self.induced_parameter_sites);

        // collect induced parameters before mutating generic tables
        let mut parameters = FxIndexMap::default();
        for site in sites {
            for (variable, role) in self.induced_memory_variables(site.ty)? {
                parameters
                    .entry(variable)
                    .or_insert((site.declaration, role));
            }
        }

        // insert parameters in allocation order
        let mut parameters = parameters.into_iter().collect::<Vec<_>>();
        parameters.sort_by_key(|(variable, _)| variable.0);

        for (variable, (declaration, role)) in parameters {
            // derive the site from the hole's origin: the elided position
            let origin = self.infer.origin(self.infer.variable(variable)?.origin);
            let site = self
                .origin_source_node(origin)?
                .into_global(origin.module());

            // report elision on type declarations, induce parameters for value signatures
            let template = self.open_generic_template(declaration)?;
            let declaration_symbol = self
                .module(declaration.module_id)
                .declaration_symbol(declaration.local_id);
            let is_type_declaration = match declaration_symbol {
                Some(symbol) => self.symbol_kind(symbol)?.is_type_definition(),
                None => false,
            };
            if is_type_declaration {
                self.report_elided_lifetime_in_named_declaration(declaration, site)?;
                let error = self.intern_type(dir::Type::Error)?;
                self.commit_solution(variable, error)?;

                continue;
            }
            let parameter = self.push_induced_memory_parameter(template, site, role)?;
            let solution = self.intern_type(dir::Type::Parameter(parameter))?;
            self.commit_solution(variable, solution)?;
        }

        Ok(())
    }
}

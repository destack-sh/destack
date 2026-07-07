use destack_dir as dir;
use indexmap::IndexMap;

use crate::check::{CheckState, GenericTemplateId, InducedLifetimeSite, Receiver, WalkState};
use crate::{CompilerError, CompilerResult};

/// One declaration that receives induced lifetime parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct InducedLifetimeOwner {
    /// The declaration node that receives induced lifetime parameters.
    pub(in crate::check) declaration: dir::GlobalNodeIdAny,
    /// The enclosing generic template.
    pub(in crate::check) parent: Option<GenericTemplateId>,
    /// The declaration symbol indexed by the generated template.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
}

impl InducedLifetimeOwner {
    /// Create one induced lifetime owner.
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
        declaration: Option<InducedLifetimeOwner>,
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

    /// Push one declaration type that can contain induced lifetime holes.
    pub(in crate::check) fn push_induced_lifetime_site(
        &mut self,
        declaration: InducedLifetimeOwner,
        ty: dir::GlobalTypeId,
    ) {
        self.check
            .generics
            .push_induced_lifetime_site(InducedLifetimeSite {
                declaration: declaration.declaration,
                parent: declaration.parent,
                symbol: declaration.symbol,
                ty,
            });
    }
}

impl CheckState<'_> {
    /// Propagate elided lifetime variables into declaration templates.
    ///
    /// Induced lifetimes still reachable from declaration types become generic parameters.
    pub(in crate::check) fn propagate_induced_lifetimes(&mut self) -> CompilerResult<()> {
        let sites = self
            .generics
            .induced_lifetime_sites()
            .cloned()
            .collect::<Vec<_>>();

        // collect induced lifetimes before mutating generic tables
        let mut lifetimes = IndexMap::new();
        for site in sites {
            for variable in self.type_variables(site.ty)? {
                let role = self.variable_role(variable)?;
                if role.is_inference() {
                    continue;
                }

                lifetimes.entry(variable).or_insert((
                    site.declaration,
                    site.parent,
                    site.symbol,
                    role,
                ));
            }
        }

        // insert lifetime parameters in allocation order
        let mut lifetimes = lifetimes.into_iter().collect::<Vec<_>>();
        lifetimes.sort_by_key(|(variable, _)| variable.0);

        for (variable, (declaration, parent, symbol, role)) in lifetimes {
            let template = self.open_generic_template(declaration, parent, symbol)?;
            let parameter = self.push_induced_lifetime_parameter(template, role)?;
            let solution =
                self.intern_type(declaration.module_id, dir::Type::Parameter(parameter))?;
            self.commit_solution(variable, solution)?;
        }

        Ok(())
    }
}

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

    /// Return the represented value type for one open annotation when needed.
    ///
    /// Interface-typed and anonymous structural annotations have no direct
    /// storage representation, so value positions erase them to `Dynamic<T>`.
    ///
    /// Example:
    /// ```ds
    /// writer: Writer
    /// ```
    pub(in crate::check) fn represented_open_type(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if !self.needs_dynamic_representation(ty)? {
            return Ok(ty);
        }

        self.check.intern_type(
            self.module,
            dir::Type::Dynamic(dir::DynamicType { constraint: ty }),
        )
    }

    /// Return whether one written type needs `Dynamic<T>` as its value representation.
    /// TODO #Suspicious: not _entirely_ sure whether needs_dynamic_representation can be decided at leaf?
    ///
    /// An annotation erases when the type it names is an interface,
    /// which has no value representation of its own.
    fn needs_dynamic_representation(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        match self.check.ty(ty)? {
            // top types have no direct layout in storage
            dir::Type::Any | dir::Type::Object | dir::Type::Unknown => Ok(true),

            // direct interface instances are constraints, not represented values
            dir::Type::Instance(instance) => Ok(matches!(
                self.check.symbol_kind(instance.symbol),
                dir::SymbolKind::Interface | dir::SymbolKind::NewtypeInterface
            )),

            // every other source-built type already has a representation
            _ => Ok(false),
        }
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

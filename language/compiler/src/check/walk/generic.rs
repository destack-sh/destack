use destack_dir as dir;
use indexmap::IndexMap;

use crate::check::{
    CheckState, GenericInductionParameter, GenericInductionSite, GenericTemplateId, Origin,
    Receiver, Relation, WalkState, Widening,
};
use crate::{CompilerError, CompilerResult};

/// One declaration that receives induced generic parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct GenericInductionDeclaration {
    /// The declaration node that receives induced generic parameters.
    pub(in crate::check) declaration: dir::GlobalNodeIdAny,
    /// The enclosing generic template.
    pub(in crate::check) parent: Option<GenericTemplateId>,
    /// The declaration symbol indexed by the generated template.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
}

impl GenericInductionDeclaration {
    /// Create one generic induction declaration.
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

/// The source position that induces a hidden generic parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum GenericInductionPosition {
    /// A parameter annotation position.
    Parameter,
    /// A storage annotation position.
    Storage,
}

impl GenericInductionPosition {
    /// Return the committed induction reason.
    pub(in crate::check) fn induction(self) -> dir::GenericParameterInduction {
        match self {
            // parameter type
            Self::Parameter => dir::GenericParameterInduction::ParameterConstraint,
            // storage type
            Self::Storage => dir::GenericParameterInduction::StorageConstraint,
        }
    }
}

impl WalkState<'_, '_> {
    /// Return the generic template enclosing one member declaration.
    pub(in crate::check) fn enclosing_generic_template(
        &self,
        receiver: Option<Receiver>,
        declaration: Option<GenericInductionDeclaration>,
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

    /// Push one declaration type that can induce generics.
    pub(in crate::check) fn push_type_induction_site(
        &mut self,
        declaration: GenericInductionDeclaration,
        ty: dir::GlobalTypeId,
    ) {
        self.check
            .generics
            .push_induction_site(GenericInductionSite {
                declaration: declaration.declaration,
                parent: declaration.parent,
                symbol: declaration.symbol,
                ty,
            });
    }

    /// Return an induced variable for one constraint type when needed.
    ///
    /// Interface-typed annotations open an inducible hole bounded by the
    /// interface: holes that escape unsolved become generated generic
    /// parameters after the walk.
    ///
    /// Example:
    /// ```ds
    /// writer: Writer
    /// ```
    pub(in crate::check) fn induce_constraint_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        ty: dir::GlobalTypeId,
        position: GenericInductionPosition,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = source.into_global(self.module);
        let origin = Origin::Node(node, self.flow().template_scope());
        if !self.induces_generic_parameter(origin, ty)? {
            return Ok(ty);
        }

        // open the inducible hole bounded by its interface
        let variable = self
            .check
            .allocate_variable(self.module, origin, Widening::Preserve);
        let induced = self.check.variable_type(variable)?;
        let induction = GenericInductionParameter {
            name_prefix: "T",
            constraint: Some(ty),
            is_comptime: false,
            induction: position.induction(),
        };
        self.check.generics.insert_induction(variable, induction)?;
        self.relate_type(origin, Relation::Assignable, induced, ty);

        Ok(induced)
    }

    /// Return whether one written type induces a generic parameter.
    ///
    /// An annotation induces when the type it names is an interface,
    /// which has no value representation of its own; the reduction
    /// resolves transparent aliases to the named head.
    fn induces_generic_parameter(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let head =
            self.check
                .require_reduced_type_head(origin, ty, "generic parameter induction")?;
        let symbol = match self.check.ty(head)? {
            dir::Type::Instance(instance) => instance.symbol,
            _ => return Ok(false),
        };

        // interfaces are always incomplete until implemented;
        // concrete declarations already have a representation
        Ok(matches!(
            self.check.symbol_kind(symbol),
            dir::SymbolKind::AssociatedType
                | dir::SymbolKind::Interface
                | dir::SymbolKind::NewtypeInterface
        ))
    }
}

impl CheckState<'_> {
    /// Complete generic walk state before solving.
    ///
    /// Inducible holes still reachable from recorded declaration types
    /// become generated generic parameters on their declarations.
    pub(in crate::check) fn propagate_induced_generics(&mut self) -> CompilerResult<()> {
        let sites = self.generics.induction_sites().cloned().collect::<Vec<_>>();

        // collect all generated parameters before mutating generic tables;
        // each hole generalizes once, on its first recorded declaration
        let mut induced = IndexMap::new();
        for site in sites {
            for variable in self.type_variables(site.ty)? {
                let Some(parameter) = self.variable_induction(variable)? else {
                    continue;
                };

                induced.entry(variable).or_insert((
                    site.declaration,
                    site.parent,
                    site.symbol,
                    parameter,
                ));
            }
        }

        // insert generated parameters in variable allocation order,
        // so hidden lifetimes number by their source positions
        let mut induced = induced.into_iter().collect::<Vec<_>>();
        induced.sort_by_key(|(variable, _)| variable.0);

        for (variable, (declaration, parent, symbol, induction)) in induced {
            let template = self.open_generic_template(declaration, parent, symbol)?;
            let parameter = self.push_induced_generic_parameter(template, induction)?;
            let solution =
                self.intern_type(declaration.module_id, dir::Type::Parameter(parameter))?;
            self.commit_solution(variable, solution)?;
        }

        Ok(())
    }
}

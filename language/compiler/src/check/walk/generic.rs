use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::check::{
    CheckState, Condition, Constraint, ConstraintCause, GenericInductionParameter,
    GenericInductionSite, GenericTemplateId, Origin, Receiver, Relation, WalkState, Widening,
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

        // use receiver-owned member scopes
        receiver
            .and_then(|receiver| receiver.owner)
            .and_then(|symbol| self.check.generics.template_by_symbol(symbol))
    }

    /// Declare one generic template header with its parameter identities.
    ///
    /// Example:
    /// ```ds
    /// class Box<T> {}
    /// ```
    pub(in crate::check) fn declare_generic_template(
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
        let template = self
            .check
            .declare_generic_template(source, parent, symbol)?;

        // declare parameter identities before walking any bounds
        for parameter in parameters {
            self.declare_generic_parameter(template, *parameter, self.tree.get(*parameter))?;
        }

        Ok(Some(template))
    }

    /// Declare one generic template and walk its parameter bounds.
    pub(in crate::check) fn walk_generic_template(
        &mut self,
        source: dir::GlobalNodeIdAny,
        parent: Option<GenericTemplateId>,
        symbol: Option<dir::GlobalSymbolId>,
        parameters: &[dir::LocalNodeId<dir::GenericParameter>],
    ) -> CompilerResult<Option<GenericTemplateId>> {
        let Some(template) = self.declare_generic_template(source, parent, symbol, parameters)?
        else {
            return Ok(None);
        };

        // walk parameter bounds after all identities exist
        for parameter in parameters {
            self.walk_generic_parameter(template, *parameter, self.tree.get(*parameter))?;
        }

        Ok(Some(template))
    }

    /// Record one declaration type that can induce generics.
    pub(in crate::check) fn record_type_induction_site(
        &mut self,
        declaration: GenericInductionDeclaration,
        ty: dir::GlobalTypeId,
    ) {
        self.check
            .generics
            .record_induction_site(GenericInductionSite {
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
        if !self.is_constraint_type(ty)? {
            return Ok(ty);
        }

        // open the inducible hole bounded by its interface
        let node = source.into_global(self.module);
        let origin = Origin::Node(node);
        let variable = self
            .check
            .allocate_variable(self.module, origin, Widening::Preserve);
        let induced = self.check.push_variable_type(variable, source)?;
        let recipe = GenericInductionParameter {
            prefix: "T",
            constraint: Some(ty),
            is_comptime: false,
            induction: position.induction(),
        };
        self.check.generics.insert_induction(variable, recipe)?;
        self.relate_type(origin, Relation::Assignable, induced, ty);

        Ok(induced)
    }

    /// Return whether one type writes an inducible constraint.
    fn is_constraint_type(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        let symbol = match self.check.ty(ty)? {
            dir::Type::Reference(instance) if instance.arguments.is_empty() => instance.symbol,
            _ => return Ok(false),
        };
        let kind = self.check.symbol_kind(symbol);

        Ok(matches!(
            kind,
            dir::SymbolKind::AssociatedType
                | dir::SymbolKind::Interface
                | dir::SymbolKind::NewtypeInterface,
        ))
    }
}

impl CheckState<'_> {
    /// Complete generic walk state before solving.
    ///
    /// Inducible holes still reachable from recorded declaration types
    /// become generated generic parameters on their declarations.
    pub(in crate::check) fn propagate_induced_generics(
        &mut self,
        modules: &[ModuleId],
    ) -> CompilerResult<()> {
        let sites = self.generics.induction_sites().cloned().collect::<Vec<_>>();

        // collect all generated parameters before mutating generic tables;
        // each hole generalizes once, on its first recorded declaration
        let mut induced = IndexMap::new();
        for site in sites {
            for variable in self.type_variables(site.ty)? {
                let representative = self.variables.representative(variable)?;
                let Some(parameter) = self.generics.induction(representative) else {
                    continue;
                };

                induced.entry(representative).or_insert((
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
        induced.sort_by_key(|(variable, _)| (variable.module_id, variable.index));

        for (variable, (declaration, parent, symbol, recipe)) in induced {
            let template = self.declare_generic_template(declaration, parent, symbol)?;
            let parameter = self.declare_induced_generic_parameter(template, recipe)?;
            let source = self.origin_source_node(Origin::Node(declaration))?;
            let solution = self.push_type(
                declaration.module_id,
                dir::Type::Parameter(parameter),
                source,
            )?;
            self.set_solution(variable, solution)?;
        }

        // tie type declarations to their own applied references
        for module in modules {
            self.declare_module_reference_types(*module)?;
        }

        Ok(())
    }

    /// Tie each type declaration symbol to its own applied reference.
    fn declare_module_reference_types(&mut self, module: ModuleId) -> CompilerResult<()> {
        let binding_table = self.module(module).binding_table();
        let symbols = binding_table
            .symbol_ids()
            .map(|symbol: dir::LocalSymbolId| symbol.into_global(module))
            .filter(|symbol| {
                let kind = self.symbol_kind(*symbol);

                kind.is_type_definition() && !kind.is_type_alias()
            })
            .collect::<Vec<_>>();

        for symbol in symbols {
            self.declare_symbol_reference_type(symbol)?;
        }

        Ok(())
    }

    /// Tie one declaration symbol to its own applied reference type.
    fn declare_symbol_reference_type(&mut self, symbol: dir::GlobalSymbolId) -> CompilerResult<()> {
        let origin = Origin::Symbol(symbol);
        let source = self.origin_source_node(origin)?;

        // apply the declaration's own parameters as arguments
        let parameters = self
            .generics
            .template_by_symbol(symbol)
            .map(|template| self.generic_template_parameters(template))
            .unwrap_or_default();
        let mut arguments = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            arguments.push(self.push_type(
                symbol.module_id,
                dir::Type::Parameter(parameter),
                source,
            )?);
        }
        let reference = self.push_type(
            symbol.module_id,
            dir::Type::Reference(dir::GenericInstance { symbol, arguments }),
            source,
        )?;

        // bind or equate the declaration's recorded type
        if let Some(existing) = self.inputs.symbol_type(symbol) {
            self.push_constraint(Constraint {
                relation: Relation::Equal,
                left: existing,
                right: reference,
                origin,
                condition: Condition::Always,
                cause: ConstraintCause::General,
            });
        } else {
            self.inputs.set_symbol_type(symbol, reference)?;
        }

        Ok(())
    }
}

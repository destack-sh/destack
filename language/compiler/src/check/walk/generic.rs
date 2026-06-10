use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};
use smallvec::SmallVec;

use crate::check::{
    CheckState, Condition, Dependency, GenericArgument, GenericInductionParameter,
    GenericInductionSite, GenericParameterBinding, GenericTemplateId, Origin, Receiver,
    StaticOperand, StaticSolution, StaticTerm, TypeOperand, TypeRelation, TypeSolution, TypeTerm,
    VariableId, VariableKind, WalkState,
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
            && let Some(template) = self.check.inference.generic_template_by_symbol(symbol)
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
            .and_then(|symbol| self.check.inference.generic_template_by_symbol(symbol))
    }
}

/// One escaping variable that receives an induced generic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GenericInductionKey {
    /// The declaration node that receives the generated generic parameter.
    declaration: dir::GlobalNodeIdAny,
    /// The enclosing generic template.
    parent: Option<dir::GlobalGenericTemplateId>,
    /// The declaration symbol indexed by the generated template.
    symbol: Option<dir::GlobalSymbolId>,
    /// The variable rewritten to the generated parameter.
    variable: VariableId,
}

impl CheckState<'_> {
    /// Complete generic walk state before solving.
    pub(in crate::check) fn finish_walk_generics(
        &mut self,
        modules: &[ModuleId],
    ) -> CompilerResult<()> {
        let sites = self
            .inference
            .generic_induction_sites()
            .cloned()
            .collect::<Vec<_>>();
        let mut parameters = IndexMap::new();

        // collect all generated parameters before mutating generic tables
        for site in sites {
            self.collect_induced_parameters(site, &mut parameters)?;
        }

        // insert generated parameters and rewrite their variables
        for (key, parameter) in parameters {
            self.insert_induced_generic(key, parameter)?;
        }

        // constrain reference declarations after generated parameters exist
        for module in modules {
            self.constrain_declaration_types(*module)?;
        }

        Ok(())
    }

    /// Collect generic parameters induced by one declaration site.
    fn collect_induced_parameters(
        &self,
        site: GenericInductionSite,
        parameters: &mut IndexMap<GenericInductionKey, GenericInductionParameter>,
    ) -> CompilerResult<()> {
        let mut visited = IndexSet::new();
        self.collect_induced_type_operand(&site, site.operand, &mut visited, parameters)
    }

    /// Collect induced parameters reachable from one type operand.
    fn collect_induced_type_operand(
        &self,
        site: &GenericInductionSite,
        operand: TypeOperand,
        visited: &mut IndexSet<VariableId>,
        parameters: &mut IndexMap<GenericInductionKey, GenericInductionParameter>,
    ) -> CompilerResult<()> {
        match operand {
            TypeOperand::Variable(variable) => {
                self.collect_induced_variable(site, variable, visited, parameters)?;
            }
            TypeOperand::Term(term) => {
                let dependencies = TypeOperand::Term(term).dependencies(self);

                for variable in Dependency::variables(dependencies) {
                    self.collect_induced_variable(site, variable, visited, parameters)?;
                }
            }
            TypeOperand::Type(_) => {}
        }

        Ok(())
    }

    /// Collect one induced variable and variables beneath its solved operand.
    fn collect_induced_variable(
        &self,
        site: &GenericInductionSite,
        variable: VariableId,
        visited: &mut IndexSet<VariableId>,
        parameters: &mut IndexMap<GenericInductionKey, GenericInductionParameter>,
    ) -> CompilerResult<()> {
        if !visited.insert(variable) {
            return Ok(());
        }

        // record leaves that walk explicitly marked as inducible
        if let Some(parameter) = self.inference.generic_induction_parameter(variable) {
            let key = GenericInductionKey {
                declaration: site.declaration,
                parent: site.parent,
                symbol: site.symbol,
                variable,
            };

            parameters.entry(key).or_insert(parameter);
        }

        match self.variable(variable).kind {
            // follow explicit type solutions
            VariableKind::Type => {
                let Some(operand) = self.solved_type_operand(variable) else {
                    return Ok(());
                };
                self.collect_induced_type_operand(site, operand, visited, parameters)?;
            }

            // follow explicit static solutions
            VariableKind::Static => {
                let Some(operand) = self.solved_static_operand(variable) else {
                    return Ok(());
                };
                self.collect_induced_static_operand(site, operand, visited, parameters)?;
            }
        }

        Ok(())
    }

    /// Collect induced parameters reachable from one static operand.
    fn collect_induced_static_operand(
        &self,
        site: &GenericInductionSite,
        operand: StaticOperand,
        visited: &mut IndexSet<VariableId>,
        parameters: &mut IndexMap<GenericInductionKey, GenericInductionParameter>,
    ) -> CompilerResult<()> {
        let dependencies = operand.dependencies(self);

        // recurse into every variable referenced by the static operand
        for variable in Dependency::variables(dependencies) {
            self.collect_induced_variable(site, variable, visited, parameters)?;
        }

        Ok(())
    }

    /// Insert one induced generic parameter.
    fn insert_induced_generic(
        &mut self,
        key: GenericInductionKey,
        generic: GenericInductionParameter,
    ) -> CompilerResult<()> {
        if key.declaration.module_id != key.variable.module {
            return Err(CompilerError::Internal {
                message: "induced generic template and variable are in different modules"
                    .to_string(),
            });
        }

        let template = self.declare_generic_template(key.declaration, key.parent, key.symbol)?;
        let header =
            self.fresh_induced_generic_parameter(template, generic.prefix, generic.induction);
        let parameter = header.id;
        let kind = generic.kind;

        match kind {
            VariableKind::Type => {
                let generic = GenericParameterBinding::r#type(header, None, generic.ty, None);
                let parameter = self.inference.push_term(TypeTerm::Parameter(parameter));
                self.inference.insert_generic_parameter(generic)?;
                self.set_variable_solution(key.variable, TypeSolution::Term(parameter).into())?;
            }
            VariableKind::Static => {
                let generic = GenericParameterBinding::r#static(header, generic.ty, None);
                let parameter = self.inference.push_term(StaticTerm::Parameter(parameter));
                self.inference.insert_generic_parameter(generic)?;
                self.set_variable_solution(key.variable, StaticSolution::Term(parameter).into())?;
            }
        }

        Ok(())
    }

    /// Constrain concrete and constraint declaration symbol types.
    fn constrain_declaration_types(&mut self, module: ModuleId) -> CompilerResult<()> {
        let binding_table = self.module(module).binding_table();
        let symbols = binding_table
            .symbol_ids()
            .map(|symbol: dir::LocalSymbolId| symbol.into_global(module))
            .filter(|symbol| {
                let kind = self.symbol_kind(*symbol);

                kind.is_type_definition() && !kind.is_type_alias()
            })
            .collect::<Vec<_>>();

        // constrain each declaration as its own applied type
        for symbol in symbols {
            self.constrain_declaration_type(symbol)?;
        }

        Ok(())
    }

    /// Constrain one declaration symbol as its own applied type.
    fn constrain_declaration_type(&mut self, symbol: dir::GlobalSymbolId) -> CompilerResult<()> {
        let term = self.declaration_type_term(symbol);
        let term = self.type_term_operand(term);
        let origin = Origin::Symbol(symbol);
        let target = match self.inputs.symbol_type(symbol) {
            Some(operand) => operand,
            None => {
                let variable = self.push_type_variable(symbol.module_id, origin);
                let operand = TypeOperand::Variable(variable);
                self.inputs.insert_symbol_type(symbol, operand)?;

                operand
            }
        };
        self.constrain_type(origin, TypeRelation::Equal, target, term, Condition::Always);

        Ok(())
    }

    /// Return one declaration symbol's applied reference type.
    pub(in crate::check) fn declaration_type_term(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> TypeTerm {
        let arguments = self
            .inference
            .generic_template_by_symbol(symbol)
            .map(|template| self.generic_template_arguments(template))
            .unwrap_or_default();

        TypeTerm::Reference {
            origin: Origin::Symbol(symbol),
            symbol,
            arguments,
        }
    }

    /// Return generic arguments that pass one template's parameters to itself.
    fn generic_template_arguments(
        &mut self,
        template: dir::GlobalGenericTemplateId,
    ) -> SmallVec<[GenericArgument; 2]> {
        // freeze parameter order before allocating parameter terms
        let parameters = self
            .inference
            .generic_template_parameters(template)
            .map(|(parameter, generic)| (parameter, generic.is_static(), generic.is_variadic()))
            .collect::<Vec<_>>();

        parameters
            .into_iter()
            .map(
                |(parameter, is_static, is_variadic)| match (is_static, is_variadic) {
                    // type parameter
                    (false, false) => {
                        let term = self.inference.push_term(TypeTerm::Parameter(parameter));

                        GenericArgument::Type(term.into())
                    }
                    // variadic type parameter
                    (false, true) => {
                        let term = self.inference.push_term(TypeTerm::Parameter(parameter));

                        GenericArgument::SpreadType(term.into())
                    }
                    // static parameter
                    (true, false) => {
                        let term = self.inference.push_term(StaticTerm::Parameter(parameter));

                        GenericArgument::Static(term.into())
                    }
                    // variadic static parameter
                    (true, true) => {
                        let term = self.inference.push_term(StaticTerm::Parameter(parameter));

                        GenericArgument::SpreadStatic(term.into())
                    }
                },
            )
            .collect()
    }
}

impl WalkState<'_, '_> {
    /// Declare one generic template header.
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

    /// Record one operand that can induce generics on a declaration.
    pub(in crate::check) fn record_generic_induction_site(
        &mut self,
        declaration: GenericInductionDeclaration,
        operand: TypeOperand,
    ) {
        let site = GenericInductionSite::new(
            declaration.declaration,
            declaration.parent,
            declaration.symbol,
            operand,
        );
        self.check.inference.record_generic_induction_site(site);
    }

    /// Record one type annotation that can induce generics on a declaration.
    pub(in crate::check) fn record_type_induction_site(
        &mut self,
        declaration: GenericInductionDeclaration,
        ty: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<()> {
        let operand = self.node_type_operand(ty)?;
        self.record_generic_induction_site(declaration, operand);

        Ok(())
    }

    /// Declare one generic template from explicit generic parameters.
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
}

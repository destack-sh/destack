use destack_dir as dir;
use indexmap::{IndexMap, IndexSet};

use crate::check::{
    CheckState, Condition, FunctionTerm, GenericInductionRoot, GenericInductionSlot, GenericSlot,
    Origin, StaticSolution, StaticTerm, TermId, TypeOperand, TypeSolution, TypeTerm, VariableId,
    VariableKind,
};
use crate::{CompilerError, CompilerResult};

/// One escaping variable that receives an owner generic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GenericInductionKey {
    /// The declaration that receives the generated generic slot.
    owner: dir::GlobalSymbolId,
    /// The variable rewritten to the generated parameter.
    variable: VariableId,
}

impl CheckState<'_> {
    /// Induce generic slots from explicit walk roots.
    pub(in crate::check) fn induce_generics(&mut self) -> CompilerResult<()> {
        let roots = self
            .inference
            .generic_induction_roots()
            .cloned()
            .collect::<Vec<_>>();
        let functions = self.induction_owner_functions(&roots);
        let generics = self.collect_induced_generics(&roots)?;

        self.insert_induced_generics(generics, &functions)
    }

    /// Collect induced generics from explicit walk roots.
    fn collect_induced_generics(
        &self,
        roots: &[GenericInductionRoot],
    ) -> CompilerResult<IndexMap<GenericInductionKey, GenericInductionSlot>> {
        let mut generics = IndexMap::new();

        for root in roots {
            let mut visited = IndexSet::new();

            self.collect_induced_operand(root.owner, root.operand, &mut visited, &mut generics)?;
        }

        Ok(generics)
    }

    /// Collect induced generics from one type operand.
    fn collect_induced_operand(
        &self,
        owner: dir::GlobalSymbolId,
        operand: TypeOperand,
        visited: &mut IndexSet<VariableId>,
        generics: &mut IndexMap<GenericInductionKey, GenericInductionSlot>,
    ) -> CompilerResult<()> {
        match operand {
            TypeOperand::Variable(variable) => {
                self.collect_induced_variable(owner, variable, visited, generics)
            }
            TypeOperand::Term(term) => {
                self.collect_induced_term(owner, self.term(term), visited, generics)
            }
            TypeOperand::Type(_) => Ok(()),
        }
    }

    /// Collect induced generics from one type term.
    fn collect_induced_term(
        &self,
        owner: dir::GlobalSymbolId,
        term: &TypeTerm,
        visited: &mut IndexSet<VariableId>,
        generics: &mut IndexMap<GenericInductionKey, GenericInductionSlot>,
    ) -> CompilerResult<()> {
        for variable in term.referenced_variables(self) {
            self.collect_induced_variable(owner, variable, visited, generics)?;
        }

        Ok(())
    }

    /// Collect induced generics from one variable solution.
    fn collect_induced_variable(
        &self,
        owner: dir::GlobalSymbolId,
        variable: VariableId,
        visited: &mut IndexSet<VariableId>,
        generics: &mut IndexMap<GenericInductionKey, GenericInductionSlot>,
    ) -> CompilerResult<()> {
        if !visited.insert(variable) {
            return Ok(());
        }
        if let Some(slot) = self.generic_induction_slot(variable) {
            let key = GenericInductionKey { owner, variable };

            generics.entry(key).or_insert(slot);

            return Ok(());
        }
        let Some(term) = self.type_solution(variable)? else {
            return Ok(());
        };
        self.collect_induced_term(owner, &term, visited, generics)
    }

    /// Insert collected induced generic slots into the type graph.
    fn insert_induced_generics(
        &mut self,
        generics: IndexMap<GenericInductionKey, GenericInductionSlot>,
        functions: &IndexMap<dir::GlobalSymbolId, TermId<FunctionTerm>>,
    ) -> CompilerResult<()> {
        for (leaf, generic) in generics {
            self.insert_induced_generic(leaf, generic, functions)?;
        }

        Ok(())
    }

    /// Insert one induced generic slot.
    fn insert_induced_generic(
        &mut self,
        key: GenericInductionKey,
        generic: GenericInductionSlot,
        functions: &IndexMap<dir::GlobalSymbolId, TermId<FunctionTerm>>,
    ) -> CompilerResult<()> {
        self.assert_induced_generic_is_local(key)?;

        let header =
            self.allocate_generic_induction_slot(key.owner, generic.prefix, generic.induction);
        let slot = header.id();
        let kind = generic.kind;
        let variable = self.allocate_variable(key.variable.module, kind, Origin::Symbol(key.owner));

        match kind {
            VariableKind::Type => {
                let generic = GenericSlot::Type {
                    slot: header,
                    variance: None,
                    constraint: generic.constraint,
                    default: None,
                };
                let parameter = self.push_term(TypeTerm::Parameter(slot));

                self.attach_generic_slot(variable, generic);
                self.equate_type(variable, TypeTerm::Parameter(slot), Condition::Always);
                self.insert_known_solution(key.variable, TypeSolution::Term(parameter).into());
            }
            VariableKind::Static => {
                let generic = GenericSlot::Static {
                    slot: header,
                    constraint: generic.constraint,
                    default: None,
                };
                let parameter = self.push_term(StaticTerm::Parameter(slot));

                self.attach_generic_slot(variable, generic);
                let origin = self.variable(variable).source;

                self.equate_static(origin, variable, parameter, Condition::Always);
                self.insert_known_solution(key.variable, StaticSolution::Term(parameter).into());
            }
        }

        self.insert_induced_generic_into_owner_function(key.owner, variable, functions);

        Ok(())
    }

    /// Assert that one induced generic belongs to its owner module.
    fn assert_induced_generic_is_local(&self, key: GenericInductionKey) -> CompilerResult<()> {
        if key.owner.module_id == key.variable.module {
            return Ok(());
        }

        Err(CompilerError::Internal {
            message: "induced generic owner and leaf are in different modules".to_string(),
        })
    }

    /// Return owner function terms that can receive induced generics.
    fn induction_owner_functions(
        &self,
        roots: &[GenericInductionRoot],
    ) -> IndexMap<dir::GlobalSymbolId, TermId<FunctionTerm>> {
        roots
            .iter()
            .filter_map(|root| match root.operand {
                TypeOperand::Term(term) => match self.term(term) {
                    TypeTerm::Function(function) => Some((root.owner, *function)),
                    _ => None,
                },
                TypeOperand::Variable(_) | TypeOperand::Type(_) => None,
            })
            .collect()
    }

    /// Insert one induced generic parameter into an owner function type.
    fn insert_induced_generic_into_owner_function(
        &mut self,
        owner: dir::GlobalSymbolId,
        variable: VariableId,
        functions: &IndexMap<dir::GlobalSymbolId, TermId<FunctionTerm>>,
    ) {
        let Some(function) = functions.get(&owner).copied() else {
            return;
        };
        let function = self.term_mut(function);

        if !function.generic_parameters.contains(&variable) {
            function.generic_parameters.push(variable);
        }
    }
}

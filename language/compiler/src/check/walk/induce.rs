use destack_dir as dir;
use indexmap::{IndexMap, IndexSet};

use crate::check::{
    CheckState, GenericInductionRoot, GenericInductionSlot, GenericSlot, StaticSolution,
    StaticTerm, TypeOperand, TypeSolution, TypeTerm, VariableId, VariableKind,
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
        let generics = self.collect_induced_generics(&roots)?;

        self.insert_induced_generics(generics)
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
                self.collect_induced_term(owner, self.inference.term(term), visited, generics)
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
        if let Some(slot) = self.inference.generic_induction_slot(variable) {
            let key = GenericInductionKey { owner, variable };

            generics.entry(key).or_insert(slot);

            return Ok(());
        }
        let Some(term) = self.type_solution(variable)? else {
            return Ok(());
        };
        self.collect_induced_term(owner, &term, visited, generics)
    }

    /// Insert collected induced generic slots into the generic table.
    fn insert_induced_generics(
        &mut self,
        generics: IndexMap<GenericInductionKey, GenericInductionSlot>,
    ) -> CompilerResult<()> {
        for (leaf, generic) in generics {
            self.insert_induced_generic(leaf, generic)?;
        }

        Ok(())
    }

    /// Insert one induced generic slot.
    fn insert_induced_generic(
        &mut self,
        key: GenericInductionKey,
        generic: GenericInductionSlot,
    ) -> CompilerResult<()> {
        self.check_induced_generic_module(key)?;

        let header =
            self.allocate_generic_induction_slot(key.owner, generic.prefix, generic.induction);
        let slot = header.id();
        let kind = generic.kind;

        match kind {
            VariableKind::Type => {
                let generic = GenericSlot::Type {
                    slot: header,
                    variance: None,
                    constraint: generic.constraint,
                    default: None,
                };
                let parameter = self.inference.push_term(TypeTerm::Parameter(slot));

                self.insert_generic_slot(generic);
                self.set_variable_solution(key.variable, TypeSolution::Term(parameter).into())?;
            }
            VariableKind::Static => {
                let generic = GenericSlot::Static {
                    slot: header,
                    constraint: generic.constraint,
                    default: None,
                };
                let parameter = self.inference.push_term(StaticTerm::Parameter(slot));

                self.insert_generic_slot(generic);
                self.set_variable_solution(key.variable, StaticSolution::Term(parameter).into())?;
            }
        }

        Ok(())
    }

    /// Check that one induced generic belongs to its owner module.
    fn check_induced_generic_module(&self, key: GenericInductionKey) -> CompilerResult<()> {
        if key.owner.module_id == key.variable.module {
            return Ok(());
        }

        Err(CompilerError::Internal {
            message: "induced generic owner and leaf are in different modules".to_string(),
        })
    }
}

use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckComponentState, ConstraintOrigin, ExportLookup, Solution, StaticTerm, TypeTerm,
    VariableId, VariableKind,
};

use super::Progress;

impl CheckComponentState<'_> {
    /// Solve one type variable and return the queue progress.
    pub(in crate::check) fn solve_type_variable(
        &mut self,
        variable: VariableId,
        term: TypeTerm,
    ) -> CompilerResult<Progress> {
        let value = Solution::Type(term);
        let changed = self
            .module_mut(variable.module)?
            .solve_variable(variable, value);

        Ok(Progress::from_change(variable, changed))
    }

    /// Solve one static variable and return the queue progress.
    pub(in crate::check) fn solve_static_variable(
        &mut self,
        variable: VariableId,
        term: StaticTerm,
    ) -> CompilerResult<Progress> {
        let value = Solution::Static(term);
        let changed = self
            .module_mut(variable.module)?
            .solve_variable(variable, value);

        Ok(Progress::from_change(variable, changed))
    }

    /// Define one reduced type term as a fresh variable.
    pub(in crate::check) fn define_reduced_type(
        &mut self,
        module: ModuleId,
        origin: ConstraintOrigin,
        term: TypeTerm,
    ) -> CompilerResult<VariableId> {
        let module = self.module_mut(module)?;
        let variable = module.allocate_anonymous_variable(VariableKind::Type, origin);
        let value = Solution::Type(term);

        module.solve_variable(variable, value);

        Ok(variable)
    }

    /// Define one reduced DIR static value as a fresh variable.
    pub(in crate::check) fn define_reduced_static_value(
        &mut self,
        module: ModuleId,
        origin: ConstraintOrigin,
        term: dir::StaticTerm,
    ) -> CompilerResult<VariableId> {
        self.define_reduced_static(module, origin, StaticTerm::Literal(term))
    }

    /// Define one reduced static term as a fresh variable.
    pub(in crate::check) fn define_reduced_static(
        &mut self,
        module: ModuleId,
        origin: ConstraintOrigin,
        term: StaticTerm,
    ) -> CompilerResult<VariableId> {
        let module = self.module_mut(module)?;
        let variable = module.allocate_anonymous_variable(VariableKind::Static, origin);
        let value = Solution::Static(term);

        module.solve_variable(variable, value);

        Ok(variable)
    }

    /// Return one variable's diagnostic origin.
    pub(in crate::check) fn variable_origin(
        &self,
        variable: VariableId,
    ) -> CompilerResult<ConstraintOrigin> {
        let origin = self.module(variable.module)?.variable(variable).source;

        Ok(origin)
    }

    /// Solve variables whose lower bounds determine a concrete solution.
    pub(in crate::check) fn solve_bound_variables(&mut self) -> CompilerResult<Progress> {
        let variables = self
            .modules
            .values()
            .flat_map(|check_module| {
                check_module
                    .work
                    .variables
                    .all
                    .iter()
                    .map(|variable| variable.id)
            })
            .collect::<Vec<_>>();
        let mut progress = Progress::Unchanged;

        // solve variables in stable module and allocation order
        for variable in variables {
            progress = progress.merge(self.solve_bound_variable(variable)?);
        }

        Ok(progress)
    }

    /// Solve one variable from its collected bounds.
    fn solve_bound_variable(&mut self, variable: VariableId) -> CompilerResult<Progress> {
        if self.variable_solution(variable)?.is_some() {
            return Ok(Progress::Unchanged);
        }
        let kind = self.module(variable.module)?.variable(variable).kind;

        match kind {
            VariableKind::Type => self.solve_bound_type_variable(variable),
            VariableKind::Static => self.solve_bound_static_variable(variable),
        }
    }

    /// Solve one type variable from lower bounds.
    fn solve_bound_type_variable(&mut self, variable: VariableId) -> CompilerResult<Progress> {
        let lower_bounds = self.module(variable.module)?.lower_bounds(variable);
        if lower_bounds.is_empty() {
            return Ok(Progress::Unchanged);
        }
        let mut candidates = Vec::with_capacity(lower_bounds.len());

        // collect solved lower bound candidates
        for lower_bound in lower_bounds {
            let Some(term) = self.solved_type_term(lower_bound)? else {
                return Ok(Progress::Unchanged);
            };

            candidates.push(term);
        }
        let origin = self.variable_origin(variable)?;
        let term = self.reduce_best_common_terms(variable.module, origin, candidates)?;

        self.solve_type_variable(variable, term)
    }

    /// Solve one static variable from lower bounds.
    fn solve_bound_static_variable(&mut self, variable: VariableId) -> CompilerResult<Progress> {
        let lower_bounds = self.module(variable.module)?.lower_bounds(variable);
        if lower_bounds.is_empty() {
            return Ok(Progress::Unchanged);
        }
        let mut terms = Vec::with_capacity(lower_bounds.len());

        // collect solved lower bound terms
        for lower_bound in lower_bounds {
            let Some(term) = self.solved_static_term(lower_bound)? else {
                return Ok(Progress::Unchanged);
            };

            terms.push(term);
        }
        let Some(first) = terms.first().cloned() else {
            return Ok(Progress::Unchanged);
        };

        // only equality compatible static bounds determine a value
        for term in terms.iter().skip(1) {
            if term != &first {
                return Ok(Progress::Unchanged);
            }
        }

        self.solve_static_variable(variable, first)
    }

    /// Return a solved variable value.
    pub(in crate::check) fn variable_solution(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<Solution>> {
        let value = self
            .module(variable.module)?
            .variable_solution(variable)
            .cloned();

        Ok(value)
    }

    /// Return a solved type term.
    pub(in crate::check) fn solved_type_term(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<TypeTerm>> {
        let value = self.variable_solution(variable)?;
        let value = match value {
            Some(Solution::Type(TypeTerm::Variable(source))) if source != variable => {
                self.solved_type_term(source)?
            }
            Some(Solution::Type(term)) => Some(term),
            _ => None,
        };

        Ok(value)
    }

    /// Return a solved static term.
    pub(in crate::check) fn solved_static_term(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<StaticTerm>> {
        let value = self.variable_solution(variable)?;
        let value = match value {
            Some(Solution::Static(StaticTerm::Variable(source))) if source != variable => {
                self.solved_static_term(source)?
            }
            Some(Solution::Static(term)) => Some(term),
            _ => None,
        };

        Ok(value)
    }

    /// Return one locally concrete static expression term.
    pub(in crate::check) fn static_expression_term(
        &mut self,
        expression: dir::GlobalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        self.static_expression_term_from_node(expression.module_id, expression.local_id)
    }

    /// Return one locally concrete static expression term by local expression id.
    fn static_expression_term_from_node(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let expression_node = self.module(module)?.input.view().get(expression).clone();
        let term = match expression_node {
            dir::Expression::Identifier { name } => {
                self.static_reference_expression_term(module, expression, name)?
            }
            dir::Expression::QualifiedReference { path, .. } => {
                self.static_path_expression_term(module, expression, &path)?
            }
            dir::Expression::ScalarLiteral(value) => Some(dir::StaticTerm::ScalarLiteral { value }),
            dir::Expression::ObjectExpression { properties } => {
                self.static_object_expression_term(module, &properties)?
            }
            _ => None,
        };

        Ok(term)
    }

    /// Return one locally concrete static reference expression term.
    fn static_reference_expression_term(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        name: dir::StringId,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(symbol) = self
            .module(module)?
            .lookup_name(expression.into_any(), name, dir::SymbolSpace::Value)
            .unique_symbol()
        else {
            return Ok(None);
        };

        self.static_symbol_expression_term(module, symbol)
    }

    /// Return one locally concrete static path expression term.
    fn static_path_expression_term(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let symbol = match self.resolve_static_path_symbol(
            module,
            expression.into_any(),
            path,
            dir::SymbolSpace::Value,
        )? {
            ExportLookup::Found(symbol) => symbol,
            ExportLookup::Missing | ExportLookup::Ambiguous(_) => return Ok(None),
        };

        self.static_symbol_expression_term(module, symbol)
    }

    /// Return one locally concrete static symbol expression term.
    fn static_symbol_expression_term(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(variable) = self.generic_static_variable(module, symbol)? else {
            return Ok(None);
        };
        let term = match self.solved_static_term(variable)? {
            Some(StaticTerm::Literal(term)) => Some(term),
            _ => None,
        };

        Ok(term)
    }

    /// Return one locally concrete static object expression term.
    fn static_object_expression_term(
        &mut self,
        module: ModuleId,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let view = self.module(module)?.input.view();
        let mut fields = Vec::with_capacity(properties.len());

        // collect field keys and value expressions
        for property in properties {
            let dir::Property::Field { key, value, .. } = view.get(*property) else {
                return Ok(None);
            };
            let Some(key) = key.static_key(view.tree()) else {
                return Ok(None);
            };

            fields.push((key, *value));
        }

        let mut terms = Vec::with_capacity(properties.len());

        // solve statically known field values
        for (key, value) in fields {
            let Some(value) = self.static_expression_term_from_node(module, value)? else {
                return Ok(None);
            };

            terms.push(dir::StaticProperty::Field { key, value });
        }

        Ok(Some(dir::StaticTerm::Object { properties: terms }))
    }
}

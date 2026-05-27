use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::CompilerResult;
use crate::check::{
    CheckState, ConstraintOrigin, ExportLookup, LayoutQuery, LayoutTerm, Solution, StaticOperand,
    StaticTerm, TypeOperand, TypeRelation, TypeTerm, VariableId, VariableKind,
};

use super::Progress;

impl CheckState<'_> {
    /// Solve one type variable and return the queue progress.
    pub(in crate::check) fn solve_type_variable(
        &mut self,
        variable: VariableId,
        term: TypeTerm,
    ) -> CompilerResult<Progress> {
        let term = self.terms.push(term);
        let value = Solution::Type(term);
        let changed = self.solve_variable(variable, value);

        Ok(Progress::from_change(variable, changed))
    }

    /// Solve one static variable and return the queue progress.
    pub(in crate::check) fn solve_static_variable(
        &mut self,
        variable: VariableId,
        term: StaticTerm,
    ) -> CompilerResult<Progress> {
        let term = self.terms.push(term);
        let value = Solution::Static(term);
        let changed = self.solve_variable(variable, value);

        Ok(Progress::from_change(variable, changed))
    }

    /// Return one variable's diagnostic origin.
    pub(in crate::check) fn variable_origin(
        &self,
        variable: VariableId,
    ) -> CompilerResult<ConstraintOrigin> {
        let origin = self.variable(variable).source;

        Ok(origin)
    }

    /// Solve variables whose lower bounds determine a concrete solution.
    pub(in crate::check) fn solve_bound_variables(&mut self) -> CompilerResult<Progress> {
        let variables = self
            .variables
            .variables
            .iter()
            .map(|variable| variable.id)
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
        let kind = self.variable(variable).kind;

        match kind {
            VariableKind::Type => self.solve_bound_type_variable(variable),
            VariableKind::Static => self.solve_bound_static_variable(variable),
        }
    }

    /// Solve one type variable from lower bounds.
    fn solve_bound_type_variable(&mut self, variable: VariableId) -> CompilerResult<Progress> {
        let lower_bounds = self.lower_bounds(variable);
        if lower_bounds.is_empty() {
            return Ok(Progress::Unchanged);
        }
        let mut candidates = Vec::with_capacity(lower_bounds.len());

        // collect solved lower bound candidates
        for lower_bound in lower_bounds {
            let term = match lower_bound {
                TypeOperand::Variable(variable) => {
                    let Some(term) = self.solved_type_term(variable)? else {
                        return Ok(Progress::Unchanged);
                    };

                    term
                }
                TypeOperand::Term(term) => self.terms.get(term).clone(),
            };

            candidates.push(term);
        }
        let term = self.reduce_best_common_terms(variable.module, candidates)?;

        self.solve_type_variable(variable, term)
    }

    /// Solve one static variable from lower bounds.
    fn solve_bound_static_variable(&mut self, variable: VariableId) -> CompilerResult<Progress> {
        let lower_bounds = self.lower_static_bounds(variable);
        if lower_bounds.is_empty() {
            return Ok(Progress::Unchanged);
        }
        let mut terms = Vec::with_capacity(lower_bounds.len());

        // collect solved lower bound terms
        for lower_bound in lower_bounds {
            let term = match lower_bound {
                StaticOperand::Variable(variable) => {
                    let Some(term) = self.solved_static_term(variable)? else {
                        return Ok(Progress::Unchanged);
                    };

                    term
                }
                StaticOperand::Term(term) => self.terms.get(term).clone(),
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
        let value = self.solutions.solution.get(&variable).cloned();

        Ok(value)
    }

    /// Return a solved type term.
    pub(in crate::check) fn solved_type_term(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<TypeTerm>> {
        let mut seen = IndexSet::new();

        self.solved_type_term_inner(variable, &mut seen)
    }

    /// Return a solved type term, stopping at variable cycles.
    fn solved_type_term_inner(
        &self,
        variable: VariableId,
        seen: &mut IndexSet<VariableId>,
    ) -> CompilerResult<Option<TypeTerm>> {
        if !seen.insert(variable) {
            return Ok(None);
        }
        let value = match self.variable_solution(variable)? {
            Some(Solution::Type(term)) => {
                let term = self.terms.get(term);
                if let TypeTerm::Variable(source) = term
                    && *source != variable
                {
                    self.solved_type_term_inner(*source, seen)?
                } else {
                    Some(term.clone())
                }
            }
            _ => None,
        };

        Ok(value)
    }

    /// Return a solved static term.
    pub(in crate::check) fn solved_static_term(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<StaticTerm>> {
        let mut seen = IndexSet::new();

        self.solved_static_term_inner(variable, &mut seen)
    }

    /// Return a solved static term, stopping at variable cycles.
    fn solved_static_term_inner(
        &self,
        variable: VariableId,
        seen: &mut IndexSet<VariableId>,
    ) -> CompilerResult<Option<StaticTerm>> {
        if !seen.insert(variable) {
            return Ok(None);
        }
        let value = match self.variable_solution(variable)? {
            Some(Solution::Static(term)) => {
                let term = self.terms.get(term);
                if let StaticTerm::Variable(source) = term
                    && *source != variable
                {
                    self.solved_static_term_inner(*source, seen)?
                } else {
                    Some(term.clone())
                }
            }
            _ => None,
        };

        Ok(value)
    }

    /// Return one locally concrete static expression value.
    pub(in crate::check) fn build_static_expression_value(
        &mut self,
        expression: dir::GlobalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        self.build_static_expression_value_from_node(expression.module_id, expression.local_id)
    }

    /// Return one static expression as a solver static term.
    pub(in crate::check) fn build_static_expression_term(
        &mut self,
        expression: dir::GlobalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<StaticTerm>> {
        let module = expression.module_id;
        let expression_id = expression.local_id;
        let expression_node = self.input(module).view().get(expression_id).clone();
        let source = expression.into_any();
        let term = match expression_node {
            dir::Expression::Parenthesized { expression } => {
                self.build_static_expression_term(expression.into_global(module))?
            }
            dir::Expression::ScalarLiteral(value) => {
                Some(StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                    value,
                }))
            }
            dir::Expression::Member {
                left,
                name: Some(name),
            }
            | dir::Expression::PrivateMember {
                left,
                name: Some(name),
            } => {
                let owner = self.intern_node_type_variable(module, left.into_global_any(module));

                Some(StaticTerm::Member {
                    source: Some(source),
                    owner,
                    key: dir::StaticKey::Name(name),
                    arguments: Vec::new().into(),
                })
            }
            dir::Expression::Type { value } => {
                self.build_static_type_expression_term(module, value)?
            }
            dir::Expression::Binary {
                left,
                operator,
                right,
            } if matches!(
                operator,
                dir::BinaryOperator::Equal
                    | dir::BinaryOperator::EqualStrict
                    | dir::BinaryOperator::NotEqual
                    | dir::BinaryOperator::NotEqualStrict
            ) =>
            {
                let Some(left) = self.build_static_expression_term(left.into_global(module))?
                else {
                    return Ok(None);
                };
                let Some(right) = self.build_static_expression_term(right.into_global(module))?
                else {
                    return Ok(None);
                };
                let is_negated = matches!(
                    operator,
                    dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict
                );

                Some(StaticTerm::Equal {
                    left: self.terms.push(left),
                    right: self.terms.push(right),
                    is_negated,
                })
            }
            dir::Expression::If {
                condition:
                    dir::IfCondition::Expression {
                        condition: condition_expression,
                    },
                then_expression,
                else_expression: Some(else_expression),
                ..
            } => {
                let Some(condition) =
                    self.build_static_expression_term(condition_expression.into_global(module))?
                else {
                    return Ok(None);
                };
                let Some(then_value) =
                    self.build_static_expression_term(then_expression.into_global(module))?
                else {
                    return Ok(None);
                };
                let Some(else_value) =
                    self.build_static_expression_term(else_expression.into_global(module))?
                else {
                    return Ok(None);
                };

                Some(StaticTerm::Conditional {
                    condition: self.terms.push(condition),
                    then_value: self.terms.push(then_value),
                    else_value: self.terms.push(else_value),
                })
            }
            dir::Expression::Call {
                left,
                generic_arguments,
                arguments,
                ..
            } => self.build_static_layout_call_term(
                module,
                source,
                left,
                &generic_arguments,
                &arguments,
            )?,
            _ => self
                .build_static_expression_value_from_node(module, expression_id)?
                .map(StaticTerm::Literal),
        };

        Ok(term)
    }

    /// Return one type-space expression as a static term.
    fn build_static_type_expression_term(
        &mut self,
        module: ModuleId,
        value: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Option<StaticTerm>> {
        let type_expression = self.input(module).view().get(value).clone();
        let term = match type_expression {
            dir::TypeExpression::Parenthesized { expression } => {
                return self.build_static_type_expression_term(module, expression);
            }
            dir::TypeExpression::ScalarLiteral { value } => {
                StaticTerm::Literal(dir::StaticTerm::ScalarLiteral { value })
            }
            dir::TypeExpression::Literal { value } => {
                StaticTerm::Literal(dir::StaticTerm::TypeLiteral { value })
            }
            dir::TypeExpression::Extends { left, right } => StaticTerm::TypeRelation {
                relation: TypeRelation::Extends,
                left: self.intern_node_type_variable(module, left.into_global_any(module)),
                right: self.intern_node_type_variable(module, right.into_global_any(module)),
            },
            dir::TypeExpression::Implements { left, right } => StaticTerm::TypeRelation {
                relation: TypeRelation::Implements,
                left: self.intern_node_type_variable(module, left.into_global_any(module)),
                right: self.intern_node_type_variable(module, right.into_global_any(module)),
            },
            dir::TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            } => {
                let condition = StaticTerm::TypeRelation {
                    relation: TypeRelation::Extends,
                    left: self.intern_node_type_variable(module, left.into_global_any(module)),
                    right: self
                        .intern_node_type_variable(module, extends_type.into_global_any(module)),
                };
                let Some(then_value) = self.build_static_type_expression_term(module, then_type)?
                else {
                    return Ok(None);
                };
                let Some(else_value) = self.build_static_type_expression_term(module, else_type)?
                else {
                    return Ok(None);
                };

                StaticTerm::Conditional {
                    condition: self.terms.push(condition),
                    then_value: self.terms.push(then_value),
                    else_value: self.terms.push(else_value),
                }
            }
            _ => return Ok(None),
        };

        Ok(Some(term))
    }

    /// Return one locally concrete static expression value by local expression id.
    fn build_static_expression_value_from_node(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let expression_node = self.input(module).view().get(expression).clone();
        let term = match expression_node {
            dir::Expression::Identifier { name } => {
                self.build_static_reference_value(module, expression, name)?
            }
            dir::Expression::QualifiedReference { path, .. } => {
                self.build_static_path_value(module, expression, &path)?
            }
            dir::Expression::ScalarLiteral(value) => Some(dir::StaticTerm::ScalarLiteral { value }),
            dir::Expression::ObjectExpression { properties } => {
                self.build_static_object_value(module, &properties)?
            }
            _ => None,
        };

        Ok(term)
    }

    /// Return one locally concrete static reference expression value.
    fn build_static_reference_value(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        name: dir::StringId,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(symbol) = self
            .lookup_symbol_by_name(module, expression.into_any(), name, dir::SymbolSpace::Value)
            .unique_symbol()
        else {
            return Ok(None);
        };

        self.build_static_symbol_value(module, symbol)
    }

    /// Return one locally concrete static path expression value.
    fn build_static_path_value(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let symbol = match self.resolve_path_symbol(
            module,
            expression.into_any(),
            path,
            dir::SymbolSpace::Value,
        )? {
            ExportLookup::Found(symbol) => symbol,
            ExportLookup::Missing | ExportLookup::Ambiguous(_) => return Ok(None),
        };

        self.build_static_symbol_value(module, symbol)
    }

    /// Return one locally concrete static symbol expression value.
    fn build_static_symbol_value(
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

    /// Return one locally concrete static object expression value.
    fn build_static_object_value(
        &mut self,
        module: ModuleId,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let view = self.input(module).view();
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
            let Some(value) = self.build_static_expression_value_from_node(module, value)? else {
                return Ok(None);
            };

            terms.push(dir::StaticProperty::Field { key, value });
        }

        Ok(Some(dir::StaticTerm::Object { properties: terms }))
    }

    /// Return a layout query term for one static reflection call.
    fn build_static_layout_call_term(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        left: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Option<StaticTerm>> {
        if !arguments.is_empty() {
            return Ok(None);
        }
        let Some(symbol) = self.static_call_symbol(module, left)? else {
            return Ok(None);
        };
        let query = match self.environment.language.item(symbol) {
            Some(dir::LanguageItem::SizeOf) => LayoutQuery::Size,
            Some(dir::LanguageItem::AlignOf) => LayoutQuery::Alignment,
            Some(dir::LanguageItem::StrideOf) => LayoutQuery::Stride,
            _ => return Ok(None),
        };
        let Some(target) = self.single_type_generic_argument(module, generic_arguments)? else {
            return Ok(None);
        };
        let layout = LayoutTerm {
            source,
            target,
            query,
        };

        Ok(Some(StaticTerm::Layout(layout)))
    }

    /// Return the symbol named by one static call callee.
    fn static_call_symbol(
        &mut self,
        module: ModuleId,
        left: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let left_node = self.input(module).view().get(left).clone();
        let symbol = match left_node {
            dir::Expression::Identifier { name } => self
                .lookup_symbol_by_name(module, left.into_any(), name, dir::SymbolSpace::Value)
                .unique_symbol(),
            dir::Expression::QualifiedReference { path, .. } => {
                match self.resolve_path_symbol(
                    module,
                    left.into_any(),
                    &path,
                    dir::SymbolSpace::Value,
                )? {
                    ExportLookup::Found(symbol) => Some(symbol),
                    ExportLookup::Missing | ExportLookup::Ambiguous(_) => None,
                }
            }
            dir::Expression::Parenthesized { expression } => {
                return self.static_call_symbol(module, expression);
            }
            _ => None,
        };

        Ok(symbol)
    }

    /// Return the one type argument used by a static reflection call.
    fn single_type_generic_argument(
        &mut self,
        module: ModuleId,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<Option<VariableId>> {
        let [argument] = generic_arguments else {
            return Ok(None);
        };
        let argument_node = self.input(module).view().get(*argument).clone();
        let dir::GenericArgument::Type { value } = argument_node else {
            return Ok(None);
        };
        let target = self.intern_local_type_variable(module, value);

        Ok(Some(target))
    }
}

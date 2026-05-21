use std::collections::VecDeque;

use destack_dir as dir;

use crate::check::{CheckModuleState, CheckResult, Constraint, TypeInferId, TypeTerm};

impl CheckModuleState {
    /// Solve collected constraints into checked DIR tables.
    pub(in crate::check) fn solve(&mut self) -> CheckResult<()> {
        let constraints = self.constraints().to_vec();
        let watchers = self.constraint_watchers(&constraints);
        let mut queue = (0..constraints.len()).collect::<VecDeque<_>>();
        let mut is_queued = vec![true; constraints.len()];

        while let Some(index) = queue.pop_front() {
            is_queued[index] = false;

            if let Some(changed) = self.solve_constraint(&constraints[index]) {
                for dependent in &watchers[changed.0 as usize] {
                    if !is_queued[*dependent] {
                        queue.push_back(*dependent);
                        is_queued[*dependent] = true;
                    }
                }
            }
        }

        self.commit_solved_types();

        Ok(())
    }

    /// Build constraint watchers keyed by type inference variable.
    fn constraint_watchers(&self, constraints: &[Constraint]) -> Vec<Vec<usize>> {
        let mut watchers = vec![Vec::new(); self.type_slot_count()];

        for (index, constraint) in constraints.iter().enumerate() {
            for infer in constraint_type_infers(constraint) {
                watchers[infer.0 as usize].push(index);
            }
        }

        watchers
    }

    /// Solve one collected constraint.
    fn solve_constraint(&mut self, constraint: &Constraint) -> Option<TypeInferId> {
        match constraint {
            Constraint::Equals { left, right } => self.solve_equals(*left, *right),
            Constraint::Bind { result, term } => self.solve_bind(*result, term),
            Constraint::Join { result, values } => self.solve_join(*result, values),
            Constraint::LowerBound { variable, bound }
            | Constraint::UpperBound { variable, bound } => self.solve_equals(*variable, *bound),
            Constraint::BindStatic { .. }
            | Constraint::Instantiate { .. }
            | Constraint::SelectMember { .. }
            | Constraint::SelectUnaryOperator { .. } => None,
            Constraint::SelectCall {
                node,
                callee,
                arguments,
                result,
            } => self.solve_call(*node, *callee, arguments, *result),
            Constraint::SelectBinaryOperator {
                node,
                operator,
                left,
                right,
                result,
            } => self.solve_binary_operator(*node, *operator, *left, *right, *result),
        }
    }

    /// Propagate equality between two inference variables.
    fn solve_equals(&mut self, left: TypeInferId, right: TypeInferId) -> Option<TypeInferId> {
        match (self.infer_type_id(left), self.infer_type_id(right)) {
            (None, Some(type_id)) => self.set_infer_type(left, type_id).then_some(left),
            (Some(type_id), None) => self.set_infer_type(right, type_id).then_some(right),
            _ => None,
        }
    }

    /// Bind one inference variable from a direct term.
    fn solve_bind(&mut self, result: TypeInferId, term: &TypeTerm) -> Option<TypeInferId> {
        if self.infer_type_id(result).is_some() {
            return None;
        }

        let Some(type_id) = self.solve_type_term(result, term) else {
            return None;
        };

        self.set_infer_type(result, type_id).then_some(result)
    }

    /// Bind one inference variable from a union join.
    fn solve_join(&mut self, result: TypeInferId, values: &[TypeInferId]) -> Option<TypeInferId> {
        if self.infer_type_id(result).is_some() {
            return None;
        }

        let mut types = Vec::new();
        for value in values {
            let Some(type_id) = self.infer_type_id(*value) else {
                return None;
            };
            if !types.contains(&type_id) {
                types.push(type_id);
            }
        }

        let source = self.infer_source_node(result);
        let type_id = match types.as_slice() {
            [] => self
                .types_mut()
                .insert_type_from_any(dir::Type::Never, source),
            [type_id] => *type_id,
            _ => self
                .types_mut()
                .insert_type_from_any(dir::Type::Union(dir::UnionType { elements: types }), source),
        };

        self.set_infer_type(result, type_id).then_some(result)
    }

    /// Solve one binary operator expression.
    fn solve_binary_operator(
        &mut self,
        node: dir::GlobalNodeIdAny,
        operator: dir::BinaryOperator,
        left: TypeInferId,
        right: TypeInferId,
        result: TypeInferId,
    ) -> Option<TypeInferId> {
        if self.infer_type_id(result).is_some() {
            return None;
        }
        let left_infer = left;
        let right_infer = right;
        let left = self.infer_type_id(left)?;
        let right = self.infer_type_id(right)?;
        let resolution = self.resolve_binary_operator_builtin(operator, left, right)?;
        let return_type = resolution.return_type?;

        if let Some(parameter) = resolution.parameters.first() {
            self.replace_infer_type(left_infer, *parameter);
        }
        if let Some(parameter) = resolution.parameters.get(1) {
            self.replace_infer_type(right_infer, *parameter);
        }

        self.resolutions_mut().set_call_resolution(node, resolution);

        self.set_infer_type(result, return_type).then_some(result)
    }
}

/// Return type inference variables referenced by one constraint.
fn constraint_type_infers(constraint: &Constraint) -> Vec<TypeInferId> {
    match constraint {
        Constraint::Equals { left, right } => vec![*left, *right],
        Constraint::Bind { result, term } => {
            let mut infers = vec![*result];
            match term {
                TypeTerm::Infer(infer) => infers.push(*infer),
                TypeTerm::Function {
                    parameters,
                    return_type,
                    ..
                } => {
                    infers.extend(parameters.iter().copied());
                    infers.extend(return_type.iter().copied());
                }
                _ => {}
            }

            infers
        }
        Constraint::Join { result, values } => {
            let mut infers = Vec::with_capacity(values.len() + 1);
            infers.push(*result);
            infers.extend(values.iter().copied());

            infers
        }
        Constraint::LowerBound { variable, bound } | Constraint::UpperBound { variable, bound } => {
            vec![*variable, *bound]
        }
        Constraint::Instantiate { result, .. } => vec![*result],
        Constraint::SelectMember {
            receiver, result, ..
        } => vec![*receiver, *result],
        Constraint::SelectCall {
            callee,
            arguments,
            result,
            ..
        } => {
            let mut infers = Vec::with_capacity(arguments.len() + 2);
            infers.push(*callee);
            infers.extend(arguments.iter().copied());
            infers.push(*result);

            infers
        }
        Constraint::SelectUnaryOperator {
            operand, result, ..
        } => vec![*operand, *result],
        Constraint::SelectBinaryOperator {
            left,
            right,
            result,
            ..
        } => vec![*left, *right, *result],
        Constraint::BindStatic { .. } => Vec::new(),
    }
}

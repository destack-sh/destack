use destack_dir::{
    Constraint, InferTable, InferVar, InferVarId, LocalTypeId, SymbolTable, Type, TypeLiteral,
    TypeTable,
};
use destack_workspace::{Module, ProfileId};

use crate::{AnalyzeOptions, Assignability, Compiler};

/// Track bounds for a single inference variable.
#[derive(Debug, Clone)]
struct Bounds {
    /// Lower bounds collected for the variable.
    lower: Vec<LocalTypeId>,
    /// Upper bounds collected for the variable.
    upper: Vec<LocalTypeId>,
    /// Default type used when no bounds resolve.
    default: Option<LocalTypeId>,
}

impl Bounds {
    /// Create bounds from an inference variable.
    fn from_var(var: &InferVar) -> Self {
        Self {
            lower: var.lower_bounds.clone(),
            upper: var.upper_bounds.clone(),
            default: var.default,
        }
    }
}

/// Select the join operation used for bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JoinKind {
    /// Join bounds into a union.
    Union,
    /// Join bounds into an intersection.
    Intersection,
}

/// The resolved types for all inference variables.
#[derive(Debug, Default)]
pub struct InferSolution {
    /// Resolved type ids by inference variable index.
    pub resolved: Vec<Option<LocalTypeId>>,
}

impl InferSolution {
    /// Get the resolved type for an inference variable.
    pub fn get(&self, id: InferVarId) -> Option<LocalTypeId> {
        self.resolved.get(id.0 as usize).copied().flatten()
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Solve inference variables and commit the results into the TypeTable.
    pub fn solve_infer_table(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        infer: &InferTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> InferSolution {
        // initialize solution slots
        let mut solution = InferSolution {
            resolved: vec![None; infer.vars.len()],
        };

        // collect initial bounds
        let mut bounds: Vec<_> = infer.vars.iter().map(Bounds::from_var).collect();

        // apply constraints to bounds
        for constraint in &infer.constraints {
            Self::apply_constraint(constraint, types, &mut bounds);
        }

        // resolve bounds to a fixed point
        let mut did_resolve = true;
        while did_resolve {
            did_resolve = false;
            for (index, bound) in bounds.iter().enumerate() {
                if solution.resolved[index].is_some() {
                    continue;
                }
                if let Some(resolved) =
                    self.resolve_bounds(module, profile, symbols, bound, &solution, types, options)
                {
                    solution.resolved[index] = Some(resolved);
                    did_resolve = true;
                }
            }
        }

        // apply resolved types into the table
        self.apply_solution(&solution, types);

        solution
    }

    /// Apply a single constraint to the current bounds.
    fn apply_constraint(constraint: &Constraint, types: &TypeTable, bounds: &mut [Bounds]) {
        match constraint {
            Constraint::Equal { left, right } => {
                Self::bind_equal(*left, *right, types, bounds);
                Self::bind_equal(*right, *left, types, bounds);
            }
            Constraint::Subtype {
                sub_type: sub,
                super_type: sup,
                ..
            } => {
                Self::bind_upper(*sub, *sup, types, bounds);
                Self::bind_lower(*sup, *sub, types, bounds);
            }
            Constraint::Join { target, sources } => {
                if let Some(bound) = bounds.get_mut(target.0 as usize) {
                    bound.lower.extend(sources.iter().copied());
                }
            }
            Constraint::Instantiate { .. } => {}
            Constraint::Conditional { .. } => {}
            Constraint::CandidateGroup { options, .. } => {
                if let Some(first) = options.first() {
                    for option in first {
                        Self::apply_constraint(option, types, bounds);
                    }
                }
            }
        }
    }

    /// Bind equality constraints for a type if it is an inference variable.
    fn bind_equal(left: LocalTypeId, right: LocalTypeId, types: &TypeTable, bounds: &mut [Bounds]) {
        if let Some(infer_id) = Self::infer_var_id_for_type(left, types)
            && let Some(bound) = bounds.get_mut(infer_id.0 as usize)
        {
            bound.upper.push(right);
            bound.lower.push(right);
        }
    }

    /// Bind an upper bound for an inference variable type.
    fn bind_upper(sub: LocalTypeId, sup: LocalTypeId, types: &TypeTable, bounds: &mut [Bounds]) {
        if let Some(infer_id) = Self::infer_var_id_for_type(sub, types)
            && let Some(bound) = bounds.get_mut(infer_id.0 as usize)
        {
            bound.upper.push(sup);
        }
    }

    /// Bind a lower bound for an inference variable type.
    fn bind_lower(sup: LocalTypeId, sub: LocalTypeId, types: &TypeTable, bounds: &mut [Bounds]) {
        if let Some(infer_id) = Self::infer_var_id_for_type(sup, types)
            && let Some(bound) = bounds.get_mut(infer_id.0 as usize)
        {
            bound.lower.push(sub);
        }
    }

    /// Resolve bounds for a single inference variable.
    fn resolve_bounds(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        bound: &Bounds,
        solution: &InferSolution,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Option<LocalTypeId> {
        // resolve lower and upper bounds
        let lower = self.resolve_joined_bounds(&bound.lower, solution, types, JoinKind::Union);
        let upper =
            self.resolve_joined_bounds(&bound.upper, solution, types, JoinKind::Intersection);

        // prefer a consistent bound when possible
        match (lower, upper) {
            (Some(lower), Some(upper)) => {
                if self.is_type_assignable(module, profile, symbols, upper, lower, types, options)
                    == Assignability::Assignable
                {
                    Some(lower)
                } else {
                    Some(upper)
                }
            }
            (Some(lower), None) => Some(lower),
            (None, Some(upper)) => Some(upper),
            (None, None) => bound.default.or_else(|| Some(self.unknown_type(types))),
        }
    }

    /// Resolve and join bounds into a single type.
    fn resolve_joined_bounds(
        &self,
        bounds: &[LocalTypeId],
        solution: &InferSolution,
        types: &mut TypeTable,
        kind: JoinKind,
    ) -> Option<LocalTypeId> {
        // resolve each bound through any solved inference variables
        let mut resolved = Vec::new();
        for bound in bounds {
            let bound = self.resolve_infer_type(*bound, solution, types);
            resolved.push(bound);
        }

        if resolved.is_empty() {
            return None;
        }

        // join the resolved types
        Some(self.join_types(resolved, types, kind))
    }

    /// Replace inference variables with resolved types when possible.
    fn resolve_infer_type(
        &self,
        ty_id: LocalTypeId,
        solution: &InferSolution,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        match types.get_type(ty_id) {
            Type::InferVar { id } => solution.get(*id).unwrap_or(ty_id),
            _ => ty_id,
        }
    }

    /// Join multiple types into a union or intersection.
    fn join_types(
        &self,
        mut types_to_join: Vec<LocalTypeId>,
        types: &mut TypeTable,
        kind: JoinKind,
    ) -> LocalTypeId {
        // flatten nested union or intersection types
        let mut elements = Vec::new();
        for ty_id in types_to_join.drain(..) {
            match types.get_type(ty_id) {
                Type::Union { elements: union } if kind == JoinKind::Union => {
                    elements.extend(union.iter().copied());
                }
                Type::Intersection {
                    elements: intersection,
                } if kind == JoinKind::Intersection => {
                    elements.extend(intersection.iter().copied());
                }
                _ => elements.push(ty_id),
            }
        }

        if elements.len() == 1 {
            return elements[0];
        }

        let ty = match kind {
            JoinKind::Union => Type::Union { elements },
            JoinKind::Intersection => Type::Intersection { elements },
        };
        types.insert_type(ty)
    }

    /// Insert an unknown type.
    fn unknown_type(&self, types: &mut TypeTable) -> LocalTypeId {
        types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Unknown,
        })
    }

    /// Extract an inference variable id for a type.
    fn infer_var_id_for_type(ty_id: LocalTypeId, types: &TypeTable) -> Option<InferVarId> {
        match types.get_type(ty_id) {
            Type::InferVar { id } => Some(*id),
            _ => None,
        }
    }

    /// Replace infer var types in the table with resolved types.
    fn apply_solution(&self, solution: &InferSolution, types: &mut TypeTable) {
        // iterate through all types to replace inference variables
        let count = types.type_count();
        for id in 0..count {
            let ty_id = LocalTypeId::new(id);
            let resolved = match types.get_type(ty_id) {
                Type::InferVar { id } => solution.get(*id),
                _ => None,
            };
            if let Some(resolved_id) = resolved {
                let resolved_ty = types.get_type(resolved_id).clone();
                *types.get_type_mut(ty_id) = resolved_ty;
            }
        }

        types.clear_normalization_cache();
    }
}

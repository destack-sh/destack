use destack_dir::{
    Constraint, InferTable, InferVar, InferVarId, LocalTypeId, SymbolTable, Type, TypeElement,
    TypeField, TypeIndexSignature, TypeLiteral, TypeMappedParameter, TypeTable,
};
use destack_workspace::{Module, ProfileId};
use std::collections::HashMap;

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
    /// Replace infer vars inside a type with resolved bounds for assignability checks.
    pub(crate) fn materialize_infer_type_for_check(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        ty_id: LocalTypeId,
        infer: &InferTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> LocalTypeId {
        let mut cache = HashMap::new();
        self.materialize_infer_type_for_check_inner(
            module, profile, symbols, ty_id, infer, types, options, &mut cache,
        )
    }

    fn materialize_infer_type_for_check_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        ty_id: LocalTypeId,
        infer: &InferTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
        cache: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> LocalTypeId {
        if let Some(mapped) = cache.get(&ty_id).copied() {
            return mapped;
        }
        cache.insert(ty_id, ty_id);

        let ty = types.get_type(ty_id).clone();
        let mapped = match ty {
            Type::InferVar { .. } => {
                if let Some(resolved) = self.resolve_infer_type_for_check(
                    module, profile, symbols, ty_id, infer, types, options,
                ) {
                    self.materialize_infer_type_for_check_inner(
                        module, profile, symbols, resolved, infer, types, options, cache,
                    )
                } else {
                    ty_id
                }
            }
            Type::Union { elements } => {
                let mut did_change = false;
                let mut mapped_elements = Vec::with_capacity(elements.len());
                for element in elements {
                    let mapped_element = self.materialize_infer_type_for_check_inner(
                        module, profile, symbols, element, infer, types, options, cache,
                    );
                    if mapped_element != element {
                        did_change = true;
                    }
                    mapped_elements.push(mapped_element);
                }
                if did_change {
                    types.insert_type_from_type(
                        Type::Union {
                            elements: mapped_elements,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Intersection { elements } => {
                let mut did_change = false;
                let mut mapped_elements = Vec::with_capacity(elements.len());
                for element in elements {
                    let mapped_element = self.materialize_infer_type_for_check_inner(
                        module, profile, symbols, element, infer, types, options, cache,
                    );
                    if mapped_element != element {
                        did_change = true;
                    }
                    mapped_elements.push(mapped_element);
                }
                if did_change {
                    types.insert_type_from_type(
                        Type::Intersection {
                            elements: mapped_elements,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Value { value } => {
                let mapped_value = self.materialize_infer_type_for_check_inner(
                    module, profile, symbols, value, infer, types, options, cache,
                );
                if mapped_value == value {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Value {
                            value: mapped_value,
                        },
                        ty_id,
                    )
                }
            }
            Type::Unary { operator, right } => {
                let mapped_right = self.materialize_infer_type_for_check_inner(
                    module, profile, symbols, right, infer, types, options, cache,
                );
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Unary {
                            operator,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Binary {
                left,
                operator,
                right,
            } => {
                let mapped_left = self.materialize_infer_type_for_check_inner(
                    module, profile, symbols, left, infer, types, options, cache,
                );
                let mapped_right = self.materialize_infer_type_for_check_inner(
                    module, profile, symbols, right, infer, types, options, cache,
                );
                if mapped_left == left && mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Binary {
                            left: mapped_left,
                            operator,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::Conditional {
                distributive_symbol,
                left,
                right,
                then_type,
                else_type,
            } => {
                let mapped_left = self.materialize_infer_type_for_check_inner(
                    module, profile, symbols, left, infer, types, options, cache,
                );
                let mapped_right = self.materialize_infer_type_for_check_inner(
                    module, profile, symbols, right, infer, types, options, cache,
                );
                let mapped_then = self.materialize_infer_type_for_check_inner(
                    module, profile, symbols, then_type, infer, types, options, cache,
                );
                let mapped_else = self.materialize_infer_type_for_check_inner(
                    module, profile, symbols, else_type, infer, types, options, cache,
                );
                if mapped_left == left
                    && mapped_right == right
                    && mapped_then == then_type
                    && mapped_else == else_type
                {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Conditional {
                            distributive_symbol,
                            left: mapped_left,
                            right: mapped_right,
                            then_type: mapped_then,
                            else_type: mapped_else,
                        },
                        ty_id,
                    )
                }
            }
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let mapped_constraint = self.materialize_infer_type_for_check_inner(
                    module,
                    profile,
                    symbols,
                    parameter.constraint,
                    infer,
                    types,
                    options,
                    cache,
                );
                let mapped_key_remap = parameter.key_remap.map(|key_remap| {
                    self.materialize_infer_type_for_check_inner(
                        module, profile, symbols, key_remap, infer, types, options, cache,
                    )
                });
                let mapped_value = self.materialize_infer_type_for_check_inner(
                    module, profile, symbols, value, infer, types, options, cache,
                );
                if mapped_constraint == parameter.constraint
                    && mapped_key_remap == parameter.key_remap
                    && mapped_value == value
                {
                    ty_id
                } else {
                    let parameter = TypeMappedParameter {
                        name: parameter.name,
                        symbol: parameter.symbol,
                        constraint: mapped_constraint,
                        key_remap: mapped_key_remap,
                    };
                    types.insert_type_from_type(
                        Type::Mapped {
                            parameter,
                            modifiers,
                            value: mapped_value,
                        },
                        ty_id,
                    )
                }
            }
            Type::Index { left, index } => {
                let mapped_left = self.materialize_infer_type_for_check_inner(
                    module, profile, symbols, left, infer, types, options, cache,
                );
                let mapped_index = self.materialize_infer_type_for_check_inner(
                    module, profile, symbols, index, infer, types, options, cache,
                );
                if mapped_left == left && mapped_index == index {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Index {
                            left: mapped_left,
                            index: mapped_index,
                        },
                        ty_id,
                    )
                }
            }
            Type::TemplateLiteral { strings, spans } => {
                let mut did_change = false;
                let mut mapped_spans = Vec::with_capacity(spans.len());
                for span in spans {
                    let mapped_span = self.materialize_infer_type_for_check_inner(
                        module, profile, symbols, span, infer, types, options, cache,
                    );
                    if mapped_span != span {
                        did_change = true;
                    }
                    mapped_spans.push(mapped_span);
                }
                if did_change {
                    types.insert_type_from_type(
                        Type::TemplateLiteral {
                            strings,
                            spans: mapped_spans,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Array {
                element,
                is_readonly,
            } => {
                let mapped_element = element.map(|element| {
                    self.materialize_infer_type_for_check_inner(
                        module, profile, symbols, element, infer, types, options, cache,
                    )
                });
                if mapped_element == element {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Array {
                            element: mapped_element,
                            is_readonly,
                        },
                        ty_id,
                    )
                }
            }
            Type::ArraySized {
                element,
                count,
                is_readonly,
            } => {
                let mapped_element = self.materialize_infer_type_for_check_inner(
                    module, profile, symbols, element, infer, types, options, cache,
                );
                if mapped_element == element {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::ArraySized {
                            element: mapped_element,
                            count,
                            is_readonly,
                        },
                        ty_id,
                    )
                }
            }
            Type::Tuple {
                elements,
                is_readonly,
            } => {
                let mut did_change = false;
                let mut mapped_elements = Vec::with_capacity(elements.len());
                for element in elements {
                    let mapped_element = self.materialize_infer_type_for_check_inner(
                        module, profile, symbols, element.ty, infer, types, options, cache,
                    );
                    if mapped_element != element.ty {
                        did_change = true;
                    }
                    mapped_elements.push(TypeElement {
                        ty: mapped_element,
                        ..element
                    });
                }
                if did_change {
                    types.insert_type_from_type(
                        Type::Tuple {
                            elements: mapped_elements,
                            is_readonly,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                let mut did_change = false;
                let mut mapped_fields = Vec::with_capacity(fields.len());
                for field in fields {
                    let mapped_ty = self.materialize_infer_type_for_check_inner(
                        module, profile, symbols, field.ty, infer, types, options, cache,
                    );
                    if mapped_ty != field.ty {
                        did_change = true;
                    }
                    mapped_fields.push(TypeField {
                        ty: mapped_ty,
                        ..field
                    });
                }
                let mut mapped_calls = Vec::with_capacity(call_signatures.len());
                for signature in call_signatures {
                    let mapped_sig = self.materialize_infer_type_for_check_inner(
                        module, profile, symbols, signature, infer, types, options, cache,
                    );
                    if mapped_sig != signature {
                        did_change = true;
                    }
                    mapped_calls.push(mapped_sig);
                }
                let mut mapped_constructs = Vec::with_capacity(construct_signatures.len());
                for signature in construct_signatures {
                    let mapped_sig = self.materialize_infer_type_for_check_inner(
                        module, profile, symbols, signature, infer, types, options, cache,
                    );
                    if mapped_sig != signature {
                        did_change = true;
                    }
                    mapped_constructs.push(mapped_sig);
                }
                let mut mapped_indexes = Vec::with_capacity(index_signatures.len());
                for signature in index_signatures {
                    let mapped_key = self.materialize_infer_type_for_check_inner(
                        module,
                        profile,
                        symbols,
                        signature.key_type,
                        infer,
                        types,
                        options,
                        cache,
                    );
                    let mapped_value = self.materialize_infer_type_for_check_inner(
                        module,
                        profile,
                        symbols,
                        signature.value_type,
                        infer,
                        types,
                        options,
                        cache,
                    );
                    if mapped_key != signature.key_type || mapped_value != signature.value_type {
                        did_change = true;
                    }
                    mapped_indexes.push(TypeIndexSignature {
                        key_type: mapped_key,
                        value_type: mapped_value,
                        ..signature
                    });
                }
                if did_change {
                    types.insert_type_from_type(
                        Type::Object {
                            fields: mapped_fields,
                            call_signatures: mapped_calls,
                            construct_signatures: mapped_constructs,
                            index_signatures: mapped_indexes,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Function {
                asynchrony,
                cardinality,
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
            } => {
                let mut did_change = false;
                let mut mapped_static = Vec::with_capacity(static_parameters.len());
                for parameter in static_parameters {
                    let mapped_param = self.materialize_infer_type_for_check_inner(
                        module, profile, symbols, parameter, infer, types, options, cache,
                    );
                    if mapped_param != parameter {
                        did_change = true;
                    }
                    mapped_static.push(mapped_param);
                }
                let mapped_this = this_parameter.map(|parameter| {
                    let mapped_param = self.materialize_infer_type_for_check_inner(
                        module, profile, symbols, parameter, infer, types, options, cache,
                    );
                    if mapped_param != parameter {
                        did_change = true;
                    }
                    mapped_param
                });
                let mut mapped_dynamic = Vec::with_capacity(dynamic_parameters.len());
                for parameter in dynamic_parameters {
                    let mapped_param = self.materialize_infer_type_for_check_inner(
                        module, profile, symbols, parameter, infer, types, options, cache,
                    );
                    if mapped_param != parameter {
                        did_change = true;
                    }
                    mapped_dynamic.push(mapped_param);
                }
                let mapped_return = return_type.map(|return_type| {
                    let mapped_return = self.materialize_infer_type_for_check_inner(
                        module,
                        profile,
                        symbols,
                        return_type,
                        infer,
                        types,
                        options,
                        cache,
                    );
                    if mapped_return != return_type {
                        did_change = true;
                    }
                    mapped_return
                });
                if did_change {
                    types.insert_type_from_type(
                        Type::Function {
                            asynchrony,
                            cardinality,
                            static_parameters: mapped_static,
                            this_parameter: mapped_this,
                            dynamic_parameters: mapped_dynamic,
                            return_type: mapped_return,
                        },
                        ty_id,
                    )
                } else {
                    ty_id
                }
            }
            Type::Predicate {
                asserts,
                subject,
                target,
            } => {
                let mapped_target = target.map(|target| {
                    self.materialize_infer_type_for_check_inner(
                        module, profile, symbols, target, infer, types, options, cache,
                    )
                });
                if mapped_target == target {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::Predicate {
                            asserts,
                            subject,
                            target: mapped_target,
                        },
                        ty_id,
                    )
                }
            }
            Type::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let mapped_right = self.materialize_infer_type_for_check_inner(
                    module, profile, symbols, right, infer, types, options, cache,
                );
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::ValueOf {
                            mutability,
                            variance,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let mapped_right = self.materialize_infer_type_for_check_inner(
                    module, profile, symbols, right, infer, types, options, cache,
                );
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::ReferenceOf {
                            mutability,
                            variance,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            Type::PointerOf { mutability, right } => {
                let mapped_right = self.materialize_infer_type_for_check_inner(
                    module, profile, symbols, right, infer, types, options, cache,
                );
                if mapped_right == right {
                    ty_id
                } else {
                    types.insert_type_from_type(
                        Type::PointerOf {
                            mutability,
                            right: mapped_right,
                        },
                        ty_id,
                    )
                }
            }
            _ => ty_id,
        };

        cache.insert(ty_id, mapped);
        mapped
    }

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
                let var_id = InferVarId::new(index as u32);
                let fallback_source_type_id = infer.type_for_var(var_id);
                if let Some(resolved) = self.resolve_bounds(
                    module,
                    profile,
                    symbols,
                    bound,
                    fallback_source_type_id,
                    &solution,
                    types,
                    options,
                ) {
                    solution.resolved[index] = Some(resolved);
                    did_resolve = true;
                }
            }
        }

        // apply resolved types into the table
        self.apply_solution(&solution, types);

        solution
    }

    /// Resolve an inference variable type for use in assignability checks.
    pub(crate) fn resolve_infer_type_for_check(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        ty_id: LocalTypeId,
        infer: &InferTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Option<LocalTypeId> {
        // return early when the type is not an inference variable
        let Type::InferVar { id } = types.get_type(ty_id) else {
            return Some(ty_id);
        };

        // collect bounds for the current inference table
        let mut bounds: Vec<_> = infer.vars.iter().map(Bounds::from_var).collect();
        for constraint in &infer.constraints {
            Self::apply_constraint(constraint, types, &mut bounds);
        }

        // resolve the specific inference variable against the collected bounds
        let bound = bounds.get(id.0 as usize)?;
        let fallback_source_type_id = infer.type_for_var(*id);
        let solution = InferSolution {
            resolved: vec![None; infer.vars.len()],
        };
        self.resolve_bounds(
            module,
            profile,
            symbols,
            bound,
            fallback_source_type_id,
            &solution,
            types,
            options,
        )
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
        fallback_source_type_id: Option<LocalTypeId>,
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
            (None, None) => bound.default.or_else(|| {
                fallback_source_type_id
                    .map(|source_type_id| self.unknown_type(source_type_id, types))
            }),
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

        let source_type_id = elements[0];
        let ty = match kind {
            JoinKind::Union => Type::Union { elements },
            JoinKind::Intersection => Type::Intersection { elements },
        };
        types.insert_type_from_type(ty, source_type_id)
    }

    /// Insert an unknown type.
    fn unknown_type(&self, source_type_id: LocalTypeId, types: &mut TypeTable) -> LocalTypeId {
        types.insert_type_from_type(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            source_type_id,
        )
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

        types.invalidate_normalization_cache();
    }
}

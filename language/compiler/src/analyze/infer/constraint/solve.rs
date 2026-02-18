use std::collections::HashMap;

use destack_dir::{
    Constraint, InferTable, InferVar, InferVarId, LocalTypeId, SymbolTable, Type, TypeLiteral,
    TypeRewriter, TypeRewriterOptions, TypeTable, rewrite_type,
};
use destack_workspace::{Module, ProfileId};
use indexmap::IndexSet;

use crate::analyze::common::{
    MaterializationMode, NormalizationMode, REWRITER_TAG_INFER_MATERIALIZER, RelationMode,
    TypeRewriteCache, TypeWalkContext, rewrite_type_with_cache,
};
use crate::timing::tags;
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

/// Cache assignability normalization results during a solve pass.
#[derive(Default)]
struct SolveNormalizationCache {
    /// Cached apparent types keyed by source type id.
    apparent_assign: HashMap<LocalTypeId, LocalTypeId>,
}

impl SolveNormalizationCache {
    /// Create an empty normalization cache.
    fn new() -> Self {
        Self {
            apparent_assign: HashMap::new(),
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

/// Rewrite types by replacing infer vars with resolved bounds.
struct InferTypeMaterializer<'a> {
    /// The compiler instance.
    compiler: &'a Compiler,
    /// The current module.
    module: &'a Module,
    /// The active profile.
    profile: ProfileId,
    /// The current symbol table.
    symbols: &'a SymbolTable,
    /// The inference table.
    infer: &'a InferTable,
    /// The analysis options.
    options: &'a AnalyzeOptions,
    /// The materialization mode.
    mode: MaterializationMode,
    /// Cached materializations by type id.
    cache: TypeRewriteCache,
    /// The cache key for rewrites.
    cache_key: u64,
    /// The rewriter options.
    rewrite_options: TypeRewriterOptions,
}

impl<'a> InferTypeMaterializer<'a> {
    /// Create a materializer for infer vars.
    fn new(
        compiler: &'a Compiler,
        module: &'a Module,
        profile: ProfileId,
        symbols: &'a SymbolTable,
        infer: &'a InferTable,
        options: &'a AnalyzeOptions,
        mode: MaterializationMode,
    ) -> Self {
        let walk_context = TypeWalkContext::for_materialization(mode)
            .with_rewriter_tag(REWRITER_TAG_INFER_MATERIALIZER);
        let context_key = infer.cache_key() ^ options.cache_key();
        let walk_context = walk_context.with_context_key(context_key);
        let rewrite_options = walk_context.rewriter_options();
        let cache_key = rewrite_options.cache_key();
        Self {
            compiler,
            module,
            profile,
            symbols,
            infer,
            options,
            mode,
            cache: TypeRewriteCache::new(),
            cache_key,
            rewrite_options,
        }
    }
}

impl TypeRewriter for InferTypeMaterializer<'_> {
    fn options(&self) -> &TypeRewriterOptions {
        &self.rewrite_options
    }

    fn rewrite_type_id(&mut self, types: &mut TypeTable, id: LocalTypeId) -> LocalTypeId {
        let mut cache = std::mem::take(&mut self.cache);
        let mapped = rewrite_type_with_cache(self, types, &mut cache, self.cache_key, id);
        self.cache = cache;
        mapped
    }

    fn rewrite_type(&mut self, types: &mut TypeTable, id: LocalTypeId, ty: &Type) -> LocalTypeId {
        match self.mode {
            MaterializationMode::Shape => rewrite_type(self, types, id, ty),
            MaterializationMode::Validation | MaterializationMode::Surface => match ty {
                Type::InferVar { .. } => {
                    if let Some(resolved) = self.compiler.resolve_infer_type_for_check(
                        self.module,
                        self.profile,
                        self.symbols,
                        id,
                        self.infer,
                        types,
                        self.options,
                    ) {
                        self.rewrite_type_id(types, resolved)
                    } else {
                        id
                    }
                }
                _ => rewrite_type(self, types, id, ty),
            },
        }
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
        let _timing = self.timing_scope(tags::ANALYZE_INFER_TYPE_MATERIALIZE);

        // shape mode placeholder to keep the variant live
        let _ = MaterializationMode::Shape;
        let mut materializer = InferTypeMaterializer::new(
            self,
            module,
            profile,
            symbols,
            infer,
            options,
            MaterializationMode::Validation,
        );
        materializer.rewrite_type_id(types, ty_id)
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
        let mut normalization_cache = SolveNormalizationCache::new();

        // collect initial bounds
        let mut bounds: Vec<_> = infer.vars.iter().map(Bounds::from_var).collect();

        // apply constraints to bounds
        for constraint in &infer.constraints {
            Self::apply_constraint(constraint, types, &mut bounds);
        }
        self.dedupe_bounds(&mut bounds);

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
                    &mut normalization_cache,
                ) {
                    solution.resolved[index] = Some(resolved);
                    did_resolve = true;
                }
            }
        }

        // apply resolved types into the table
        self.apply_solution(&solution, infer, types);

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
        // NOTE #Performance: recomputes bounds and normalization cache per query
        // return early when the type is not an inference variable
        let Type::InferVar { id } = types.get_type(ty_id) else {
            return Some(ty_id);
        };
        let mut normalization_cache = SolveNormalizationCache::new();

        // collect bounds for the current inference table
        let mut bounds: Vec<_> = infer.vars.iter().map(Bounds::from_var).collect();
        for constraint in &infer.constraints {
            Self::apply_constraint(constraint, types, &mut bounds);
        }
        self.dedupe_bounds(&mut bounds);

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
            &mut normalization_cache,
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
        normalization_cache: &mut SolveNormalizationCache,
    ) -> Option<LocalTypeId> {
        // NOTE #Performance: resolve_bounds recomputes assignability per iteration without caching
        // resolve lower and upper bounds
        let lower = self.resolve_joined_bounds(&bound.lower, solution, types, JoinKind::Union);
        let upper =
            self.resolve_joined_bounds(&bound.upper, solution, types, JoinKind::Intersection);

        // prefer a consistent bound when possible
        match (lower, upper) {
            (Some(lower), Some(upper)) => {
                let normalized_target = self.normalize_apparent_type_for_assignability_cached(
                    module,
                    profile,
                    upper,
                    symbols,
                    types,
                    normalization_cache,
                );
                let normalized_source = self.normalize_apparent_type_for_assignability_cached(
                    module,
                    profile,
                    lower,
                    symbols,
                    types,
                    normalization_cache,
                );
                if self.is_type_assignable_normalized(
                    module,
                    profile,
                    symbols,
                    normalized_target,
                    normalized_source,
                    types,
                    options,
                ) == Assignability::Assignable
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

        self.dedupe_type_list(&mut resolved);
        if resolved.is_empty() {
            return None;
        }

        // join the resolved types
        Some(self.join_types(resolved, types, kind))
    }

    /// Normalize a type for assignability using the per-solve cache.
    fn normalize_apparent_type_for_assignability_cached(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        normalization_cache: &mut SolveNormalizationCache,
    ) -> LocalTypeId {
        if let Some(cached) = normalization_cache.apparent_assign.get(&type_id) {
            return *cached;
        }

        // normalize apparent types for assignability
        let normalized = self.normalize_apparent_type(
            module,
            profile,
            type_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::ASSIGN,
        );
        normalization_cache
            .apparent_assign
            .insert(type_id, normalized);
        normalized
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

    /// Deduplicate lower and upper bound lists.
    fn dedupe_bounds(&self, bounds: &mut [Bounds]) {
        for bound in bounds {
            self.dedupe_type_list(&mut bound.lower);
            self.dedupe_type_list(&mut bound.upper);
        }
    }

    /// Deduplicate a list of type ids while preserving order.
    fn dedupe_type_list(&self, list: &mut Vec<LocalTypeId>) {
        let mut seen = IndexSet::new();
        list.retain(|id| seen.insert(*id));
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
    fn apply_solution(&self, solution: &InferSolution, infer: &InferTable, types: &mut TypeTable) {
        // update the concrete infer var type ids rather than scanning every type
        for (index, resolved) in solution.resolved.iter().enumerate() {
            let Some(resolved_id) = *resolved else {
                continue;
            };
            let Some(var_type_id) = infer.type_for_var(InferVarId::new(index as u32)) else {
                continue;
            };
            let resolved_ty = types.get_type(resolved_id).clone();
            types.update_type(var_type_id, resolved_ty);
        }
    }
}

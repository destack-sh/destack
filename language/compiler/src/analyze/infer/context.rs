use destack_dir::{
    Asynchrony, FunctionCardinality, FunctionSignature, GlobalSymbolId, LocalNodeIdAny, LocalTypeId,
};
use destack_workspace::{DsConfigCompilerOptions, ProfileId};

use crate::AnalyzeOptions;

/// InferContext holds contextual, flow sensitive information during type analysis.
#[derive(Debug)]
pub struct InferContext {
    /// The active profile id.
    pub profile: ProfileId,
    /// Semantic options for the current module.
    pub options: AnalyzeOptions,
    /// Type narrowings currently in scope.
    pub narrowings: Vec<(GlobalSymbolId, LocalTypeId)>,
    /// Expected type from the surrounding context.
    pub expected_type: Option<LocalTypeId>,
    /// Return type for the current function.
    pub return_type: Option<LocalTypeId>,
    /// Whether we're in an unreachable code region (after `return`, `throw`, etc.).
    pub is_unreachable: bool,
    /// The enclosing loop node (if any) (enables break and continue).
    pub in_loop: Option<LocalNodeIdAny>,
    /// The enclosing match node (if any) (enables break only).
    pub in_match: Option<LocalNodeIdAny>,
    /// The enclosing function node (if any) (enables return).
    pub in_function: Option<LocalNodeIdAny>,
    /// Whether the enclosing function is async (enables await).
    pub is_async: bool,
    /// Whether the enclosing function is a generator (enables yield).
    pub is_generator: bool,
    /// Whether we're in an abstract class/struct (abstract methods allowed).
    pub in_abstract_class: bool,
}

impl InferContext {
    /// Create a new empty context.
    pub fn new(profile: ProfileId, options: AnalyzeOptions) -> Self {
        Self {
            profile,
            options,
            narrowings: Vec::new(),
            expected_type: None,
            return_type: None,
            is_unreachable: false,
            in_loop: None,
            in_match: None,
            in_function: None,
            is_async: false,
            is_generator: false,
            in_abstract_class: false,
        }
    }

    /// Fork the context.
    pub fn fork(&self) -> Self {
        Self {
            profile: self.profile,
            options: self.options,
            narrowings: self.narrowings.clone(),
            expected_type: self.expected_type,
            return_type: self.return_type,
            is_unreachable: self.is_unreachable,
            in_loop: self.in_loop,
            in_match: self.in_match,
            in_function: self.in_function,
            is_async: self.is_async,
            is_generator: self.is_generator,
            in_abstract_class: self.in_abstract_class,
        }
    }

    /// Reset flow context.
    pub fn reset(&self) -> Self {
        Self {
            profile: self.profile,
            options: self.options,
            narrowings: self.narrowings.clone(),
            expected_type: self.expected_type,
            return_type: self.return_type,
            is_unreachable: false,
            in_loop: None,
            in_match: None,
            in_function: None,
            is_async: false,
            is_generator: false,
            in_abstract_class: false,
        }
    }

    /// Enter a function context with the given signature.
    pub fn in_function_with_signature(
        self,
        function_id: LocalNodeIdAny,
        signature: &FunctionSignature,
    ) -> Self {
        self.in_function(function_id)
            .is_async_maybe(signature.asynchrony == Asynchrony::Async)
            .is_generator_maybe(signature.cardinality == FunctionCardinality::Generator)
    }

    /// Enter a function context.
    pub fn in_function(mut self, function_id: LocalNodeIdAny) -> Self {
        self.in_function = Some(function_id);
        self
    }

    /// Enter a loop context.
    pub fn in_loop(mut self, loop_id: LocalNodeIdAny) -> Self {
        self.in_loop = Some(loop_id);
        self
    }

    /// Enter a match context.
    pub fn in_match(mut self, match_id: LocalNodeIdAny) -> Self {
        self.in_match = Some(match_id);
        self
    }

    /// Set async context conditionally.
    pub fn is_async_maybe(mut self, is_async: bool) -> Self {
        self.is_async = is_async;
        self
    }

    /// Set generator context conditionally.
    pub fn is_generator_maybe(mut self, is_generator: bool) -> Self {
        self.is_generator = is_generator;
        self
    }

    /// Set the expected type for the current context.
    pub fn with_expected_type(mut self, expected_type: Option<LocalTypeId>) -> Self {
        self.expected_type = expected_type;
        self
    }

    /// Set the return type for the current context.
    pub fn with_return_type(mut self, return_type: Option<LocalTypeId>) -> Self {
        self.return_type = return_type;
        self
    }

    /// Set abstract class context conditionally.
    pub fn in_abstract_class_maybe(mut self, is_abstract: bool) -> Self {
        self.in_abstract_class = is_abstract;
        self
    }

    /// Check if we can break (in loop or match).
    pub fn can_break(&self) -> bool {
        self.in_loop.is_some() || self.in_match.is_some()
    }

    /// Check if we can continue (in loop only).
    pub fn can_continue(&self) -> bool {
        self.in_loop.is_some()
    }

    /// Check if we can return (in function).
    pub fn can_return(&self) -> bool {
        self.in_function.is_some()
    }

    /// Check if we can await (in async function or at top level).
    pub fn can_await(&self) -> bool {
        // await is valid in async functions or at module top level when not in any function
        self.is_async || self.in_function.is_none()
    }

    /// Check if we can yield (in generator function).
    pub fn can_yield(&self) -> bool {
        self.is_generator
    }

    /// Mark this context as unreachable.
    pub fn mark_unreachable(&mut self) {
        self.is_unreachable = true;
    }

    /// Add a narrowing for a symbol.
    pub fn narrow(&mut self, symbol: GlobalSymbolId, ty: LocalTypeId) {
        if let Some(index) = self.narrowings.iter().position(|(s, _)| *s == symbol) {
            self.narrowings[index] = (symbol, ty);
        } else {
            self.narrowings.push((symbol, ty));
        }
    }

    /// Get the narrowed type for a symbol, if any.
    pub fn get_narrowed(&self, symbol: GlobalSymbolId) -> Option<LocalTypeId> {
        self.narrowings
            .iter()
            .find(|(s, _)| *s == symbol)
            .map(|(_, ty)| *ty)
    }

    /// Merge two "branched" context together.
    pub fn merge(&mut self, other: &InferContext) {
        // merge reachability and narrowings
        // if neither is unreachable
        if self.is_unreachable && other.is_unreachable {
            // both unreachable, stay unreachable
        }
        // if one is unreachable, keep the other's narrowings
        // if both branches are unreachable, result is unreachable
        else if self.is_unreachable {
            // self is unreachable, take other's state
            self.narrowings = other.narrowings.clone();
            self.is_unreachable = false;
        }
        // other is unreachable
        else if other.is_unreachable {
            // keep self's state
        }
        // both reachable: keep only narrowings that exist in both
        else {
            self.narrowings.retain(|(s, ty)| {
                other
                    .narrowings
                    .iter()
                    .any(|(o_s, o_ty)| *s == *o_s && *ty == *o_ty)
            });
        }
    }
}

impl Default for InferContext {
    fn default() -> Self {
        let options = AnalyzeOptions::from(&DsConfigCompilerOptions::default());
        Self::new(ProfileId::new(0), options)
    }
}

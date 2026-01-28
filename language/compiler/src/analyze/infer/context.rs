use std::sync::Arc;

use destack_dir::{
    Asynchrony, FlowGraph, FlowTable, FunctionCardinality, FunctionSignature, GlobalSymbolId,
    LocalNodeIdAny, LocalTypeId, Mutability,
};
use destack_source::ModuleId;
use destack_workspace::{DsConfigCompilerOptions, ProfileId};

use crate::AnalyzeOptions;
use crate::analyze::common::{ConstContext, ContextualTypingMode, LiteralFreshness, WideningMode};

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
    /// The enclosing match node (if any).
    pub in_match: Option<LocalNodeIdAny>,
    /// The enclosing switch node (if any) (enables break only).
    pub in_switch: Option<LocalNodeIdAny>,
    /// Stack of active break targets (innermost last).
    pub break_stack: Vec<BreakTargetKind>,
    /// The enclosing function node (if any) (enables return).
    pub in_function: Option<LocalNodeIdAny>,
    /// Whether the enclosing function is async (enables await).
    pub is_async: bool,
    /// Whether the enclosing function is a generator (enables yield).
    pub is_generator: bool,
    /// Whether we're in an abstract class/struct (abstract methods allowed).
    pub in_abstract_class: bool,
    /// The enclosing nominal type symbol, if any.
    pub in_nominal_symbol: Option<GlobalSymbolId>,
    /// Track nested try frames for error propagation.
    pub try_stack: Vec<TryContextFrame>,
    /// Flow context for the current function body.
    pub flow: Option<FlowContext>,
    /// Whether this context should avoid caching inferred types.
    pub is_surface_inference: bool,
    /// Whether the current expression is under an explicit ownership operator.
    pub is_explicit_ownership: bool,
    /// The literal freshness mode for this context.
    pub literal_freshness: LiteralFreshness,
    /// The widening mode for this context.
    pub widening_mode: WideningMode,
    /// The const context mode for this context.
    pub const_context: ConstContext,
    /// The contextual typing mode for this context.
    pub contextual_typing: ContextualTypingMode,
}

/// Describe flow data used for inference.
#[derive(Debug, Clone)]
pub struct FlowContext {
    /// Identify the module owning this flow graph.
    pub module_id: ModuleId,
    /// Store the control flow graph for the current body.
    pub graph: Arc<FlowGraph>,
    /// Store flow environments for the control flow graph.
    pub table: Arc<FlowTable>,
}

/// Track the innermost break target kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BreakTargetKind {
    /// Break exits a loop.
    Loop,
    /// Break exits a switch.
    Switch,
}

/// Track try catch state during inference.
#[derive(Debug, Clone)]
pub struct TryContextFrame {
    /// Whether this try has a catch clause.
    pub has_catch: bool,
    /// Collected error types for `?` in this try.
    pub error_types: Vec<LocalTypeId>,
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
            in_switch: None,
            break_stack: Vec::new(),
            in_function: None,
            is_async: false,
            is_generator: false,
            in_abstract_class: false,
            in_nominal_symbol: None,
            try_stack: Vec::new(),
            flow: None,
            is_surface_inference: false,
            is_explicit_ownership: false,
            literal_freshness: LiteralFreshness::Fresh,
            widening_mode: WideningMode::Widen,
            const_context: ConstContext::None,
            contextual_typing: ContextualTypingMode::Default,
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
            in_switch: self.in_switch,
            break_stack: self.break_stack.clone(),
            in_function: self.in_function,
            is_async: self.is_async,
            is_generator: self.is_generator,
            in_abstract_class: self.in_abstract_class,
            in_nominal_symbol: self.in_nominal_symbol,
            try_stack: self.try_stack.clone(),
            flow: self.flow.clone(),
            is_surface_inference: self.is_surface_inference,
            is_explicit_ownership: self.is_explicit_ownership,
            literal_freshness: self.literal_freshness,
            widening_mode: self.widening_mode,
            const_context: self.const_context,
            contextual_typing: self.contextual_typing,
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
            in_switch: None,
            break_stack: Vec::new(),
            in_function: None,
            is_async: false,
            is_generator: false,
            in_abstract_class: false,
            in_nominal_symbol: self.in_nominal_symbol,
            try_stack: Vec::new(),
            flow: self.flow.clone(),
            is_surface_inference: self.is_surface_inference,
            is_explicit_ownership: self.is_explicit_ownership,
            literal_freshness: self.literal_freshness,
            widening_mode: self.widening_mode,
            const_context: self.const_context,
            contextual_typing: self.contextual_typing,
        }
    }

    /// Attach flow context to this inference context.
    pub fn with_flow_context(mut self, flow: FlowContext) -> Self {
        self.flow = Some(flow);
        self
    }

    /// Override semantic options for this context.
    pub fn with_options(mut self, options: AnalyzeOptions) -> Self {
        self.options = options;
        self
    }

    /// Mark this context as explicitly controlling ownership.
    pub fn with_explicit_ownership(mut self) -> Self {
        self.is_explicit_ownership = true;
        self
    }

    /// Mark this context as preserving literal freshness.
    pub fn with_fresh_literals(mut self) -> Self {
        self.literal_freshness = LiteralFreshness::Fresh;
        self
    }

    /// Mark this context as regularizing literal freshness.
    pub fn with_regularized_literals(mut self) -> Self {
        self.literal_freshness = LiteralFreshness::Regularized;
        self
    }

    /// Mark this context as widening literals.
    pub fn with_widening(mut self) -> Self {
        self.widening_mode = WideningMode::Widen;
        self
    }

    /// Mark this context as preserving literal types.
    pub fn with_preserve_literals(mut self) -> Self {
        self.widening_mode = WideningMode::Preserve;
        self
    }

    /// Apply a const context mode to this context.
    pub fn with_const_context(mut self, const_context: ConstContext) -> Self {
        self.const_context = const_context;
        self
    }

    /// Apply binding defaults based on mutability.
    pub fn with_binding_mutability(self, mutability: Mutability) -> Self {
        match mutability {
            Mutability::Immutable => self
                .with_const_context(ConstContext::Const)
                .with_preserve_literals()
                .with_fresh_literals(),
            Mutability::Mutable => self
                .with_const_context(ConstContext::None)
                .with_widening()
                .with_fresh_literals(),
        }
    }

    /// Apply binding defaults for using bindings.
    pub fn with_using_binding(self) -> Self {
        self.with_const_context(ConstContext::Const)
            .with_preserve_literals()
            .with_fresh_literals()
    }

    /// Override contextual typing behavior for this context.
    pub fn with_contextual_typing_mode(mut self, contextual_typing: ContextualTypingMode) -> Self {
        self.contextual_typing = contextual_typing;
        self
    }

    /// Mark this context as surface inference.
    pub fn for_surface_inference(mut self) -> Self {
        self.is_surface_inference = true;
        self
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
        // record loop as the innermost break target
        self.in_loop = Some(loop_id);
        self.break_stack.push(BreakTargetKind::Loop);

        self
    }

    /// Enter a match context.
    pub fn in_match(mut self, match_id: LocalNodeIdAny) -> Self {
        self.in_match = Some(match_id);
        self
    }

    /// Enter a switch context.
    pub fn in_switch(mut self, switch_id: LocalNodeIdAny) -> Self {
        self.in_switch = Some(switch_id);
        self.break_stack.push(BreakTargetKind::Switch);
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

    /// Set class or struct context conditionally.
    pub fn in_nominal_symbol_maybe(mut self, symbol: Option<GlobalSymbolId>) -> Self {
        self.in_nominal_symbol = symbol;
        self
    }

    /// Push a new try frame onto the stack.
    pub fn push_try_frame(&mut self, has_catch: bool) {
        self.try_stack.push(TryContextFrame {
            has_catch,
            error_types: Vec::new(),
        });
    }

    /// Pop the current try frame from the stack.
    pub fn pop_try_frame(&mut self) -> Option<TryContextFrame> {
        self.try_stack.pop()
    }

    /// Record a try error type for the nearest catch.
    pub fn record_try_error(&mut self, error_type_id: LocalTypeId) -> bool {
        // search for the nearest catch frame
        for frame in self.try_stack.iter_mut().rev() {
            if frame.has_catch {
                frame.error_types.push(error_type_id);
                return true;
            }
        }

        false
    }

    /// Merge try error types from a forked context.
    pub fn merge_try_error_types_from(&mut self, other: &InferContext) {
        if self.try_stack.len() != other.try_stack.len() {
            return;
        }

        for (frame, other_frame) in self.try_stack.iter_mut().zip(other.try_stack.iter()) {
            frame
                .error_types
                .extend_from_slice(&other_frame.error_types);
        }
    }

    /// Check if we can break (in loop or switch).
    pub fn can_break(&self) -> bool {
        !self.break_stack.is_empty()
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

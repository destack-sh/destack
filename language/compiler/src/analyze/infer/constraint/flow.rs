use std::collections::VecDeque;

use indexmap::IndexMap;

use super::r#type::TypeGuardTarget;
use crate::analyze::common::{NormalizationMode, TreeSymbolView, TypeContext};
use crate::analyze::infer::RemoteValueTypeReadDomain;
use destack_core::StringId;
use destack_dir::{
    Argument, BinaryOperator, Block, Declaration, Expression, FlowBlock, FlowEdge, FlowEdgeKind,
    FlowEnvironment, FlowGraph, FlowGuard, FlowTable, FunctionSignature, GlobalSymbolId, Key,
    LocalNodeId, LocalNodeIdAny, LocalTypeId, NodeTree, NodeType, NodeVisitor, NodeVisitorOptions,
    Parameter, Pattern, PatternField, PredicateSubject, RuntimeCheckKind, ScalarLiteral, StaticKey,
    Type, TypeExpression, TypeField, TypeLiteral, TypeTable, UnaryOperator, are_types_equal,
    walk_expression,
};

use crate::{AnalyzeError, AnalyzeResult, Compiler, InferState};

/// Track whether a tree walk encounters flow sensitive constructs.
#[derive(Debug, Default)]
struct FlowSensitiveVisitor {
    /// The visitor options.
    options: NodeVisitorOptions,
    /// Whether we found flow sensitive constructs.
    requires_flow: bool,
}

impl FlowSensitiveVisitor {
    /// Create a new flow requirement visitor.
    fn new() -> Self {
        Self {
            options: NodeVisitorOptions::default(),
            requires_flow: false,
        }
    }

    /// Report whether the current traversal requires flow typing.
    fn requires_flow(&self) -> bool {
        self.requires_flow
    }

    fn mark_flow_required(&mut self) {
        self.requires_flow = true;
    }
}

impl NodeVisitor for FlowSensitiveVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // stop once flow is required
        if self.requires_flow {
            return;
        }

        // detect control flow constructs
        if matches!(
            expression,
            Expression::If { .. }
                | Expression::Loop { .. }
                | Expression::ForEach { .. }
                | Expression::For { .. }
                | Expression::Match { .. }
                | Expression::Try { .. }
                | Expression::Return { .. }
                | Expression::Break { .. }
                | Expression::UnresolvedBreak { .. }
                | Expression::Continue { .. }
                | Expression::UnresolvedContinue { .. }
                | Expression::Throw { .. }
        ) {
            self.mark_flow_required();
            return;
        }

        // detect short circuit operators
        if matches!(
            expression,
            Expression::Binary {
                operator: BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce,
                ..
            }
        ) {
            self.mark_flow_required();
            return;
        }

        // detect block-leading calls for assertion narrowing
        if matches!(expression, Expression::Call { .. })
            && let Some(parent) = tree.get_parent(id.id)
            && parent.ty == NodeType::Block
        {
            let block = tree.get(parent.into_typed::<Block>());
            if block.leading_expressions.contains(&id) {
                self.mark_flow_required();
                return;
            }
        }

        // visit nested expressions
        walk_expression(self, tree, id, expression);
    }

    fn visit_declaration(
        &mut self,
        tree: &NodeTree,
        _id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) {
        // stop once flow is required
        if self.requires_flow {
            return;
        }

        // only visit declarations that execute immediately
        match declaration {
            Declaration::Global(declaration) => {
                for expression_id in &declaration.expressions {
                    let expression = tree.get(*expression_id);
                    self.visit_expression(tree, *expression_id, expression);
                    if self.requires_flow {
                        return;
                    }
                }
            }
            Declaration::Namespace(declaration) => {
                for expression_id in &declaration.expressions {
                    let expression = tree.get(*expression_id);
                    self.visit_expression(tree, *expression_id, expression);
                    if self.requires_flow {
                        return;
                    }
                }
            }
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Report whether an expression body needs flow typing.
    pub fn expression_requires_flow(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let mut visitor = FlowSensitiveVisitor::new();
        let expression = tree.get(expression_id);
        visitor.visit_expression(tree, expression_id, expression);

        visitor.requires_flow()
    }

    /// Report whether any root expression needs flow typing.
    pub fn roots_require_flow(&self, tree: &NodeTree, roots: &[LocalNodeId<Expression>]) -> bool {
        let mut visitor = FlowSensitiveVisitor::new();
        for root_id in roots {
            let expression = tree.get(*root_id);
            visitor.visit_expression(tree, *root_id, expression);
            if visitor.requires_flow() {
                return true;
            }
        }

        false
    }

    /// Compute a flow table for a control flow graph.
    pub(crate) fn compute_flow_table_for_graph(
        &self,
        ctx: &mut TypeContext<'_>,
        graph: &FlowGraph,
        context: &InferState,
    ) -> AnalyzeResult<FlowTable> {
        // seed the flow table with the current context
        let mut flow = FlowTable::new();
        let base_environment_id =
            flow.push_environment(self.flow_environment_from_context(context));
        let block_count = graph.blocks.len();
        let mut entry_environment_ids = vec![None; block_count];
        let mut exit_environment_ids = vec![None; block_count];

        // start from the entry block
        entry_environment_ids[graph.entry_block.0 as usize] = Some(base_environment_id);

        // propagate environments
        let mut worklist = VecDeque::new();
        worklist.push_back(graph.entry_block);
        while let Some(block_id) = worklist.pop_front() {
            let block_index = block_id.0 as usize;

            // begin with the entry environment
            let Some(entry_environment_id) = entry_environment_ids[block_index] else {
                continue;
            };
            let entry_environment = flow
                .environment(entry_environment_id)
                .cloned()
                .unwrap_or_else(|| FlowEnvironment::new(false));

            // compute the exit environment for the block
            let mut exit_environment = self.flow_environment_after_block_nodes(
                &mut ctx.reborrow(),
                &graph.blocks[block_index],
                &entry_environment,
                context,
            )?;
            if graph.blocks[block_index].is_terminal {
                exit_environment.is_reachable = false;
            }
            let exit_environment_id = if let Some(existing_id) = exit_environment_ids[block_index] {
                let existing_environment = flow
                    .environment(existing_id)
                    .cloned()
                    .unwrap_or_else(|| FlowEnvironment::new(false));
                if self.flow_environment_equals(&existing_environment, &exit_environment, ctx.types)
                {
                    existing_id
                } else {
                    flow.push_environment(exit_environment.clone())
                }
            } else {
                flow.push_environment(exit_environment.clone())
            };
            exit_environment_ids[block_index] = Some(exit_environment_id);

            // propagate environments over outgoing edges
            for edge in &graph.blocks[block_index].successors {
                let edge_environment = self.flow_environment_for_edge(
                    &mut ctx.reborrow(),
                    edge,
                    &exit_environment,
                    context,
                )?;
                let target_index = edge.target.0 as usize;

                // merge with any existing entry environment for the target
                let next_environment_id =
                    if let Some(existing_id) = entry_environment_ids[target_index] {
                        let existing_environment = flow
                            .environment(existing_id)
                            .cloned()
                            .unwrap_or_else(|| FlowEnvironment::new(false));
                        let merged_environment = self.merge_flow_environments(
                            &mut ctx.reborrow(),
                            &existing_environment,
                            &edge_environment,
                            Some(&existing_environment),
                        );

                        if self.flow_environment_equals(
                            &existing_environment,
                            &merged_environment,
                            ctx.types,
                        ) {
                            existing_id
                        } else {
                            flow.push_environment(merged_environment)
                        }
                    } else {
                        flow.push_environment(edge_environment)
                    };

                // enqueue the target when its entry environment changes
                if entry_environment_ids[target_index] != Some(next_environment_id) {
                    entry_environment_ids[target_index] = Some(next_environment_id);
                    worklist.push_back(edge.target);
                }
            }
        }

        // fill missing environments with a shared unreachable snapshot
        let unreachable_environment_id = flow.push_environment(FlowEnvironment::new(false));
        flow.entry_environment_by_block = entry_environment_ids
            .into_iter()
            .map(|environment_id| environment_id.unwrap_or(unreachable_environment_id))
            .collect();
        flow.exit_environment_by_block = exit_environment_ids
            .into_iter()
            .map(|environment_id| environment_id.unwrap_or(unreachable_environment_id))
            .collect();

        // map node environments using the finalized entry environment for each block
        for block in &graph.blocks {
            let entry_environment_id = flow.entry_environment_by_block[block.id.0 as usize];
            let entry_environment = flow
                .environment(entry_environment_id)
                .cloned()
                .unwrap_or_else(|| FlowEnvironment::new(false));
            self.map_flow_environment_for_block_nodes(
                &mut ctx.reborrow(),
                block,
                &entry_environment,
                context,
                &mut flow,
            )?;
        }

        // emit unreachable diagnostics when enabled
        if !context.options.allow_unreachable_code {
            self.report_unreachable_blocks(ctx, graph, &flow);
        }

        Ok(flow)
    }

    /// Compute the exit environment for a block with assertion-aware narrowing.
    fn flow_environment_after_block_nodes(
        &self,
        ctx: &mut TypeContext<'_>,
        block: &FlowBlock,
        environment: &FlowEnvironment,
        context: &InferState,
    ) -> AnalyzeResult<FlowEnvironment> {
        // keep unreachable environments unchanged
        if !environment.is_reachable {
            return Ok(environment.clone());
        }

        // apply assertion call narrowings in order
        let mut current_environment = environment.clone();
        for node_id in &block.nodes {
            if let Some(updated) = self.flow_environment_after_node(
                &mut ctx.reborrow(),
                *node_id,
                &current_environment,
                context,
            )? {
                current_environment = updated;
            }
        }

        Ok(current_environment)
    }

    /// Record per node environments for a block with assertion-aware narrowing.
    fn map_flow_environment_for_block_nodes(
        &self,
        ctx: &mut TypeContext<'_>,
        block: &FlowBlock,
        environment: &FlowEnvironment,
        context: &InferState,
        flow: &mut FlowTable,
    ) -> AnalyzeResult<()> {
        // seed the environment for the first node
        let mut current_environment = environment.clone();
        let mut current_environment_id = flow.push_environment(current_environment.clone());

        // walk nodes and apply assertion call updates
        for node_id in &block.nodes {
            let node_global = (*node_id).into_global(ctx.module.id);
            flow.environment_by_node
                .insert(node_global, current_environment_id);

            if let Some(updated) = self.flow_environment_after_node(
                &mut ctx.reborrow(),
                *node_id,
                &current_environment,
                context,
            )? && !self.flow_environment_equals(&current_environment, &updated, ctx.types)
            {
                current_environment = updated;
                current_environment_id = flow.push_environment(current_environment.clone());
            }
        }

        Ok(())
    }

    /// Apply assertion call narrowings for a single node when applicable.
    fn flow_environment_after_node(
        &self,
        ctx: &mut TypeContext<'_>,
        node_id: LocalNodeIdAny,
        environment: &FlowEnvironment,
        context: &InferState,
    ) -> AnalyzeResult<Option<FlowEnvironment>> {
        // only expressions can update flow state
        if node_id.ty != NodeType::Expression {
            return Ok(None);
        }

        // only apply assertion call narrowings for block expressions
        let parent = ctx.tree.get_parent(node_id.id);
        let is_block_expression = parent
            .map(|parent| parent.ty == NodeType::Block)
            .unwrap_or(true);
        if !is_block_expression {
            return Ok(None);
        }

        let mut expression_id = node_id.into_typed::<Expression>();
        while let Expression::Parenthesized { expression } = ctx.tree.get(expression_id) {
            expression_id = *expression;
        }

        let expression = ctx.tree.get(expression_id);
        let Expression::Call {
            left,
            dynamic_arguments,
            ..
        } = expression
        else {
            return Ok(None);
        };

        self.narrow_environment_for_assertion_call(
            &mut ctx.reborrow(),
            expression_id,
            *left,
            dynamic_arguments,
            environment,
            context,
        )
    }

    /// Apply assertion narrowing for a call expression when possible.
    fn narrow_environment_for_assertion_call(
        &self,
        ctx: &mut TypeContext<'_>,
        call_id: LocalNodeId<Expression>,
        callee_id: LocalNodeId<Expression>,
        dynamic_arguments: &[LocalNodeId<Argument>],
        environment: &FlowEnvironment,
        context: &InferState,
    ) -> AnalyzeResult<Option<FlowEnvironment>> {
        // skip when custom guards are disabled
        if context.options.no_custom_type_guards {
            return Ok(None);
        }

        // resolve the callee symbol
        let callee_id = self.unwrap_parenthesized_expression(callee_id, ctx.tree);
        let callee_symbol = self.reference_symbol_for_expression(ctx.tree_symbol_view(), callee_id);
        let Some(callee_symbol) = callee_symbol else {
            return Ok(None);
        };

        // skip remote signatures until we can translate predicate types across modules
        if callee_symbol.module_id != ctx.module.id {
            return Ok(None);
        }

        // resolve the guard signature for parameter mapping
        let signature = self.guard_signature_for_symbol(ctx, callee_symbol);
        let Some(signature) = signature else {
            return Ok(None);
        };

        // resolve the predicate return type for the call
        let return_type_id = if let Some(return_type) = signature.return_type {
            Some(self.resolve_declared_type_expression(
                &mut ctx.reborrow(),
                return_type,
                true,
                true,
            )?)
        } else {
            None
        };
        let Some(return_type_id) = return_type_id else {
            return Ok(None);
        };
        let Type::Predicate {
            asserts,
            subject,
            target,
        } = ctx.types.get_type(return_type_id)
        else {
            return Ok(None);
        };
        if !*asserts {
            return Ok(None);
        }
        let Some(target_type_id) = *target else {
            return Ok(None);
        };
        let target_type_id = self.unwrap_type_value(target_type_id, ctx.types);

        // find the argument expression for the asserted subject
        let parameter = self.guard_parameter_for_subject(ctx, *subject, &signature);
        let Some((parameter_index, parameter_name)) = parameter else {
            return Ok(None);
        };
        let argument_value = self.guard_argument_for_parameter(
            ctx,
            parameter_index,
            parameter_name,
            dynamic_arguments,
        );
        let Some(argument_value) = argument_value else {
            return Ok(None);
        };
        let argument_value = self.unwrap_parenthesized_expression(argument_value, ctx.tree);
        let argument_symbol =
            self.reference_symbol_for_expression(ctx.tree_symbol_view(), argument_value);
        let Some(argument_symbol) = argument_symbol else {
            return Ok(None);
        };

        // resolve the base type for the symbol
        let base_type_id = self.symbol_type_for_guard(
            &mut ctx.reborrow(),
            call_id,
            argument_symbol,
            environment,
            context,
        )?;

        // compute runtime check kind for guard validity
        let runtime_check_kind =
            self.runtime_check_kind_for_relation(&mut ctx.reborrow(), base_type_id, target_type_id);
        if let Some(kind) = runtime_check_kind {
            ctx.types
                .set_runtime_check_kind(call_id.into_global_any(ctx.module.id), kind);
        }

        // compute the asserted type
        let (true_type_id, _) =
            self.type_guard_types(&mut ctx.reborrow(), base_type_id, target_type_id);
        let Some(true_type_id) = true_type_id else {
            return Ok(None);
        };

        // apply the narrowing
        let mut updated_environment = environment.clone();
        updated_environment
            .bindings
            .insert(argument_symbol, true_type_id);
        Ok(Some(updated_environment))
    }

    /// Emit diagnostics for unreachable blocks when enabled.
    fn report_unreachable_blocks(
        &self,
        ctx: &TypeContext<'_>,
        graph: &FlowGraph,
        flow: &FlowTable,
    ) {
        for block in &graph.blocks {
            // skip the entry block
            if block.id == graph.entry_block {
                continue;
            }

            // skip empty blocks
            let Some(node_id) = block.nodes.first() else {
                continue;
            };
            let Some(environment_id) = flow.entry_environment_for_block(block.id) else {
                continue;
            };
            let Some(environment) = flow.environment(environment_id) else {
                continue;
            };
            if environment.is_reachable {
                continue;
            }

            self.error(AnalyzeError::UnreachableCode {
                node: node_id
                    .into_global(ctx.module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }
    }

    /// Compute a flow environment for a control flow edge.
    fn flow_environment_for_edge(
        &self,
        ctx: &mut TypeContext<'_>,
        edge: &FlowEdge,
        environment: &FlowEnvironment,
        context: &InferState,
    ) -> AnalyzeResult<FlowEnvironment> {
        // skip guard evaluation for unreachable environments
        if !environment.is_reachable {
            return Ok(environment.clone());
        }

        // return unchanged when there is no guard expression
        let Some(guard) = edge.guard else {
            return Ok(environment.clone());
        };

        match edge.kind {
            FlowEdgeKind::True => {
                let (true_environment, _) = self.narrow_environment_for_guard(
                    &mut ctx.reborrow(),
                    guard,
                    environment,
                    context,
                )?;
                Ok(true_environment)
            }
            FlowEdgeKind::False => {
                let (_, false_environment) = self.narrow_environment_for_guard(
                    &mut ctx.reborrow(),
                    guard,
                    environment,
                    context,
                )?;
                Ok(false_environment)
            }
            FlowEdgeKind::Case | FlowEdgeKind::Guard => {
                let (guard_environment, _) = self.narrow_environment_for_guard(
                    &mut ctx.reborrow(),
                    guard,
                    environment,
                    context,
                )?;
                Ok(guard_environment)
            }
            FlowEdgeKind::Unconditional => Ok(environment.clone()),
        }
    }

    /// Merge two flow environments into a single environment.
    fn merge_flow_environments(
        &self,
        ctx: &mut TypeContext<'_>,
        left: &FlowEnvironment,
        right: &FlowEnvironment,
        baseline: Option<&FlowEnvironment>,
    ) -> FlowEnvironment {
        // flow merges currently union differing types
        // preserve reachability when one side is unreachable
        if !left.is_reachable {
            return right.clone();
        }
        if !right.is_reachable {
            return left.clone();
        }

        // collect all symbols mentioned by either branch or the baseline
        let mut bindings = IndexMap::new();
        let mut symbols = IndexMap::new();
        for symbol in left.bindings.keys() {
            symbols.insert(*symbol, ());
        }
        for symbol in right.bindings.keys() {
            symbols.insert(*symbol, ());
        }
        if let Some(baseline) = baseline {
            for symbol in baseline.bindings.keys() {
                symbols.insert(*symbol, ());
            }
        }

        // merge symbol types, falling back to the baseline type when missing
        for symbol in symbols.keys() {
            let baseline_type_id = baseline
                .and_then(|environment| environment.bindings.get(symbol).copied())
                .or_else(|| ctx.types.get_value_type_id(*symbol));
            let left_type_id = left.bindings.get(symbol).copied().or(baseline_type_id);
            let right_type_id = right.bindings.get(symbol).copied().or(baseline_type_id);

            let (Some(left_type_id), Some(right_type_id)) = (left_type_id, right_type_id) else {
                continue;
            };

            // keep the type when both branches agree
            if left_type_id == right_type_id {
                bindings.insert(*symbol, left_type_id);
                continue;
            }

            // build a union for differing branch types
            let merged_type_id = self.merge_flow_types(
                &mut ctx.reborrow(),
                left_type_id,
                right_type_id,
                baseline_type_id,
            );
            bindings.insert(*symbol, merged_type_id);
        }

        FlowEnvironment {
            bindings,
            is_reachable: true,
        }
    }

    /// Merge two type ids for flow environments, preserving baseline ordering when possible.
    fn merge_flow_types(
        &self,
        ctx: &mut TypeContext<'_>,
        left_type_id: LocalTypeId,
        right_type_id: LocalTypeId,
        baseline_type_id: Option<LocalTypeId>,
    ) -> LocalTypeId {
        // return early when types already match
        if left_type_id == right_type_id {
            return left_type_id;
        }

        // reuse one existing type id when the types already match semantically
        if are_types_equal(left_type_id, right_type_id, ctx.types) {
            if let Some(baseline_type_id) = baseline_type_id
                && are_types_equal(left_type_id, baseline_type_id, ctx.types)
            {
                return baseline_type_id;
            }

            return left_type_id;
        }

        // collect and deduplicate union elements
        let mut elements = Vec::new();
        self.append_flow_union_elements(left_type_id, &mut elements, ctx.types);
        self.append_flow_union_elements(right_type_id, &mut elements, ctx.types);

        // keep baseline ordering for stable diagnostics
        if let Some(baseline_type_id) = baseline_type_id
            && let Type::Union {
                elements: baseline_elements,
            } = ctx.types.get_type(baseline_type_id)
        {
            let mut ordered_elements = Vec::with_capacity(elements.len());
            for element_id in baseline_elements {
                if elements
                    .iter()
                    .any(|existing| are_types_equal(*existing, *element_id, ctx.types))
                {
                    self.push_flow_union_element(*element_id, &mut ordered_elements, ctx.types);
                }
            }
            for element_id in elements {
                self.push_flow_union_element(element_id, &mut ordered_elements, ctx.types);
            }

            if self.flow_union_elements_match_type(&ordered_elements, baseline_type_id, ctx.types) {
                return baseline_type_id;
            }

            if self.flow_union_elements_match_type(&ordered_elements, left_type_id, ctx.types) {
                return left_type_id;
            }

            if self.flow_union_elements_match_type(&ordered_elements, right_type_id, ctx.types) {
                return right_type_id;
            }

            return self.finish_flow_union_elements(ordered_elements, left_type_id, ctx.types);
        }

        if self.flow_union_elements_match_type(&elements, left_type_id, ctx.types) {
            return left_type_id;
        }

        if self.flow_union_elements_match_type(&elements, right_type_id, ctx.types) {
            return right_type_id;
        }

        self.finish_flow_union_elements(elements, left_type_id, ctx.types)
    }

    /// Append union elements for a type id, avoiding duplicates.
    fn append_flow_union_elements(
        &self,
        type_id: LocalTypeId,
        elements: &mut Vec<LocalTypeId>,
        types: &TypeTable,
    ) {
        match types.get_type(type_id) {
            Type::Union { elements: union } => {
                // flatten union elements into the merged list
                for element_id in union {
                    self.push_flow_union_element(*element_id, elements, types);
                }
            }
            _ => {
                // append non union types when missing
                self.push_flow_union_element(type_id, elements, types);
            }
        }
    }

    /// Append one flow union element when it is not already present semantically.
    fn push_flow_union_element(
        &self,
        type_id: LocalTypeId,
        elements: &mut Vec<LocalTypeId>,
        types: &TypeTable,
    ) {
        if elements
            .iter()
            .any(|existing| are_types_equal(*existing, type_id, types))
        {
            return;
        }

        elements.push(type_id);
    }

    /// Finalize union elements into a type id.
    fn finish_flow_union_elements(
        &self,
        elements: Vec<LocalTypeId>,
        source_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        self.union_type_from_list(elements, source_type_id, types)
    }

    /// Return whether one merged flow union matches one existing type exactly enough to reuse it.
    fn flow_union_elements_match_type(
        &self,
        elements: &[LocalTypeId],
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        match types.get_type(type_id) {
            Type::Union {
                elements: candidate_elements,
            } => {
                candidate_elements.len() == elements.len()
                    && candidate_elements.iter().all(|candidate_id| {
                        elements
                            .iter()
                            .any(|element_id| are_types_equal(*element_id, *candidate_id, types))
                    })
            }
            _ => elements.len() == 1 && are_types_equal(elements[0], type_id, types),
        }
    }

    /// Compare two flow environments for equality.
    fn flow_environment_equals(
        &self,
        left: &FlowEnvironment,
        right: &FlowEnvironment,
        types: &TypeTable,
    ) -> bool {
        left.is_reachable == right.is_reachable
            && left.bindings.len() == right.bindings.len()
            && left.bindings.iter().all(|(symbol, left_type_id)| {
                let Some(right_type_id) = right.bindings.get(symbol).copied() else {
                    return false;
                };

                are_types_equal(*left_type_id, right_type_id, types)
            })
    }

    /// Apply a flow environment to an inference context.
    pub fn apply_flow_environment_to_context(
        &self,
        environment: &FlowEnvironment,
        context: &mut InferState,
    ) {
        context.narrowings = environment
            .bindings
            .iter()
            .map(|(symbol, type_id)| (*symbol, *type_id))
            .collect();
        context.is_unreachable = context.is_unreachable || !environment.is_reachable;
    }

    /// Build a flow environment snapshot from an inference context.
    pub fn flow_environment_from_context(&self, context: &InferState) -> FlowEnvironment {
        // copy context narrowings into a flow environment
        let mut bindings = IndexMap::with_capacity(context.narrowings.len());
        for (symbol, type_id) in &context.narrowings {
            bindings.insert(*symbol, *type_id);
        }

        FlowEnvironment {
            bindings,
            is_reachable: !context.is_unreachable,
        }
    }

    /// Split the environment based on a guard expression.
    fn narrow_environment_for_guard(
        &self,
        ctx: &mut TypeContext<'_>,
        guard: FlowGuard,
        environment: &FlowEnvironment,
        context: &InferState,
    ) -> AnalyzeResult<(FlowEnvironment, FlowEnvironment)> {
        match guard {
            FlowGuard::Expression(guard_id) => self.narrow_environment_for_expression_guard(
                &mut ctx.reborrow(),
                guard_id,
                environment,
                context,
            ),
            FlowGuard::Pattern { value, pattern } => self.narrow_environment_for_pattern_guard(
                &mut ctx.reborrow(),
                value,
                pattern,
                environment,
                context,
            ),
        }
    }

    /// Return unchanged true and false guard environments.
    fn unchanged_guard_environments(
        &self,
        environment: &FlowEnvironment,
    ) -> (FlowEnvironment, FlowEnvironment) {
        (environment.clone(), environment.clone())
    }

    /// Split the environment for one expression guard.
    fn narrow_environment_for_expression_guard(
        &self,
        ctx: &mut TypeContext<'_>,
        guard_id: LocalNodeId<Expression>,
        environment: &FlowEnvironment,
        context: &InferState,
    ) -> AnalyzeResult<(FlowEnvironment, FlowEnvironment)> {
        let guard_id = self.unwrap_parenthesized_expression(guard_id, ctx.tree);

        match ctx.tree.get(guard_id) {
            Expression::Comptime { body } => self.narrow_environment_for_guard(
                &mut ctx.reborrow(),
                FlowGuard::Expression(*body),
                environment,
                context,
            ),
            Expression::Unary {
                operator: UnaryOperator::Not,
                right,
            } => {
                let (true_environment, false_environment) = self.narrow_environment_for_guard(
                    &mut ctx.reborrow(),
                    FlowGuard::Expression(*right),
                    environment,
                    context,
                )?;
                Ok((false_environment, true_environment))
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => self.narrow_environment_for_binary_expression_guard(
                &mut ctx.reborrow(),
                guard_id,
                *left,
                *operator,
                *right,
                environment,
                context,
            ),
            Expression::Is { value, target_type } => self.narrow_environment_for_type_guard(
                &mut ctx.reborrow(),
                guard_id,
                *value,
                *target_type,
                environment,
                context,
            ),
            Expression::InstanceOf { value, target } => {
                if let Some(environments) = self.narrow_environment_for_runtime_type_guard(
                    &mut ctx.reborrow(),
                    guard_id,
                    *value,
                    *target,
                    environment,
                    context,
                )? {
                    return Ok(environments);
                }

                Ok(self.unchanged_guard_environments(environment))
            }
            Expression::Call {
                left,
                dynamic_arguments,
                ..
            } => {
                let _ = (left, dynamic_arguments);
                Ok(self.unchanged_guard_environments(environment))
            }
            _ => Ok(self.unchanged_guard_environments(environment)),
        }
    }

    /// Split the environment for one binary expression guard.
    fn narrow_environment_for_binary_expression_guard(
        &self,
        ctx: &mut TypeContext<'_>,
        guard_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        operator: BinaryOperator,
        right: LocalNodeId<Expression>,
        environment: &FlowEnvironment,
        context: &InferState,
    ) -> AnalyzeResult<(FlowEnvironment, FlowEnvironment)> {
        match operator {
            BinaryOperator::In => {
                if let Some(environments) = self.narrow_environment_for_in_guard(
                    &mut ctx.reborrow(),
                    guard_id,
                    left,
                    right,
                    environment,
                    context,
                )? {
                    return Ok(environments);
                }
                Ok(self.unchanged_guard_environments(environment))
            }
            BinaryOperator::And => {
                let (left_true, left_false) = self.narrow_environment_for_guard(
                    &mut ctx.reborrow(),
                    FlowGuard::Expression(left),
                    environment,
                    context,
                )?;
                let (right_true, right_false) = self.narrow_environment_for_guard(
                    &mut ctx.reborrow(),
                    FlowGuard::Expression(right),
                    &left_true,
                    context,
                )?;
                let false_environment = self.merge_flow_environments(
                    &mut ctx.reborrow(),
                    &left_false,
                    &right_false,
                    Some(environment),
                );
                Ok((right_true, false_environment))
            }
            BinaryOperator::Or => {
                let (left_true, left_false) = self.narrow_environment_for_guard(
                    &mut ctx.reborrow(),
                    FlowGuard::Expression(left),
                    environment,
                    context,
                )?;
                let (right_true, right_false) = self.narrow_environment_for_guard(
                    &mut ctx.reborrow(),
                    FlowGuard::Expression(right),
                    &left_false,
                    context,
                )?;
                let true_environment = self.merge_flow_environments(
                    &mut ctx.reborrow(),
                    &left_true,
                    &right_true,
                    Some(environment),
                );
                Ok((true_environment, right_false))
            }
            BinaryOperator::Equal
            | BinaryOperator::EqualStrict
            | BinaryOperator::NotEqual
            | BinaryOperator::NotEqualStrict => {
                let is_negated = matches!(
                    operator,
                    BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict
                );
                if let Some(environments) = self.narrow_environment_for_typeof_guard(
                    &mut ctx.reborrow(),
                    guard_id,
                    left,
                    right,
                    is_negated,
                    environment,
                    context,
                )? {
                    return Ok(environments);
                }

                let is_strict = matches!(
                    operator,
                    BinaryOperator::EqualStrict | BinaryOperator::NotEqualStrict
                );
                if let Some(environments) = self.narrow_environment_for_nullish_guard(
                    &mut ctx.reborrow(),
                    guard_id,
                    left,
                    right,
                    is_strict,
                    is_negated,
                    environment,
                    context,
                )? {
                    return Ok(environments);
                }

                if let Some(environments) = self.narrow_environment_for_discriminant_guard(
                    &mut ctx.reborrow(),
                    guard_id,
                    left,
                    right,
                    is_negated,
                    environment,
                    context,
                )? {
                    return Ok(environments);
                }

                Ok(self.unchanged_guard_environments(environment))
            }
            _ => Ok(self.unchanged_guard_environments(environment)),
        }
    }

    /// Split the environment for one `value is Type` guard.
    fn narrow_environment_for_type_guard(
        &self,
        ctx: &mut TypeContext<'_>,
        guard_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_type_id: LocalNodeId<TypeExpression>,
        environment: &FlowEnvironment,
        context: &InferState,
    ) -> AnalyzeResult<(FlowEnvironment, FlowEnvironment)> {
        let target_type_id =
            self.resolve_declared_type_expression(&mut ctx.reborrow(), target_type_id, true, true)?;

        if let Some(environments) = self
            .narrow_environment_for_runtime_type_guard_with_target_type(
                &mut ctx.reborrow(),
                guard_id,
                value_id,
                target_type_id,
                environment,
                context,
            )?
        {
            return Ok(environments);
        }

        Ok(self.unchanged_guard_environments(environment))
    }

    /// Get a symbol type from a flow environment.
    pub fn flow_environment_for_symbol(
        &self,
        symbol: GlobalSymbolId,
        environment: &FlowEnvironment,
    ) -> Option<LocalTypeId> {
        environment.bindings.get(&symbol).copied()
    }

    /// Split the environment based on a pattern guard.
    fn narrow_environment_for_pattern_guard(
        &self,
        ctx: &mut TypeContext<'_>,
        value_id: LocalNodeId<Expression>,
        pattern_id: LocalNodeId<Pattern>,
        environment: &FlowEnvironment,
        context: &InferState,
    ) -> AnalyzeResult<(FlowEnvironment, FlowEnvironment)> {
        let value_id = self.unwrap_parenthesized_expression(value_id, ctx.tree);
        let symbol = self.reference_symbol_for_expression(ctx.tree_symbol_view(), value_id);
        let Some(symbol) = symbol else {
            return Ok((environment.clone(), environment.clone()));
        };

        let base_type_id = self.symbol_type_for_guard(
            &mut ctx.reborrow(),
            value_id,
            symbol,
            environment,
            context,
        )?;

        // treat irrefutable patterns as non narrowing guards
        if self.is_irrefutable_pattern_for_type(&mut ctx.reborrow(), pattern_id, base_type_id) {
            return Ok((environment.clone(), environment.clone()));
        }

        // narrow must patterns by stripping nullish values
        if matches!(ctx.tree.get(pattern_id), Pattern::Must(_)) {
            let (nullish_type_id, non_nullish_type_id) =
                self.nullish_guard_types(NullishGuardKind::Nullish, base_type_id, ctx.types);
            let mut true_environment = environment.clone();
            let mut false_environment = environment.clone();
            if let Some(type_id) = non_nullish_type_id {
                true_environment.bindings.insert(symbol, type_id);
            }
            if let Some(type_id) = nullish_type_id {
                false_environment.bindings.insert(symbol, type_id);
            }
            return Ok((true_environment, false_environment));
        }

        let Some(target_type_id) =
            self.pattern_guard_target_type(&mut ctx.reborrow(), pattern_id)?
        else {
            return Ok((environment.clone(), environment.clone()));
        };

        let (true_type_id, false_type_id) =
            self.type_guard_types(&mut ctx.reborrow(), base_type_id, target_type_id);

        let mut true_environment = environment.clone();
        if let Some(type_id) = true_type_id {
            true_environment.bindings.insert(symbol, type_id);
        }
        let mut false_environment = environment.clone();
        if let Some(type_id) = false_type_id {
            false_environment.bindings.insert(symbol, type_id);
        }

        Ok((true_environment, false_environment))
    }

    /// Resolve the target type used for pattern-based guards.
    fn pattern_guard_target_type(
        &self,
        ctx: &mut TypeContext<'_>,
        pattern_id: LocalNodeId<Pattern>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        match ctx.tree.get(pattern_id) {
            Pattern::Expression { value } => {
                if let Some(literal) = self.scalar_literal_for_expression(ctx.tree, *value) {
                    let literal_type = self.infer_scalar_literal(&literal);
                    let type_id = ctx.types.insert_type_from(
                        Type::TypeLiteral {
                            value: literal_type,
                        },
                        pattern_id,
                    );
                    return Ok(Some(type_id));
                }

                let Expression::Type {
                    value: target_expression,
                    ..
                } = ctx.tree.get(*value)
                else {
                    return Ok(None);
                };

                let target_type = self.resolve_declared_type_expression_value(
                    &mut ctx.reborrow(),
                    *target_expression,
                    true,
                    true,
                    true,
                    true,
                    true,
                )?;
                if matches!(target_type, Type::Unevaluated(_)) {
                    return Ok(None);
                }
                let type_id = ctx.types.insert_type_from(target_type, *value);
                Ok(Some(self.unwrap_type_value(type_id, ctx.types)))
            }
            Pattern::TaggedTuple { ty, .. } | Pattern::TaggedObject { ty, .. } => {
                let target_type_id = self.guard_target_type_expression(&mut ctx.reborrow(), *ty)?;
                Ok(Some(self.unwrap_type_value(target_type_id, ctx.types)))
            }
            Pattern::Union { patterns } => {
                let mut target_types = Vec::new();
                for pattern_id in patterns {
                    if let Some(target_type_id) =
                        self.pattern_guard_target_type(&mut ctx.reborrow(), *pattern_id)?
                    {
                        target_types.push(target_type_id);
                    }
                }
                if target_types.is_empty() {
                    return Ok(None);
                }
                let target_type_id = if target_types.len() == 1 {
                    target_types[0]
                } else {
                    ctx.types.insert_type_from(
                        Type::Union {
                            elements: target_types,
                        },
                        pattern_id,
                    )
                };
                Ok(Some(target_type_id))
            }
            Pattern::Object { fields } => {
                let mut type_fields = Vec::new();
                for field_id in fields {
                    let field = ctx.tree.get(*field_id);
                    let (key, pattern) = match field {
                        PatternField::Named { name, pattern, .. } => {
                            (Some(StaticKey::Name(*name)), *pattern)
                        }
                        PatternField::Alias { name, .. } => (Some(StaticKey::Name(*name)), None),
                        PatternField::Computed { key, pattern, .. } => (
                            self.static_key_from_key(
                                ctx.compiler_context.revision(),
                                ctx.profile,
                                ctx.tree,
                                ctx.symbols,
                                ctx.types,
                                Key::Expression(*key),
                            ),
                            *pattern,
                        ),
                        PatternField::Positional { .. }
                        | PatternField::Spread { .. }
                        | PatternField::Elision => (None, None),
                    };
                    let Some(key) = key else {
                        continue;
                    };
                    let field_type_id = if let Some(pattern_id) = pattern {
                        self.pattern_guard_field_type(&mut ctx.reborrow(), pattern_id)?
                    } else {
                        ctx.types.insert_type_from_any(
                            Type::TypeLiteral {
                                value: TypeLiteral::Unknown,
                            },
                            field_id.into_any(),
                        )
                    };
                    type_fields.push(TypeField {
                        key,
                        ty: field_type_id,
                        is_optional: false,
                        is_readonly: false,
                    });
                }
                if type_fields.is_empty() {
                    return Ok(None);
                }
                let target_type = Type::Object {
                    fields: type_fields,
                    call_signatures: Vec::new(),
                    construct_signatures: Vec::new(),
                    index_signatures: Vec::new(),
                };
                Ok(Some(ctx.types.insert_type_from(target_type, pattern_id)))
            }
            _ => Ok(None),
        }
    }

    fn pattern_guard_field_type(
        &self,
        ctx: &mut TypeContext<'_>,
        pattern_id: LocalNodeId<Pattern>,
    ) -> AnalyzeResult<LocalTypeId> {
        if let Some(target_type_id) =
            self.pattern_guard_target_type(&mut ctx.reborrow(), pattern_id)?
        {
            return Ok(target_type_id);
        }
        Ok(ctx.types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            pattern_id.into_any(),
        ))
    }

    /// Build true and false environments for one symbol narrowing.
    fn narrowed_environments_for_symbol(
        &self,
        environment: &FlowEnvironment,
        symbol: GlobalSymbolId,
        true_type_id: Option<LocalTypeId>,
        false_type_id: Option<LocalTypeId>,
        is_negated: bool,
    ) -> (FlowEnvironment, FlowEnvironment) {
        let mut true_environment = environment.clone();
        let mut false_environment = environment.clone();
        if let Some(type_id) = true_type_id {
            true_environment.bindings.insert(symbol, type_id);
        }
        if let Some(type_id) = false_type_id {
            false_environment.bindings.insert(symbol, type_id);
        }
        if is_negated {
            (false_environment, true_environment)
        } else {
            (true_environment, false_environment)
        }
    }

    /// Split the environment based on a nullish equality guard.
    fn narrow_environment_for_nullish_guard(
        &self,
        ctx: &mut TypeContext<'_>,
        guard_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        is_strict: bool,
        is_negated: bool,
        environment: &FlowEnvironment,
        context: &InferState,
    ) -> AnalyzeResult<Option<(FlowEnvironment, FlowEnvironment)>> {
        // normalize the guard expressions for matching
        let left_id = self.unwrap_parenthesized_expression(left_id, ctx.tree);
        let right_id = self.unwrap_parenthesized_expression(right_id, ctx.tree);

        // resolve symbols and literal kinds for both sides
        let left_symbol = self.reference_symbol_for_expression(ctx.tree_symbol_view(), left_id);
        let right_symbol = self.reference_symbol_for_expression(ctx.tree_symbol_view(), right_id);

        let left_literal = self.nullish_literal_kind(ctx.tree, left_id);
        let right_literal = self.nullish_literal_kind(ctx.tree, right_id);

        // select the symbol and literal to narrow
        let (symbol, literal_kind) = match (left_symbol, right_literal, right_symbol, left_literal)
        {
            (Some(symbol), Some(literal_kind), _, _) => (symbol, literal_kind),
            (_, _, Some(symbol), Some(literal_kind)) => (symbol, literal_kind),
            _ => return Ok(None),
        };

        // decide which nullish guard we have
        let guard_kind = match (literal_kind, is_strict) {
            (NullishLiteralKind::Null, true) => NullishGuardKind::Null,
            (NullishLiteralKind::Undefined, true) => NullishGuardKind::Undefined,
            _ => NullishGuardKind::Nullish,
        };

        // resolve the base type to narrow
        let base_type_id = self.symbol_type_for_guard(
            &mut ctx.reborrow(),
            guard_id,
            symbol,
            environment,
            context,
        )?;

        let (true_type_id, false_type_id) =
            self.nullish_guard_types(guard_kind, base_type_id, ctx.types);

        Ok(Some(self.narrowed_environments_for_symbol(
            environment,
            symbol,
            true_type_id,
            false_type_id,
            is_negated,
        )))
    }

    /// Split the environment based on a typeof equality guard.
    fn narrow_environment_for_typeof_guard(
        &self,
        ctx: &mut TypeContext<'_>,
        guard_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        is_negated: bool,
        environment: &FlowEnvironment,
        context: &InferState,
    ) -> AnalyzeResult<Option<(FlowEnvironment, FlowEnvironment)>> {
        // normalize both sides for matching
        let left_id = self.unwrap_parenthesized_expression(left_id, ctx.tree);
        let right_id = self.unwrap_parenthesized_expression(right_id, ctx.tree);

        // identify the typeof expression and the string literal
        let (typeof_id, literal_id) = match (
            self.typeof_expression_id(ctx.tree, left_id),
            self.string_literal_id(ctx.tree, right_id),
            self.typeof_expression_id(ctx.tree, right_id),
            self.string_literal_id(ctx.tree, left_id),
        ) {
            (Some(typeof_id), Some(literal_id), _, _) => (typeof_id, literal_id),
            (_, _, Some(typeof_id), Some(literal_id)) => (typeof_id, literal_id),
            _ => return Ok(None),
        };

        // resolve the guard symbol from the typeof argument
        let typeof_id = self.unwrap_parenthesized_expression(typeof_id, ctx.tree);
        let right_id = match ctx.tree.get(typeof_id) {
            Expression::Unary {
                operator: UnaryOperator::Typeof,
                right,
            } => self.unwrap_parenthesized_expression(*right, ctx.tree),
            _ => return Ok(None),
        };
        let symbol = self.reference_symbol_for_expression(ctx.tree_symbol_view(), right_id);
        let Some(symbol) = symbol else {
            return Ok(None);
        };

        // resolve the typeof guard target
        let Some(target) =
            self.type_guard_target_for_typeof_string(literal_id, typeof_id.into_any(), ctx.types)
        else {
            return Ok(None);
        };

        // resolve the base type for the symbol
        let base_type_id = self.symbol_type_for_guard(
            &mut ctx.reborrow(),
            guard_id,
            symbol,
            environment,
            context,
        )?;

        self.resolve_declared_type(&mut ctx.reborrow(), base_type_id)?;
        let mut union_elements = Vec::new();
        if let Type::Union { elements } = ctx.types.get_type(base_type_id) {
            union_elements.extend(elements.iter().copied());
        }
        for element_id in union_elements {
            self.resolve_declared_type(&mut ctx.reborrow(), element_id)?;
        }

        // compute narrowed types for each branch
        let (true_type_id, false_type_id) =
            self.type_guard_target_types(&mut ctx.reborrow(), base_type_id, target);

        Ok(Some(self.narrowed_environments_for_symbol(
            environment,
            symbol,
            true_type_id,
            false_type_id,
            is_negated,
        )))
    }

    /// Split the environment based on a discriminant equality guard.
    fn narrow_environment_for_discriminant_guard(
        &self,
        ctx: &mut TypeContext<'_>,
        guard_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        is_negated: bool,
        environment: &FlowEnvironment,
        context: &InferState,
    ) -> AnalyzeResult<Option<(FlowEnvironment, FlowEnvironment)>> {
        // normalize both sides for matching
        let left_id = self.unwrap_parenthesized_expression(left_id, ctx.tree);
        let right_id = self.unwrap_parenthesized_expression(right_id, ctx.tree);

        // identify the discriminant access and literal
        let (symbol, key, literal) = match (
            self.discriminant_access_for_expression(&mut ctx.reborrow(), left_id, context),
            self.scalar_literal_for_expression(ctx.tree, right_id),
            self.discriminant_access_for_expression(&mut ctx.reborrow(), right_id, context),
            self.scalar_literal_for_expression(ctx.tree, left_id),
        ) {
            (Some((symbol, key)), Some(literal), _, _) => (symbol, key, literal),
            (_, _, Some((symbol, key)), Some(literal)) => (symbol, key, literal),
            _ => return Ok(None),
        };

        // resolve the base type for the symbol
        let base_type_id = self.symbol_type_for_guard(
            &mut ctx.reborrow(),
            guard_id,
            symbol,
            environment,
            context,
        )?;

        self.resolve_declared_type(&mut ctx.reborrow(), base_type_id)?;
        let mut union_elements = Vec::new();
        if let Type::Union { elements } = ctx.types.get_type(base_type_id) {
            union_elements.extend(elements.iter().copied());
        }
        for element_id in union_elements {
            self.resolve_declared_type(&mut ctx.reborrow(), element_id)?;
        }

        // compute narrowed types for each branch
        let (true_type_id, false_type_id) =
            self.discriminant_guard_types(&mut ctx.reborrow(), base_type_id, &key, literal)?;

        Ok(Some(self.narrowed_environments_for_symbol(
            environment,
            symbol,
            true_type_id,
            false_type_id,
            is_negated,
        )))
    }

    /// Split the environment based on an `x in y` guard.
    fn narrow_environment_for_in_guard(
        &self,
        ctx: &mut TypeContext<'_>,
        guard_id: LocalNodeId<Expression>,
        key_id: LocalNodeId<Expression>,
        target_id: LocalNodeId<Expression>,
        environment: &FlowEnvironment,
        context: &InferState,
    ) -> AnalyzeResult<Option<(FlowEnvironment, FlowEnvironment)>> {
        // normalize the guard expressions
        let key_id = self.unwrap_parenthesized_expression(key_id, ctx.tree);
        let target_id = self.unwrap_parenthesized_expression(target_id, ctx.tree);

        // only narrow for string literal keys
        let Some(key) = self.string_literal_id(ctx.tree, key_id) else {
            return Ok(None);
        };

        // resolve the target symbol
        let symbol = self.reference_symbol_for_expression(ctx.tree_symbol_view(), target_id);
        let Some(symbol) = symbol else {
            return Ok(None);
        };

        // build the key we are guarding on
        let key = StaticKey::Name(key);

        // resolve the base type for the symbol
        let base_type_id = self.symbol_type_for_guard(
            &mut ctx.reborrow(),
            guard_id,
            symbol,
            environment,
            context,
        )?;

        // ensure sound property narrowing when requested
        if context.options.no_unsound_narrowing {
            let is_required = self.type_has_required_property(
                &mut ctx.reborrow(),
                base_type_id,
                &key,
                guard_id.into_any(),
            )?;
            if !is_required {
                return Ok(None);
            }
        }

        // compute narrowed types for each branch
        let (true_type_id, false_type_id) =
            self.property_guard_types(&mut ctx.reborrow(), base_type_id, &key);

        Ok(Some(self.narrowed_environments_for_symbol(
            environment,
            symbol,
            true_type_id,
            false_type_id,
            false,
        )))
    }

    /// Split the environment for one runtime type relation guard.
    fn narrow_environment_for_runtime_type_guard(
        &self,
        ctx: &mut TypeContext<'_>,
        guard_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_id: LocalNodeId<Expression>,
        environment: &FlowEnvironment,
        context: &InferState,
    ) -> AnalyzeResult<Option<(FlowEnvironment, FlowEnvironment)>> {
        // resolve the target type from the guard expression
        let target_id = self.unwrap_parenthesized_expression(target_id, ctx.tree);
        let target_type_id = self.guard_target_type(&mut ctx.reborrow(), target_id)?;
        let target_type_id = self.unwrap_type_value(target_type_id, ctx.types);

        self.narrow_environment_for_runtime_type_guard_with_target_type(
            &mut ctx.reborrow(),
            guard_id,
            value_id,
            target_type_id,
            environment,
            context,
        )
    }

    /// Split the environment for one runtime type guard with a resolved target type.
    fn narrow_environment_for_runtime_type_guard_with_target_type(
        &self,
        ctx: &mut TypeContext<'_>,
        guard_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_type_id: LocalTypeId,
        environment: &FlowEnvironment,
        context: &InferState,
    ) -> AnalyzeResult<Option<(FlowEnvironment, FlowEnvironment)>> {
        // resolve the target symbol
        let value_id = self.unwrap_parenthesized_expression(value_id, ctx.tree);
        let symbol = self.reference_symbol_for_expression(ctx.tree_symbol_view(), value_id);
        let Some(symbol) = symbol else {
            return Ok(None);
        };

        // resolve the base type for the symbol
        let base_type_id = self.symbol_type_for_guard(
            &mut ctx.reborrow(),
            guard_id,
            symbol,
            environment,
            context,
        )?;

        // compute runtime check kind for guard validity
        let runtime_check_kind =
            self.runtime_check_kind_for_relation(&mut ctx.reborrow(), base_type_id, target_type_id);
        if let Some(kind) = runtime_check_kind {
            ctx.types
                .set_runtime_check_kind(guard_id.into_global_any(ctx.module.id), kind);
        }

        // skip unsound narrowing when runtime checks are unavailable
        let is_sound_narrowing = matches!(
            runtime_check_kind,
            Some(RuntimeCheckKind::UnionTag)
                | Some(RuntimeCheckKind::Constant(_))
                | Some(RuntimeCheckKind::TypeDescriptor)
        );
        if context.options.no_unsound_narrowing && !is_sound_narrowing {
            return Ok(None);
        }

        // compute narrowed types for each branch
        let (true_type_id, false_type_id) =
            self.type_guard_types(&mut ctx.reborrow(), base_type_id, target_type_id);

        Ok(Some(self.narrowed_environments_for_symbol(
            environment,
            symbol,
            true_type_id,
            false_type_id,
            false,
        )))
    }

    /// Determine the target type for a guard expression.
    fn guard_target_type(
        &self,
        ctx: &mut TypeContext<'_>,
        target_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<LocalTypeId> {
        // prefer explicit type nodes
        if let Expression::Type { resolved_type, .. } = ctx.tree.get(target_id) {
            return Ok(*resolved_type);
        }

        // otherwise use the inferred value type and unwrap type-as-value wrappers
        let Some(target_type_id) = ctx
            .types
            .get_declared_or_inferred_type_id(target_id.into_global_any(ctx.module.id))
        else {
            return Ok(ctx
                .types
                .insert_type_from_any(Type::Error, target_id.into_any()));
        };

        Ok(ctx.types.unwrap_value_type_id(target_type_id))
    }

    /// Determine the target type for a guard type expression.
    fn guard_target_type_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        target_id: LocalNodeId<TypeExpression>,
    ) -> AnalyzeResult<LocalTypeId> {
        self.resolve_declared_type_expression(&mut ctx.reborrow(), target_id, true, true)
    }

    /// Resolve a guard signature for a callable symbol.
    fn guard_signature_for_symbol(
        &self,
        ctx: &TypeContext<'_>,
        symbol: GlobalSymbolId,
    ) -> Option<FunctionSignature> {
        // remote signatures are not yet supported in flow predicates
        if symbol.module_id != ctx.module.id {
            return None;
        }

        self.guard_signature_for_symbol_in_tree(ctx.tree_symbol_view(), symbol)
    }

    /// Resolve a guard signature for a symbol within a tree.
    fn guard_signature_for_symbol_in_tree(
        &self,
        ctx: TreeSymbolView<'_>,
        symbol: GlobalSymbolId,
    ) -> Option<FunctionSignature> {
        let symbol_entry = ctx.symbols.get_symbol(symbol.local_id);
        let primary_declaration = symbol_entry.primary_declaration?;
        if primary_declaration.local_id.ty != NodeType::Declaration {
            return None;
        }

        let declaration_id = LocalNodeId::<Declaration>::new(primary_declaration.local_id.id);
        let declaration = ctx.tree.get(declaration_id);
        let Declaration::Function(declaration) = declaration else {
            return None;
        };

        Some(declaration.signature.clone())
    }

    /// Find the parameter that matches a guard predicate subject.
    fn guard_parameter_for_subject(
        &self,
        ctx: &TypeContext<'_>,
        subject: PredicateSubject,
        signature: &FunctionSignature,
    ) -> Option<(usize, Option<StringId>)> {
        for (index, parameter_id) in signature.parameters.iter().enumerate() {
            let parameter = ctx.tree.get(*parameter_id);
            let parameter_symbol = parameter.symbol().into_global(ctx.module.id);
            let parameter_name = match parameter {
                Parameter::Named { name, .. } | Parameter::VariadicNamed { name, .. } => {
                    Some(*name)
                }
                Parameter::Pattern { .. }
                | Parameter::VariadicPattern { .. }
                | Parameter::Error { .. } => None,
            };

            match subject {
                PredicateSubject::Symbol(symbol) if symbol == parameter_symbol => {
                    return Some((index, parameter_name));
                }
                PredicateSubject::Unresolved(name)
                    if parameter_name.is_some_and(|parameter_name| parameter_name == name) =>
                {
                    return Some((index, parameter_name));
                }
                PredicateSubject::This => return None,
                _ => {}
            }
        }

        None
    }

    /// Resolve the argument value corresponding to a guard parameter.
    fn guard_argument_for_parameter(
        &self,
        ctx: &TypeContext<'_>,
        parameter_index: usize,
        parameter_name: Option<StringId>,
        arguments: &[LocalNodeId<Argument>],
    ) -> Option<LocalNodeId<Expression>> {
        // prefer named arguments when available
        if let Some(parameter_name) = parameter_name {
            for argument_id in arguments {
                match ctx.tree.get(*argument_id) {
                    Argument::Named { name, value, .. } if name.string() == parameter_name => {
                        return Some(*value);
                    }
                    Argument::Labeled { label, value, .. } if *label == parameter_name => {
                        return Some(*value);
                    }
                    Argument::Error { .. } => {}
                    _ => {}
                }
            }
        }

        // fall back to positional arguments
        let mut positional_index = 0;
        for argument_id in arguments {
            match ctx.tree.get(*argument_id) {
                Argument::Positional { value, .. } => {
                    if positional_index == parameter_index {
                        return Some(*value);
                    }
                    positional_index += 1;
                }
                Argument::Spread { .. } => {
                    if positional_index == parameter_index {
                        return None;
                    }
                    positional_index += 1;
                }
                Argument::Error { value } => {
                    if positional_index == parameter_index {
                        return Some(*value);
                    }
                    positional_index += 1;
                }
                Argument::Named { .. } | Argument::Labeled { .. } => {}
            }
        }

        None
    }

    /// Derive guard types for a symbol based on a target type.
    fn type_guard_types(
        &self,
        ctx: &mut TypeContext<'_>,
        base_type_id: LocalTypeId,
        target_type_id: LocalTypeId,
    ) -> (Option<LocalTypeId>, Option<LocalTypeId>) {
        // handle union and non union cases separately
        // filter union members that satisfy the target guard
        let base_type = ctx.types.get_type(base_type_id).clone();
        let true_type_id = match base_type {
            Type::Union { elements } => {
                let mut matching_elements = Vec::new();

                // collect assignable union members
                for element_id in elements {
                    let is_assignable = self
                        .is_type_assignable(&mut ctx.reborrow(), target_type_id, element_id)
                        .is_assignable();
                    if is_assignable {
                        matching_elements.push(element_id);
                    }
                }

                match matching_elements.len() {
                    0 => None,
                    1 => Some(matching_elements[0]),
                    _ => Some(ctx.types.insert_type_from_type(
                        Type::Union {
                            elements: matching_elements,
                        },
                        base_type_id,
                    )),
                }
            }
            _ => {
                // keep the base type when it is already narrow enough
                let base_is_assignable = self
                    .is_type_assignable(&mut ctx.reborrow(), target_type_id, base_type_id)
                    .is_assignable();
                let target_is_assignable = self
                    .is_type_assignable(&mut ctx.reborrow(), base_type_id, target_type_id)
                    .is_assignable();

                if base_is_assignable {
                    Some(base_type_id)
                } else if target_is_assignable {
                    Some(target_type_id)
                } else {
                    Some(ctx.types.insert_type_from_type(
                        Type::Intersection {
                            elements: vec![base_type_id, target_type_id],
                        },
                        base_type_id,
                    ))
                }
            }
        };

        // drop assignable types for the false branch
        let (false_type_id, _) =
            self.strip_assignable_from_union(&mut ctx.reborrow(), base_type_id, target_type_id);

        (true_type_id, false_type_id)
    }

    /// Derive guard types for a typed guard target.
    fn type_guard_target_types(
        &self,
        ctx: &mut TypeContext<'_>,
        base_type_id: LocalTypeId,
        target: TypeGuardTarget,
    ) -> (Option<LocalTypeId>, Option<LocalTypeId>) {
        // route guard targets to their narrowing strategy
        match target {
            TypeGuardTarget::TypeId(target_type_id) => {
                self.type_guard_types(&mut ctx.reborrow(), base_type_id, target_type_id)
            }
            TypeGuardTarget::ObjectLike => {
                self.predicate_guard_types(base_type_id, &mut ctx.reborrow(), |type_id, state| {
                    self.type_is_object_like(&mut state.reborrow(), type_id)
                })
            }
            TypeGuardTarget::FunctionLike => {
                self.predicate_guard_types(base_type_id, &mut ctx.reborrow(), |type_id, state| {
                    self.type_is_function_like(&mut state.reborrow(), type_id)
                })
            }
        }
    }

    /// Derive guard types for a discriminant equality check.
    fn discriminant_guard_types(
        &self,
        ctx: &mut TypeContext<'_>,
        base_type_id: LocalTypeId,
        key: &StaticKey,
        literal: ScalarLiteral,
    ) -> AnalyzeResult<(Option<LocalTypeId>, Option<LocalTypeId>)> {
        // avoid narrowing any or unknown types
        if self.type_is_semantic_top_like(base_type_id, ctx.types) {
            return Ok((Some(base_type_id), Some(base_type_id)));
        }

        // build a literal type for assignability checks
        let literal_type = self.infer_scalar_literal(&literal);
        let literal_type_id = ctx.types.insert_type_from_type(
            Type::TypeLiteral {
                value: literal_type.clone(),
            },
            base_type_id,
        );

        // split union and non union targets
        let base_type = ctx.types.get_type(base_type_id).clone();
        match base_type {
            Type::Union { elements } => {
                // avoid narrowing unions with any or unknown members
                if elements
                    .iter()
                    .any(|element_id| self.type_is_semantic_top_like(*element_id, ctx.types))
                {
                    return Ok((Some(base_type_id), Some(base_type_id)));
                }

                let mut matching_elements = Vec::new();
                let mut remaining_elements = Vec::new();

                // collect union members based on discriminant compatibility
                for element_id in elements {
                    let field_info =
                        self.type_field_type_for_key(&mut ctx.reborrow(), element_id, key)?;

                    let Some((field_type_id, is_optional)) = field_info else {
                        remaining_elements.push(element_id);
                        continue;
                    };

                    let is_assignable = self
                        .is_type_assignable(&mut ctx.reborrow(), field_type_id, literal_type_id)
                        .is_assignable();

                    if is_assignable {
                        matching_elements.push(element_id);
                    }

                    if !is_assignable {
                        remaining_elements.push(element_id);
                        continue;
                    }

                    let (remaining_literal, _) = self.strip_literal_from_union(
                        field_type_id,
                        ctx.types,
                        literal_type.clone(),
                    );
                    if is_optional || remaining_literal.is_some() {
                        remaining_elements.push(element_id);
                    }
                }

                let true_type_id = match matching_elements.len() {
                    0 => None,
                    1 => Some(matching_elements[0]),
                    _ => Some(ctx.types.insert_type_from_type(
                        Type::Union {
                            elements: matching_elements,
                        },
                        base_type_id,
                    )),
                };
                let false_type_id = match remaining_elements.len() {
                    0 => None,
                    1 => Some(remaining_elements[0]),
                    _ => Some(ctx.types.insert_type_from_type(
                        Type::Union {
                            elements: remaining_elements,
                        },
                        base_type_id,
                    )),
                };

                Ok((true_type_id, false_type_id))
            }
            _ => {
                let Some((field_type_id, is_optional)) =
                    self.type_field_type_for_key(&mut ctx.reborrow(), base_type_id, key)?
                else {
                    return Ok((Some(base_type_id), Some(base_type_id)));
                };

                let is_assignable = self
                    .is_type_assignable(&mut ctx.reborrow(), field_type_id, literal_type_id)
                    .is_assignable();

                if !is_assignable {
                    return Ok((None, Some(base_type_id)));
                }

                let (remaining_literal, _) =
                    self.strip_literal_from_union(field_type_id, ctx.types, literal_type);
                if is_optional || remaining_literal.is_some() {
                    Ok((Some(base_type_id), Some(base_type_id)))
                } else {
                    Ok((Some(base_type_id), None))
                }
            }
        }
    }

    /// Derive guard types using a predicate for the true branch.
    fn predicate_guard_types<F>(
        &self,
        base_type_id: LocalTypeId,
        ctx: &mut TypeContext<'_>,
        predicate: F,
    ) -> (Option<LocalTypeId>, Option<LocalTypeId>)
    where
        F: Fn(LocalTypeId, &mut TypeContext<'_>) -> bool,
    {
        // avoid narrowing any or unknown types
        if self.type_is_semantic_top_like(base_type_id, ctx.types) {
            return (Some(base_type_id), Some(base_type_id));
        }

        // split union and non union targets
        let base_ty = ctx.types.get_type(base_type_id).clone();
        match base_ty {
            Type::Union { elements } => {
                // avoid narrowing unions with any or unknown members
                if elements
                    .iter()
                    .any(|element_id| self.type_is_semantic_top_like(*element_id, ctx.types))
                {
                    return (Some(base_type_id), Some(base_type_id));
                }

                let mut matching_elements = Vec::new();
                let mut remaining_elements = Vec::new();

                // collect union members by predicate
                for element_id in elements {
                    if predicate(element_id, &mut ctx.reborrow()) {
                        matching_elements.push(element_id);
                    } else {
                        remaining_elements.push(element_id);
                    }
                }

                let true_type_id = match matching_elements.len() {
                    0 => None,
                    1 => Some(matching_elements[0]),
                    _ => Some(ctx.types.insert_type_from_type(
                        Type::Union {
                            elements: matching_elements,
                        },
                        base_type_id,
                    )),
                };
                let false_type_id = match remaining_elements.len() {
                    0 => None,
                    1 => Some(remaining_elements[0]),
                    _ => Some(ctx.types.insert_type_from_type(
                        Type::Union {
                            elements: remaining_elements,
                        },
                        base_type_id,
                    )),
                };

                (true_type_id, false_type_id)
            }
            _ => {
                // narrow based on the predicate result
                if predicate(base_type_id, &mut ctx.reborrow()) {
                    (Some(base_type_id), None)
                } else {
                    (None, Some(base_type_id))
                }
            }
        }
    }

    /// Derive guard types for an `in` property check.
    fn property_guard_types(
        &self,
        ctx: &mut TypeContext<'_>,
        base_type_id: LocalTypeId,
        key: &StaticKey,
    ) -> (Option<LocalTypeId>, Option<LocalTypeId>) {
        // split union and non union targets
        let base_ty = ctx.types.get_type(base_type_id).clone();
        match base_ty {
            Type::Union { elements } => {
                let mut matching_elements = Vec::new();
                let mut remaining_elements = Vec::new();

                // collect union members with and without the key
                for element_id in elements {
                    if self.type_has_property(&mut ctx.reborrow(), element_id, key) {
                        matching_elements.push(element_id);
                    } else {
                        remaining_elements.push(element_id);
                    }
                }

                let true_type_id = match matching_elements.len() {
                    0 => None,
                    1 => Some(matching_elements[0]),
                    _ => Some(ctx.types.insert_type_from_type(
                        Type::Union {
                            elements: matching_elements,
                        },
                        base_type_id,
                    )),
                };
                let false_type_id = match remaining_elements.len() {
                    0 => None,
                    1 => Some(remaining_elements[0]),
                    _ => Some(ctx.types.insert_type_from_type(
                        Type::Union {
                            elements: remaining_elements,
                        },
                        base_type_id,
                    )),
                };

                (true_type_id, false_type_id)
            }
            _ => {
                // return the base type on the branch that matches
                if self.type_has_property(&mut ctx.reborrow(), base_type_id, key) {
                    (Some(base_type_id), None)
                } else {
                    (None, Some(base_type_id))
                }
            }
        }
    }

    /// Check whether a property guard is guaranteed by the type.
    fn type_has_required_property(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        key: &StaticKey,
        anchor_node: LocalNodeIdAny,
    ) -> AnalyzeResult<bool> {
        // avoid soundness gaps when index signatures accept the key
        if self.type_has_index_signature(&mut ctx.reborrow(), type_id, key, anchor_node) {
            return Ok(false);
        }

        // read the field and ensure it is required
        let Some((_, is_optional)) =
            self.type_field_type_for_key(&mut ctx.reborrow(), type_id, key)?
        else {
            return Ok(false);
        };

        Ok(!is_optional)
    }

    /// Check if a type provides an index signature for a static key.
    fn type_has_index_signature(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        key: &StaticKey,
        anchor_node: LocalNodeIdAny,
    ) -> bool {
        let mut visited = Vec::new();
        let type_value = ctx.types.get_type(type_id).clone();
        let Some(type_id) = self.resolve_index_signature_value_type_for_key(
            &mut ctx.reborrow(),
            anchor_node,
            &type_value,
            key,
            &mut visited,
        ) else {
            return false;
        };

        !matches!(
            ctx.types.get_type(type_id),
            Type::TypeLiteral {
                value: TypeLiteral::Never
            }
        )
    }

    /// Strip assignable elements from a union for guard negation.
    fn strip_assignable_from_union(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        target_type_id: LocalTypeId,
    ) -> (Option<LocalTypeId>, bool) {
        // split unions from non union types
        let type_value = ctx.types.get_type(type_id).clone();
        match type_value {
            Type::Union { elements } => {
                let mut filtered_elements = Vec::new();
                let mut removed = false;

                // drop assignable elements while tracking removals
                for element_id in elements {
                    let is_assignable = self
                        .is_type_assignable(&mut ctx.reborrow(), target_type_id, element_id)
                        .is_assignable();
                    if is_assignable {
                        removed = true;
                    } else {
                        filtered_elements.push(element_id);
                    }
                }

                // keep the original type when nothing was removed
                if !removed {
                    return (Some(type_id), false);
                }

                // rebuild the union from remaining elements
                let filtered_type_id = match filtered_elements.len() {
                    0 => None,
                    1 => Some(filtered_elements[0]),
                    _ => Some(ctx.types.insert_type_from_type(
                        Type::Union {
                            elements: filtered_elements,
                        },
                        type_id,
                    )),
                };

                (filtered_type_id, true)
            }
            _ => {
                // remove the type when it is assignable to the target
                let is_assignable = self
                    .is_type_assignable(&mut ctx.reborrow(), target_type_id, type_id)
                    .is_assignable();
                if is_assignable {
                    (None, true)
                } else {
                    (Some(type_id), false)
                }
            }
        }
    }

    /// Extract a typeof expression id when present.
    fn typeof_expression_id(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<LocalNodeId<Expression>> {
        match tree.get(expression_id) {
            Expression::Unary {
                operator: UnaryOperator::Typeof,
                ..
            } => Some(expression_id),
            _ => None,
        }
    }

    /// Extract a string literal id when present.
    fn string_literal_id(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<StringId> {
        match tree.get(expression_id) {
            Expression::ScalarLiteral {
                value: ScalarLiteral::String(string_id),
            } => Some(*string_id),
            _ => None,
        }
    }

    /// Extract a scalar literal when present.
    fn scalar_literal_for_expression(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<ScalarLiteral> {
        match tree.get(expression_id) {
            Expression::ScalarLiteral { value } => Some(value.clone()),
            _ => None,
        }
    }

    /// Resolve a discriminant access from an expression.
    fn discriminant_access_for_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        _context: &InferState,
    ) -> Option<(GlobalSymbolId, StaticKey)> {
        match ctx.tree.get(expression_id) {
            Expression::Member { left, name, .. } => {
                let left_id = self.unwrap_parenthesized_expression(*left, ctx.tree);
                let symbol =
                    self.reference_symbol_for_expression(ctx.tree_symbol_view(), left_id)?;
                let name = (*name)?;
                Some((symbol, StaticKey::Name(name)))
            }
            Expression::Index { left, right, .. } => {
                let right_id = right.as_ref()?;
                let right_id = self.unwrap_parenthesized_expression(*right_id, ctx.tree);
                let key = self.static_key_from_key(
                    ctx.compiler_context.revision(),
                    ctx.profile,
                    ctx.tree,
                    ctx.symbols,
                    ctx.types,
                    Key::Expression(right_id),
                )?;
                let left_id = self.unwrap_parenthesized_expression(*left, ctx.tree);
                let symbol =
                    self.reference_symbol_for_expression(ctx.tree_symbol_view(), left_id)?;
                Some((symbol, key))
            }
            _ => None,
        }
    }

    /// Apply nullish guard types to a symbol type.
    fn nullish_guard_types(
        &self,
        guard_kind: NullishGuardKind,
        base_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> (Option<LocalTypeId>, Option<LocalTypeId>) {
        // materialize nullish literal types once
        let null_type_id = types.insert_type_from_type(
            Type::TypeLiteral {
                value: TypeLiteral::Null,
            },
            base_type_id,
        );
        let undefined_type_id = types.insert_type_from_type(
            Type::TypeLiteral {
                value: TypeLiteral::Undefined,
            },
            base_type_id,
        );

        // build branch types based on the guard kind
        match guard_kind {
            NullishGuardKind::Nullish => {
                let nullish_type_id = types.insert_type_from_type(
                    Type::Union {
                        elements: vec![null_type_id, undefined_type_id],
                    },
                    base_type_id,
                );
                let (non_nullish_type_id, _) = self.strip_nullish_from_union(base_type_id, types);
                (Some(nullish_type_id), non_nullish_type_id)
            }
            NullishGuardKind::Null => {
                let (non_null_type_id, _) =
                    self.strip_literal_from_union(base_type_id, types, TypeLiteral::Null);
                (Some(null_type_id), non_null_type_id)
            }
            NullishGuardKind::Undefined => {
                let (non_undefined_type_id, _) =
                    self.strip_literal_from_union(base_type_id, types, TypeLiteral::Undefined);
                (Some(undefined_type_id), non_undefined_type_id)
            }
        }
    }

    /// Strip a literal type from a union.
    fn strip_literal_from_union(
        &self,
        type_id: LocalTypeId,
        types: &mut TypeTable,
        literal: TypeLiteral,
    ) -> (Option<LocalTypeId>, bool) {
        // split unions from single literals
        let type_value = types.get_type(type_id);

        match type_value {
            Type::Union { elements } => {
                let mut filtered_elements = Vec::new();
                let mut removed = false;

                // keep only non matching literal members
                for element_id in elements {
                    let element_type = types.get_type(*element_id);
                    if let Type::TypeLiteral { value } = element_type
                        && *value == literal
                    {
                        removed = true;
                    } else {
                        filtered_elements.push(*element_id);
                    }
                }

                // keep the original type when nothing was removed
                if !removed {
                    return (Some(type_id), false);
                }

                // rebuild the union from the remaining elements
                let filtered_type_id = match filtered_elements.len() {
                    0 => None,
                    1 => Some(filtered_elements[0]),
                    _ => Some(types.insert_type_from_type(
                        Type::Union {
                            elements: filtered_elements,
                        },
                        type_id,
                    )),
                };

                (filtered_type_id, true)
            }
            Type::TypeLiteral { value } if *value == literal => (None, true),
            _ => (Some(type_id), false),
        }
    }

    /// Resolve the type for a guard symbol.
    fn symbol_type_for_guard(
        &self,
        ctx: &mut TypeContext<'_>,
        guard_id: LocalNodeId<Expression>,
        symbol: GlobalSymbolId,
        environment: &FlowEnvironment,
        _context: &InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // reuse an environment narrowing when present
        let resolved_type = if let Some(type_id) = environment.bindings.get(&symbol) {
            self.unwrap_type_alias_reference(&mut ctx.reborrow(), *type_id)?
        }
        // fall back to any known value type
        else if let Some(type_id) = ctx.types.get_value_type_id(symbol) {
            self.unwrap_type_alias_reference(&mut ctx.reborrow(), type_id)?
        }
        // resolve remote symbol types through the compiler
        else if symbol.module_id != ctx.module.id {
            // resolve remote symbol types through the compiler
            self.resolve_remote_symbol_value_type(
                &mut ctx.reborrow(),
                guard_id.into_any(),
                symbol,
                RemoteValueTypeReadDomain::Interface,
            )?
        } else if let Some(declarator_id) =
            self.direct_binding_declarator_for_symbol(ctx.tree_symbol_view(), symbol)
        {
            let declarator_node_id = declarator_id.into_global_any(ctx.module.id);
            if let Some(type_id) = ctx.types.get_declared_type_id(declarator_node_id) {
                self.unwrap_type_alias_reference(&mut ctx.reborrow(), type_id)?
            } else {
                ctx.types.insert_type_from(
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    },
                    guard_id,
                )
            }
        } else if let Some(primary_declaration) =
            ctx.symbols.get_symbol(symbol.into()).primary_declaration
        {
            if let Some(type_id) = ctx.types.get_declared_type_id(primary_declaration) {
                self.unwrap_type_alias_reference(&mut ctx.reborrow(), type_id)?
            } else {
                ctx.types.insert_type_from(
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    },
                    guard_id,
                )
            }
        } else {
            ctx.types.insert_type_from(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                guard_id,
            )
        };

        Ok(self.normalize_type(&mut ctx.reborrow(), resolved_type, NormalizationMode::Flow))
    }

    /// Extract a nullish literal kind from an expression.
    fn nullish_literal_kind(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<NullishLiteralKind> {
        match tree.get(expression_id) {
            Expression::TypeLiteral {
                value: TypeLiteral::Null,
            } => Some(NullishLiteralKind::Null),
            Expression::TypeLiteral {
                value: TypeLiteral::Undefined,
            } => Some(NullishLiteralKind::Undefined),
            _ => None,
        }
    }
}

/// Describe the nullish guard literal kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NullishLiteralKind {
    /// Represent a null literal.
    Null,
    /// Represent an undefined literal.
    Undefined,
}

/// Describe the nullish guard kind for narrowing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NullishGuardKind {
    /// Represent a null or undefined guard.
    Nullish,
    /// Represent a null only guard.
    Null,
    /// Represent an undefined only guard.
    Undefined,
}

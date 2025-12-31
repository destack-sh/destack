use std::collections::VecDeque;

use indexmap::IndexMap;

use destack_base::StringId;
use destack_dir::{
    BinaryOperator, Expression, FlowEdge, FlowEdgeKind, FlowEnvironment, FlowGraph, FlowTable,
    GlobalSymbolId, InferTable, LocalNodeId, LocalTypeId, NodeTree, ScalarLiteral, StaticKey,
    SymbolTable, SymbolType, Type, TypeBinaryOperator, TypeLiteral, TypeTable, TypeUnaryOperator,
    UnaryOperator,
};
use destack_workspace::{Module, ProfileId};

use super::super::common::NormalizationMode;
use super::r#type::TypeGuardTarget;

use crate::{AnalyzeOptions, AnalyzeResult, AnalyzeWarning, Compiler, InferContext};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Compute a flow table for a control flow graph.
    pub fn compute_flow_table_for_graph(
        &self,
        module: &Module,
        graph: &FlowGraph,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        _infer: &mut InferTable,
        context: &InferContext,
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
            let mut exit_environment = entry_environment.clone();
            if graph.blocks[block_index].is_terminal {
                exit_environment.is_reachable = false;
            }
            let exit_environment_id = if let Some(existing_id) = exit_environment_ids[block_index] {
                let existing_environment = flow
                    .environment(existing_id)
                    .cloned()
                    .unwrap_or_else(|| FlowEnvironment::new(false));
                if self.flow_environment_equals(&existing_environment, &exit_environment) {
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
                    module,
                    edge,
                    tree,
                    symbols,
                    types,
                    &exit_environment,
                    context,
                )?;
                let target_index = edge.target.0 as usize;

                // merge with any existing entry environment for the target
                let next_environment_id = if let Some(existing_id) =
                    entry_environment_ids[target_index]
                {
                    let existing_environment = flow
                        .environment(existing_id)
                        .cloned()
                        .unwrap_or_else(|| FlowEnvironment::new(false));
                    let merged_environment = self.merge_flow_environments(
                        &existing_environment,
                        &edge_environment,
                        Some(&existing_environment),
                        types,
                    );

                    if self.flow_environment_equals(&existing_environment, &merged_environment) {
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
            for node_id in &block.nodes {
                flow.environment_by_node
                    .insert(node_id.into_global(module.id), entry_environment_id);
            }
        }

        // emit unreachable warnings when enabled
        if !context.options.allow_unreachable_code {
            self.warn_unreachable_blocks(module, graph, &flow);
        }

        Ok(flow)
    }

    /// Emit warnings for unreachable blocks when diagnostics are enabled.
    fn warn_unreachable_blocks(&self, module: &Module, graph: &FlowGraph, flow: &FlowTable) {
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

            self.warning(AnalyzeWarning::UnreachableCode {
                node: node_id.into_global(module.id),
            });
        }
    }

    /// Compute a flow environment for a control flow edge.
    fn flow_environment_for_edge(
        &self,
        module: &Module,
        edge: &FlowEdge,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        environment: &FlowEnvironment,
        context: &InferContext,
    ) -> AnalyzeResult<FlowEnvironment> {
        // skip guard evaluation for unreachable environments
        if !environment.is_reachable {
            return Ok(environment.clone());
        }

        // return unchanged when there is no guard expression
        let Some(guard_id) = edge.guard else {
            return Ok(environment.clone());
        };

        match edge.kind {
            FlowEdgeKind::True => {
                let (true_environment, _) = self.narrow_environment_for_guard(
                    module,
                    guard_id,
                    tree,
                    symbols,
                    types,
                    environment,
                    context,
                )?;
                Ok(true_environment)
            }
            FlowEdgeKind::False => {
                let (_, false_environment) = self.narrow_environment_for_guard(
                    module,
                    guard_id,
                    tree,
                    symbols,
                    types,
                    environment,
                    context,
                )?;
                Ok(false_environment)
            }
            FlowEdgeKind::MatchCase | FlowEdgeKind::Guard => {
                let (guard_environment, _) = self.narrow_environment_for_guard(
                    module,
                    guard_id,
                    tree,
                    symbols,
                    types,
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
        left: &FlowEnvironment,
        right: &FlowEnvironment,
        baseline: Option<&FlowEnvironment>,
        types: &mut TypeTable,
    ) -> FlowEnvironment {
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
                .or_else(|| types.get_value_type_id(*symbol));
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
            let merged_type_id =
                self.merge_flow_types(left_type_id, right_type_id, baseline_type_id, types);
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
        left_type_id: LocalTypeId,
        right_type_id: LocalTypeId,
        baseline_type_id: Option<LocalTypeId>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // return early when types already match
        if left_type_id == right_type_id {
            return left_type_id;
        }

        // collect and deduplicate union elements
        let mut elements = Vec::new();
        self.append_flow_union_elements(left_type_id, &mut elements, types);
        self.append_flow_union_elements(right_type_id, &mut elements, types);

        // keep baseline ordering for stable diagnostics
        if let Some(baseline_type_id) = baseline_type_id
            && let Type::Union {
                elements: baseline_elements,
            } = types.get_type(baseline_type_id)
        {
            let mut ordered_elements = Vec::with_capacity(elements.len());
            for element_id in baseline_elements {
                if elements.contains(element_id) {
                    ordered_elements.push(*element_id);
                }
            }
            for element_id in &elements {
                if !ordered_elements.contains(element_id) {
                    ordered_elements.push(*element_id);
                }
            }
            return self.finish_flow_union_elements(ordered_elements, types);
        }

        self.finish_flow_union_elements(elements, types)
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
                    if !elements.contains(element_id) {
                        elements.push(*element_id);
                    }
                }
            }
            _ => {
                // append non union types when missing
                if !elements.contains(&type_id) {
                    elements.push(type_id);
                }
            }
        }
    }

    /// Finalize union elements into a type id.
    fn finish_flow_union_elements(
        &self,
        elements: Vec<LocalTypeId>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // reuse a single element union when possible
        if elements.len() == 1 {
            elements[0]
        } else {
            types.insert_type(Type::Union { elements })
        }
    }

    /// Compare two flow environments for equality.
    fn flow_environment_equals(&self, left: &FlowEnvironment, right: &FlowEnvironment) -> bool {
        left.is_reachable == right.is_reachable && left.bindings == right.bindings
    }

    /// Apply a flow environment to an inference context.
    pub fn apply_flow_environment_to_context(
        &self,
        environment: &FlowEnvironment,
        context: &mut InferContext,
    ) {
        context.narrowings = environment
            .bindings
            .iter()
            .map(|(symbol, type_id)| (*symbol, *type_id))
            .collect();
        context.is_unreachable = context.is_unreachable || !environment.is_reachable;
    }

    /// Build a flow environment snapshot from an inference context.
    pub fn flow_environment_from_context(&self, context: &InferContext) -> FlowEnvironment {
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
    pub fn narrow_environment_for_guard(
        &self,
        module: &Module,
        guard_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        environment: &FlowEnvironment,
        context: &InferContext,
    ) -> AnalyzeResult<(FlowEnvironment, FlowEnvironment)> {
        // apply narrowing for recognized guard shapes
        let guard_id = self.unwrap_parenthesized_expression(guard_id, tree);

        match tree.get(guard_id) {
            Expression::Unary {
                operator: UnaryOperator::Not,
                right,
            } => {
                let (true_environment, false_environment) = self.narrow_environment_for_guard(
                    module,
                    *right,
                    tree,
                    symbols,
                    types,
                    environment,
                    context,
                )?;
                Ok((false_environment, true_environment))
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => match operator {
                BinaryOperator::InstanceOf => {
                    // narrow using class identity guard
                    if let Some(environments) = self.narrow_environment_for_type_guard(
                        module,
                        guard_id,
                        *left,
                        *right,
                        tree,
                        symbols,
                        types,
                        environment,
                        context,
                    )? {
                        return Ok(environments);
                    }
                    Ok((environment.clone(), environment.clone()))
                }
                BinaryOperator::In => {
                    // narrow using a key in guard
                    if let Some(environments) = self.narrow_environment_for_in_guard(
                        module,
                        guard_id,
                        *left,
                        *right,
                        tree,
                        symbols,
                        types,
                        environment,
                        context,
                    )? {
                        return Ok(environments);
                    }
                    Ok((environment.clone(), environment.clone()))
                }
                BinaryOperator::And => {
                    // evaluate the left guard first
                    let (left_true, left_false) = self.narrow_environment_for_guard(
                        module,
                        *left,
                        tree,
                        symbols,
                        types,
                        environment,
                        context,
                    )?;
                    // evaluate the right guard using the true environment
                    let (right_true, right_false) = self.narrow_environment_for_guard(
                        module, *right, tree, symbols, types, &left_true, context,
                    )?;
                    // merge false branches from either guard
                    let false_environment = self.merge_flow_environments(
                        &left_false,
                        &right_false,
                        Some(environment),
                        types,
                    );
                    Ok((right_true, false_environment))
                }
                BinaryOperator::Or => {
                    // evaluate the left guard first
                    let (left_true, left_false) = self.narrow_environment_for_guard(
                        module,
                        *left,
                        tree,
                        symbols,
                        types,
                        environment,
                        context,
                    )?;
                    // evaluate the right guard using the false environment
                    let (right_true, right_false) = self.narrow_environment_for_guard(
                        module,
                        *right,
                        tree,
                        symbols,
                        types,
                        &left_false,
                        context,
                    )?;
                    // merge true branches from either guard
                    let true_environment = self.merge_flow_environments(
                        &left_true,
                        &right_true,
                        Some(environment),
                        types,
                    );
                    Ok((true_environment, right_false))
                }
                BinaryOperator::Equal
                | BinaryOperator::EqualStrict
                | BinaryOperator::NotEqual
                | BinaryOperator::NotEqualStrict => {
                    // narrow based on typeof equality
                    let is_negated = matches!(
                        operator,
                        BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict
                    );
                    if let Some(environments) = self.narrow_environment_for_typeof_guard(
                        module,
                        guard_id,
                        *left,
                        *right,
                        is_negated,
                        tree,
                        symbols,
                        types,
                        environment,
                        context,
                    )? {
                        return Ok(environments);
                    }

                    // narrow based on nullish equality
                    let is_strict = matches!(
                        operator,
                        BinaryOperator::EqualStrict | BinaryOperator::NotEqualStrict
                    );
                    if let Some(environments) = self.narrow_environment_for_nullish_guard(
                        module,
                        guard_id,
                        *left,
                        *right,
                        is_strict,
                        is_negated,
                        tree,
                        symbols,
                        types,
                        environment,
                        context,
                    )? {
                        return Ok(environments);
                    }

                    // narrow based on discriminant equality
                    if let Some(environments) = self.narrow_environment_for_discriminant_guard(
                        module,
                        guard_id,
                        *left,
                        *right,
                        is_negated,
                        tree,
                        symbols,
                        types,
                        environment,
                        context,
                    )? {
                        return Ok(environments);
                    }

                    Ok((environment.clone(), environment.clone()))
                }
                _ => Ok((environment.clone(), environment.clone())),
            },
            Expression::TypeBinary {
                left,
                operator,
                right,
            } => match operator {
                TypeBinaryOperator::Is => {
                    // narrow using an `x is T` guard
                    if let Some(environments) = self.narrow_environment_for_is_guard(
                        module,
                        guard_id,
                        *left,
                        *right,
                        tree,
                        symbols,
                        types,
                        environment,
                        context,
                    )? {
                        return Ok(environments);
                    }
                    Ok((environment.clone(), environment.clone()))
                }
                _ => Ok((environment.clone(), environment.clone())),
            },
            _ => Ok((environment.clone(), environment.clone())),
        }
    }

    /// Get a symbol type from a flow environment.
    pub fn flow_environment_for_symbol(
        &self,
        symbol: GlobalSymbolId,
        environment: &FlowEnvironment,
    ) -> Option<LocalTypeId> {
        environment.bindings.get(&symbol).copied()
    }

    /// Split the environment based on a nullish equality guard.
    fn narrow_environment_for_nullish_guard(
        &self,
        module: &Module,
        guard_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        is_strict: bool,
        is_negated: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        environment: &FlowEnvironment,
        context: &InferContext,
    ) -> AnalyzeResult<Option<(FlowEnvironment, FlowEnvironment)>> {
        // normalize the guard expressions for matching
        let left_id = self.unwrap_parenthesized_expression(left_id, tree);
        let right_id = self.unwrap_parenthesized_expression(right_id, tree);

        // resolve symbols and literal kinds for both sides
        let left_symbol =
            self.reference_symbol_for_expression(module, left_id, context.profile, tree, symbols);
        let right_symbol =
            self.reference_symbol_for_expression(module, right_id, context.profile, tree, symbols);

        let left_literal = self.nullish_literal_kind(tree, left_id);
        let right_literal = self.nullish_literal_kind(tree, right_id);

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
            module,
            guard_id,
            symbol,
            tree,
            symbols,
            types,
            environment,
            context,
        )?;

        // build the true and false environments
        let mut true_environment = environment.clone();
        let mut false_environment = environment.clone();

        let (true_type_id, false_type_id) =
            self.nullish_guard_types(guard_kind, base_type_id, types);

        // apply the narrowings for each branch
        if let Some(type_id) = true_type_id {
            true_environment.bindings.insert(symbol, type_id);
        }
        if let Some(type_id) = false_type_id {
            false_environment.bindings.insert(symbol, type_id);
        }

        // flip environments when the guard is negated
        if is_negated {
            return Ok(Some((false_environment, true_environment)));
        }

        Ok(Some((true_environment, false_environment)))
    }

    /// Split the environment based on a typeof equality guard.
    fn narrow_environment_for_typeof_guard(
        &self,
        module: &Module,
        guard_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        is_negated: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        environment: &FlowEnvironment,
        context: &InferContext,
    ) -> AnalyzeResult<Option<(FlowEnvironment, FlowEnvironment)>> {
        // normalize both sides for matching
        let left_id = self.unwrap_parenthesized_expression(left_id, tree);
        let right_id = self.unwrap_parenthesized_expression(right_id, tree);

        // identify the typeof expression and the string literal
        let (typeof_id, literal_id) = match (
            self.typeof_expression_id(tree, left_id),
            self.string_literal_id(tree, right_id),
            self.typeof_expression_id(tree, right_id),
            self.string_literal_id(tree, left_id),
        ) {
            (Some(typeof_id), Some(literal_id), _, _) => (typeof_id, literal_id),
            (_, _, Some(typeof_id), Some(literal_id)) => (typeof_id, literal_id),
            _ => return Ok(None),
        };

        // resolve the guard symbol from the typeof argument
        let typeof_id = self.unwrap_parenthesized_expression(typeof_id, tree);
        let Expression::TypeUnary {
            operator: TypeUnaryOperator::Typeof,
            right,
        } = tree.get(typeof_id)
        else {
            return Ok(None);
        };
        let right_id = self.unwrap_parenthesized_expression(*right, tree);
        let symbol =
            self.reference_symbol_for_expression(module, right_id, context.profile, tree, symbols);
        let Some(symbol) = symbol else {
            return Ok(None);
        };

        // resolve the typeof guard target
        let Some(target) = self.type_guard_target_for_typeof_string(literal_id, types) else {
            return Ok(None);
        };

        // resolve the base type for the symbol
        let base_type_id = self.symbol_type_for_guard(
            module,
            guard_id,
            symbol,
            tree,
            symbols,
            types,
            environment,
            context,
        )?;

        // evaluate unevaluated types before applying typeof narrowing
        self.evaluate_type(module, context.profile, base_type_id, tree, symbols, types)?;
        // clone union elements to avoid holding a borrow across evaluation
        let mut union_elements = Vec::new();
        if let Type::Union { elements } = types.get_type(base_type_id) {
            union_elements.extend(elements.iter().copied());
        }
        for element_id in union_elements {
            self.evaluate_type(module, context.profile, element_id, tree, symbols, types)?;
        }

        // compute narrowed types for each branch
        let (true_type_id, false_type_id) = self.type_guard_target_types(
            module,
            context.profile,
            symbols,
            base_type_id,
            target,
            types,
            &context.options,
        );

        // apply the narrowings for each branch
        let mut true_environment = environment.clone();
        let mut false_environment = environment.clone();
        if let Some(type_id) = true_type_id {
            true_environment.bindings.insert(symbol, type_id);
        }
        if let Some(type_id) = false_type_id {
            false_environment.bindings.insert(symbol, type_id);
        }

        // flip environments when the guard is negated
        if is_negated {
            return Ok(Some((false_environment, true_environment)));
        }

        Ok(Some((true_environment, false_environment)))
    }

    /// Split the environment based on a discriminant equality guard.
    fn narrow_environment_for_discriminant_guard(
        &self,
        module: &Module,
        guard_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        is_negated: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        environment: &FlowEnvironment,
        context: &InferContext,
    ) -> AnalyzeResult<Option<(FlowEnvironment, FlowEnvironment)>> {
        // normalize both sides for matching
        let left_id = self.unwrap_parenthesized_expression(left_id, tree);
        let right_id = self.unwrap_parenthesized_expression(right_id, tree);

        // identify the discriminant access and literal
        let (symbol, key, literal) = match (
            self.discriminant_access_for_expression(module, left_id, tree, symbols, context),
            self.scalar_literal_for_expression(tree, right_id),
            self.discriminant_access_for_expression(module, right_id, tree, symbols, context),
            self.scalar_literal_for_expression(tree, left_id),
        ) {
            (Some((symbol, key)), Some(literal), _, _) => (symbol, key, literal),
            (_, _, Some((symbol, key)), Some(literal)) => (symbol, key, literal),
            _ => return Ok(None),
        };

        // resolve the base type for the symbol
        let base_type_id = self.symbol_type_for_guard(
            module,
            guard_id,
            symbol,
            tree,
            symbols,
            types,
            environment,
            context,
        )?;

        // evaluate unevaluated types before applying discriminant narrowing
        self.evaluate_type(module, context.profile, base_type_id, tree, symbols, types)?;
        // clone union elements to avoid holding a borrow across evaluation
        let mut union_elements = Vec::new();
        if let Type::Union { elements } = types.get_type(base_type_id) {
            union_elements.extend(elements.iter().copied());
        }
        for element_id in union_elements {
            self.evaluate_type(module, context.profile, element_id, tree, symbols, types)?;
        }

        // compute narrowed types for each branch
        let (true_type_id, false_type_id) = self.discriminant_guard_types(
            module,
            context.profile,
            base_type_id,
            &key,
            literal,
            tree,
            symbols,
            types,
            &context.options,
        )?;

        // apply the narrowings for each branch
        let mut true_environment = environment.clone();
        let mut false_environment = environment.clone();
        if let Some(type_id) = true_type_id {
            true_environment.bindings.insert(symbol, type_id);
        }
        if let Some(type_id) = false_type_id {
            false_environment.bindings.insert(symbol, type_id);
        }

        // flip environments when the guard is negated
        if is_negated {
            return Ok(Some((false_environment, true_environment)));
        }

        Ok(Some((true_environment, false_environment)))
    }

    /// Split the environment based on an `x in y` guard.
    fn narrow_environment_for_in_guard(
        &self,
        module: &Module,
        guard_id: LocalNodeId<Expression>,
        key_id: LocalNodeId<Expression>,
        target_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        environment: &FlowEnvironment,
        context: &InferContext,
    ) -> AnalyzeResult<Option<(FlowEnvironment, FlowEnvironment)>> {
        // normalize the guard expressions
        let key_id = self.unwrap_parenthesized_expression(key_id, tree);
        let target_id = self.unwrap_parenthesized_expression(target_id, tree);

        // only narrow for string literal keys
        let Some(key) = self.string_literal_id(tree, key_id) else {
            return Ok(None);
        };

        // resolve the target symbol
        let symbol =
            self.reference_symbol_for_expression(module, target_id, context.profile, tree, symbols);
        let Some(symbol) = symbol else {
            return Ok(None);
        };

        // build the key we are guarding on
        let key = StaticKey::Name(key);

        // resolve the base type for the symbol
        let base_type_id = self.symbol_type_for_guard(
            module,
            guard_id,
            symbol,
            tree,
            symbols,
            types,
            environment,
            context,
        )?;

        // compute narrowed types for each branch
        let (true_type_id, false_type_id) = self.property_guard_types(base_type_id, &key, types);

        // apply the narrowings for each branch
        let mut true_environment = environment.clone();
        let mut false_environment = environment.clone();
        if let Some(type_id) = true_type_id {
            true_environment.bindings.insert(symbol, type_id);
        }
        if let Some(type_id) = false_type_id {
            false_environment.bindings.insert(symbol, type_id);
        }

        Ok(Some((true_environment, false_environment)))
    }

    /// Split the environment based on an `x is T` guard.
    fn narrow_environment_for_is_guard(
        &self,
        module: &Module,
        guard_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        environment: &FlowEnvironment,
        context: &InferContext,
    ) -> AnalyzeResult<Option<(FlowEnvironment, FlowEnvironment)>> {
        // resolve the target symbol
        let value_id = self.unwrap_parenthesized_expression(value_id, tree);
        let symbol =
            self.reference_symbol_for_expression(module, value_id, context.profile, tree, symbols);
        let Some(symbol) = symbol else {
            return Ok(None);
        };

        // resolve the target type
        let target_type_id = self.guard_target_type(
            module,
            context.profile,
            self.unwrap_parenthesized_expression(target_id, tree),
            tree,
            symbols,
            types,
        )?;
        let target_type_id = self.unwrap_type_value(target_type_id, types);
        if !self.guard_target_is_class(target_type_id, types) {
            return Ok(None);
        }

        // resolve the base type for the symbol
        let base_type_id = self.symbol_type_for_guard(
            module,
            guard_id,
            symbol,
            tree,
            symbols,
            types,
            environment,
            context,
        )?;

        // compute narrowed types for each branch
        let (true_type_id, false_type_id) = self.type_guard_types(
            module,
            context.profile,
            symbols,
            base_type_id,
            target_type_id,
            types,
            &context.options,
        );

        // apply the narrowings for each branch
        let mut true_environment = environment.clone();
        let mut false_environment = environment.clone();
        if let Some(type_id) = true_type_id {
            true_environment.bindings.insert(symbol, type_id);
        }
        if let Some(type_id) = false_type_id {
            false_environment.bindings.insert(symbol, type_id);
        }

        Ok(Some((true_environment, false_environment)))
    }

    /// Split the environment based on a class identity guard.
    fn narrow_environment_for_type_guard(
        &self,
        module: &Module,
        guard_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        target_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        environment: &FlowEnvironment,
        context: &InferContext,
    ) -> AnalyzeResult<Option<(FlowEnvironment, FlowEnvironment)>> {
        // resolve the target symbol
        let value_id = self.unwrap_parenthesized_expression(value_id, tree);
        let symbol =
            self.reference_symbol_for_expression(module, value_id, context.profile, tree, symbols);
        let Some(symbol) = symbol else {
            return Ok(None);
        };

        // resolve the target type
        let target_type_id = self.guard_target_type(
            module,
            context.profile,
            self.unwrap_parenthesized_expression(target_id, tree),
            tree,
            symbols,
            types,
        )?;
        let target_type_id = self.unwrap_type_value(target_type_id, types);

        // resolve the base type for the symbol
        let base_type_id = self.symbol_type_for_guard(
            module,
            guard_id,
            symbol,
            tree,
            symbols,
            types,
            environment,
            context,
        )?;

        // compute narrowed types for each branch
        let (true_type_id, false_type_id) = self.type_guard_types(
            module,
            context.profile,
            symbols,
            base_type_id,
            target_type_id,
            types,
            &context.options,
        );

        // apply the narrowings for each branch
        let mut true_environment = environment.clone();
        let mut false_environment = environment.clone();
        if let Some(type_id) = true_type_id {
            true_environment.bindings.insert(symbol, type_id);
        }
        if let Some(type_id) = false_type_id {
            false_environment.bindings.insert(symbol, type_id);
        }

        Ok(Some((true_environment, false_environment)))
    }

    /// Determine the target type for a guard expression.
    fn guard_target_type(
        &self,
        module: &Module,
        profile: ProfileId,
        target_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        // prefer explicit type nodes
        if let Expression::Type { value } = tree.get(target_id) {
            return Ok(*value);
        }

        // fall back to evaluating the expression as a type
        self.try_evaluate_expression_to_type(module, profile, target_id, tree, symbols, types)
    }

    /// Unwrap a Type::Value wrapper to a usable guard target.
    fn unwrap_type_value(&self, type_id: LocalTypeId, types: &TypeTable) -> LocalTypeId {
        match types.get_type(type_id) {
            Type::Value { value } => *value,
            _ => type_id,
        }
    }

    /// Check whether a guard target is a class type.
    fn guard_target_is_class(&self, type_id: LocalTypeId, types: &TypeTable) -> bool {
        types
            .get_type(type_id)
            .symbol()
            .is_some_and(|symbol| symbol.local_id.ty == SymbolType::Class)
    }

    /// Derive guard types for a symbol based on a target type.
    fn type_guard_types(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        base_type_id: LocalTypeId,
        target_type_id: LocalTypeId,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> (Option<LocalTypeId>, Option<LocalTypeId>) {
        // handle union and non union cases separately
        // filter union members that satisfy the target guard
        let base_type = types.get_type(base_type_id).clone();
        let true_type_id = match base_type {
            Type::Union { elements } => {
                let mut matching_elements = Vec::new();

                // collect assignable union members
                for element_id in elements {
                    let is_assignable = self
                        .is_type_assignable(
                            module,
                            profile,
                            symbols,
                            target_type_id,
                            element_id,
                            types,
                            options,
                        )
                        .is_assignable();
                    if is_assignable {
                        matching_elements.push(element_id);
                    }
                }

                match matching_elements.len() {
                    0 => None,
                    1 => Some(matching_elements[0]),
                    _ => Some(types.insert_type(Type::Union {
                        elements: matching_elements,
                    })),
                }
            }
            _ => {
                // keep the base type when it is already narrow enough
                let base_is_assignable = self
                    .is_type_assignable(
                        module,
                        profile,
                        symbols,
                        target_type_id,
                        base_type_id,
                        types,
                        options,
                    )
                    .is_assignable();
                let target_is_assignable = self
                    .is_type_assignable(
                        module,
                        profile,
                        symbols,
                        base_type_id,
                        target_type_id,
                        types,
                        options,
                    )
                    .is_assignable();

                if base_is_assignable {
                    Some(base_type_id)
                } else if target_is_assignable {
                    Some(target_type_id)
                } else {
                    Some(types.insert_type(Type::Intersection {
                        elements: vec![base_type_id, target_type_id],
                    }))
                }
            }
        };

        // drop assignable types for the false branch
        let (false_type_id, _) = self.strip_assignable_from_union(
            module,
            profile,
            symbols,
            base_type_id,
            target_type_id,
            types,
            options,
        );

        (true_type_id, false_type_id)
    }

    /// Derive guard types for a typed guard target.
    fn type_guard_target_types(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        base_type_id: LocalTypeId,
        target: TypeGuardTarget,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> (Option<LocalTypeId>, Option<LocalTypeId>) {
        // route guard targets to their narrowing strategy
        match target {
            TypeGuardTarget::TypeId(target_type_id) => self.type_guard_types(
                module,
                profile,
                symbols,
                base_type_id,
                target_type_id,
                types,
                options,
            ),
            TypeGuardTarget::ObjectLike => {
                self.predicate_guard_types(base_type_id, types, |type_id, types| {
                    self.type_is_object_like(type_id, types)
                })
            }
            TypeGuardTarget::FunctionLike => {
                self.predicate_guard_types(base_type_id, types, |type_id, types| {
                    self.type_is_function_like(type_id, types)
                })
            }
        }
    }

    /// Derive guard types for a discriminant equality check.
    fn discriminant_guard_types(
        &self,
        module: &Module,
        profile: ProfileId,
        base_type_id: LocalTypeId,
        key: &StaticKey,
        literal: ScalarLiteral,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<(Option<LocalTypeId>, Option<LocalTypeId>)> {
        // avoid narrowing any or unknown types
        if self.type_is_any_or_unknown(base_type_id, types) {
            return Ok((Some(base_type_id), Some(base_type_id)));
        }

        // build a literal type for assignability checks
        let literal_type = self.infer_scalar_literal(&literal);
        let literal_type_id = types.insert_type(Type::TypeLiteral {
            value: literal_type.clone(),
        });

        // split union and non union targets
        let base_type = types.get_type(base_type_id).clone();
        match base_type {
            Type::Union { elements } => {
                // avoid narrowing unions with any or unknown members
                if elements
                    .iter()
                    .any(|element_id| self.type_is_any_or_unknown(*element_id, types))
                {
                    return Ok((Some(base_type_id), Some(base_type_id)));
                }

                let mut matching_elements = Vec::new();
                let mut remaining_elements = Vec::new();

                // collect union members based on discriminant compatibility
                for element_id in elements {
                    let field_info = self.type_field_type_for_key(
                        module, profile, element_id, key, tree, symbols, types,
                    )?;

                    let Some((field_type_id, is_optional)) = field_info else {
                        remaining_elements.push(element_id);
                        continue;
                    };

                    let is_assignable = self
                        .is_type_assignable(
                            module,
                            profile,
                            symbols,
                            field_type_id,
                            literal_type_id,
                            types,
                            options,
                        )
                        .is_assignable();

                    if is_assignable {
                        matching_elements.push(element_id);
                    }

                    if !is_assignable {
                        remaining_elements.push(element_id);
                        continue;
                    }

                    let (remaining_literal, _) =
                        self.strip_literal_from_union(field_type_id, types, literal_type.clone());
                    if is_optional || remaining_literal.is_some() {
                        remaining_elements.push(element_id);
                    }
                }

                let true_type_id = match matching_elements.len() {
                    0 => None,
                    1 => Some(matching_elements[0]),
                    _ => Some(types.insert_type(Type::Union {
                        elements: matching_elements,
                    })),
                };
                let false_type_id = match remaining_elements.len() {
                    0 => None,
                    1 => Some(remaining_elements[0]),
                    _ => Some(types.insert_type(Type::Union {
                        elements: remaining_elements,
                    })),
                };

                Ok((true_type_id, false_type_id))
            }
            _ => {
                let Some((field_type_id, is_optional)) = self.type_field_type_for_key(
                    module,
                    profile,
                    base_type_id,
                    key,
                    tree,
                    symbols,
                    types,
                )?
                else {
                    return Ok((Some(base_type_id), Some(base_type_id)));
                };

                let is_assignable = self
                    .is_type_assignable(
                        module,
                        profile,
                        symbols,
                        field_type_id,
                        literal_type_id,
                        types,
                        options,
                    )
                    .is_assignable();

                if !is_assignable {
                    return Ok((None, Some(base_type_id)));
                }

                let (remaining_literal, _) =
                    self.strip_literal_from_union(field_type_id, types, literal_type);
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
        types: &mut TypeTable,
        predicate: F,
    ) -> (Option<LocalTypeId>, Option<LocalTypeId>)
    where
        F: Fn(LocalTypeId, &TypeTable) -> bool,
    {
        // avoid narrowing any or unknown types
        if self.type_is_any_or_unknown(base_type_id, types) {
            return (Some(base_type_id), Some(base_type_id));
        }

        // split union and non union targets
        match types.get_type(base_type_id) {
            Type::Union { elements } => {
                // avoid narrowing unions with any or unknown members
                if elements
                    .iter()
                    .any(|element_id| self.type_is_any_or_unknown(*element_id, types))
                {
                    return (Some(base_type_id), Some(base_type_id));
                }

                let mut matching_elements = Vec::new();
                let mut remaining_elements = Vec::new();

                // collect union members by predicate
                for element_id in elements {
                    if predicate(*element_id, types) {
                        matching_elements.push(*element_id);
                    } else {
                        remaining_elements.push(*element_id);
                    }
                }

                let true_type_id = match matching_elements.len() {
                    0 => None,
                    1 => Some(matching_elements[0]),
                    _ => Some(types.insert_type(Type::Union {
                        elements: matching_elements,
                    })),
                };
                let false_type_id = match remaining_elements.len() {
                    0 => None,
                    1 => Some(remaining_elements[0]),
                    _ => Some(types.insert_type(Type::Union {
                        elements: remaining_elements,
                    })),
                };

                (true_type_id, false_type_id)
            }
            _ => {
                // narrow based on the predicate result
                if predicate(base_type_id, types) {
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
        base_type_id: LocalTypeId,
        key: &StaticKey,
        types: &mut TypeTable,
    ) -> (Option<LocalTypeId>, Option<LocalTypeId>) {
        // split union and non union targets
        match types.get_type(base_type_id) {
            Type::Union { elements } => {
                let mut matching_elements = Vec::new();
                let mut remaining_elements = Vec::new();

                // collect union members with and without the key
                for element_id in elements {
                    if self.type_has_property(*element_id, key, types) {
                        matching_elements.push(*element_id);
                    } else {
                        remaining_elements.push(*element_id);
                    }
                }

                let true_type_id = match matching_elements.len() {
                    0 => None,
                    1 => Some(matching_elements[0]),
                    _ => Some(types.insert_type(Type::Union {
                        elements: matching_elements,
                    })),
                };
                let false_type_id = match remaining_elements.len() {
                    0 => None,
                    1 => Some(remaining_elements[0]),
                    _ => Some(types.insert_type(Type::Union {
                        elements: remaining_elements,
                    })),
                };

                (true_type_id, false_type_id)
            }
            _ => {
                // return the base type on the branch that matches
                if self.type_has_property(base_type_id, key, types) {
                    (Some(base_type_id), None)
                } else {
                    (None, Some(base_type_id))
                }
            }
        }
    }

    /// Strip assignable elements from a union for guard negation.
    fn strip_assignable_from_union(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        type_id: LocalTypeId,
        target_type_id: LocalTypeId,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> (Option<LocalTypeId>, bool) {
        // split unions from non union types
        let type_value = types.get_type(type_id).clone();
        match type_value {
            Type::Union { elements } => {
                let mut filtered_elements = Vec::new();
                let mut removed = false;

                // drop assignable elements while tracking removals
                for element_id in elements {
                    let is_assignable = self
                        .is_type_assignable(
                            module,
                            profile,
                            symbols,
                            target_type_id,
                            element_id,
                            types,
                            options,
                        )
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
                    _ => Some(types.insert_type(Type::Union {
                        elements: filtered_elements,
                    })),
                };

                (filtered_type_id, true)
            }
            _ => {
                // remove the type when it is assignable to the target
                let is_assignable = self
                    .is_type_assignable(
                        module,
                        profile,
                        symbols,
                        target_type_id,
                        type_id,
                        types,
                        options,
                    )
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
            Expression::TypeUnary {
                operator: TypeUnaryOperator::Typeof,
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
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        context: &InferContext,
    ) -> Option<(GlobalSymbolId, StaticKey)> {
        match tree.get(expression_id) {
            Expression::Member { left, name, .. } => {
                let left_id = self.unwrap_parenthesized_expression(*left, tree);
                let symbol = self.reference_symbol_for_expression(
                    module,
                    left_id,
                    context.profile,
                    tree,
                    symbols,
                )?;
                Some((symbol, StaticKey::Name(*name)))
            }
            Expression::Index { left, right, .. } => {
                let right_id = right.as_ref()?;
                let right_id = self.unwrap_parenthesized_expression(*right_id, tree);
                let key = self.string_literal_id(tree, right_id)?;
                let left_id = self.unwrap_parenthesized_expression(*left, tree);
                let symbol = self.reference_symbol_for_expression(
                    module,
                    left_id,
                    context.profile,
                    tree,
                    symbols,
                )?;
                Some((symbol, StaticKey::Name(key)))
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
        let null_type_id = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Null,
        });
        let undefined_type_id = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Undefined,
        });

        // build branch types based on the guard kind
        match guard_kind {
            NullishGuardKind::Nullish => {
                let nullish_type_id = types.insert_type(Type::Union {
                    elements: vec![null_type_id, undefined_type_id],
                });
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
                    _ => Some(types.insert_type(Type::Union {
                        elements: filtered_elements,
                    })),
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
        module: &Module,
        guard_id: LocalNodeId<Expression>,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        environment: &FlowEnvironment,
        context: &InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // reuse an environment narrowing when present
        let resolved_type = if let Some(type_id) = environment.bindings.get(&symbol) {
            self.unwrap_type_alias_reference(
                module,
                context.profile,
                *type_id,
                tree,
                symbols,
                types,
            )?
        }
        // fall back to any known value type
        else if let Some(type_id) = types.get_value_type_id(symbol) {
            self.unwrap_type_alias_reference(
                module,
                context.profile,
                type_id,
                tree,
                symbols,
                types,
            )?
        }
        // resolve declarations in the current module
        else if symbol.module_id == module.id {
            let symbol = symbols.get_symbol(symbol.into());
            // use the primary declaration type when available
            if let Some(primary_declaration) = symbol.primary_declaration
                && let Some(type_id) = types.get_declared_type_id(primary_declaration)
            {
                self.unwrap_type_alias_reference(
                    module,
                    context.profile,
                    type_id,
                    tree,
                    symbols,
                    types,
                )?
            } else {
                // walk parent declarations to recover contextual types
                let mut declared_type = None;
                if let Some(primary_declaration) = symbol.primary_declaration {
                    let mut current_id = primary_declaration.local_id;
                    while let Some(parent_id) = tree.get_parent(current_id.id) {
                        if let Some(type_id) =
                            types.get_declared_type_id(parent_id.into_global(module.id))
                        {
                            declared_type = Some(self.unwrap_type_alias_reference(
                                module,
                                context.profile,
                                type_id,
                                tree,
                                symbols,
                                types,
                            )?);
                            break;
                        }
                        current_id = parent_id;
                    }
                }

                // fall back to unknown when no type is available
                declared_type.unwrap_or_else(|| {
                    types.insert_type_from(
                        Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        },
                        guard_id,
                    )
                })
            }
        } else {
            // resolve remote symbol types through the compiler
            self.resolve_remote_symbol_value_type(module, context.profile, guard_id, symbol, types)?
        };

        Ok(self.normalize_type(
            module,
            context.profile,
            resolved_type,
            symbols,
            types,
            NormalizationMode::Flow,
        ))
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

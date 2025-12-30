use std::collections::VecDeque;

use indexmap::IndexMap;

use destack_dir::{
    BinaryOperator, Expression, FlowEdge, FlowEdgeKind, FlowEnvironment, FlowGraph, FlowTable,
    GlobalSymbolId, InferTable, LocalNodeId, LocalTypeId, NodeTree, SymbolTable, Type, TypeLiteral,
    TypeTable, UnaryOperator,
};
use destack_workspace::Module;

use crate::{AnalyzeResult, AnalyzeWarning, Compiler, InferContext};

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
                self.merge_flow_type_ids(left_type_id, right_type_id, baseline_type_id, types);
            bindings.insert(*symbol, merged_type_id);
        }

        FlowEnvironment {
            bindings,
            is_reachable: true,
        }
    }

    /// Merge two type ids for flow environments, preserving baseline ordering when possible.
    fn merge_flow_type_ids(
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
            } => {
                if *operator == BinaryOperator::And {
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
                    return Ok((right_true, false_environment));
                }

                if *operator == BinaryOperator::Or {
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
                    return Ok((true_environment, right_false));
                }

                // fall back when the binary operator does not narrow
                let is_equal = matches!(
                    operator,
                    BinaryOperator::Equal
                        | BinaryOperator::EqualStrict
                        | BinaryOperator::NotEqual
                        | BinaryOperator::NotEqualStrict
                );
                if !is_equal {
                    return Ok((environment.clone(), environment.clone()));
                }

                // narrow based on nullish equality
                let is_strict = matches!(
                    operator,
                    BinaryOperator::EqualStrict | BinaryOperator::NotEqualStrict
                );
                let is_negated = matches!(
                    operator,
                    BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict
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

                Ok((environment.clone(), environment.clone()))
            }
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

    /// Apply nullish guard types to a symbol type.
    fn nullish_guard_types(
        &self,
        guard_kind: NullishGuardKind,
        base_type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> (Option<LocalTypeId>, Option<LocalTypeId>) {
        let null_type_id = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Null,
        });
        let undefined_type_id = types.insert_type(Type::TypeLiteral {
            value: TypeLiteral::Undefined,
        });

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
        let type_value = types.get_type(type_id);

        match type_value {
            Type::Union { elements } => {
                let mut filtered_elements = Vec::new();
                let mut removed = false;

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

                if !removed {
                    return (Some(type_id), false);
                }

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
        if let Some(type_id) = environment.bindings.get(&symbol) {
            return Ok(*type_id);
        }

        if let Some(type_id) = types.get_value_type_id(symbol) {
            return Ok(type_id);
        }

        if symbol.module_id == module.id {
            let symbol = symbols.get_symbol(symbol.into());
            if let Some(primary_declaration) = symbol.primary_declaration
                && let Some(type_id) = types.get_declared_type_id(primary_declaration)
            {
                return Ok(type_id);
            }

            if let Some(primary_declaration) = symbol.primary_declaration {
                let mut current_id = primary_declaration.local_id;
                while let Some(parent_id) = tree.get_parent(current_id.id) {
                    if let Some(type_id) =
                        types.get_declared_type_id(parent_id.into_global(module.id))
                    {
                        return Ok(type_id);
                    }
                    current_id = parent_id;
                }
            }
        } else {
            return self.resolve_remote_symbol_value_type(
                module,
                context.profile,
                guard_id,
                symbol,
                types,
            );
        }

        let unknown_type = Type::TypeLiteral {
            value: TypeLiteral::Unknown,
        };
        Ok(types.insert_type_from(unknown_type, guard_id))
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

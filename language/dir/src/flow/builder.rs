use indexmap::IndexMap;

use destack_source::ModuleId;

use crate::{
    Argument, BinaryOperator, Block, Declaration, Declarator, DynamicKey, Expression, FlowBlock,
    FlowBlockId, FlowEdge, FlowEdgeKind, FlowGraph, ForEachBinding, GlobalSymbolId, LocalNodeId,
    LocalNodeIdAny, LocalSymbolId, LoopKind, MatchCase, MatchSelector, MatchSource, NodeTree,
    Pattern, PatternField, Property, TemplateLiteral, UnaryOperator,
};

/// Describe what kind of control target we are tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlTargetKind {
    /// Track loop targets for break and continue.
    Loop,
    /// Track match targets for break.
    Match,
    /// Track labelled statements for break.
    Label,
}

/// Track break and continue targets for control expressions.
#[derive(Debug, Clone, Copy)]
struct ControlTarget {
    /// Identify the target kind.
    kind: ControlTargetKind,
    /// The label symbol.
    symbol: Option<GlobalSymbolId>,
    /// The break destination.
    break_target: FlowBlockId,
    /// The continue destination for loops.
    continue_target: Option<FlowBlockId>,
}

/// Build a control flow graph from DIR expressions.
#[derive(Debug)]
pub struct FlowGraphBuilder<'tree> {
    /// Identify the module that owns the graph.
    module_id: ModuleId,
    /// Provide access to the DIR tree for traversal.
    tree: &'tree NodeTree,
    /// The blocks built so far.
    blocks: Vec<FlowBlock>,
    /// Map nodes to their containing block.
    block_by_node: IndexMap<LocalNodeIdAny, FlowBlockId>,
    /// Track control flow targets for break and continue.
    control_stack: Vec<ControlTarget>,
}

impl<'tree> FlowGraphBuilder<'tree> {
    /// Create a new flow graph builder.
    pub fn new(module_id: ModuleId, tree: &'tree NodeTree) -> Self {
        Self {
            module_id,
            tree,
            blocks: Vec::new(),
            block_by_node: IndexMap::new(),
            control_stack: Vec::new(),
        }
    }

    /// Build a control flow graph for a body expression.
    pub fn build(mut self, body_id: LocalNodeId<Expression>) -> FlowGraph {
        let entry_block = self.create_block();
        let body_exit_block = self.build_expression(body_id, entry_block);
        let exit_block = self.create_block();

        if let Some(body_exit_block) = body_exit_block {
            self.connect_blocks(
                body_exit_block,
                exit_block,
                FlowEdgeKind::Unconditional,
                None,
            );
        }

        FlowGraph {
            entry_block,
            exit_block,
            blocks: self.blocks,
            block_by_node: self.block_by_node,
        }
    }

    /// Build a control flow graph for a sequence of root expressions.
    pub fn build_roots(mut self, roots: &[LocalNodeId<Expression>]) -> FlowGraph {
        let entry_block = self.create_block();
        let roots_exit_block = self.build_expression_sequence(roots, entry_block);
        let exit_block = self.create_block();

        if let Some(roots_exit_block) = roots_exit_block {
            self.connect_blocks(
                roots_exit_block,
                exit_block,
                FlowEdgeKind::Unconditional,
                None,
            );
        }

        FlowGraph {
            entry_block,
            exit_block,
            blocks: self.blocks,
            block_by_node: self.block_by_node,
        }
    }

    /// Create a new block and return its identifier.
    fn create_block(&mut self) -> FlowBlockId {
        let block_id = FlowBlockId(self.blocks.len() as u32);
        self.blocks.push(FlowBlock {
            id: block_id,
            nodes: Vec::new(),
            predecessors: Vec::new(),
            successors: Vec::new(),
            is_terminal: false,
        });
        block_id
    }

    /// Record a node inside a block.
    fn record_node(&mut self, block_id: FlowBlockId, node_id: LocalNodeIdAny) {
        if self.block_by_node.insert(node_id, block_id).is_some() {
            return;
        }

        let block_index = block_id.0 as usize;
        self.blocks[block_index].nodes.push(node_id);
    }

    /// Add an edge between two blocks.
    fn connect_blocks(
        &mut self,
        source_block_id: FlowBlockId,
        target_block_id: FlowBlockId,
        kind: FlowEdgeKind,
        guard: Option<LocalNodeId<Expression>>,
    ) {
        let source_index = source_block_id.0 as usize;
        let target_index = target_block_id.0 as usize;

        // record the outgoing edge on the source block
        self.blocks[source_index].successors.push(FlowEdge {
            target: target_block_id,
            kind,
            guard,
        });

        // record the incoming edge on the target block
        self.blocks[target_index].predecessors.push(FlowEdge {
            target: source_block_id,
            kind,
            guard,
        });
    }

    /// Report whether a block has any predecessors.
    fn block_has_predecessors(&self, block_id: FlowBlockId) -> bool {
        !self.blocks[block_id.0 as usize].predecessors.is_empty()
    }

    /// Build an expression and return the fallthrough block when it exists.
    fn build_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        self.record_node(current_block_id, expression_id.into_any());
        let expression = self.tree.get(expression_id);

        match expression {
            Expression::Block { block } => self.build_block(*block, current_block_id),
            Expression::Statement { statement } => {
                self.build_expression(*statement, current_block_id)
            }
            Expression::Labelled { body, symbol, .. } => {
                self.build_labelled_expression(*symbol, *body, current_block_id)
            }

            Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => self.build_if_expression(
                *condition,
                *then_expression,
                *else_expression,
                current_block_id,
            ),
            Expression::Loop {
                kind,
                condition,
                body,
                symbol,
                ..
            } => self.build_loop_expression(*kind, *condition, *body, *symbol, current_block_id),
            Expression::ForEach {
                binding,
                iterator,
                body,
                symbol,
                ..
            } => {
                self.build_for_each_expression(binding, *iterator, *body, *symbol, current_block_id)
            }
            Expression::For {
                initialization,
                condition,
                increment,
                body,
                symbol,
                ..
            } => self.build_for_expression(
                *initialization,
                *condition,
                *increment,
                *body,
                *symbol,
                current_block_id,
            ),
            Expression::Match {
                value,
                cases,
                source,
                symbol,
                ..
            } => self.build_match_expression(*value, cases, *source, *symbol, current_block_id),
            Expression::Binary {
                left,
                operator,
                right,
            } if matches!(operator, BinaryOperator::And | BinaryOperator::Or) => {
                self.build_short_circuit_expression(*operator, *left, *right, current_block_id)
            }
            Expression::Try {
                try_expression,
                catch_pattern,
                catch_expression,
                finally_expression,
                ..
            } => self.build_try_expression(
                *try_expression,
                *catch_pattern,
                *catch_expression,
                *finally_expression,
                current_block_id,
            ),
            Expression::Return { value } => self.build_return_expression(*value, current_block_id),
            Expression::Throw { value } => self.build_throw_expression(*value, current_block_id),
            Expression::Break {
                target_symbol,
                value,
                ..
            } => self.build_break_expression(*target_symbol, *value, current_block_id),
            Expression::UnresolvedBreak { value, .. } => {
                self.build_break_expression(None, *value, current_block_id)
            }
            Expression::Continue { target_symbol, .. } => {
                self.build_continue_expression(*target_symbol, current_block_id)
            }
            Expression::UnresolvedContinue { .. } => {
                self.build_continue_expression(None, current_block_id)
            }
            _ => self.build_expression_children(expression, current_block_id),
        }
    }

    /// Build a short circuit binary expression.
    fn build_short_circuit_expression(
        &mut self,
        operator: BinaryOperator,
        left_id: LocalNodeId<Expression>,
        right_id: LocalNodeId<Expression>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        // evaluate the left side first
        let left_exit_block_id = self.build_expression(left_id, current_block_id)?;

        // allocate the right side and join blocks
        let right_block_id = self.create_block();
        let join_block_id = self.create_block();

        // wire guard edges for the short circuit operator
        match operator {
            BinaryOperator::And => {
                // flow to the right side when the left guard is true
                self.connect_blocks(
                    left_exit_block_id,
                    right_block_id,
                    FlowEdgeKind::True,
                    Some(left_id),
                );
                // short circuit to the join when the left guard is false
                self.connect_blocks(
                    left_exit_block_id,
                    join_block_id,
                    FlowEdgeKind::False,
                    Some(left_id),
                );
            }
            BinaryOperator::Or => {
                // short circuit to the join when the left guard is true
                self.connect_blocks(
                    left_exit_block_id,
                    join_block_id,
                    FlowEdgeKind::True,
                    Some(left_id),
                );
                // flow to the right side when the left guard is false
                self.connect_blocks(
                    left_exit_block_id,
                    right_block_id,
                    FlowEdgeKind::False,
                    Some(left_id),
                );
            }
            _ => {
                unreachable!("non short circuit operator routed to build_short_circuit_expression")
            }
        }

        // evaluate the right side and connect to the join when it falls through
        if let Some(right_exit_block_id) = self.build_expression(right_id, right_block_id) {
            self.connect_blocks(
                right_exit_block_id,
                join_block_id,
                FlowEdgeKind::Unconditional,
                None,
            );
        }

        Some(join_block_id)
    }

    /// Build a block expression and return the fallthrough block when it exists.
    fn build_block(
        &mut self,
        block_id: LocalNodeId<Block>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        self.record_node(current_block_id, block_id.into_any());
        let block = self.tree.get(block_id);
        self.build_expression_sequence(&block.expressions, current_block_id)
    }

    /// Build a sequence of expressions.
    fn build_expression_sequence(
        &mut self,
        expression_ids: &[LocalNodeId<Expression>],
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let mut active_block_id = current_block_id;
        let mut fallthrough_block_id = Some(current_block_id);
        let mut is_reachable = true;

        for expression_id in expression_ids {
            // start a new unreachable block for the remainder
            if !is_reachable {
                active_block_id = self.create_block();
                is_reachable = true;
            }

            let next_block_id = self.build_expression(*expression_id, active_block_id);
            match next_block_id {
                Some(next_block_id) => {
                    active_block_id = next_block_id;
                    fallthrough_block_id = Some(next_block_id);
                }
                None => {
                    fallthrough_block_id = None;
                    is_reachable = false;
                }
            }
        }

        fallthrough_block_id
    }

    /// Build a labelled expression with a dedicated break target.
    fn build_labelled_expression(
        &mut self,
        symbol: LocalSymbolId,
        body_id: LocalNodeId<Expression>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let label_symbol = symbol.into_global(self.module_id);
        let break_target = self.create_block();

        self.control_stack.push(ControlTarget {
            kind: ControlTargetKind::Label,
            symbol: Some(label_symbol),
            break_target,
            continue_target: None,
        });

        let body_exit_block_id = self.build_expression(body_id, current_block_id);

        self.control_stack.pop();

        if let Some(body_exit_block_id) = body_exit_block_id {
            self.connect_blocks(
                body_exit_block_id,
                break_target,
                FlowEdgeKind::Unconditional,
                None,
            );
        }

        self.block_has_predecessors(break_target)
            .then_some(break_target)
    }

    /// Build an if expression and return the join block when it exists.
    fn build_if_expression(
        &mut self,
        condition_id: LocalNodeId<Expression>,
        then_expression_id: LocalNodeId<Expression>,
        else_expression_id: Option<LocalNodeId<Expression>>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        // allocate branch blocks
        let then_block_id = self.create_block();
        let join_block_id = self.create_block();
        let has_else_expression = else_expression_id.is_some();
        let else_block_id = has_else_expression.then_some(self.create_block());
        let false_target_id = else_block_id.unwrap_or(join_block_id);

        // connect guard evaluation to the branch targets
        self.build_guard_expression(
            condition_id,
            then_block_id,
            false_target_id,
            current_block_id,
        );

        // evaluate then branch
        let then_exit_block_id = self.build_expression(then_expression_id, then_block_id);

        // evaluate else branch
        let mut else_exit_block_id = None;
        if let (Some(else_expression_id), Some(else_block_id)) = (else_expression_id, else_block_id)
        {
            else_exit_block_id = self.build_expression(else_expression_id, else_block_id);
        }

        // fallthrough
        let then_reachable = self.block_has_predecessors(then_block_id);
        let else_reachable = else_block_id
            .map(|else_block_id| self.block_has_predecessors(else_block_id))
            .unwrap_or(false);
        let then_fallthrough = then_reachable && then_exit_block_id.is_some();
        let else_fallthrough = if has_else_expression {
            else_reachable && else_exit_block_id.is_some()
        } else {
            self.block_has_predecessors(join_block_id)
        };
        if !then_fallthrough && !else_fallthrough {
            return None;
        }

        // join
        if then_reachable && let Some(then_exit_block_id) = then_exit_block_id {
            self.connect_blocks(
                then_exit_block_id,
                join_block_id,
                FlowEdgeKind::Unconditional,
                None,
            );
        }
        if else_reachable && let Some(else_exit_block_id) = else_exit_block_id {
            self.connect_blocks(
                else_exit_block_id,
                join_block_id,
                FlowEdgeKind::Unconditional,
                None,
            );
        }

        Some(join_block_id)
    }

    /// Build guard evaluation with short circuit semantics.
    fn build_guard_expression(
        &mut self,
        guard_id: LocalNodeId<Expression>,
        true_block_id: FlowBlockId,
        false_block_id: FlowBlockId,
        current_block_id: FlowBlockId,
    ) -> bool {
        let guard_id = self.unwrap_parenthesized_expression(guard_id);

        match self.tree.get(guard_id) {
            Expression::Unary {
                operator: UnaryOperator::Not,
                right,
            } => {
                self.build_guard_expression(*right, false_block_id, true_block_id, current_block_id)
            }
            Expression::Binary {
                left,
                operator: BinaryOperator::And,
                right,
            } => {
                // evaluate the left guard first
                let right_block_id = self.create_block();
                let left_reachable = self.build_guard_expression(
                    *left,
                    right_block_id,
                    false_block_id,
                    current_block_id,
                );

                // evaluate the right guard when the left is true
                let right_reachable = if left_reachable {
                    self.build_guard_expression(
                        *right,
                        true_block_id,
                        false_block_id,
                        right_block_id,
                    )
                } else {
                    false
                };

                left_reachable || right_reachable
            }
            Expression::Binary {
                left,
                operator: BinaryOperator::Or,
                right,
            } => {
                // evaluate the left guard first
                let right_block_id = self.create_block();
                let left_reachable = self.build_guard_expression(
                    *left,
                    true_block_id,
                    right_block_id,
                    current_block_id,
                );

                // evaluate the right guard when the left is false
                let right_reachable = if left_reachable {
                    self.build_guard_expression(
                        *right,
                        true_block_id,
                        false_block_id,
                        right_block_id,
                    )
                } else {
                    false
                };

                left_reachable || right_reachable
            }
            _ => {
                // fall back to a single guard edge
                let Some(guard_exit_block_id) = self.build_expression(guard_id, current_block_id)
                else {
                    return false;
                };
                self.connect_blocks(
                    guard_exit_block_id,
                    true_block_id,
                    FlowEdgeKind::True,
                    Some(guard_id),
                );
                self.connect_blocks(
                    guard_exit_block_id,
                    false_block_id,
                    FlowEdgeKind::False,
                    Some(guard_id),
                );
                true
            }
        }
    }

    /// Strip parenthesized expressions to the underlying expression.
    fn unwrap_parenthesized_expression(
        &self,
        mut expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        loop {
            let Expression::Parenthesized { expression } = self.tree.get(expression_id) else {
                break;
            };
            expression_id = *expression;
        }

        expression_id
    }

    /// Build a loop expression and return the exit block.
    fn build_loop_expression(
        &mut self,
        kind: LoopKind,
        condition_id: Option<LocalNodeId<Expression>>,
        body_id: LocalNodeId<Block>,
        symbol: LocalSymbolId,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let loop_symbol = symbol.into_global(self.module_id);
        let exit_block_id = self.create_block();

        match kind {
            LoopKind::NoTest => {
                let body_block_id = self.create_block();
                self.connect_blocks(
                    current_block_id,
                    body_block_id,
                    FlowEdgeKind::Unconditional,
                    None,
                );

                self.control_stack.push(ControlTarget {
                    kind: ControlTargetKind::Loop,
                    symbol: Some(loop_symbol),
                    break_target: exit_block_id,
                    continue_target: Some(body_block_id),
                });

                let body_exit_block_id = self.build_block(body_id, body_block_id);

                self.control_stack.pop();

                if let Some(body_exit_block_id) = body_exit_block_id {
                    self.connect_blocks(
                        body_exit_block_id,
                        body_block_id,
                        FlowEdgeKind::Unconditional,
                        None,
                    );
                }

                self.block_has_predecessors(exit_block_id)
                    .then_some(exit_block_id)
            }
            LoopKind::PreTest => {
                let condition_block_id = self.create_block();
                self.connect_blocks(
                    current_block_id,
                    condition_block_id,
                    FlowEdgeKind::Unconditional,
                    None,
                );

                let body_block_id = self.create_block();
                self.control_stack.push(ControlTarget {
                    kind: ControlTargetKind::Loop,
                    symbol: Some(loop_symbol),
                    break_target: exit_block_id,
                    continue_target: Some(condition_block_id),
                });

                let condition_exit_block_id = if let Some(condition_id) = condition_id {
                    self.build_expression(condition_id, condition_block_id)?
                } else {
                    condition_block_id
                };

                if let Some(condition_id) = condition_id {
                    self.connect_blocks(
                        condition_exit_block_id,
                        body_block_id,
                        FlowEdgeKind::True,
                        Some(condition_id),
                    );
                    self.connect_blocks(
                        condition_exit_block_id,
                        exit_block_id,
                        FlowEdgeKind::False,
                        Some(condition_id),
                    );
                } else {
                    self.connect_blocks(
                        condition_exit_block_id,
                        body_block_id,
                        FlowEdgeKind::Unconditional,
                        None,
                    );
                }

                let body_exit_block_id = self.build_block(body_id, body_block_id);

                self.control_stack.pop();

                if let Some(body_exit_block_id) = body_exit_block_id {
                    self.connect_blocks(
                        body_exit_block_id,
                        condition_block_id,
                        FlowEdgeKind::Unconditional,
                        None,
                    );
                }

                self.block_has_predecessors(exit_block_id)
                    .then_some(exit_block_id)
            }
            LoopKind::PostTest => {
                let body_block_id = self.create_block();
                self.connect_blocks(
                    current_block_id,
                    body_block_id,
                    FlowEdgeKind::Unconditional,
                    None,
                );

                let condition_block_id = self.create_block();
                self.control_stack.push(ControlTarget {
                    kind: ControlTargetKind::Loop,
                    symbol: Some(loop_symbol),
                    break_target: exit_block_id,
                    continue_target: Some(condition_block_id),
                });

                let body_exit_block_id = self.build_block(body_id, body_block_id);
                if let Some(body_exit_block_id) = body_exit_block_id {
                    self.connect_blocks(
                        body_exit_block_id,
                        condition_block_id,
                        FlowEdgeKind::Unconditional,
                        None,
                    );
                }

                let condition_exit_block_id = if let Some(condition_id) = condition_id {
                    self.build_expression(condition_id, condition_block_id)?
                } else {
                    condition_block_id
                };

                if let Some(condition_id) = condition_id {
                    self.connect_blocks(
                        condition_exit_block_id,
                        body_block_id,
                        FlowEdgeKind::True,
                        Some(condition_id),
                    );
                    self.connect_blocks(
                        condition_exit_block_id,
                        exit_block_id,
                        FlowEdgeKind::False,
                        Some(condition_id),
                    );
                } else {
                    self.connect_blocks(
                        condition_exit_block_id,
                        body_block_id,
                        FlowEdgeKind::Unconditional,
                        None,
                    );
                }

                self.control_stack.pop();

                self.block_has_predecessors(exit_block_id)
                    .then_some(exit_block_id)
            }
        }
    }

    /// Build a for each expression and return the exit block.
    fn build_for_each_expression(
        &mut self,
        binding: &ForEachBinding,
        iterator_id: LocalNodeId<Expression>,
        body_id: LocalNodeId<Block>,
        symbol: LocalSymbolId,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let iterator_exit_block_id = self.build_expression(iterator_id, current_block_id)?;
        let condition_block_id = self.create_block();
        let body_block_id = self.create_block();
        let exit_block_id = self.create_block();

        self.connect_blocks(
            iterator_exit_block_id,
            condition_block_id,
            FlowEdgeKind::Unconditional,
            None,
        );

        self.connect_blocks(condition_block_id, body_block_id, FlowEdgeKind::Guard, None);
        self.connect_blocks(condition_block_id, exit_block_id, FlowEdgeKind::Guard, None);

        let loop_symbol = symbol.into_global(self.module_id);
        self.control_stack.push(ControlTarget {
            kind: ControlTargetKind::Loop,
            symbol: Some(loop_symbol),
            break_target: exit_block_id,
            continue_target: Some(condition_block_id),
        });

        let mut body_entry_block_id = body_block_id;
        match binding {
            ForEachBinding::Pattern { pattern } | ForEachBinding::Using { pattern, .. } => {
                if let Some(pattern_exit_block_id) =
                    self.build_pattern(*pattern, body_entry_block_id)
                {
                    body_entry_block_id = pattern_exit_block_id;
                }
            }
        }

        let body_exit_block_id = self.build_block(body_id, body_entry_block_id);

        self.control_stack.pop();

        if let Some(body_exit_block_id) = body_exit_block_id {
            self.connect_blocks(
                body_exit_block_id,
                condition_block_id,
                FlowEdgeKind::Unconditional,
                None,
            );
        }

        self.block_has_predecessors(exit_block_id)
            .then_some(exit_block_id)
    }

    /// Build a for expression and return the exit block.
    fn build_for_expression(
        &mut self,
        initialization_id: Option<LocalNodeId<Expression>>,
        condition_id: Option<LocalNodeId<Expression>>,
        increment_id: Option<LocalNodeId<Expression>>,
        body_id: LocalNodeId<Block>,
        symbol: LocalSymbolId,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let mut initialization_block_id = current_block_id;
        if let Some(initialization_id) = initialization_id {
            initialization_block_id =
                self.build_expression(initialization_id, initialization_block_id)?;
        }

        let condition_block_id = self.create_block();
        self.connect_blocks(
            initialization_block_id,
            condition_block_id,
            FlowEdgeKind::Unconditional,
            None,
        );

        let body_block_id = self.create_block();
        let exit_block_id = self.create_block();
        let increment_block_id = if increment_id.is_some() {
            self.create_block()
        } else {
            condition_block_id
        };

        let loop_symbol = symbol.into_global(self.module_id);
        self.control_stack.push(ControlTarget {
            kind: ControlTargetKind::Loop,
            symbol: Some(loop_symbol),
            break_target: exit_block_id,
            continue_target: Some(increment_block_id),
        });

        let condition_exit_block_id = if let Some(condition_id) = condition_id {
            self.build_expression(condition_id, condition_block_id)?
        } else {
            condition_block_id
        };

        if let Some(condition_id) = condition_id {
            self.connect_blocks(
                condition_exit_block_id,
                body_block_id,
                FlowEdgeKind::True,
                Some(condition_id),
            );
            self.connect_blocks(
                condition_exit_block_id,
                exit_block_id,
                FlowEdgeKind::False,
                Some(condition_id),
            );
        } else {
            self.connect_blocks(
                condition_exit_block_id,
                body_block_id,
                FlowEdgeKind::Unconditional,
                None,
            );
        }

        let body_exit_block_id = self.build_block(body_id, body_block_id);
        if let Some(body_exit_block_id) = body_exit_block_id {
            self.connect_blocks(
                body_exit_block_id,
                increment_block_id,
                FlowEdgeKind::Unconditional,
                None,
            );
        }

        if let Some(increment_id) = increment_id {
            let increment_exit_block_id =
                self.build_expression(increment_id, increment_block_id)?;
            self.connect_blocks(
                increment_exit_block_id,
                condition_block_id,
                FlowEdgeKind::Unconditional,
                None,
            );
        }

        self.control_stack.pop();

        self.block_has_predecessors(exit_block_id)
            .then_some(exit_block_id)
    }

    /// Build a match expression and return the exit block when it exists.
    fn build_match_expression(
        &mut self,
        value_id: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
        source: MatchSource,
        symbol: LocalSymbolId,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let match_value_block_id = self.build_expression(value_id, current_block_id)?;
        let match_exit_block_id = self.create_block();

        let should_push_match = source == MatchSource::Match;
        if should_push_match {
            let match_symbol = symbol.into_global(self.module_id);
            self.control_stack.push(ControlTarget {
                kind: ControlTargetKind::Match,
                symbol: Some(match_symbol),
                break_target: match_exit_block_id,
                continue_target: None,
            });
        }

        for case_id in cases {
            let case_block_id = self.create_block();
            let guard = self.guard_for_match_case(*case_id);
            self.connect_blocks(
                match_value_block_id,
                case_block_id,
                FlowEdgeKind::MatchCase,
                guard,
            );

            let case_exit_block_id = self.build_match_case(*case_id, case_block_id);
            if let Some(case_exit_block_id) = case_exit_block_id {
                self.connect_blocks(
                    case_exit_block_id,
                    match_exit_block_id,
                    FlowEdgeKind::Unconditional,
                    None,
                );
            }
        }

        if should_push_match {
            self.control_stack.pop();
        }

        self.block_has_predecessors(match_exit_block_id)
            .then_some(match_exit_block_id)
    }

    /// Build a match case body.
    fn build_match_case(
        &mut self,
        case_id: LocalNodeId<MatchCase>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        self.record_node(current_block_id, case_id.into_any());
        let match_case = self.tree.get(case_id);

        match match_case {
            MatchCase::Expression { selector, body, .. } => {
                let selector_exit_block_id =
                    self.build_match_selector(selector, current_block_id)?;
                self.build_expression(*body, selector_exit_block_id)
            }
            MatchCase::Block { selector, body, .. } => {
                let selector_exit_block_id =
                    self.build_match_selector(selector, current_block_id)?;
                self.build_block(*body, selector_exit_block_id)
            }
        }
    }

    /// Build selector evaluation for a match case.
    fn build_match_selector(
        &mut self,
        selector: &MatchSelector,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        match selector {
            MatchSelector::Pattern { pattern, guard } => {
                let pattern_exit_block_id = self.build_pattern(*pattern, current_block_id)?;
                if let Some(guard_id) = guard {
                    return self.build_expression(*guard_id, pattern_exit_block_id);
                }
                Some(pattern_exit_block_id)
            }
            MatchSelector::Default => Some(current_block_id),
        }
    }

    /// Build a try expression without modeling control flow yet.
    fn build_try_expression(
        &mut self,
        try_expression_id: LocalNodeId<Expression>,
        catch_pattern_id: Option<LocalNodeId<Pattern>>,
        catch_expression_id: Option<LocalNodeId<Expression>>,
        finally_expression_id: Option<LocalNodeId<Expression>>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        // NOTE #Incomplete: build try catch finally control flow
        // linearize try catch finally to keep nodes in the graph
        // evaluate the try body first
        let mut active_block_id = self.build_expression(try_expression_id, current_block_id)?;

        // evaluate the catch pattern
        if let Some(catch_pattern_id) = catch_pattern_id
            && let Some(pattern_exit_block_id) =
                self.build_pattern(catch_pattern_id, active_block_id)
        {
            active_block_id = pattern_exit_block_id;
        }

        // evaluate the catch expression
        if let Some(catch_expression_id) = catch_expression_id {
            active_block_id = self.build_expression(catch_expression_id, active_block_id)?;
        }

        // evaluate the finally expression
        if let Some(finally_expression_id) = finally_expression_id {
            active_block_id = self.build_expression(finally_expression_id, active_block_id)?;
        }

        Some(active_block_id)
    }

    /// Build a return expression.
    fn build_return_expression(
        &mut self,
        value_id: Option<LocalNodeId<Expression>>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let mut return_block_id = current_block_id;
        if let Some(value_id) = value_id {
            return_block_id = self.build_expression(value_id, return_block_id)?;
        }

        let return_block_index = return_block_id.0 as usize;
        self.blocks[return_block_index].is_terminal = true;
        None
    }

    /// Build a throw expression.
    fn build_throw_expression(
        &mut self,
        value_id: LocalNodeId<Expression>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let throw_block_id = self.build_expression(value_id, current_block_id)?;
        let throw_block_index = throw_block_id.0 as usize;
        self.blocks[throw_block_index].is_terminal = true;
        None
    }

    /// Build a break expression.
    fn build_break_expression(
        &mut self,
        target_symbol: Option<GlobalSymbolId>,
        value_id: Option<LocalNodeId<Expression>>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let mut break_block_id = current_block_id;
        if let Some(value_id) = value_id {
            break_block_id = self.build_expression(value_id, break_block_id)?;
        }

        if let Some(target_block_id) = self.resolve_break_target(target_symbol) {
            self.connect_blocks(
                break_block_id,
                target_block_id,
                FlowEdgeKind::Unconditional,
                None,
            );
        }

        None
    }

    /// Build a continue expression.
    fn build_continue_expression(
        &mut self,
        target_symbol: Option<GlobalSymbolId>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        if let Some(target_block_id) = self.resolve_continue_target(target_symbol) {
            self.connect_blocks(
                current_block_id,
                target_block_id,
                FlowEdgeKind::Unconditional,
                None,
            );
        }

        None
    }

    /// Resolve the guard expression for a match case.
    fn guard_for_match_case(
        &self,
        case_id: LocalNodeId<MatchCase>,
    ) -> Option<LocalNodeId<Expression>> {
        let match_case = self.tree.get(case_id);
        let selector = match match_case {
            MatchCase::Expression { selector, .. } => selector,
            MatchCase::Block { selector, .. } => selector,
        };

        match selector {
            MatchSelector::Pattern { guard, .. } => *guard,
            MatchSelector::Default => None,
        }
    }

    /// Resolve the break target for a break expression.
    fn resolve_break_target(&self, target_symbol: Option<GlobalSymbolId>) -> Option<FlowBlockId> {
        if let Some(target_symbol) = target_symbol {
            for target in self.control_stack.iter().rev() {
                if target.symbol == Some(target_symbol) {
                    return Some(target.break_target);
                }
            }

            return None;
        }

        for target in self.control_stack.iter().rev() {
            if matches!(
                target.kind,
                ControlTargetKind::Loop | ControlTargetKind::Match
            ) {
                return Some(target.break_target);
            }
        }

        None
    }

    /// Resolve the continue target for a continue expression.
    fn resolve_continue_target(
        &self,
        target_symbol: Option<GlobalSymbolId>,
    ) -> Option<FlowBlockId> {
        if let Some(target_symbol) = target_symbol {
            for target in self.control_stack.iter().rev() {
                if target.kind == ControlTargetKind::Loop && target.symbol == Some(target_symbol) {
                    return target.continue_target;
                }
            }

            return None;
        }

        for target in self.control_stack.iter().rev() {
            if target.kind == ControlTargetKind::Loop {
                return target.continue_target;
            }
        }

        None
    }

    /// Build the subexpressions for non control expressions.
    fn build_expression_children(
        &mut self,
        expression: &Expression,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        match expression {
            Expression::Declaration { declaration } => {
                self.build_declaration_expression(*declaration, current_block_id)
            }
            Expression::UnresolvedImport { arguments, .. }
            | Expression::Import { arguments, .. } => {
                self.build_arguments(arguments.as_deref(), current_block_id)
            }
            Expression::UnresolvedReExport { .. }
            | Expression::ReExport { .. }
            | Expression::Export { .. } => Some(current_block_id),
            Expression::Let { declarators, .. } | Expression::Using { declarators, .. } => {
                self.build_declarators(declarators, current_block_id)
            }
            Expression::TypeUnary { .. }
            | Expression::TypeBinary { .. }
            | Expression::TypeConditional { .. }
            | Expression::TypeMapped { .. }
            | Expression::TypeIndex { .. }
            | Expression::TypeTemplateLiteral { .. }
            | Expression::TypeImport { .. }
            | Expression::TypeInfer { .. }
            | Expression::TypePredicate { .. }
            | Expression::PointerOf { .. } => Some(current_block_id),
            Expression::Unary { right, .. }
            | Expression::ValueOf { right, .. }
            | Expression::ReferenceOf { right, .. }
            | Expression::Maybe { left: right }
            | Expression::Must { left: right } => self.build_expression(*right, current_block_id),
            Expression::Binary { left, right, .. }
            | Expression::Assign { left, right }
            | Expression::AssignBinary { left, right, .. } => {
                let left_block_id = self.build_expression(*left, current_block_id)?;
                self.build_expression(*right, left_block_id)
            }
            Expression::Member {
                left,
                static_arguments,
                ..
            } => {
                let left_block_id = self.build_expression(*left, current_block_id)?;
                self.build_arguments(static_arguments.as_deref(), left_block_id)
            }
            Expression::Call {
                left,
                static_arguments,
                dynamic_arguments,
            } => {
                let left_block_id = self.build_expression(*left, current_block_id)?;
                let static_block_id =
                    self.build_arguments(static_arguments.as_deref(), left_block_id)?;
                self.build_arguments(Some(dynamic_arguments.as_slice()), static_block_id)
            }
            Expression::Index { left, right } => {
                let left_block_id = self.build_expression(*left, current_block_id)?;
                if let Some(right_id) = right {
                    return self.build_expression(*right_id, left_block_id);
                }
                Some(left_block_id)
            }
            Expression::New {
                left,
                static_arguments,
                dynamic_arguments,
            } => {
                let left_block_id = self.build_expression(*left, current_block_id)?;
                let static_block_id =
                    self.build_arguments(static_arguments.as_deref(), left_block_id)?;
                self.build_arguments(Some(dynamic_arguments), static_block_id)
            }
            Expression::Delete { value } => self.build_expression(*value, current_block_id),
            Expression::Await { expression }
            | Expression::AwaitMaybe { expression }
            | Expression::Comptime { body: expression } => {
                self.build_expression(*expression, current_block_id)
            }
            Expression::UnresolvedPath {
                static_arguments, ..
            }
            | Expression::LocalReference {
                static_arguments, ..
            }
            | Expression::ModuleReference {
                static_arguments, ..
            }
            | Expression::GlobalReference {
                static_arguments, ..
            } => self.build_arguments(static_arguments.as_deref(), current_block_id),
            Expression::ImportMeta | Expression::This => Some(current_block_id),
            Expression::Type { .. }
            | Expression::ScalarLiteral { .. }
            | Expression::TypeLiteral { .. }
            | Expression::Debugger
            | Expression::Stub
            | Expression::Error => Some(current_block_id),
            Expression::TemplateExpression { value } => {
                self.build_template_literal(value, current_block_id)
            }
            Expression::TaggedTemplateExpression { tag, value } => {
                let tag_block_id = self.build_expression(*tag, current_block_id)?;
                self.build_template_literal(value, tag_block_id)
            }
            Expression::RangeExpression { start, end, .. } => {
                let start_block_id = self.build_expression(*start, current_block_id)?;
                self.build_expression(*end, start_block_id)
            }
            Expression::ArrayExpression { elements } | Expression::TupleExpression { elements } => {
                self.build_arguments(Some(elements.as_slice()), current_block_id)
            }
            Expression::SequenceExpression { expressions } => {
                self.build_expression_sequence(expressions, current_block_id)
            }
            Expression::ObjectExpression { properties } => {
                self.build_properties(properties, current_block_id)
            }
            Expression::TreeExpression {
                left,
                arguments,
                elements,
            } => {
                let mut tree_block_id = current_block_id;
                if let Some(left_id) = left {
                    tree_block_id = self.build_expression(*left_id, tree_block_id)?;
                }

                tree_block_id = self.build_arguments(arguments.as_deref(), tree_block_id)?;
                self.build_arguments(elements.as_deref(), tree_block_id)
            }
            Expression::TaggedScalarExpression { ty, value } => {
                let ty_block_id = self.build_expression(*ty, current_block_id)?;
                self.build_expression(*value, ty_block_id)
            }
            Expression::TaggedTupleExpression { ty, elements } => {
                let ty_block_id = self.build_expression(*ty, current_block_id)?;
                self.build_arguments(Some(elements.as_slice()), ty_block_id)
            }
            Expression::TaggedObjectExpression { ty, properties } => {
                let ty_block_id = self.build_expression(*ty, current_block_id)?;
                self.build_properties(properties, ty_block_id)
            }
            Expression::Parenthesized { expression } => {
                self.build_expression(*expression, current_block_id)
            }
            Expression::Yield { value, .. } => {
                if let Some(value_id) = value {
                    return self.build_expression(*value_id, current_block_id);
                }
                Some(current_block_id)
            }
            Expression::Block { .. }
            | Expression::Statement { .. }
            | Expression::Labelled { .. }
            | Expression::If { .. }
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
            | Expression::Throw { .. } => {
                unreachable!("control expression handled in build_expression")
            }
        }
    }

    /// Build the expressions inside a declaration that executes immediately.
    fn build_declaration_expression(
        &mut self,
        declaration_id: LocalNodeId<Declaration>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let declaration = self.tree.get(declaration_id);

        match declaration {
            Declaration::Global { expressions, .. }
            | Declaration::Namespace { expressions, .. } => {
                self.build_expression_sequence(expressions, current_block_id)
            }
            _ => {
                // NOTE #Incomplete: declaration bodies and static blocks are not expanded
                Some(current_block_id)
            }
        }
    }

    /// Build a list of declarators.
    fn build_declarators(
        &mut self,
        declarators: &[LocalNodeId<Declarator>],
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let mut declarator_block_id = current_block_id;
        for declarator_id in declarators {
            declarator_block_id = self.build_declarator(*declarator_id, declarator_block_id)?;
        }
        Some(declarator_block_id)
    }

    /// Build a declarator initializer and pattern.
    fn build_declarator(
        &mut self,
        declarator_id: LocalNodeId<Declarator>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        self.record_node(current_block_id, declarator_id.into_any());
        let declarator = self.tree.get(declarator_id);

        let mut declarator_block_id = current_block_id;
        if let Some(value_id) = declarator.value {
            declarator_block_id = self.build_expression(value_id, declarator_block_id)?;
        }

        if let Some(pattern_exit_block_id) =
            self.build_pattern(declarator.pattern, declarator_block_id)
        {
            declarator_block_id = pattern_exit_block_id;
        }

        Some(declarator_block_id)
    }

    /// Build a list of arguments.
    fn build_arguments(
        &mut self,
        arguments: Option<&[LocalNodeId<Argument>]>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let Some(arguments) = arguments else {
            return Some(current_block_id);
        };

        let mut argument_block_id = current_block_id;
        for argument_id in arguments {
            argument_block_id = self.build_argument(*argument_id, argument_block_id)?;
        }
        Some(argument_block_id)
    }

    /// Build a single argument.
    fn build_argument(
        &mut self,
        argument_id: LocalNodeId<Argument>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        self.record_node(current_block_id, argument_id.into_any());
        let argument = self.tree.get(argument_id);
        self.build_expression(argument.value(), current_block_id)
    }

    /// Build a pattern and return the exit block when it exists.
    fn build_pattern(
        &mut self,
        pattern_id: LocalNodeId<Pattern>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        self.record_node(current_block_id, pattern_id.into_any());
        let pattern = self.tree.get(pattern_id);

        match pattern {
            Pattern::Wildcard => Some(current_block_id),
            Pattern::Must(inner)
            | Pattern::ReferenceOf { right: inner, .. }
            | Pattern::ValueOf { right: inner, .. } => self.build_pattern(*inner, current_block_id),
            Pattern::Binding { pattern, .. } => {
                if let Some(pattern_id) = pattern {
                    return self.build_pattern(*pattern_id, current_block_id);
                }
                Some(current_block_id)
            }
            Pattern::Expression { value } => self.build_expression(*value, current_block_id),
            Pattern::Range { start, end, .. } => {
                let mut range_block_id = current_block_id;
                if let Some(start_id) = start {
                    range_block_id = self.build_pattern(*start_id, range_block_id)?;
                }
                if let Some(end_id) = end {
                    return self.build_pattern(*end_id, range_block_id);
                }
                Some(range_block_id)
            }
            Pattern::Tuple { fields } | Pattern::Array { fields } | Pattern::Object { fields } => {
                self.build_pattern_fields(fields, current_block_id)
            }
            Pattern::TaggedTuple { ty, fields } | Pattern::TaggedObject { ty, fields } => {
                let ty_block_id = self.build_expression(*ty, current_block_id)?;
                self.build_pattern_fields(fields, ty_block_id)
            }
            Pattern::Union { patterns } => {
                let mut union_block_id = current_block_id;
                for pattern_id in patterns {
                    union_block_id = self.build_pattern(*pattern_id, union_block_id)?;
                }
                Some(union_block_id)
            }
        }
    }

    /// Build a list of pattern fields.
    fn build_pattern_fields(
        &mut self,
        fields: &[LocalNodeId<PatternField>],
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let mut field_block_id = current_block_id;
        for field_id in fields {
            field_block_id = self.build_pattern_field(*field_id, field_block_id)?;
        }
        Some(field_block_id)
    }

    /// Build a pattern field.
    fn build_pattern_field(
        &mut self,
        field_id: LocalNodeId<PatternField>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        self.record_node(current_block_id, field_id.into_any());
        let field = self.tree.get(field_id);

        match field {
            PatternField::Named {
                pattern, default, ..
            } => {
                let mut field_block_id = current_block_id;
                if let Some(pattern_id) = pattern {
                    field_block_id = self.build_pattern(*pattern_id, field_block_id)?;
                }
                if let Some(default_id) = default {
                    return self.build_expression(*default_id, field_block_id);
                }
                Some(field_block_id)
            }
            PatternField::Alias { default, .. } => {
                if let Some(default_id) = default {
                    return self.build_expression(*default_id, current_block_id);
                }
                Some(current_block_id)
            }
            PatternField::Positional { pattern } => self.build_pattern(*pattern, current_block_id),
            PatternField::Spread { .. } | PatternField::Elision => Some(current_block_id),
        }
    }

    /// Build a list of properties.
    fn build_properties(
        &mut self,
        properties: &[LocalNodeId<Property>],
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let mut property_block_id = current_block_id;
        for property_id in properties {
            property_block_id = self.build_property(*property_id, property_block_id)?;
        }
        Some(property_block_id)
    }

    /// Build a property expression.
    fn build_property(
        &mut self,
        property_id: LocalNodeId<Property>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        self.record_node(current_block_id, property_id.into_any());
        let property = self.tree.get(property_id);

        match property {
            Property::Field {
                key,
                value,
                default,
                ..
            } => {
                let mut property_block_id =
                    self.build_dynamic_key(key.as_ref(), current_block_id)?;
                if let Some(value_id) = value {
                    property_block_id = self.build_expression(*value_id, property_block_id)?;
                }
                if let Some(default_id) = default {
                    return self.build_expression(*default_id, property_block_id);
                }
                Some(property_block_id)
            }
            Property::Method { key, .. } => self.build_dynamic_key(key.as_ref(), current_block_id),
            Property::Spread { value, .. } => self.build_expression(*value, current_block_id),
        }
    }

    /// Build a dynamic key expression.
    fn build_dynamic_key(
        &mut self,
        key: Option<&DynamicKey>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let Some(key) = key else {
            return Some(current_block_id);
        };

        match key {
            DynamicKey::Expression(expression_id) => {
                self.build_expression(*expression_id, current_block_id)
            }
            DynamicKey::NamedExpression { key, .. } => {
                self.build_expression(*key, current_block_id)
            }
            DynamicKey::Name(_) | DynamicKey::Number(_) => Some(current_block_id),
        }
    }

    /// Build a template literal expression.
    fn build_template_literal(
        &mut self,
        literal: &TemplateLiteral,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        match literal {
            TemplateLiteral::String { .. } => Some(current_block_id),
            TemplateLiteral::InterpolatedString { arguments, .. } => {
                self.build_arguments(Some(arguments.as_slice()), current_block_id)
            }
        }
    }
}

/// Build a control flow graph for a single expression body.
pub fn build_flow_graph_for_body(
    module_id: ModuleId,
    body_id: LocalNodeId<Expression>,
    tree: &NodeTree,
) -> FlowGraph {
    let builder = FlowGraphBuilder::new(module_id, tree);
    builder.build(body_id)
}

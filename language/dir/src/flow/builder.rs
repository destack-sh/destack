use indexmap::IndexMap;

use destack_source::ModuleId;

use crate::{
    Argument, AssignPattern, AssignPatternField, BinaryOperator, Block, Declaration, Declarator,
    Expression, FlowBlock, FlowBlockId, FlowEdge, FlowEdgeKind, FlowGraph, FlowGuard,
    ForEachBinding, GenericArgument, GenericParameter, GlobalSymbolId, IfCondition, ImportTarget,
    Key, LocalNodeId, LocalNodeIdAny, LocalSymbolId, LoopKind, MatchCase, MatchKind, MatchSelector,
    MatchSource, Pattern, PatternField, Property, TemplateLiteral, Tree, TupleElement,
    TypeExpression, TypeMember, UnaryOperator,
};

/// Describe what kind of control target we are tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlTargetKind {
    /// Track loop targets for break and continue.
    Loop,
    /// Track switch targets for break.
    Switch,
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

/// Track finally blocks for abrupt control flow.
#[derive(Debug, Clone)]
struct FinallyTarget {
    /// The finally block to execute.
    finally_block: FlowBlockId,
    /// Exit targets to run after the finally block.
    exit_targets: Vec<FlowBlockId>,
}

/// Build a control flow graph from DIR expressions.
#[derive(Debug)]
pub struct FlowGraphBuilder<'tree> {
    /// Identify the module that owns the graph.
    module_id: ModuleId,
    /// Provide access to the DIR tree for traversal.
    tree: &'tree Tree,
    /// The blocks built so far.
    blocks: Vec<FlowBlock>,
    /// Map nodes to their containing block.
    block_by_node: IndexMap<LocalNodeIdAny, FlowBlockId>,
    /// Track control flow targets for break and continue.
    control_stack: Vec<ControlTarget>,
    /// Track finally blocks for abrupt control flow.
    finally_stack: Vec<FinallyTarget>,
}

impl<'tree> FlowGraphBuilder<'tree> {
    /// Create a new flow graph builder.
    pub fn new(module_id: ModuleId, tree: &'tree Tree) -> Self {
        Self {
            module_id,
            tree,
            blocks: Vec::new(),
            block_by_node: IndexMap::new(),
            control_stack: Vec::new(),
            finally_stack: Vec::new(),
        }
    }

    /// Build a control flow graph for a body expression.
    pub fn build(mut self, body_id: LocalNodeId<Expression>) -> FlowGraph {
        // create entry and exit blocks
        let entry_block = self.create_block();
        let body_exit_block = self.build_expression(body_id, entry_block);
        let exit_block = self.create_block();

        // connect fallthrough to the exit block
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
        // create entry and exit blocks
        let entry_block = self.create_block();
        let roots_exit_block = self.build_expression_sequence(roots, entry_block);
        let exit_block = self.create_block();

        // connect fallthrough to the exit block
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

    /// Create a new terminal block.
    fn create_terminal_block(&mut self) -> FlowBlockId {
        let block_id = self.create_block();
        let block_index = block_id.0 as usize;
        self.blocks[block_index].is_terminal = true;
        block_id
    }

    /// Record a node inside a block.
    fn record_node(&mut self, block_id: FlowBlockId, node_id: LocalNodeIdAny) {
        // skip nodes that are already recorded
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
        guard: Option<FlowGuard>,
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

    /// Get the innermost finally block when present.
    fn current_finally_block(&self) -> Option<FlowBlockId> {
        self.finally_stack.last().map(|target| target.finally_block)
    }

    /// Record an abrupt exit to run after all finally blocks.
    fn record_finally_exit(&mut self, final_target: FlowBlockId) {
        // skip when no finally stack is active
        if self.finally_stack.is_empty() {
            return;
        }

        // wire exits from inner to outer finally blocks
        for index in (0..self.finally_stack.len()).rev() {
            let next_target = if index > 0 {
                self.finally_stack[index - 1].finally_block
            } else {
                final_target
            };

            let target = &mut self.finally_stack[index];
            if !target.exit_targets.contains(&next_target) {
                target.exit_targets.push(next_target);
            }
        }
    }

    /// Build an expression and return the fallthrough block when it exists.
    fn build_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        self.record_node(current_block_id, expression_id.into_any());
        let expression = self.tree.get(expression_id);

        // dispatch based on expression kind
        match expression {
            Expression::Block(block) => self.build_block(*block, current_block_id),
            Expression::Labelled { body, symbol, .. } => {
                self.build_labelled_expression(*symbol, *body, current_block_id)
            }

            Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => self.build_if_expression(
                condition,
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
                kind,
                value,
                cases,
                source,
                symbol,
                ..
            } => self.build_match_expression(
                *kind,
                *value,
                cases,
                *source,
                *symbol,
                current_block_id,
            ),
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
                    Some(FlowGuard::Expression(left_id)),
                );
                // short circuit to the join when the left guard is false
                self.connect_blocks(
                    left_exit_block_id,
                    join_block_id,
                    FlowEdgeKind::False,
                    Some(FlowGuard::Expression(left_id)),
                );
            }
            BinaryOperator::Or => {
                // short circuit to the join when the left guard is true
                self.connect_blocks(
                    left_exit_block_id,
                    join_block_id,
                    FlowEdgeKind::True,
                    Some(FlowGuard::Expression(left_id)),
                );
                // flow to the right side when the left guard is false
                self.connect_blocks(
                    left_exit_block_id,
                    right_block_id,
                    FlowEdgeKind::False,
                    Some(FlowGuard::Expression(left_id)),
                );
            }
            _ => {
                unreachable!("non short circuit operator routed to build_short_circuit_expression")
            }
        }

        // evaluate the right side and connect to the join when it falls through
        if let Some(right_exit_block_id) = self.build_expression(right_id, right_block_id) {
            // connect right fallthrough into the join block
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
        let expression_ids = block.iter_expressions().collect::<Vec<_>>();
        self.build_expression_sequence(&expression_ids, current_block_id)
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

        // walk each expression in order
        for expression_id in expression_ids {
            // start a new unreachable block for the remainder
            if !is_reachable {
                active_block_id = self.create_block();
                is_reachable = true;
            }

            // update the active block from the expression fallthrough
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

        // register the labelled control target
        self.control_stack.push(ControlTarget {
            kind: ControlTargetKind::Label,
            symbol: Some(label_symbol),
            break_target,
            continue_target: None,
        });

        // build the labelled body
        let body_exit_block_id = self.build_expression(body_id, current_block_id);

        // pop the label target after building the body
        self.control_stack.pop();

        // connect fallthrough to the break target
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
        condition: &IfCondition,
        then_expression_id: LocalNodeId<Expression>,
        else_expression_id: Option<LocalNodeId<Expression>>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        match condition {
            IfCondition::Expression { condition } => self.build_if_expression_from_condition(
                *condition,
                then_expression_id,
                else_expression_id,
                current_block_id,
            ),
            IfCondition::Let { declarator, .. } => self.build_if_let_expression(
                *declarator,
                then_expression_id,
                else_expression_id,
                current_block_id,
            ),
        }
    }

    /// Build an if expression with a regular condition and return the join block when it exists.
    fn build_if_expression_from_condition(
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

        // determine if either branch can fall through
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

        // connect branch exits to the join block
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

    /// Build an if let expression and return the join block when it exists.
    fn build_if_let_expression(
        &mut self,
        declarator_id: LocalNodeId<Declarator>,
        then_expression_id: LocalNodeId<Expression>,
        else_expression_id: Option<LocalNodeId<Expression>>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let declarator = self.tree.get(declarator_id);
        let Declarator {
            pattern,
            ty: _,
            value,
        } = declarator;
        let value_id = value.as_ref().copied()?;

        // allocate branch blocks
        let then_block_id = self.create_block();
        let join_block_id = self.create_block();
        let has_else_expression = else_expression_id.is_some();
        let else_block_id = has_else_expression.then_some(self.create_block());
        let false_target_id = else_block_id.unwrap_or(join_block_id);

        // evaluate the value expression before branching
        let value_exit_block_id = self.build_expression(value_id, current_block_id)?;
        self.connect_blocks(
            value_exit_block_id,
            then_block_id,
            FlowEdgeKind::True,
            Some(FlowGuard::Pattern {
                value: value_id,
                pattern: *pattern,
            }),
        );
        self.connect_blocks(
            value_exit_block_id,
            false_target_id,
            FlowEdgeKind::False,
            Some(FlowGuard::Pattern {
                value: value_id,
                pattern: *pattern,
            }),
        );

        // evaluate then branch
        let then_exit_block_id = self.build_expression(then_expression_id, then_block_id);

        // evaluate else branch
        let mut else_exit_block_id = None;
        if let (Some(else_expression_id), Some(else_block_id)) = (else_expression_id, else_block_id)
        {
            else_exit_block_id = self.build_expression(else_expression_id, else_block_id);
        }

        // determine if either branch can fall through
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

        // connect branch exits to the join block
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

        // decide how to expand the guard expression
        match self.tree.get(guard_id) {
            Expression::Unary {
                operator: UnaryOperator::Not,
                right,
            } => {
                // invert guard targets for logical not
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
                // wire guard edges to true and false targets
                self.connect_blocks(
                    guard_exit_block_id,
                    true_block_id,
                    FlowEdgeKind::True,
                    Some(FlowGuard::Expression(guard_id)),
                );
                self.connect_blocks(
                    guard_exit_block_id,
                    false_block_id,
                    FlowEdgeKind::False,
                    Some(FlowGuard::Expression(guard_id)),
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
        // peel off parenthesized layers
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

        // dispatch based on loop kind
        match kind {
            LoopKind::NoTest => {
                // connect entry to the loop body
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

                // build the loop body
                let body_exit_block_id = self.build_block(body_id, body_block_id);

                // pop loop control targets after the body
                self.control_stack.pop();

                // loop back to the body block
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
                // build condition and body blocks for the loop
                let condition_block_id = self.create_block();
                self.connect_blocks(
                    current_block_id,
                    condition_block_id,
                    FlowEdgeKind::Unconditional,
                    None,
                );

                // body block
                let body_block_id = self.create_block();
                self.control_stack.push(ControlTarget {
                    kind: ControlTargetKind::Loop,
                    symbol: Some(loop_symbol),
                    break_target: exit_block_id,
                    continue_target: Some(condition_block_id),
                });

                // evaluate the loop condition
                let condition_exit_block_id = if let Some(condition_id) = condition_id {
                    self.build_expression(condition_id, condition_block_id)?
                } else {
                    condition_block_id
                };

                // connect condition to body or exit
                if let Some(condition_id) = condition_id {
                    self.connect_blocks(
                        condition_exit_block_id,
                        body_block_id,
                        FlowEdgeKind::True,
                        Some(FlowGuard::Expression(condition_id)),
                    );
                    self.connect_blocks(
                        condition_exit_block_id,
                        exit_block_id,
                        FlowEdgeKind::False,
                        Some(FlowGuard::Expression(condition_id)),
                    );
                } else {
                    self.connect_blocks(
                        condition_exit_block_id,
                        body_block_id,
                        FlowEdgeKind::Unconditional,
                        None,
                    );
                }

                // build the loop body
                let body_exit_block_id = self.build_block(body_id, body_block_id);

                // pop loop control targets after the body
                self.control_stack.pop();

                if let Some(body_exit_block_id) = body_exit_block_id {
                    // loop back to the condition block
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
                // build body and condition blocks for the loop
                let body_block_id = self.create_block();
                self.connect_blocks(
                    current_block_id,
                    body_block_id,
                    FlowEdgeKind::Unconditional,
                    None,
                );

                // condition block
                let condition_block_id = self.create_block();
                self.control_stack.push(ControlTarget {
                    kind: ControlTargetKind::Loop,
                    symbol: Some(loop_symbol),
                    break_target: exit_block_id,
                    continue_target: Some(condition_block_id),
                });

                // evaluate the loop body
                let body_exit_block_id = self.build_block(body_id, body_block_id);
                if let Some(body_exit_block_id) = body_exit_block_id {
                    // connect body to the condition block
                    self.connect_blocks(
                        body_exit_block_id,
                        condition_block_id,
                        FlowEdgeKind::Unconditional,
                        None,
                    );
                }

                // evaluate the loop condition
                let condition_exit_block_id = if let Some(condition_id) = condition_id {
                    self.build_expression(condition_id, condition_block_id)?
                } else {
                    condition_block_id
                };

                // connect condition to body or exit
                if let Some(condition_id) = condition_id {
                    self.connect_blocks(
                        condition_exit_block_id,
                        body_block_id,
                        FlowEdgeKind::True,
                        Some(FlowGuard::Expression(condition_id)),
                    );
                    self.connect_blocks(
                        condition_exit_block_id,
                        exit_block_id,
                        FlowEdgeKind::False,
                        Some(FlowGuard::Expression(condition_id)),
                    );
                } else {
                    self.connect_blocks(
                        condition_exit_block_id,
                        body_block_id,
                        FlowEdgeKind::Unconditional,
                        None,
                    );
                }

                // pop loop control targets after the body
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
        // evaluate the iterator expression
        let iterator_exit_block_id = self.build_expression(iterator_id, current_block_id)?;
        let condition_block_id = self.create_block();
        let body_block_id = self.create_block();
        let exit_block_id = self.create_block();

        // connect iterator to the condition block
        self.connect_blocks(
            iterator_exit_block_id,
            condition_block_id,
            FlowEdgeKind::Unconditional,
            None,
        );

        // wire condition to body and exit
        self.connect_blocks(condition_block_id, body_block_id, FlowEdgeKind::Guard, None);
        self.connect_blocks(condition_block_id, exit_block_id, FlowEdgeKind::Guard, None);

        // register loop control targets
        let loop_symbol = symbol.into_global(self.module_id);
        self.control_stack.push(ControlTarget {
            kind: ControlTargetKind::Loop,
            symbol: Some(loop_symbol),
            break_target: exit_block_id,
            continue_target: Some(condition_block_id),
        });

        // evaluate the binding pattern before the loop body
        let mut body_entry_block_id = body_block_id;
        match binding {
            ForEachBinding::Pattern { pattern, .. } | ForEachBinding::Using { pattern, .. } => {
                if let Some(pattern_exit_block_id) =
                    self.build_pattern(*pattern, body_entry_block_id)
                {
                    body_entry_block_id = pattern_exit_block_id;
                }
            }
        }

        // evaluate the loop body
        let body_exit_block_id = self.build_block(body_id, body_entry_block_id);

        // pop loop control targets after the body
        self.control_stack.pop();

        if let Some(body_exit_block_id) = body_exit_block_id {
            // loop back to the condition block
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
        // evaluate the initialization expression
        let mut initialization_block_id = current_block_id;
        if let Some(initialization_id) = initialization_id {
            initialization_block_id =
                self.build_expression(initialization_id, initialization_block_id)?;
        }

        // connect initialization to the condition block
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

        // register loop control targets
        let loop_symbol = symbol.into_global(self.module_id);
        self.control_stack.push(ControlTarget {
            kind: ControlTargetKind::Loop,
            symbol: Some(loop_symbol),
            break_target: exit_block_id,
            continue_target: Some(increment_block_id),
        });

        // evaluate the loop condition
        let condition_exit_block_id = if let Some(condition_id) = condition_id {
            self.build_expression(condition_id, condition_block_id)?
        } else {
            condition_block_id
        };

        // connect condition to body or exit
        if let Some(condition_id) = condition_id {
            self.connect_blocks(
                condition_exit_block_id,
                body_block_id,
                FlowEdgeKind::True,
                Some(FlowGuard::Expression(condition_id)),
            );
            self.connect_blocks(
                condition_exit_block_id,
                exit_block_id,
                FlowEdgeKind::False,
                Some(FlowGuard::Expression(condition_id)),
            );
        } else {
            self.connect_blocks(
                condition_exit_block_id,
                body_block_id,
                FlowEdgeKind::Unconditional,
                None,
            );
        }

        // evaluate the loop body
        let body_exit_block_id = self.build_block(body_id, body_block_id);
        if let Some(body_exit_block_id) = body_exit_block_id {
            // connect body to the increment block
            self.connect_blocks(
                body_exit_block_id,
                increment_block_id,
                FlowEdgeKind::Unconditional,
                None,
            );
        }

        // evaluate the increment expression
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

        // pop loop control targets after the loop body
        self.control_stack.pop();

        self.block_has_predecessors(exit_block_id)
            .then_some(exit_block_id)
    }

    /// Build a match expression and return the exit block when it exists.
    fn build_match_expression(
        &mut self,
        kind: MatchKind,
        value_id: LocalNodeId<Expression>,
        cases: &[LocalNodeId<MatchCase>],
        _source: MatchSource,
        symbol: LocalSymbolId,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        // evaluate the match value
        let match_value_block_id = self.build_expression(value_id, current_block_id)?;
        let match_exit_block_id = self.create_block();

        // register match control targets when needed
        let should_push_switch = kind == MatchKind::Switch;
        if should_push_switch {
            let match_symbol = symbol.into_global(self.module_id);
            self.control_stack.push(ControlTarget {
                kind: ControlTargetKind::Switch,
                symbol: Some(match_symbol),
                break_target: match_exit_block_id,
                continue_target: None,
            });
        }

        // build each match case
        for case_id in cases {
            let case_block_id = self.create_block();
            let guard = self.guard_for_match_case(value_id, *case_id);
            self.connect_blocks(
                match_value_block_id,
                case_block_id,
                FlowEdgeKind::Case,
                guard,
            );

            // evaluate the case body
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

        // pop match control targets after processing cases
        if should_push_switch {
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
                // evaluate selector then expression body
                self.build_match_case_body(selector, current_block_id, |builder, block_id| {
                    builder.build_expression(*body, block_id)
                })
            }
            MatchCase::Block { selector, body, .. } => {
                // evaluate selector then block body
                self.build_match_case_body(selector, current_block_id, |builder, block_id| {
                    builder.build_block(*body, block_id)
                })
            }
        }
    }

    /// Build selector evaluation and body for a match case.
    fn build_match_case_body(
        &mut self,
        selector: &MatchSelector,
        current_block_id: FlowBlockId,
        build_body: impl FnOnce(&mut Self, FlowBlockId) -> Option<FlowBlockId>,
    ) -> Option<FlowBlockId> {
        match selector {
            MatchSelector::Pattern { pattern, guard } => {
                // evaluate the pattern before any guard
                let pattern_exit_block_id = self.build_pattern(*pattern, current_block_id)?;

                // evaluate the guard before the body when present
                if let Some(guard_id) = guard {
                    let guard_block_id = self.create_block();
                    self.connect_blocks(
                        pattern_exit_block_id,
                        guard_block_id,
                        FlowEdgeKind::Unconditional,
                        None,
                    );

                    let guard_exit_block_id = self.build_expression(*guard_id, guard_block_id)?;
                    let body_block_id = self.create_block();
                    self.connect_blocks(
                        guard_exit_block_id,
                        body_block_id,
                        FlowEdgeKind::Guard,
                        Some(FlowGuard::Expression(*guard_id)),
                    );

                    return build_body(self, body_block_id);
                }

                build_body(self, pattern_exit_block_id)
            }
            MatchSelector::Default => Some(current_block_id),
        }
    }

    /// Build a try expression with catch and finally control flow.
    fn build_try_expression(
        &mut self,
        try_expression_id: LocalNodeId<Expression>,
        catch_pattern_id: Option<LocalNodeId<Pattern>>,
        catch_expression_id: Option<LocalNodeId<Expression>>,
        finally_expression_id: Option<LocalNodeId<Expression>>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        // allocate blocks for try, catch, and finally
        let try_block_id = self.create_block();
        let catch_block_id = catch_expression_id.map(|_| self.create_block());
        let finally_block_id = finally_expression_id.map(|_| self.create_block());
        let join_block_id = self.create_block();

        // connect the current block into the try body
        self.connect_blocks(
            current_block_id,
            try_block_id,
            FlowEdgeKind::Unconditional,
            None,
        );

        // enable finally tracking while building the try body
        if let Some(finally_block_id) = finally_block_id {
            self.finally_stack.push(FinallyTarget {
                finally_block: finally_block_id,
                exit_targets: Vec::new(),
            });
        }

        // evaluate the try body
        let try_exit_block_id = self.build_expression(try_expression_id, try_block_id);

        // evaluate the catch body
        let mut catch_exit_block_id = None;
        if let (Some(catch_block_id), Some(catch_expression_id)) =
            (catch_block_id, catch_expression_id)
        {
            // connect the catch block when present
            self.connect_blocks(
                current_block_id,
                catch_block_id,
                FlowEdgeKind::Unconditional,
                None,
            );

            // evaluate the catch pattern before the catch body
            let mut catch_block_id = catch_block_id;
            if let Some(catch_pattern_id) = catch_pattern_id
                && let Some(pattern_exit_block_id) =
                    self.build_pattern(catch_pattern_id, catch_block_id)
            {
                catch_block_id = pattern_exit_block_id;
            }

            // evaluate the catch expression
            catch_exit_block_id = self.build_expression(catch_expression_id, catch_block_id);
        }

        // compute try and catch fallthrough reachability
        let try_reachable = try_exit_block_id
            .map(|block_id| self.block_has_predecessors(block_id))
            .unwrap_or(false);
        let catch_reachable = catch_exit_block_id
            .map(|block_id| self.block_has_predecessors(block_id))
            .unwrap_or(false);
        let has_fallthrough = try_reachable || catch_reachable;

        // wire try and catch exits through finally when present
        if let Some(finally_block_id) = finally_block_id {
            // connect try and catch exits into finally
            if try_reachable && let Some(try_exit_block_id) = try_exit_block_id {
                self.connect_blocks(
                    try_exit_block_id,
                    finally_block_id,
                    FlowEdgeKind::Unconditional,
                    None,
                );
            }
            if catch_reachable && let Some(catch_exit_block_id) = catch_exit_block_id {
                self.connect_blocks(
                    catch_exit_block_id,
                    finally_block_id,
                    FlowEdgeKind::Unconditional,
                    None,
                );
            }

            // unwind finally tracking before evaluating the finally block
            let finally_target = self.finally_stack.pop();

            // evaluate the finally expression
            let finally_exit_block_id = if let Some(finally_expression_id) = finally_expression_id {
                self.build_expression(finally_expression_id, finally_block_id)
            } else {
                Some(finally_block_id)
            };

            // connect normal fallthrough out of finally
            if has_fallthrough && let Some(finally_exit_block_id) = finally_exit_block_id {
                self.connect_blocks(
                    finally_exit_block_id,
                    join_block_id,
                    FlowEdgeKind::Unconditional,
                    None,
                );
            }

            // connect abrupt exits out of finally
            if let (Some(finally_exit_block_id), Some(finally_target)) =
                (finally_exit_block_id, finally_target)
            {
                for target in finally_target.exit_targets {
                    self.connect_blocks(
                        finally_exit_block_id,
                        target,
                        FlowEdgeKind::Unconditional,
                        None,
                    );
                }
            }

            return has_fallthrough
                .then_some(join_block_id)
                .filter(|block_id| self.block_has_predecessors(*block_id));
        }

        // connect try and catch exits into the join block
        if try_reachable && let Some(try_exit_block_id) = try_exit_block_id {
            self.connect_blocks(
                try_exit_block_id,
                join_block_id,
                FlowEdgeKind::Unconditional,
                None,
            );
        }
        if catch_reachable && let Some(catch_exit_block_id) = catch_exit_block_id {
            self.connect_blocks(
                catch_exit_block_id,
                join_block_id,
                FlowEdgeKind::Unconditional,
                None,
            );
        }

        has_fallthrough
            .then_some(join_block_id)
            .filter(|block_id| self.block_has_predecessors(*block_id))
    }

    /// Build a return expression.
    fn build_return_expression(
        &mut self,
        value_id: Option<LocalNodeId<Expression>>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        // evaluate the return value when present
        let mut return_block_id = current_block_id;
        if let Some(value_id) = value_id {
            return_block_id = self.build_expression(value_id, return_block_id)?;
        }

        // route return through finally when needed
        if let Some(finally_block_id) = self.current_finally_block() {
            let return_target = self.create_terminal_block();
            self.connect_blocks(
                return_block_id,
                finally_block_id,
                FlowEdgeKind::Unconditional,
                None,
            );
            self.record_finally_exit(return_target);
            return None;
        }

        // mark the current block as terminal when no finally is active
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
        // evaluate the throw value
        let throw_block_id = self.build_expression(value_id, current_block_id)?;

        // route throw through finally when needed
        if let Some(finally_block_id) = self.current_finally_block() {
            let throw_target = self.create_terminal_block();
            self.connect_blocks(
                throw_block_id,
                finally_block_id,
                FlowEdgeKind::Unconditional,
                None,
            );
            self.record_finally_exit(throw_target);
            return None;
        }

        // mark the current block as terminal when no finally is active
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
        // evaluate the break value when present
        let mut break_block_id = current_block_id;
        if let Some(value_id) = value_id {
            break_block_id = self.build_expression(value_id, break_block_id)?;
        }

        if let Some(target_block_id) = self.resolve_break_target(target_symbol) {
            // route break through finally when needed
            if let Some(finally_block_id) = self.current_finally_block() {
                self.connect_blocks(
                    break_block_id,
                    finally_block_id,
                    FlowEdgeKind::Unconditional,
                    None,
                );
                self.record_finally_exit(target_block_id);
                return None;
            }

            // connect break to its target
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
            // route continue through finally when needed
            if let Some(finally_block_id) = self.current_finally_block() {
                self.connect_blocks(
                    current_block_id,
                    finally_block_id,
                    FlowEdgeKind::Unconditional,
                    None,
                );
                self.record_finally_exit(target_block_id);
                return None;
            }

            // connect continue to its target
            self.connect_blocks(
                current_block_id,
                target_block_id,
                FlowEdgeKind::Unconditional,
                None,
            );
        }

        None
    }

    /// Resolve the guard for a match case.
    fn guard_for_match_case(
        &self,
        value_id: LocalNodeId<Expression>,
        case_id: LocalNodeId<MatchCase>,
    ) -> Option<FlowGuard> {
        let match_case = self.tree.get(case_id);
        let selector = match match_case {
            MatchCase::Expression { selector, .. } => selector,
            MatchCase::Block { selector, .. } => selector,
        };

        match selector {
            MatchSelector::Pattern { pattern, .. } => Some(FlowGuard::Pattern {
                value: value_id,
                pattern: *pattern,
            }),
            MatchSelector::Default => None,
        }
    }

    /// Resolve the break target for a break expression.
    fn resolve_break_target(&self, target_symbol: Option<GlobalSymbolId>) -> Option<FlowBlockId> {
        if let Some(target_symbol) = target_symbol {
            // find a matching labelled target
            for target in self.control_stack.iter().rev() {
                if target.symbol == Some(target_symbol) {
                    return Some(target.break_target);
                }
            }

            return None;
        }

        // fall back to the innermost loop or match
        for target in self.control_stack.iter().rev() {
            if matches!(
                target.kind,
                ControlTargetKind::Loop | ControlTargetKind::Switch
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
            // find a matching labelled loop target
            for target in self.control_stack.iter().rev() {
                if target.kind == ControlTargetKind::Loop && target.symbol == Some(target_symbol) {
                    return target.continue_target;
                }
            }

            return None;
        }

        // fall back to the innermost loop
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
        // walk child expressions based on expression shape
        match expression {
            Expression::Declaration(declaration) => {
                self.build_declaration_expression(*declaration, current_block_id)
            }
            Expression::UnresolvedImport {
                target, arguments, ..
            } => {
                let current_block_id = match target {
                    ImportTarget::String(_) => Some(current_block_id),
                    ImportTarget::Expression { target } => {
                        self.build_expression(*target, current_block_id)
                    }
                };
                let current_block_id = current_block_id?;
                self.build_arguments(arguments.as_deref(), current_block_id)
            }
            Expression::Import { arguments, .. } => {
                self.build_arguments(arguments.as_deref(), current_block_id)
            }
            Expression::UnresolvedReExport { .. }
            | Expression::ReExport { .. }
            | Expression::Export { .. } => Some(current_block_id),
            Expression::ExportNamespace { .. } => Some(current_block_id),
            Expression::Let { declarators, .. } => {
                self.build_declarators(declarators, current_block_id)
            }
            Expression::LetElse {
                declarator,
                else_branch,
                ..
            } => {
                self.record_node(current_block_id, (*declarator).into_any());
                let declarator = self.tree.get(*declarator);

                // evaluate the initializer before the branch split
                let branch_entry_block_id =
                    self.build_declarator_value(declarator.value, current_block_id)?;

                // follow the success path through the pattern
                let declarator_block_id =
                    self.build_pattern(declarator.pattern, branch_entry_block_id)?;

                // follow the failure path through the else branch
                let _else_branch_block_id =
                    self.build_expression(*else_branch, branch_entry_block_id);

                Some(declarator_block_id)
            }
            Expression::Using { declarators, .. } => {
                self.build_declarators(declarators, current_block_id)
            }
            Expression::PointerOf { right, .. } => self.build_expression(*right, current_block_id),
            Expression::As {
                operator: _,
                source: _,
                expression: value,
                target_type: _,
            }
            | Expression::Satisfies {
                expression: value,
                target_type: _,
            } => self.build_expression(*value, current_block_id),
            Expression::Unary { right, .. }
            | Expression::ValueOf { right, .. }
            | Expression::ReferenceOf { right, .. }
            | Expression::Maybe { left: right }
            | Expression::Must { left: right } => self.build_expression(*right, current_block_id),
            Expression::Is { value, target_type } => {
                let value_block_id = self.build_expression(*value, current_block_id)?;
                self.build_type_expression(*target_type, value_block_id)
            }
            Expression::InstanceOf { value, target } => {
                let value_block_id = self.build_expression(*value, current_block_id)?;
                self.build_expression(*target, value_block_id)
            }
            Expression::Binary { left, right, .. }
            | Expression::AssignBinary { left, right, .. } => {
                let left_block_id = self.build_expression(*left, current_block_id)?;
                self.build_expression(*right, left_block_id)
            }
            Expression::Assign { left, right } => {
                // evaluate the rhs before any destructuring targets or defaults
                let right_block_id = self.build_expression(*right, current_block_id)?;
                self.build_assign_pattern(*left, right_block_id)
            }
            Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
                self.build_expression(*left, current_block_id)
            }
            Expression::Instantiation {
                left,
                generic_arguments,
            } => {
                let left_block_id = self.build_expression(*left, current_block_id)?;
                self.build_generic_arguments(generic_arguments.as_slice(), left_block_id)
            }
            Expression::Call {
                left,
                generic_arguments,
                arguments,
            } => {
                let left_block_id = self.build_expression(*left, current_block_id)?;
                let generic_block_id =
                    self.build_generic_arguments(generic_arguments.as_slice(), left_block_id)?;
                self.build_arguments(Some(arguments.as_slice()), generic_block_id)
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
                generic_arguments,
                arguments,
            } => {
                let left_block_id = self.build_expression(*left, current_block_id)?;
                let generic_block_id =
                    self.build_generic_arguments(generic_arguments.as_slice(), left_block_id)?;
                self.build_arguments(Some(arguments.as_slice()), generic_block_id)
            }
            Expression::Delete { value } => self.build_expression(*value, current_block_id),
            Expression::Await { expression }
            | Expression::AwaitMaybe { expression }
            | Expression::Comptime { body: expression } => {
                self.build_expression(*expression, current_block_id)
            }
            Expression::UnresolvedPath {
                generic_arguments, ..
            }
            | Expression::LocalReference {
                generic_arguments, ..
            }
            | Expression::ModuleReference {
                generic_arguments, ..
            }
            | Expression::GlobalReference {
                generic_arguments, ..
            } => self.build_generic_arguments(generic_arguments.as_slice(), current_block_id),
            Expression::PrivateIdentifier { .. }
            | Expression::ImportMeta
            | Expression::NewTarget
            | Expression::This
            | Expression::Super => Some(current_block_id),
            Expression::Type { value, .. } => self.build_type_expression(*value, current_block_id),
            Expression::ScalarLiteral { .. }
            | Expression::TypeLiteral { .. }
            | Expression::Debugger
            | Expression::Missing
            | Expression::Stub
            | Expression::Error => Some(current_block_id),
            Expression::TemplateExpression { value } => {
                self.build_template_literal(value, current_block_id)
            }
            Expression::TaggedTemplateExpression {
                tag,
                generic_arguments,
                value,
            } => {
                let tag_block_id = self.build_expression(*tag, current_block_id)?;
                let tag_block_id =
                    self.build_generic_arguments(generic_arguments.as_slice(), tag_block_id)?;
                self.build_template_literal(value, tag_block_id)
            }
            Expression::ArrayExpression { elements } | Expression::TupleExpression { elements } => {
                self.build_arguments(Some(elements.as_slice()), current_block_id)
            }
            Expression::SequenceExpression { expressions } => {
                self.build_expression_sequence(expressions, current_block_id)
            }
            Expression::ObjectExpression { ty, properties } => {
                let current_block_id = if let Some(type_id) = ty {
                    self.build_type_expression(*type_id, current_block_id)?
                } else {
                    current_block_id
                };

                self.build_properties(properties, current_block_id)
            }
            Expression::TreeExpression {
                left,
                generic_arguments,
                arguments,
                elements,
            } => {
                let mut tree_block_id = current_block_id;
                if let Some(left_id) = left {
                    tree_block_id = self.build_expression(*left_id, tree_block_id)?;
                }
                tree_block_id =
                    self.build_generic_arguments(generic_arguments.as_slice(), tree_block_id)?;
                tree_block_id = self.build_arguments(arguments.as_deref(), tree_block_id)?;
                self.build_arguments(elements.as_deref(), tree_block_id)
            }
            Expression::TaggedScalarExpression { ty, value } => {
                let ty_block_id = self.build_type_expression(*ty, current_block_id)?;
                self.build_expression(*value, ty_block_id)
            }
            Expression::TaggedTupleExpression { ty, elements } => {
                let ty_block_id = self.build_type_expression(*ty, current_block_id)?;
                self.build_arguments(Some(elements.as_slice()), ty_block_id)
            }
            Expression::TaggedObjectExpression { ty, properties } => {
                let ty_block_id = self.build_type_expression(*ty, current_block_id)?;
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
            Expression::Block(..)
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
            Declaration::Global(declaration) => {
                self.build_expression_sequence(&declaration.expressions, current_block_id)
            }
            Declaration::Namespace(declaration) => {
                self.build_expression_sequence(&declaration.expressions, current_block_id)
            }
            _ => {
                // skip declaration bodies for now
                // NOTE #Incomplete: declaration bodies and static blocks are not expanded in flow
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

        // evaluate the initializer first
        let mut declarator_block_id =
            self.build_declarator_value(declarator.value, current_block_id)?;

        // evaluate the binding pattern next
        if let Some(pattern_exit_block_id) =
            self.build_pattern(declarator.pattern, declarator_block_id)
        {
            declarator_block_id = pattern_exit_block_id;
        }

        Some(declarator_block_id)
    }

    /// Build one declarator initializer when it exists.
    fn build_declarator_value(
        &mut self,
        value_id: Option<LocalNodeId<Expression>>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        // skip missing initializers
        let Some(value_id) = value_id else {
            return Some(current_block_id);
        };

        // evaluate the initializer expression
        self.build_expression(value_id, current_block_id)
    }

    /// Build a list of generic arguments.
    fn build_generic_arguments(
        &mut self,
        arguments: &[LocalNodeId<GenericArgument>],
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        // evaluate each argument in order
        let mut argument_block_id = current_block_id;
        for argument_id in arguments {
            argument_block_id = self.build_generic_argument(*argument_id, argument_block_id)?;
        }
        Some(argument_block_id)
    }

    /// Build a single generic argument.
    fn build_generic_argument(
        &mut self,
        argument_id: LocalNodeId<GenericArgument>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        self.record_node(current_block_id, argument_id.into_any());
        let argument = self.tree.get(argument_id);

        // evaluate the argument payload
        match argument {
            GenericArgument::Type { value } => self.build_type_expression(*value, current_block_id),
            GenericArgument::Value { value } => self.build_expression(*value, current_block_id),
            GenericArgument::Error => Some(current_block_id),
        }
    }

    /// Build a list of tuple elements.
    fn build_tuple_elements(
        &mut self,
        elements: &[LocalNodeId<TupleElement>],
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let mut element_block_id = current_block_id;
        for element_id in elements {
            element_block_id = self.build_tuple_element(*element_id, element_block_id)?;
        }
        Some(element_block_id)
    }

    /// Build a single tuple element.
    fn build_tuple_element(
        &mut self,
        element_id: LocalNodeId<TupleElement>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        self.record_node(current_block_id, element_id.into_any());
        let element = self.tree.get(element_id);

        match element {
            TupleElement::Element { value, .. } | TupleElement::Spread { value, .. } => {
                self.build_type_expression(*value, current_block_id)
            }
            TupleElement::Error => Some(current_block_id),
        }
    }

    /// Build a type expression.
    fn build_type_expression(
        &mut self,
        type_expression_id: LocalNodeId<TypeExpression>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        self.record_node(current_block_id, type_expression_id.into_any());
        let type_expression = self.tree.get(type_expression_id);

        match type_expression {
            TypeExpression::Parenthesized { expression } => {
                self.build_type_expression(*expression, current_block_id)
            }
            TypeExpression::ScalarLiteral { .. }
            | TypeExpression::Literal { .. }
            | TypeExpression::Intrinsic
            | TypeExpression::Const
            | TypeExpression::This
            | TypeExpression::Missing
            | TypeExpression::Error => Some(current_block_id),
            TypeExpression::Tuple { elements } | TypeExpression::ArrayTuple { elements } => {
                self.build_tuple_elements(elements, current_block_id)
            }
            TypeExpression::Array { element } => {
                self.build_type_expression(*element, current_block_id)
            }
            TypeExpression::Object { members } => {
                self.build_type_members(members, current_block_id)
            }
            TypeExpression::Declaration { declaration } => {
                self.build_declaration_expression(*declaration, current_block_id)
            }
            TypeExpression::FunctionTypeDeclaration(function) => {
                if let Some(return_type) = function.return_type {
                    self.build_type_expression(return_type, current_block_id)
                } else {
                    Some(current_block_id)
                }
            }
            TypeExpression::ConstructorTypeDeclaration(function) => {
                if let Some(return_type) = function.return_type {
                    self.build_type_expression(return_type, current_block_id)
                } else {
                    Some(current_block_id)
                }
            }
            TypeExpression::Reference {
                generic_arguments, ..
            } => self.build_generic_arguments(generic_arguments.as_slice(), current_block_id),
            TypeExpression::LocalReference {
                generic_arguments, ..
            }
            | TypeExpression::ModuleReference {
                generic_arguments, ..
            }
            | TypeExpression::GlobalReference {
                generic_arguments, ..
            } => self.build_generic_arguments(generic_arguments.as_slice(), current_block_id),
            TypeExpression::Member {
                left,
                generic_arguments,
                ..
            } => {
                let left_block_id = self.build_type_expression(*left, current_block_id)?;
                self.build_generic_arguments(generic_arguments.as_slice(), left_block_id)
            }
            TypeExpression::Import {
                target,
                arguments,
                generic_arguments,
                ..
            } => {
                let target_block_id = self.build_expression(*target, current_block_id)?;
                let argument_block_id =
                    self.build_arguments(Some(arguments.as_slice()), target_block_id)?;
                self.build_generic_arguments(generic_arguments.as_slice(), argument_block_id)
            }
            TypeExpression::Readonly { target_type }
            | TypeExpression::KeyOf { target_type }
            | TypeExpression::Must { target_type }
            | TypeExpression::AsComptime { target_type }
            | TypeExpression::Not { target_type }
            | TypeExpression::ValueOf { target_type, .. }
            | TypeExpression::ReferenceOf { target_type, .. }
            | TypeExpression::PointerOf { target_type, .. } => {
                self.build_type_expression(*target_type, current_block_id)
            }
            TypeExpression::TypeOfValue { value } => {
                self.build_expression(*value, current_block_id)
            }
            TypeExpression::Index { left, index: right } => {
                let left_block_id = self.build_type_expression(*left, current_block_id)?;
                self.build_type_expression(*right, left_block_id)
            }
            TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
                let mut element_block_id = current_block_id;
                for element_id in elements {
                    element_block_id = self.build_type_expression(*element_id, element_block_id)?;
                }
                Some(element_block_id)
            }
            TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            } => {
                let left_block_id = self.build_type_expression(*left, current_block_id)?;
                let right_block_id = self.build_type_expression(*extends_type, left_block_id)?;
                let then_block_id = self.build_type_expression(*then_type, right_block_id)?;
                self.build_type_expression(*else_type, then_block_id)
            }
            TypeExpression::Mapped {
                parameter, value, ..
            } => {
                let source_block_id =
                    self.build_type_expression(parameter.source_type, current_block_id)?;

                let key_remap_block_id = if let Some(key_remap) = parameter.key_remap {
                    self.build_type_expression(key_remap, source_block_id)?
                } else {
                    source_block_id
                };

                self.build_type_expression(*value, key_remap_block_id)
            }
            TypeExpression::TemplateLiteral { spans, .. } => {
                let mut span_block_id = current_block_id;
                for span_id in spans {
                    span_block_id = self.build_type_expression(*span_id, span_block_id)?;
                }
                Some(span_block_id)
            }
            TypeExpression::Infer { constraint, .. } => {
                if let Some(constraint) = constraint {
                    return self.build_type_expression(*constraint, current_block_id);
                }

                Some(current_block_id)
            }
            TypeExpression::Predicate { target, .. } => {
                if let Some(target) = target {
                    return self.build_type_expression(*target, current_block_id);
                }

                Some(current_block_id)
            }
        }
    }

    /// Build a list of arguments.
    fn build_arguments(
        &mut self,
        arguments: Option<&[LocalNodeId<Argument>]>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        // return early when there are no arguments
        let Some(arguments) = arguments else {
            return Some(current_block_id);
        };

        // evaluate each argument in order
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
        // evaluate the argument value
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

        // walk the pattern structure
        match pattern {
            Pattern::Wildcard => Some(current_block_id),
            Pattern::Assign { pattern, value } => {
                let pattern_block_id = self.build_pattern(*pattern, current_block_id)?;
                self.build_expression(*value, pattern_block_id)
            }
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
            Pattern::TypeExpression { value } => {
                self.build_type_expression(*value, current_block_id)
            }
            Pattern::Tuple { fields } | Pattern::Array { fields } | Pattern::Object { fields } => {
                self.build_pattern_fields(fields, current_block_id)
            }
            Pattern::TaggedTuple { ty, fields } | Pattern::TaggedObject { ty, fields } => {
                let ty_block_id = self.build_type_expression(*ty, current_block_id)?;
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

        // walk the pattern field shape
        match field {
            PatternField::Named { pattern, .. } => {
                let mut field_block_id = current_block_id;
                if let Some(pattern_id) = pattern {
                    field_block_id = self.build_pattern(*pattern_id, field_block_id)?;
                }
                Some(field_block_id)
            }
            PatternField::Computed { key, pattern, .. } => {
                let mut field_block_id = self.build_expression(*key, current_block_id)?;
                field_block_id = self.build_pattern(*pattern, field_block_id)?;
                Some(field_block_id)
            }
            PatternField::Positional { pattern } => self.build_pattern(*pattern, current_block_id),
            PatternField::Spread { pattern, .. } => {
                if let Some(pattern_id) = pattern {
                    self.build_pattern(*pattern_id, current_block_id)
                } else {
                    Some(current_block_id)
                }
            }
            PatternField::Elision => Some(current_block_id),
        }
    }

    /// Build an assign pattern and return the exit block when it exists.
    fn build_assign_pattern(
        &mut self,
        assign_pattern_id: LocalNodeId<AssignPattern>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        self.record_node(current_block_id, assign_pattern_id.into_any());
        let assign_pattern = self.tree.get(assign_pattern_id);

        // walk the assign pattern structure
        match assign_pattern {
            AssignPattern::Expression { value } => self.build_expression(*value, current_block_id),
            AssignPattern::Assign { pattern, value } => {
                let pattern_block_id = self.build_assign_pattern(*pattern, current_block_id)?;
                self.build_expression(*value, pattern_block_id)
            }
            AssignPattern::Array { fields } | AssignPattern::Object { fields } => {
                self.build_assign_pattern_fields(fields, current_block_id)
            }
        }
    }

    /// Build a list of assign pattern fields.
    fn build_assign_pattern_fields(
        &mut self,
        fields: &[LocalNodeId<AssignPatternField>],
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let mut field_block_id = current_block_id;

        for field_id in fields {
            field_block_id = self.build_assign_pattern_field(*field_id, field_block_id)?;
        }

        Some(field_block_id)
    }

    /// Build an assign pattern field.
    fn build_assign_pattern_field(
        &mut self,
        field_id: LocalNodeId<AssignPatternField>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        self.record_node(current_block_id, field_id.into_any());
        let field = self.tree.get(field_id);

        // walk the assign pattern field shape
        match field {
            AssignPatternField::Named { pattern, .. } => {
                if let Some(pattern_id) = pattern {
                    self.build_assign_pattern(*pattern_id, current_block_id)
                } else {
                    Some(current_block_id)
                }
            }
            AssignPatternField::Computed { key, pattern } => {
                let key_block_id = self.build_expression(*key, current_block_id)?;
                self.build_assign_pattern(*pattern, key_block_id)
            }
            AssignPatternField::Positional { pattern } => {
                self.build_assign_pattern(*pattern, current_block_id)
            }
            AssignPatternField::Spread { pattern } => {
                if let Some(pattern_id) = pattern {
                    self.build_assign_pattern(*pattern_id, current_block_id)
                } else {
                    Some(current_block_id)
                }
            }
            AssignPatternField::Elision => Some(current_block_id),
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

        // walk the property shape
        match property {
            Property::Field { key, value, .. } => {
                let key_block_id = self.build_key(key, current_block_id)?;
                self.build_expression(*value, key_block_id)
            }
            Property::Method { key, .. } => key
                .as_ref()
                .and_then(|key| self.build_key(key, current_block_id))
                .or(Some(current_block_id)),
            Property::Spread { value, .. } => self.build_expression(*value, current_block_id),
            Property::Error { .. } => Some(current_block_id),
        }
    }

    /// Build a list of type members.
    fn build_type_members(
        &mut self,
        members: &[LocalNodeId<TypeMember>],
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        let mut member_block_id = current_block_id;
        for member_id in members {
            member_block_id = self.build_type_member(*member_id, member_block_id)?;
        }

        Some(member_block_id)
    }

    /// Build a type member.
    fn build_type_member(
        &mut self,
        member_id: LocalNodeId<TypeMember>,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        self.record_node(current_block_id, member_id.into_any());
        let member = self.tree.get(member_id);

        match member {
            TypeMember::Field {
                key, declared_type, ..
            } => {
                let key_block_id = self.build_key(key, current_block_id)?;

                if let Some(declared_type) = declared_type {
                    self.build_type_expression(*declared_type, key_block_id)
                } else {
                    Some(key_block_id)
                }
            }
            TypeMember::Method { key, body, .. } => {
                let key_block_id = self.build_key(key, current_block_id)?;

                if let Some(body) = body {
                    return self.build_expression(*body, key_block_id);
                }

                Some(key_block_id)
            }
            TypeMember::CallSignature { .. } | TypeMember::ConstructSignature { .. } => {
                Some(current_block_id)
            }
            TypeMember::IndexSignature {
                name: _,
                key_type,
                value_type,
                ..
            } => {
                let key_block_id = self.build_type_expression(*key_type, current_block_id)?;
                self.build_type_expression(*value_type, key_block_id)
            }
            TypeMember::Embed { value, .. } => self.build_type_expression(*value, current_block_id),
            TypeMember::AssociatedType {
                generic_parameters,
                where_clauses,
                constraint,
                value,
                ..
            } => {
                let mut member_block_id = current_block_id;

                for generic_parameter_id in generic_parameters {
                    let generic_parameter = self.tree.get(*generic_parameter_id);

                    match generic_parameter {
                        GenericParameter::Type {
                            constraint,
                            default,
                            ..
                        } => {
                            if let Some(constraint) = constraint {
                                member_block_id =
                                    self.build_type_expression(*constraint, member_block_id)?;
                            }

                            if let Some(default) = default {
                                member_block_id =
                                    self.build_type_expression(*default, member_block_id)?;
                            }
                        }
                        GenericParameter::Value {
                            declared_type,
                            default,
                            ..
                        } => {
                            if let Some(declared_type) = declared_type {
                                member_block_id =
                                    self.build_type_expression(*declared_type, member_block_id)?;
                            }

                            if let Some(default) = default {
                                member_block_id =
                                    self.build_expression(*default, member_block_id)?;
                            }
                        }
                        GenericParameter::Error { .. } => {}
                    }
                }

                for where_clause_id in where_clauses {
                    let where_clause = self.tree.get(*where_clause_id);
                    member_block_id =
                        self.build_type_expression(where_clause.right, member_block_id)?;
                }

                if let Some(constraint) = constraint {
                    member_block_id = self.build_type_expression(*constraint, member_block_id)?;
                }

                if let Some(value) = value {
                    return self.build_type_expression(*value, member_block_id);
                }

                Some(member_block_id)
            }
            TypeMember::AssociatedConst {
                declared_type,
                value,
                ..
            } => {
                let mut member_block_id = current_block_id;

                if let Some(declared_type) = declared_type {
                    member_block_id =
                        self.build_type_expression(*declared_type, member_block_id)?;
                }

                if let Some(value) = value {
                    return self.build_expression(*value, member_block_id);
                }

                Some(member_block_id)
            }
            TypeMember::Error { .. } => Some(current_block_id),
        }
    }

    /// Build a key payload.
    fn build_key(&mut self, key: &Key, current_block_id: FlowBlockId) -> Option<FlowBlockId> {
        match key {
            Key::Expression(expression_id) => {
                self.build_expression(*expression_id, current_block_id)
            }
            Key::Name(_) | Key::Private(_) => Some(current_block_id),
        }
    }

    /// Build a template literal expression.
    fn build_template_literal(
        &mut self,
        literal: &TemplateLiteral,
        current_block_id: FlowBlockId,
    ) -> Option<FlowBlockId> {
        // evaluate interpolation arguments when present
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
    tree: &Tree,
) -> FlowGraph {
    let builder = FlowGraphBuilder::new(module_id, tree);
    builder.build(body_id)
}

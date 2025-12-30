use indexmap::IndexMap;

use destack_dir::{
    Expression, GlobalSymbolId, LocalNodeId, LocalNodeIdAny, LocalTypeId, NodeTree, SymbolTable,
    TypeTable,
};
use destack_workspace::Module;

use crate::{
    AnalyzeResult, Compiler, FlowBlockId, FlowEnvironment, FlowEnvironmentId, FlowTable,
    InferContext, InferTable,
};

/// Describe a control flow graph for a single expression body.
#[derive(Debug, Clone)]
pub struct FlowGraph {
    /// Identify the entry block for the graph.
    pub entry_block: FlowBlockId,
    /// Identify the exit block for the graph.
    pub exit_block: FlowBlockId,
    /// Store all blocks in the graph.
    pub blocks: Vec<FlowBlock>,
    /// Map nodes to their containing block.
    pub block_by_node: IndexMap<LocalNodeIdAny, FlowBlockId>,
}

/// Represent a single basic block in the control flow graph.
#[derive(Debug, Clone)]
pub struct FlowBlock {
    /// Identify the block.
    pub id: FlowBlockId,
    /// Store the nodes in the block.
    pub nodes: Vec<LocalNodeIdAny>,
    /// Store incoming edges for the block.
    pub predecessors: Vec<FlowEdge>,
    /// Store outgoing edges for the block.
    pub successors: Vec<FlowEdge>,
    /// Mark the block as terminal when it ends control flow.
    pub is_terminal: bool,
}

/// Represent an edge between blocks.
#[derive(Debug, Clone)]
pub struct FlowEdge {
    /// Identify the edge target block.
    pub target: FlowBlockId,
    /// Describe the edge kind.
    pub kind: FlowEdgeKind,
    /// Store an optional guard expression for the edge.
    pub guard: Option<LocalNodeId<Expression>>,
}

/// Describe the kind of control flow edge between blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowEdgeKind {
    /// Represent an unconditional branch.
    Unconditional,
    /// Represent a true branch from a guard.
    True,
    /// Represent a false branch from a guard.
    False,
    /// Represent a match case branch.
    MatchCase,
    /// Represent a guard branch that is not tied to a boolean condition.
    Guard,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Build a control flow graph for a single expression body.
    pub fn build_flow_graph_for_body(
        &self,
        body_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<FlowGraph> {
        // NOTE #Incomplete: emit a single block graph until the control flow graph builder exists
        // seed the graph with a single block
        let block_id = FlowBlockId(0);
        let mut block_by_node = IndexMap::new();
        block_by_node.insert(body_id.into_any(), block_id);

        // return a minimal graph for the body expression
        Ok(FlowGraph {
            entry_block: block_id,
            exit_block: block_id,
            blocks: vec![FlowBlock {
                id: block_id,
                nodes: vec![body_id.into_any()],
                predecessors: Vec::new(),
                successors: Vec::new(),
                is_terminal: false,
            }],
            block_by_node,
        })
    }

    /// Compute a flow table for a control flow graph.
    pub fn compute_flow_table_for_graph(
        &self,
        module: &Module,
        graph: &FlowGraph,
        _tree: &NodeTree,
        _symbols: &SymbolTable,
        _types: &mut TypeTable,
        _infer: &mut InferTable,
        context: &InferContext,
    ) -> AnalyzeResult<FlowTable> {
        // seed the flow table with the current context
        let mut flow = FlowTable::new();
        let environment_id = flow.push_environment(FlowEnvironment::from_context(context));

        // apply the same environment to every block and node
        for block in &graph.blocks {
            flow.entry_environment_by_block.push(environment_id);
            flow.exit_environment_by_block.push(environment_id);

            for node_id in &block.nodes {
                flow.environment_by_node
                    .insert(node_id.into_global(module.id), environment_id);
            }
        }

        Ok(flow)
    }

    /// Infer a function body using flow aware typing.
    pub fn infer_function_body_with_flow(
        &self,
        module: &Module,
        body_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        context: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // build a graph and flow table for the function body
        let graph = self.build_flow_graph_for_body(body_id)?;
        let flow = self
            .compute_flow_table_for_graph(module, &graph, tree, symbols, types, infer, context)?;

        // choose the entry environment for inference
        let fallback_environment = FlowEnvironment::from_context(context);
        let entry_environment = flow
            .entry_environment_for_block(graph.entry_block)
            .and_then(|environment_id| flow.environment(environment_id))
            .unwrap_or(&fallback_environment);

        self.infer_expression_in_block(
            module,
            body_id,
            tree,
            symbols,
            types,
            infer,
            context,
            entry_environment,
        )
    }

    /// Infer an expression using a provided block environment.
    pub fn infer_expression_in_block(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        context: &mut InferContext,
        environment: &FlowEnvironment,
    ) -> AnalyzeResult<LocalTypeId> {
        // copy environment narrowings into the block context
        let mut block_context = context.fork();
        block_context.narrowings = environment
            .bindings
            .iter()
            .map(|(symbol, type_id)| (*symbol, *type_id))
            .collect();
        block_context.is_unreachable = block_context.is_unreachable || !environment.is_reachable;

        // infer the expression using the block context
        self.infer_expression(
            module,
            expression_id,
            tree,
            symbols,
            types,
            infer,
            &mut block_context,
        )
    }

    /// Split the environment based on a guard expression.
    pub fn narrow_environment_for_guard(
        &self,
        _module: &Module,
        _guard_id: LocalNodeId<Expression>,
        _tree: &NodeTree,
        _symbols: &SymbolTable,
        _types: &mut TypeTable,
        _infer: &mut InferTable,
        environment: &FlowEnvironment,
        _context: &InferContext,
    ) -> AnalyzeResult<(FlowEnvironment, FlowEnvironment)> {
        Ok((environment.clone(), environment.clone()))
    }

    /// Join multiple environments into a single environment.
    pub fn join_flow_environments(
        &self,
        environment_ids: &[FlowEnvironmentId],
        flow: &mut FlowTable,
    ) -> FlowEnvironmentId {
        // reuse the first environment for now
        if let Some(environment_id) = environment_ids.first() {
            return *environment_id;
        }

        // fall back to a reachable empty environment
        flow.push_environment(FlowEnvironment::new(true))
    }

    /// Get a symbol type from a flow environment.
    pub fn flow_environment_for_symbol(
        &self,
        symbol: GlobalSymbolId,
        environment: &FlowEnvironment,
    ) -> Option<LocalTypeId> {
        environment.bindings.get(&symbol).copied()
    }
}

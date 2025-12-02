use crate::{AnalyzeResult, Compiler, Task, TaskDebug, TaskOutput, TaskResultCollector};

use destack_dir::{Expression, LocalNodeId, ModuleId, NodeTree, Program, SymbolTable, TypeTable};

/// Task to analyze something.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum AnalyzeTask {
    /// Analyze a module.
    Analyze { module: ModuleId },
}

impl AnalyzeTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Analyze { .. } => 1,
        }
    }
}

impl TaskDebug for AnalyzeTask {
    fn name(&self) -> &'static str {
        match self {
            Self::Analyze { .. } => "module",
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::Analyze { module } => {
                let module = program.modules.get(*module);
                let uri = module.read().uri.clone().to_string();
                format!(r#"module="{uri}""#)
            }
        }
    }
}

impl From<AnalyzeTask> for Task {
    fn from(task: AnalyzeTask) -> Self {
        Task::Analyze(task)
    }
}

/// Output of a analyze task.
#[derive(Debug, Clone, PartialEq)]
pub struct AnalyzeOutput {}

impl From<AnalyzeOutput> for TaskOutput {
    fn from(output: AnalyzeOutput) -> Self {
        TaskOutput::Analyze(output)
    }
}

impl Compiler {
    /// Process a analyze task.
    pub fn process_analyze(&self, task: AnalyzeTask) -> AnalyzeResult<AnalyzeOutput> {
        match task {
            AnalyzeTask::Analyze { module } => self.analyze_module(module)?,
        }
        Ok(AnalyzeOutput {})
    }

    /// Analyze a module.
    ///
    /// This is the main entry point for type analysis. It processes the module in order:
    /// 1. Evaluate declared types (from annotations)
    /// 2. Infer expression types (interleaved with overload resolution)
    /// 3. Check type compatibility (future)
    pub fn analyze_module(&self, module_id: ModuleId) -> AnalyzeResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let tree = module.tree.read();
        let symbols = module.symbols.read();
        let mut types = module.types.write();
        let _collector = TaskResultCollector::new();

        // step 1: evaluate declared types from annotations
        self.evaluate_declared_types(&tree, &symbols, &mut types)?;

        // step 2: infer expression types (interleaved with overload resolution)
        for &root in &module.roots {
            self.infer_expression(&tree, &symbols, &mut types, root)?;
        }

        // step 3: check type compatibility (future)
        // self.check_compatibility(&tree, &symbols, &types)?;

        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Step 1: Evaluate declared types
    // ─────────────────────────────────────────────────────────────────────────────

    /// Evaluate all declared types from type annotations.
    pub(super) fn evaluate_declared_types(
        &self,
        _tree: &NodeTree,
        _symbols: &SymbolTable,
        _types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // TODO #Incomplete: evaluate declared types
        Ok(())
    }

    /// Infer the type of an expression.
    pub(super) fn infer_expression(
        &self,
        _tree: &NodeTree,
        _symbols: &SymbolTable,
        _types: &mut TypeTable,
        _expression: LocalNodeId<Expression>,
    ) -> AnalyzeResult<()> {
        // TODO #Incomplete: infer expression type
        Ok(())
    }

    /// Resolve which overload is called for a call/operator expression.
    pub(super) fn resolve_overload(
        &self,
        _tree: &NodeTree,
        _symbols: &SymbolTable,
        _types: &mut TypeTable,
        _expression: LocalNodeId<Expression>,
    ) -> AnalyzeResult<()> {
        // TODO #Incomplete: resolve overload
        Ok(())
    }

    /// Resolve member access (like `a.foo`).
    pub(super) fn resolve_member(
        &self,
        _tree: &NodeTree,
        _symbols: &SymbolTable,
        _types: &mut TypeTable,
        _expression: LocalNodeId<Expression>,
    ) -> AnalyzeResult<()> {
        // TODO #Incomplete: resolve member
        Ok(())
    }
}

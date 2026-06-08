use std::collections::HashMap;

use destack_dir::{
    self as dir, NodeVisitor, NodeVisitorOptions, walk_declaration, walk_expression,
};
use destack_repository::LintSeverity;
use destack_source::LabeledSpan;

use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow functions in loops that capture mutable outer bindings.
    ///
    /// Functions created inside loops can accidentally capture mutable values
    /// from loop scope or outer scope, which often causes stale or surprising reads.
    #[lint(
        id = "no-loop-func",
        code = "LU046",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoLoopFunc,
    "Disallow functions in loops that capture mutable outer bindings"
}

impl LintRule for NoLoopFunc {
    fn meta(&self) -> &'static LintMeta {
        NoLoopFunc::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoLoopFuncVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that reports loop function captures.
struct NoLoopFuncVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// Active loop scopes for the current traversal stack.
    active_loop_scopes: Vec<dir::LocalScopeId>,
    /// Visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoLoopFuncVisitor<'a, 'b> {
    /// Build a visitor for loop function checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        Self {
            ctx,
            meta,
            active_loop_scopes: Vec::new(),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk all module roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.dir.tree();
        for root_id in roots {
            let root_expression = tree.get(root_id);
            self.visit_expression(tree, root_id, root_expression);
        }
    }

    /// Check one function declaration expression when inside a loop.
    fn check_loop_function_declaration(
        &mut self,
        function_expression_id: dir::LocalNodeId<dir::Expression>,
        function_scope: Option<dir::LocalScope>,
        body_expression_id: dir::LocalNodeId<dir::Expression>,
    ) {
        // skip when no loop is active
        if self.active_loop_scopes.is_empty() {
            return;
        }
        let Some(function_scope) = function_scope else {
            return;
        };

        // skip immediately invoked function expressions
        if expression_is_immediately_invoked(self.ctx.dir.tree(), function_expression_id) {
            return;
        }

        // collect mutable captured symbols that are loop related
        let mut collector = CapturedMutableSymbolCollector::new(
            self.ctx.module_id(),
            self.ctx.dir.tree(),
            &self.ctx.symbols,
            self.ctx.resolutions,
            function_scope.id,
            self.active_loop_scopes.as_slice(),
        );
        collector.collect(body_expression_id);

        if collector.captured_symbols.is_empty() {
            return;
        }

        let severity = self
            .ctx
            .get_effective_severity(self.meta, function_expression_id);
        if !severity.is_enabled() {
            return;
        }

        // resolve deterministic symbol names for diagnostics
        let mut captured_names: Vec<String> = collector
            .captured_symbols
            .keys()
            .filter_map(|symbol_id| {
                let symbol = self.ctx.symbols.get_symbol(*symbol_id);
                let symbol_name_id = symbol.name()?;
                Some(self.ctx.strings.get(symbol_name_id).to_string())
            })
            .collect();
        captured_names.sort();
        captured_names.dedup();

        let description = if captured_names.len() == 1 {
            format!(
                "function in loop captures mutable binding `{}`",
                captured_names[0]
            )
        } else {
            format!(
                "function in loop captures mutable bindings: {}",
                captured_names.join(", ")
            )
        };

        // report one diagnostic per function declaration
        let function_span = self.ctx.get_span(function_expression_id);
        let mut diagnostic = LintReport::new(
            NO_LOOP_FUNC.id,
            NO_LOOP_FUNC.code,
            NO_LOOP_FUNC.category,
            severity,
            description,
            function_span,
        )
        .label("this function captures mutable state across loop iterations");

        if let Some((_, reference_expression_id)) = collector.captured_symbols.iter().next() {
            let reference_span = self.ctx.get_span(*reference_expression_id);
            diagnostic = diagnostic.secondary(LabeledSpan::new(
                reference_span,
                "captured mutable binding referenced here",
            ));
        }

        self.ctx.report(diagnostic);
    }
}

impl NodeVisitor for NoLoopFuncVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // track loop scope while visiting loop expressions
        let loop_scope = match expression {
            dir::Expression::Loop { .. }
            | dir::Expression::ForEach { .. }
            | dir::Expression::For { .. } => self.ctx.scope_for_node(id),
            _ => None,
        };
        if let Some(loop_scope_id) = loop_scope {
            self.active_loop_scopes.push(loop_scope_id.id);
            walk_expression(self, tree, id, expression);
            self.active_loop_scopes.pop();
            return;
        }

        // check function declaration expressions and reset loop context for nested bodies
        if let dir::Expression::Declaration(declaration) = expression {
            let declaration_node = tree.get(*declaration);
            if let dir::Declaration::Function(function_declaration) = declaration_node
                && let Some(body_expression_id) = function_declaration.body
            {
                self.check_loop_function_declaration(
                    id,
                    self.ctx.scope_for_node(*declaration),
                    body_expression_id,
                );

                let saved_loop_scopes = std::mem::take(&mut self.active_loop_scopes);
                walk_declaration(self, tree, *declaration, declaration_node);
                self.active_loop_scopes = saved_loop_scopes;
                return;
            }
        }

        // continue default traversal
        walk_expression(self, tree, id, expression);
    }
}

/// Collector for mutable captured symbols in one function body.
struct CapturedMutableSymbolCollector<'a> {
    /// The current module id.
    module_id: destack_source::ModuleId,
    /// The DIR tree.
    tree: &'a dir::Tree,
    /// The symbol table.
    symbols: &'a dir::BindingTable<'a>,
    /// The resolution table carrying semantic targets.
    resolutions: &'a dir::ResolutionTable<'a>,
    /// The function scope id for local ownership checks.
    function_scope_id: dir::LocalScopeId,
    /// The active loop scopes for relation checks.
    active_loop_scopes: &'a [dir::LocalScopeId],
    /// Captured mutable symbols and one reference expression each.
    captured_symbols: HashMap<dir::LocalSymbolId, dir::LocalNodeId<dir::Expression>>,
    /// Visitor options.
    options: NodeVisitorOptions,
}

impl<'a> CapturedMutableSymbolCollector<'a> {
    /// Build a collector for one function body.
    fn new(
        module_id: destack_source::ModuleId,
        tree: &'a dir::Tree,
        symbols: &'a dir::BindingTable<'a>,
        resolutions: &'a dir::ResolutionTable<'a>,
        function_scope_id: dir::LocalScopeId,
        active_loop_scopes: &'a [dir::LocalScopeId],
    ) -> Self {
        Self {
            module_id,
            tree,
            symbols,
            resolutions,
            function_scope_id,
            active_loop_scopes,
            captured_symbols: HashMap::new(),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk one function body expression.
    fn collect(&mut self, body_expression_id: dir::LocalNodeId<dir::Expression>) {
        let body_expression = self.tree.get(body_expression_id);
        self.visit_expression(self.tree, body_expression_id, body_expression);
    }

    /// Record one mutable captured reference if it is loop related.
    fn record_reference(
        &mut self,
        reference_expression_id: dir::LocalNodeId<dir::Expression>,
        target_symbol: dir::GlobalSymbolId,
    ) {
        // keep local module symbols only
        if target_symbol.module_id != self.module_id {
            return;
        }

        let local_symbol_id = target_symbol.local_id;
        let symbol = self.symbols.get_symbol(local_symbol_id);

        // keep mutable value bindings only
        if symbol.binding_mutability != Some(dir::Mutability::Mutable) {
            return;
        }

        // skip symbols declared inside this function scope
        let symbol_scope_id = symbol.scope.id;
        if scope_is_descendant_of(self.symbols, symbol_scope_id, self.function_scope_id) {
            return;
        }

        // keep symbols declared in or around one active loop scope
        let is_loop_related = self
            .active_loop_scopes
            .iter()
            .copied()
            .any(|loop_scope_id| {
                scope_is_descendant_of(self.symbols, symbol_scope_id, loop_scope_id)
                    || scope_is_descendant_of(self.symbols, loop_scope_id, symbol_scope_id)
            });
        if !is_loop_related {
            return;
        }

        self.captured_symbols
            .entry(local_symbol_id)
            .or_insert(reference_expression_id);
    }
}

impl NodeVisitor for CapturedMutableSymbolCollector<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // skip nested function declarations: those captures do not belong to this function body
        if let dir::Expression::Declaration(declaration) = expression {
            let declaration = tree.get(*declaration);
            if matches!(declaration, dir::Declaration::Function(_)) {
                return;
            }
        }

        // collect symbol-backed references
        if let Some(target_symbol) = self
            .resolutions
            .symbol_resolution(id.into_global_any(self.module_id))
        {
            self.record_reference(id, target_symbol);
        }

        walk_expression(self, tree, id, expression);
    }
}

/// Return true when one scope is equal to or nested under one ancestor scope.
fn scope_is_descendant_of(
    symbols: &dir::BindingTable<'_>,
    mut scope_id: dir::LocalScopeId,
    ancestor_scope_id: dir::LocalScopeId,
) -> bool {
    loop {
        if scope_id == ancestor_scope_id {
            return true;
        }

        let scope = symbols.get_scope_by_id(scope_id);
        let Some(parent_scope) = scope.parent else {
            return false;
        };
        scope_id = parent_scope.id;
    }
}

/// Return true when one declaration expression is immediately invoked.
fn expression_is_immediately_invoked(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let mut current_expression_id = expression_id;

    loop {
        let Some(parent_node_id) = tree.get_parent(current_expression_id.id) else {
            return false;
        };
        if parent_node_id.ty != dir::NodeType::Expression {
            return false;
        }

        let parent_expression_id = dir::LocalNodeId::<dir::Expression>::new(parent_node_id.id);
        let parent_expression = tree.get(parent_expression_id);
        match parent_expression {
            dir::Expression::Parenthesized { expression }
                if *expression == current_expression_id =>
            {
                current_expression_id = parent_expression_id;
            }
            dir::Expression::Call { left, .. } => {
                return *left == current_expression_id;
            }
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag mutable loop binding capture in a function declared in the loop body.
    #[test]
    fn test_flags_mutable_loop_binding_capture() {
        let test = TestProgram::for_rule_without_prelude(NoLoopFunc);
        let result = test.lint_dir(
            "no_loop_func/test_flags_mutable_loop_binding_capture.ds",
            r#"
let callbacks = [];
for (let index = 0; index < 3; index += 1) {
    function next() {
        return index;
    }
    callbacks.push(next);
}
"#,
        );
        test.result(result).assert_lint("no-loop-func");
    }

    /// Flag mutable outer binding capture from inside a loop function.
    #[test]
    fn test_flags_mutable_outer_binding_capture() {
        let test = TestProgram::for_rule_without_prelude(NoLoopFunc);
        let result = test.lint_dir(
            "no_loop_func/test_flags_mutable_outer_binding_capture.ds",
            r#"
let cursor = 0;
while (cursor < 2) {
    function read() {
        return cursor;
    }
    cursor += 1;
    read();
}
"#,
        );
        test.result(result).assert_lint("no-loop-func");
    }

    /// Allow immutable captures inside loops.
    #[test]
    fn test_allows_immutable_capture() {
        let test = TestProgram::for_rule_without_prelude(NoLoopFunc);
        let result = test.lint_dir(
            "no_loop_func/test_allows_immutable_capture.ds",
            r#"
let callbacks = [];
for (const item of [1, 2, 3]) {
    function read() {
        return item;
    }
    callbacks.push(read);
}
"#,
        );
        test.result(result).assert_no_lint("no-loop-func");
    }

    /// Allow functions that are not declared in loops.
    #[test]
    fn test_allows_function_outside_loop() {
        let test = TestProgram::for_rule_without_prelude(NoLoopFunc);
        let result = test.lint_dir(
            "no_loop_func/test_allows_function_outside_loop.ds",
            r#"
let count = 0;
function read() {
    return count;
}
"#,
        );
        test.result(result).assert_no_lint("no-loop-func");
    }

    /// Skip immediately invoked functions in loops.
    #[test]
    fn test_allows_immediately_invoked_function_in_loop() {
        let test = TestProgram::for_rule_without_prelude(NoLoopFunc);
        let result = test.lint_dir(
            "no_loop_func/test_allows_immediately_invoked_function_in_loop.ds",
            r#"
for (let i = 0; i < 2; i += 1) {
    (function read() {
        return i;
    })();
}
"#,
        );
        test.result(result).assert_no_lint("no-loop-func");
    }

    /// Skip captures that are local to the inner function itself.
    #[test]
    fn test_allows_inner_local_binding() {
        let test = TestProgram::for_rule_without_prelude(NoLoopFunc);
        let result = test.lint_dir(
            "no_loop_func/test_allows_inner_local_binding.ds",
            r#"
for (let i = 0; i < 2; i += 1) {
    function read() {
        let local = 1;
        return local;
    }
    read();
}
"#,
        );
        test.result(result).assert_no_lint("no-loop-func");
    }

    /// Flag mutable outer captures inside for-of loop functions.
    #[test]
    fn test_flags_mutable_outer_capture_in_for_of_loop_function() {
        let test = TestProgram::for_rule_without_prelude(NoLoopFunc);
        let result = test.lint_dir(
            "no_loop_func/test_flags_mutable_outer_capture_in_for_of_loop_function.ds",
            r#"
let cursor = 0;
for (const value of [1, 2, 3]) {
    function read() {
        return cursor + value;
    }
    cursor += value;
    read();
}
"#,
        );
        test.result(result).assert_lint("no-loop-func");
    }

    /// Allow immutable outer captures inside loop functions.
    #[test]
    fn test_allows_immutable_outer_capture_in_loop_function() {
        let test = TestProgram::for_rule_without_prelude(NoLoopFunc);
        let result = test.lint_dir(
            "no_loop_func/test_allows_immutable_outer_capture_in_loop_function.ds",
            r#"
const offset = 1;
for (let i = 0; i < 2; i += 1) {
    function read() {
        return offset;
    }
    read();
}
"#,
        );
        test.result(result).assert_no_lint("no-loop-func");
    }

    /// Flag arrow functions in loops that capture mutable loop bindings.
    #[test]
    fn test_flags_arrow_function_capture_in_loop() {
        let test = TestProgram::for_rule_without_prelude(NoLoopFunc);
        let result = test.lint_dir(
            "no_loop_func/test_flags_arrow_function_capture_in_loop.ds",
            r#"
for (let i = 0; i < 3; i += 1) {
    const read = (): int32 => i;
    read();
}
"#,
        );
        test.result(result).assert_lint("no-loop-func");
    }

    /// Allow immediately invoked arrow functions in loops.
    #[test]
    fn test_allows_immediately_invoked_arrow_function_in_loop() {
        let test = TestProgram::for_rule_without_prelude(NoLoopFunc);
        let result = test.lint_dir(
            "no_loop_func/test_allows_immediately_invoked_arrow_function_in_loop.ds",
            r#"
for (let i = 0; i < 3; i += 1) {
    (() => i)();
}
"#,
        );
        test.result(result).assert_no_lint("no-loop-func");
    }
}

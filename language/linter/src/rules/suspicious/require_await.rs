use destack_dir::{
    self as dir, Asynchrony, Expression, FunctionCardinality, NodeVisitor, NodeVisitorOptions,
    WellKnownSymbol, walk_expression,
};
use destack_source::{ModuleId, Span};
use destack_workspace::{LintSeverity, ProfileId, Repository, Revision};
use std::collections::HashSet;

use crate::rules::common::{
    collect_pattern_value_binding_symbols, expression_enters_nested_declaration_scope,
    expression_is_promise_like, expression_type_or_call_return_type_map,
    expression_unwrap_parenthesized, is_promise_type, remove_first_async_keyword,
    well_known_symbol_candidates,
};
use crate::{LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow async functions with no await expression.
    ///
    /// Async functions without await often indicate accidental async usage.
    /// Functions that directly return Promise like values are allowed.
    #[lint(
        id = "require-await",
        code = "LU042",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub RequireAwait,
    "Require await in async functions"
}

impl LintRule for RequireAwait {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        RequireAwait::meta()
    }

    /// Check module DIR nodes for async callables without await or Promise like returns.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        // resolve metadata and Promise symbols once
        let meta = self.meta();
        let promise_symbols = resolve_promise_symbols(ctx);
        let async_function_symbols = collect_async_function_symbols(ctx);

        // check function declarations
        for (declaration_id, declaration) in ctx.tree.iter_nodes_of_type::<dir::Declaration>() {
            let dir::Declaration::Function(declaration) = declaration else {
                continue;
            };

            check_async_callable(
                ctx,
                meta,
                declaration_id,
                &declaration.signature,
                declaration.body,
                &promise_symbols,
                &async_function_symbols,
            );
        }

        // check class or struct methods
        for (member_id, member) in ctx.tree.iter_nodes_of_type::<dir::Member>() {
            let dir::Member::Method {
                signature, body, ..
            } = member
            else {
                continue;
            };

            check_async_callable(
                ctx,
                meta,
                member_id,
                signature,
                *body,
                &promise_symbols,
                &async_function_symbols,
            );
        }

        // check object literal methods
        for (property_id, property) in ctx.tree.iter_nodes_of_type::<dir::Property>() {
            let dir::Property::Method {
                signature, body, ..
            } = property
            else {
                continue;
            };

            check_async_callable(
                ctx,
                meta,
                property_id,
                signature,
                *body,
                &promise_symbols,
                &async_function_symbols,
            );
        }
    }
}

/// Resolve concrete Promise symbols when the module has typed well known symbols.
fn resolve_promise_symbols(ctx: &LintModuleDirContext<'_>) -> Vec<dir::GlobalSymbolId> {
    let Some(well_known_symbols) = ctx.get_well_known_symbols() else {
        return Vec::new();
    };

    well_known_symbol_candidates(&well_known_symbols, WellKnownSymbol::Promise)
}

/// Collect async callable symbols for Promise call fallback checks.
fn collect_async_function_symbols(ctx: &LintModuleDirContext<'_>) -> HashSet<dir::GlobalSymbolId> {
    let mut symbols = HashSet::new();

    // keep named async function declarations only
    for declaration_id in ctx.tree.iter_node_ids_of_type::<dir::Declaration>() {
        let declaration = ctx.tree.get(declaration_id);
        let dir::Declaration::Function(declaration) = declaration else {
            continue;
        };
        if declaration.signature.asynchrony != Asynchrony::Async {
            continue;
        }
        let symbol_id = declaration.symbol.into_global(ctx.module_id());
        symbols.insert(symbol_id);
    }

    // keep async function expression and arrow bindings
    for declarator_id in ctx.tree.iter_node_ids_of_type::<dir::Declarator>() {
        let declarator = ctx.tree.get(declarator_id);
        let Some(value_id) = declarator.value else {
            continue;
        };
        if !expression_is_async_callable_value(ctx.tree, value_id) {
            continue;
        }

        let mut binding_symbols = HashSet::new();
        collect_pattern_value_binding_symbols(
            ctx.tree,
            ctx.symbols,
            declarator.pattern,
            &mut binding_symbols,
        );
        for symbol_id in binding_symbols {
            let symbol_id = symbol_id.into_global(ctx.module_id());
            symbols.insert(symbol_id);
        }
    }

    symbols
}

/// Check one async callable node for missing await usage.
fn check_async_callable<T: dir::Node>(
    ctx: &mut LintModuleDirContext<'_>,
    meta: &LintMeta,
    node_id: dir::LocalNodeId<T>,
    signature: &dir::FunctionSignature,
    body_id: Option<dir::LocalNodeId<dir::Expression>>,
    promise_symbols: &[dir::GlobalSymbolId],
    async_function_symbols: &HashSet<dir::GlobalSymbolId>,
) {
    // keep only async callables with bodies
    if signature.asynchrony != Asynchrony::Async {
        return;
    }
    let Some(body_id) = body_id else {
        return;
    };

    // ignore empty async bodies
    if function_body_is_empty(ctx.tree, body_id) {
        return;
    }

    // analyze await and Promise like return signals
    let analysis = analyze_async_callable_body(
        ctx.repository.as_ref(),
        ctx.revision,
        ctx.profile_id,
        ctx.module_id(),
        ctx.tree,
        ctx.symbols,
        ctx.types,
        body_id,
        signature.cardinality == FunctionCardinality::Generator,
        promise_symbols,
        async_function_symbols,
    );
    if analysis.has_await || analysis.has_thenable_return {
        return;
    }
    if signature.cardinality == FunctionCardinality::Generator && analysis.has_async_yield {
        return;
    }

    // honor per node severity
    let node_id_raw = node_id.id;
    let severity = ctx.get_effective_severity(meta, dir::LocalNodeId::<T>::new(node_id_raw));
    if !severity.is_enabled() {
        return;
    }

    // build one diagnostic
    let span = ctx.get_span(dir::LocalNodeId::<T>::new(node_id_raw));
    let mut diagnostic = LintReport::new(
        REQUIRE_AWAIT.id,
        REQUIRE_AWAIT.code,
        REQUIRE_AWAIT.category,
        severity,
        "async function has no await expression",
        span,
    )
    .label("add await or remove async keyword");

    // attach one unsafe async removal fix when token shape is known
    if ctx.include_fixes
        && let Some(fix) = require_await_fix(ctx, span)
    {
        diagnostic = diagnostic.fix(fix);
    }

    ctx.report(diagnostic);
}

/// Return true when a function body is an empty explicit block.
fn function_body_is_empty(tree: &dir::Tree, body_id: dir::LocalNodeId<dir::Expression>) -> bool {
    let body = tree.get(body_id);
    let dir::Expression::Block(block) = body else {
        return false;
    };
    let block = tree.get(*block);
    block.is_empty()
}

/// Analyze one async function body for await and Promise like return signals.
fn analyze_async_callable_body(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
    module_id: ModuleId,
    tree: &dir::Tree,
    symbols: &dir::SymbolTable,
    types: &dir::TypeTable,
    body_id: dir::LocalNodeId<dir::Expression>,
    is_generator: bool,
    promise_symbols: &[dir::GlobalSymbolId],
    async_function_symbols: &HashSet<dir::GlobalSymbolId>,
) -> RequireAwaitBodyAnalysis {
    // run body traversal analysis
    let mut visitor = RequireAwaitBodyVisitor::new(
        repository,
        revision,
        profile_id,
        module_id,
        symbols,
        types,
        is_generator,
        promise_symbols,
        async_function_symbols,
    );
    visitor.run(tree, body_id);

    // treat expression bodies as implicit returns
    if !visitor.has_thenable_return
        && expression_is_implicit_thenable_return(
            repository,
            revision,
            profile_id,
            module_id,
            tree,
            symbols,
            types,
            promise_symbols,
            async_function_symbols,
            body_id,
        )
    {
        visitor.has_thenable_return = true;
    }

    RequireAwaitBodyAnalysis {
        has_await: visitor.has_await,
        has_thenable_return: visitor.has_thenable_return,
        has_async_yield: visitor.has_async_yield,
    }
}

/// One body analysis result for require await.
struct RequireAwaitBodyAnalysis {
    /// Whether this callable body contains an await signal.
    has_await: bool,
    /// Whether this callable body returns a Promise like value.
    has_thenable_return: bool,
    /// Whether this async generator yields Promise like values.
    has_async_yield: bool,
}

/// One visitor that collects await and Promise like returns in one body scope.
struct RequireAwaitBodyVisitor<'a> {
    /// Node visitor options.
    options: NodeVisitorOptions,
    /// Program for symbol backed type lookups.
    repository: &'a Repository,
    /// Active revision for symbol backed type lookups.
    revision: Revision,
    /// Active profile id for symbol backed type lookups.
    profile_id: ProfileId,
    /// Current module id for type lookups.
    module_id: ModuleId,
    /// Symbol table used for symbol backed type lookups.
    symbols: &'a dir::SymbolTable,
    /// Type table used for Promise like checks.
    types: &'a dir::TypeTable,
    /// Whether this callable is a generator.
    is_generator: bool,
    /// Known Promise symbols in this module profile.
    promise_symbols: &'a [dir::GlobalSymbolId],
    /// Async callable symbols in this module.
    async_function_symbols: &'a HashSet<dir::GlobalSymbolId>,
    /// Whether an await signal has been seen.
    has_await: bool,
    /// Whether a Promise like return has been seen.
    has_thenable_return: bool,
    /// Whether a Promise like yield has been seen.
    has_async_yield: bool,
    /// Nested declaration depth to skip inner function scopes.
    nested_declaration_depth: usize,
}

impl<'a> RequireAwaitBodyVisitor<'a> {
    /// Build one body visitor.
    fn new(
        repository: &'a Repository,
        revision: Revision,
        profile_id: ProfileId,
        module_id: ModuleId,
        symbols: &'a dir::SymbolTable,
        types: &'a dir::TypeTable,
        is_generator: bool,
        promise_symbols: &'a [dir::GlobalSymbolId],
        async_function_symbols: &'a HashSet<dir::GlobalSymbolId>,
    ) -> Self {
        Self {
            options: NodeVisitorOptions::default(),
            repository,
            revision,
            profile_id,
            module_id,
            symbols,
            types,
            is_generator,
            promise_symbols,
            async_function_symbols,
            has_await: false,
            has_thenable_return: false,
            has_async_yield: false,
            nested_declaration_depth: 0,
        }
    }

    /// Walk one function body expression.
    fn run(&mut self, tree: &dir::Tree, body_id: dir::LocalNodeId<dir::Expression>) {
        let body = tree.get(body_id);
        self.visit_expression(tree, body_id, body);
    }
}

impl NodeVisitor for RequireAwaitBodyVisitor<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &Expression,
    ) {
        // track nested declarations to avoid counting inner scope control flow
        let enters_nested_scope = expression_enters_nested_declaration_scope(tree, expression);
        if enters_nested_scope {
            self.nested_declaration_depth += 1;
        }

        // collect signals only in the current callable scope
        if self.nested_declaration_depth == 0 {
            if expression_has_await_signal(expression) {
                self.has_await = true;
            }

            if self.is_generator
                && let Expression::Yield {
                    value: Some(value_id),
                    ..
                } = expression
                && expression_is_thenable_return_value(
                    self.repository,
                    self.revision,
                    self.profile_id,
                    self.module_id,
                    tree,
                    self.symbols,
                    self.types,
                    self.promise_symbols,
                    self.async_function_symbols,
                    *value_id,
                )
            {
                self.has_async_yield = true;
            }

            if let Expression::Return {
                value: Some(value_id),
            } = expression
                && expression_is_thenable_return_value(
                    self.repository,
                    self.revision,
                    self.profile_id,
                    self.module_id,
                    tree,
                    self.symbols,
                    self.types,
                    self.promise_symbols,
                    self.async_function_symbols,
                    *value_id,
                )
            {
                self.has_thenable_return = true;
            }
        }

        // walk child expressions
        walk_expression(self, tree, expression_id, expression);

        // leave nested declaration scope
        if enters_nested_scope {
            self.nested_declaration_depth -= 1;
        }
    }
}

/// Return true when one expression kind satisfies await usage for this lint.
fn expression_has_await_signal(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Await { .. } | Expression::AwaitMaybe { .. }
    ) || matches!(expression, Expression::ForEach { asynchrony, .. } if *asynchrony == Asynchrony::Async)
        || matches!(expression, Expression::Using { asynchrony, .. } if *asynchrony == Asynchrony::Async)
}

/// Return true when one return value expression is Promise like.
fn expression_is_thenable_return_value(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
    module_id: ModuleId,
    tree: &dir::Tree,
    symbols: &dir::SymbolTable,
    types: &dir::TypeTable,
    promise_symbols: &[dir::GlobalSymbolId],
    async_function_symbols: &HashSet<dir::GlobalSymbolId>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // keep typed Promise like checks first
    if promise_symbols.iter().any(|promise_symbol| {
        expression_is_promise_like(module_id, tree, types, *promise_symbol, expression_id)
    }) {
        return true;
    }

    // keep symbol backed Promise checks for unresolved expression types
    let has_symbol_backed_promise_type = promise_symbols.iter().any(|promise_symbol| {
        expression_type_or_call_return_type_map(
            repository,
            revision,
            profile_id,
            module_id,
            tree,
            symbols,
            types,
            expression_id,
            |types, type_id| is_promise_type(types, type_id, Some(*promise_symbol)),
        )
        .unwrap_or(false)
    });
    if has_symbol_backed_promise_type {
        return true;
    }

    // keep one declaration fallback for async calls
    expression_is_async_symbol_call(tree, expression_id, async_function_symbols)
}

/// Return true when a function body expression is an implicit Promise like return.
fn expression_is_implicit_thenable_return(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
    module_id: ModuleId,
    tree: &dir::Tree,
    symbols: &dir::SymbolTable,
    types: &dir::TypeTable,
    promise_symbols: &[dir::GlobalSymbolId],
    async_function_symbols: &HashSet<dir::GlobalSymbolId>,
    body_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // ignore block bodies because explicit return handling covers them
    if matches!(tree.get(body_id), Expression::Block(..)) {
        return false;
    }

    expression_is_thenable_return_value(
        repository,
        revision,
        profile_id,
        module_id,
        tree,
        symbols,
        types,
        promise_symbols,
        async_function_symbols,
        body_id,
    )
}

/// Return true when one expression is a direct call to one known async callable symbol.
fn expression_is_async_symbol_call(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    async_function_symbols: &HashSet<dir::GlobalSymbolId>,
) -> bool {
    // normalize expression wrappers
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);
    let Expression::Call { left, .. } = expression else {
        return false;
    };

    // resolve one direct callee symbol
    let callee_id = expression_unwrap_parenthesized(tree, *left);
    let callee = tree.get(callee_id);
    let Some(symbol_id) = callee.target_symbol() else {
        return false;
    };

    async_function_symbols.contains(&symbol_id)
}

/// Return true when one expression is an async callable value expression.
fn expression_is_async_callable_value(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);
    let Expression::Declaration(declaration) = expression else {
        return false;
    };

    let declaration = tree.get(*declaration);
    let dir::Declaration::Function(declaration) = declaration else {
        return false;
    };

    declaration.signature.asynchrony == Asynchrony::Async
}

/// Build one unsafe fix that removes the `async` keyword token.
fn require_await_fix(ctx: &LintModuleDirContext<'_>, callable_span: Span) -> Option<LintFix> {
    // resolve callable source text
    let callable_text = ctx.get_span_text(callable_span);
    let replacement = remove_first_async_keyword(callable_text)?;

    // build one edit set
    let edits = ctx
        .edit_builder()
        .replace(callable_span, replacement)
        .into_edits();
    Some(LintFix::suggestion("Remove async keyword").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_async_without_await_detected() {
        let test = TestProgram::for_rule_with_prelude(RequireAwait);
        let result = test.lint_dir(
            "require_await/test_async_without_await_detected.ds",
            r#"
async function foo() {
    return 42
}
"#,
        );
        test.result(result).assert_lint("require-await");
    }

    #[test]
    fn test_async_with_await_allowed() {
        let test = TestProgram::for_rule_with_prelude(RequireAwait);
        let result = test.lint_dir(
            "require_await/test_async_with_await_allowed.ds",
            r#"
async function foo() {
    let result = await fetch()
    return result
}
"#,
        );
        test.result(result).assert_no_lint("require-await");
    }

    #[test]
    fn test_non_async_function_allowed() {
        let test = TestProgram::for_rule_with_prelude(RequireAwait);
        let result = test.lint_dir(
            "require_await/test_non_async_function_allowed.ds",
            r#"
function foo() {
    return 42
}
"#,
        );
        test.result(result).assert_no_lint("require-await");
    }

    #[test]
    fn test_fix_removes_async() {
        let test = TestProgram::for_rule_with_prelude(RequireAwait);
        let result = test.lint_dir(
            "require_await/test_fix_removes_async.ds",
            r#"
async function foo() {
    return 42
}
"#,
        );
        test.result(result)
            .assert_lint("require-await")
            .assert_suggested_fixed(
                r#"
function foo() {
    return 42;
}
"#,
            );
    }

    #[test]
    fn test_allows_empty_async_function() {
        let test = TestProgram::for_rule_with_prelude(RequireAwait);
        let result = test.lint_dir(
            "require_await/test_allows_empty_async_function.ds",
            r#"
async function foo() {}
"#,
        );
        test.result(result).assert_no_lint("require-await");
    }

    #[test]
    fn test_allows_promise_return_without_await() {
        let test = TestProgram::for_rule_with_prelude(RequireAwait);
        let result = test.lint_dir(
            "require_await/test_allows_promise_return_without_await.ds",
            r#"
async function foo() {
    return Promise.resolve(42)
}
"#,
        );
        test.result(result).assert_no_lint("require-await");
    }

    #[test]
    fn test_flags_async_method_without_await() {
        let test = TestProgram::for_rule_with_prelude(RequireAwait);
        let result = test.lint_dir(
            "require_await/test_flags_async_method_without_await.ds",
            r#"
class Worker {
    async run() {
        return 1
    }
}
"#,
        );
        test.result(result).assert_lint("require-await");
    }

    #[test]
    fn test_allows_async_generator_with_promise_yield() {
        let test = TestProgram::for_rule_with_prelude(RequireAwait);
        let result = test.lint_dir(
            "require_await/test_allows_async_generator_with_promise_yield.ds",
            r#"
async function* numbers() {
    yield Promise.resolve(1)
}
"#,
        );
        test.result(result).assert_no_lint("require-await");
    }

    #[test]
    fn test_flags_async_generator_without_await_or_promise_yield() {
        let test = TestProgram::for_rule_with_prelude(RequireAwait);
        let result = test.lint_dir(
            "require_await/test_flags_async_generator_without_await_or_promise_yield.ds",
            r#"
async function* numbers() {
    yield 1
}
"#,
        );
        test.result(result).assert_lint("require-await");
    }

    #[test]
    fn test_allows_async_function_expression_return_call() {
        let test = TestProgram::for_rule_with_prelude(RequireAwait);
        let result = test.lint_dir(
            "require_await/test_allows_async_function_expression_return_call.ds",
            r#"
const fetch_value = async function (): Promise<int32> {
    return await Promise.resolve(1)
}

async function load(): Promise<int32> {
    return fetch_value()
}
"#,
        );
        test.result(result).assert_no_lint("require-await");
    }

    #[test]
    fn test_allows_async_arrow_return_call() {
        let test = TestProgram::for_rule_with_prelude(RequireAwait);
        let result = test.lint_dir(
            "require_await/test_allows_async_arrow_return_call.ds",
            r#"
const fetch_value = async (): Promise<int32> => Promise.resolve(1)

async function load(): Promise<int32> {
    return fetch_value()
}
"#,
        );
        test.result(result).assert_no_lint("require-await");
    }

    #[test]
    fn test_allows_async_arrow_expression_body_promise_return() {
        let test = TestProgram::for_rule_with_prelude(RequireAwait);
        let result = test.lint_dir(
            "require_await/test_allows_async_arrow_expression_body_promise_return.ds",
            r#"
const fetch_value = async (): Promise<int32> => Promise.resolve(1)
const load = async (): Promise<int32> => fetch_value()
"#,
        );
        test.result(result).assert_no_lint("require-await");
    }

    #[test]
    fn test_flags_async_arrow_expression_body_without_await_or_promise_return() {
        let test = TestProgram::for_rule_with_prelude(RequireAwait);
        let result = test.lint_dir(
            "require_await/test_flags_async_arrow_expression_body_without_await_or_promise_return.ds",
            r#"
const load = async (): Promise<int32> => 1
"#,
        );
        test.result(result).assert_lint("require-await");
    }

    #[test]
    fn test_allows_async_function_with_await_using() {
        let test = TestProgram::for_rule_with_prelude(RequireAwait);
        let result = test.lint_dir(
            "require_await/test_allows_async_function_with_await_using.ds",
            r#"
async function load(): Promise<int32> {
    await using resource = makeResource()
    return 1
}

function makeResource(): int32 {
    return 1
}
"#,
        );
        test.result(result).assert_no_lint("require-await");
    }

    #[test]
    fn test_flags_async_function_with_plain_using_only() {
        let test = TestProgram::for_rule_with_prelude(RequireAwait);
        let result = test.lint_dir(
            "require_await/test_flags_async_function_with_plain_using_only.ds",
            r#"
async function load(): Promise<int32> {
    using resource = makeResource()
    return 1
}

function makeResource(): int32 {
    return 1
}
"#,
        );
        test.result(result).assert_lint("require-await");
    }
}

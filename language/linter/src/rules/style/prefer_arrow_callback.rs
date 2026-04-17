use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_enters_nested_declaration_scope, expression_is_new_target, expression_target_symbol,
    expression_unwrap_parenthesized, signature_declares_value_name,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer arrow functions for callbacks.
    ///
    /// Arrow functions are more concise and don't bind their own `this`.
    /// Use arrow functions for callbacks unless `this` binding is needed.
    #[lint(
        id = "prefer-arrow-callback",
        code = "LY033",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferArrowCallback,
    "Prefer arrow functions for callbacks"
}

impl LintRule for PreferArrowCallback {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferArrowCallback::meta()
    }

    /// Check module DIR nodes for callback functions that can be arrows.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let options = prefer_arrow_callback_options(ctx);

        // inspect callback arguments in call like expressions
        for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
            let expression = ctx.tree.get(expression_id);
            let arguments = match expression {
                dir::Expression::Call { arguments, .. }
                | dir::Expression::New { arguments, .. } => arguments,
                _ => continue,
            };

            for argument_id in arguments {
                check_callback_argument(ctx, meta, options, *argument_id);
            }
        }
    }
}

/// Configuration used by prefer-arrow-callback checks.
#[derive(Debug, Clone, Copy)]
struct PreferArrowCallbackOptions {
    /// Allow named callbacks when true.
    allow_named_functions: bool,
    /// Allow callbacks with unbound `this` references when true.
    allow_unbound_this: bool,
}

/// Resolve the rule options from linter configuration.
fn prefer_arrow_callback_options(ctx: &LintModuleDirContext<'_>) -> PreferArrowCallbackOptions {
    PreferArrowCallbackOptions {
        allow_named_functions: ctx
            .options
            .style
            .prefer_arrow_callback_allow_named_functions,
        allow_unbound_this: ctx.options.style.prefer_arrow_callback_allow_unbound_this,
    }
}

/// Check whether one argument is a callback that should use an arrow function.
fn check_callback_argument(
    ctx: &mut LintModuleDirContext<'_>,
    meta: &'static LintMeta,
    options: PreferArrowCallbackOptions,
    argument_id: dir::LocalNodeId<dir::Argument>,
) {
    let argument = ctx.tree.get(argument_id);
    if matches!(argument, dir::Argument::Spread { .. }) {
        return;
    }

    // resolve callback candidates from the argument value
    let candidates = callback_candidates(ctx, argument.value());
    for candidate in candidates {
        let declaration = ctx.tree.get(candidate.declaration_id);
        let dir::Declaration::Function(declaration) = declaration else {
            continue;
        };

        // keep callbacks that are already arrows or cannot be arrows
        if declaration.signature.kind == dir::FunctionKind::Lambda
            || declaration.signature.cardinality == dir::FunctionCardinality::Generator
        {
            continue;
        }

        // keep named callbacks when configured
        if options.allow_named_functions && declaration.name.is_some() {
            continue;
        }

        // inspect callback body semantics with DIR level symbol information
        let function_symbol = declaration.symbol.into_global(ctx.module_id());
        let function_name = declaration.name.map(|name| name.string());
        let body_usage = callback_body_usage(
            ctx,
            declaration.body,
            &declaration.signature,
            function_symbol,
            function_name,
        );
        if body_usage.uses_super
            || body_usage.uses_new_target
            || body_usage.uses_arguments
            || body_usage.uses_recursive_name
        {
            continue;
        }

        // keep unbound `this` callbacks when configured
        if body_usage.uses_this && !candidate.is_lexical_this && options.allow_unbound_this {
            continue;
        }

        let severity = ctx.get_effective_severity(meta, argument_id);
        if !severity.is_enabled() {
            continue;
        }

        // keep fixes out of cases where arrow conversion is not source safe
        let can_fix = candidate.can_fix
            && declaration.signature.this_parameter.is_none()
            && (!body_usage.uses_this || candidate.is_lexical_this);

        let mut diagnostic = LintDiagnostic::new(
            PREFER_ARROW_CALLBACK.id,
            PREFER_ARROW_CALLBACK.code,
            PREFER_ARROW_CALLBACK.category,
            severity,
            "prefer arrow function for callback",
            ctx.module.file_id,
            candidate.replacement_span,
        )
        .with_label("use an arrow function for this callback");

        // attach the source rewrite when the wrapper shape is fixable
        if can_fix && let Some(fix) = callback_fix(ctx, &candidate) {
            diagnostic = diagnostic.with_fix(fix);
        }

        ctx.report(diagnostic);
    }
}

/// One callback function candidate and conversion mode.
struct CallbackCandidate {
    /// The callback function declaration id.
    declaration_id: dir::LocalNodeId<dir::Declaration>,
    /// The source span to replace in autofixes.
    replacement_span: Span,
    /// Whether the callback uses lexical `this` through `.bind(this)`.
    is_lexical_this: bool,
    /// Whether this candidate supports safe autofixes.
    can_fix: bool,
    /// Whether the replacement must wrap the arrow in parentheses.
    wrap_arrow: bool,
}

/// Candidate fix policy while walking wrapper expressions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CallbackFixMode {
    /// Produce fixes when shape checks pass.
    Allowed,
    /// Report without fixes for this branch.
    Disabled,
}

/// Resolve callback candidates from one argument value expression.
fn callback_candidates(
    ctx: &LintModuleDirContext<'_>,
    value_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Vec<CallbackCandidate> {
    let mut candidates = Vec::new();

    // walk supported callback wrapper shapes
    collect_callback_candidates(
        ctx,
        value_expression_id,
        false,
        false,
        CallbackFixMode::Allowed,
        &mut candidates,
    );

    candidates
}

/// Collect callback candidates through wrapper expressions.
fn collect_callback_candidates(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    is_lexical_this: bool,
    wrap_arrow: bool,
    fix_mode: CallbackFixMode,
    candidates: &mut Vec<CallbackCandidate>,
) {
    let expression_id = expression_unwrap_parenthesized(ctx.tree, expression_id);
    let expression = ctx.tree.get(expression_id);

    // match direct function expression callbacks
    if let dir::Expression::Declaration(declaration) = expression {
        let declaration_node = ctx.tree.get(*declaration);
        if matches!(declaration_node, dir::Declaration::Function(_)) {
            push_callback_candidate(
                candidates,
                CallbackCandidate {
                    declaration_id: *declaration,
                    replacement_span: ctx.get_span(*declaration),
                    is_lexical_this,
                    can_fix: fix_mode == CallbackFixMode::Allowed,
                    wrap_arrow,
                },
            );
        }
        return;
    }

    // recurse through logical wrappers
    if let dir::Expression::Binary {
        left,
        operator: dir::BinaryOperator::And | dir::BinaryOperator::Or | dir::BinaryOperator::Coalesce,
        right,
    } = expression
    {
        collect_callback_candidates(ctx, *left, is_lexical_this, true, fix_mode, candidates);
        collect_callback_candidates(ctx, *right, is_lexical_this, true, fix_mode, candidates);
        return;
    }

    // recurse through ternary wrappers
    if let dir::Expression::If {
        kind: dir::IfKind::Ternary,
        then_expression,
        else_expression: Some(else_expression_id),
        ..
    } = expression
    {
        collect_callback_candidates(
            ctx,
            *then_expression,
            is_lexical_this,
            false,
            fix_mode,
            candidates,
        );
        collect_callback_candidates(
            ctx,
            *else_expression_id,
            is_lexical_this,
            false,
            fix_mode,
            candidates,
        );
        return;
    }

    // recurse through optional and non-null wrappers
    if let dir::Expression::Maybe { left } | dir::Expression::Must { left } = expression {
        collect_callback_candidates(
            ctx,
            *left,
            is_lexical_this,
            wrap_arrow,
            fix_mode,
            candidates,
        );
        return;
    }

    // handle `.bind(...)` wrappers specially
    let dir::Expression::Call {
        left: call_left_id,
        arguments,
        ..
    } = expression
    else {
        return;
    };
    let Some(bind_shape) = bind_call_shape(ctx, *call_left_id, arguments.as_slice()) else {
        return;
    };

    let bind_target_id = expression_unwrap_parenthesized(ctx.tree, bind_shape.target_id);
    let bind_target_expression = ctx.tree.get(bind_target_id);

    // remove direct `.bind(this)` wrappers around function literals
    if bind_shape.is_lexical_this
        && let dir::Expression::Declaration(declaration) = bind_target_expression
    {
        let declaration_node = ctx.tree.get(*declaration);
        if matches!(declaration_node, dir::Declaration::Function(_)) {
            push_callback_candidate(
                candidates,
                CallbackCandidate {
                    declaration_id: *declaration,
                    replacement_span: ctx.get_span(expression_id),
                    is_lexical_this: true,
                    can_fix: fix_mode == CallbackFixMode::Allowed,
                    wrap_arrow,
                },
            );
            return;
        }
    }

    // keep outer wrappers when direct bind removal is not available
    let nested_fix_mode = if bind_shape.is_lexical_this {
        CallbackFixMode::Disabled
    } else {
        fix_mode
    };
    let nested_wrap_arrow = wrap_arrow || !bind_shape.is_lexical_this;

    collect_callback_candidates(
        ctx,
        bind_target_id,
        bind_shape.is_lexical_this,
        nested_wrap_arrow,
        nested_fix_mode,
        candidates,
    );
}

/// Push one callback candidate while preventing duplicate reports.
fn push_callback_candidate(candidates: &mut Vec<CallbackCandidate>, candidate: CallbackCandidate) {
    let exists = candidates.iter().any(|existing| {
        existing.declaration_id == candidate.declaration_id
            && existing.replacement_span == candidate.replacement_span
    });
    if exists {
        return;
    }

    candidates.push(candidate);
}

/// Parsed shape for one `.bind(...)` call wrapper.
struct BindCallShape {
    /// The callback target expression.
    target_id: dir::LocalNodeId<dir::Expression>,
    /// Whether this bind call is lexical `.bind(this)`.
    is_lexical_this: bool,
}

/// Parse one `.bind(...)` call wrapper.
fn bind_call_shape(
    ctx: &LintModuleDirContext<'_>,
    call_left_id: dir::LocalNodeId<dir::Expression>,
    arguments: &[dir::LocalNodeId<dir::Argument>],
) -> Option<BindCallShape> {
    let bind_name = ctx.repository.strings.intern("bind");
    let call_left_id = expression_unwrap_parenthesized(ctx.tree, call_left_id);
    let call_left = ctx.tree.get(call_left_id);

    // require one plain member access named `bind`
    let dir::Expression::Member {
        left,
        name,
        generic_arguments,
    } = call_left
    else {
        return None;
    };
    if !generic_arguments.is_empty() {
        return None;
    }
    if *name != Some(bind_name) {
        return None;
    }

    // treat only single `this` arguments as lexical binds
    let is_lexical_this = arguments.len() == 1
        && arguments
            .first()
            .is_some_and(|argument_id| argument_is_this_expression(ctx, *argument_id));

    Some(BindCallShape {
        target_id: *left,
        is_lexical_this,
    })
}

/// Return true when one argument is exactly `this`.
fn argument_is_this_expression(
    ctx: &LintModuleDirContext<'_>,
    argument_id: dir::LocalNodeId<dir::Argument>,
) -> bool {
    let argument = ctx.tree.get(argument_id);
    let expression_id = expression_unwrap_parenthesized(ctx.tree, argument.value());

    matches!(ctx.tree.get(expression_id), dir::Expression::This)
}

/// One summary of callback body references relevant to arrow conversion.
#[derive(Debug, Default, Clone, Copy)]
struct CallbackBodyUsage {
    /// Whether the callback body references `this`.
    uses_this: bool,
    /// Whether the callback body references `super`.
    uses_super: bool,
    /// Whether the callback body references `new.target`.
    uses_new_target: bool,
    /// Whether the callback body references function-local `arguments`.
    uses_arguments: bool,
    /// Whether the callback body references its own function symbol.
    uses_recursive_name: bool,
}

/// Analyze callback body references that affect arrow conversion.
fn callback_body_usage(
    ctx: &LintModuleDirContext<'_>,
    body_expression_id: Option<dir::LocalNodeId<dir::Expression>>,
    signature: &dir::FunctionSignature,
    function_symbol: dir::GlobalSymbolId,
    function_name: Option<dir::StringId>,
) -> CallbackBodyUsage {
    let Some(body_expression_id) = body_expression_id else {
        return CallbackBodyUsage::default();
    };

    let arguments_name = ctx.repository.strings.intern("arguments");
    let new_name = ctx.repository.strings.intern("new");
    let target_name = ctx.repository.strings.intern("target");
    let ignore_arguments_reference =
        signature_declares_value_name(ctx.tree, ctx.symbols, signature, arguments_name);

    // walk only the current callback body
    let mut visitor = CallbackBodyUsageVisitor {
        root_expression_id: body_expression_id,
        function_symbol,
        function_name,
        arguments_name,
        new_name,
        target_name,
        ignore_arguments_reference,
        usage: CallbackBodyUsage::default(),
        options: NodeVisitorOptions::default(),
    };
    let body_expression = ctx.tree.get(body_expression_id);
    visitor.visit_expression(ctx.tree, body_expression_id, body_expression);

    visitor.usage
}

/// Visitor that collects callback body references and skips nested callables.
struct CallbackBodyUsageVisitor {
    /// The root expression for the callback body.
    root_expression_id: dir::LocalNodeId<dir::Expression>,
    /// The callback function symbol.
    function_symbol: dir::GlobalSymbolId,
    /// The callback function name when present.
    function_name: Option<dir::StringId>,
    /// The string id for `arguments`.
    arguments_name: dir::StringId,
    /// The string id for `new`.
    new_name: dir::StringId,
    /// The string id for `target`.
    target_name: dir::StringId,
    /// Whether one parameter shadows `arguments`.
    ignore_arguments_reference: bool,
    /// Collected body usage flags.
    usage: CallbackBodyUsage,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl NodeVisitor for CallbackBodyUsageVisitor {
    /// Return visitor options.
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    /// Visit one expression in the callback body.
    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // keep nested callable scopes out of the current callback
        if expression_id != self.root_expression_id
            && expression_enters_nested_declaration_scope(tree, expression)
        {
            return;
        }

        // collect callback semantics from the current expression
        match expression {
            dir::Expression::This => {
                self.usage.uses_this = true;
            }
            dir::Expression::Super => {
                self.usage.uses_super = true;
            }
            _ => {}
        }

        // detect `new.target` directly from the current expression
        if expression_is_new_target(tree, expression_id, self.new_name, self.target_name) {
            self.usage.uses_new_target = true;
        }

        // detect `arguments` and recursive function references semantically
        if !self.ignore_arguments_reference
            && expression_is_single_name_reference(tree, expression_id, self.arguments_name)
        {
            self.usage.uses_arguments = true;
        }
        if expression_target_symbol(tree, expression_id) == Some(self.function_symbol)
            || self
                .function_name
                .is_some_and(|name| expression_is_single_name_reference(tree, expression_id, name))
        {
            self.usage.uses_recursive_name = true;
        }

        // walk the current expression subtree
        walk_expression(self, tree, expression_id, expression);
    }
}

/// Return true when one expression is a single segment reference to the target name.
fn expression_is_single_name_reference(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    name: dir::StringId,
) -> bool {
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);

    match expression {
        dir::Expression::UnresolvedPath {
            path,
            generic_arguments,
            ..
        }
        | dir::Expression::LocalReference {
            path,
            generic_arguments,
            ..
        }
        | dir::Expression::ModuleReference {
            path,
            generic_arguments,
            ..
        }
        | dir::Expression::GlobalReference {
            path,
            generic_arguments,
            ..
        } => generic_arguments.is_empty() && path.segments.len() == 1 && path.segments[0] == name,
        _ => false,
    }
}

/// Build a safe fix for one callback candidate.
fn callback_fix(ctx: &LintModuleDirContext<'_>, candidate: &CallbackCandidate) -> Option<LintFix> {
    let declaration_span = ctx.get_span(candidate.declaration_id);

    // keep comment carrying wrapper suffixes out of autofix
    if candidate.replacement_span.end > declaration_span.end {
        let suffix_span = Span::new(
            candidate.replacement_span.file,
            declaration_span.end,
            candidate.replacement_span.end,
        );
        let suffix_text = ctx.get_span_text(suffix_span);
        if source_text_contains_comment(suffix_text) {
            return None;
        }
    }

    // slice the declaration text around the actual body span
    let declaration_text = ctx.get_span_text(declaration_span);
    let declaration = ctx.tree.get(candidate.declaration_id);
    let dir::Declaration::Function(declaration) = declaration else {
        return None;
    };
    let body_expression_id = declaration.body?;
    let body_span = ctx.get_span(body_expression_id);
    if body_span.start < declaration_span.start || body_span.start > declaration_span.end {
        return None;
    }

    let body_start_offset = (body_span.start - declaration_span.start) as usize;
    let prefix_text = &declaration_text[..body_start_offset];
    let body_text = &declaration_text[body_start_offset..];

    // rewrite `function` form to arrow form while preserving return annotations
    let (async_prefix, rest) = if let Some(rest) = prefix_text.strip_prefix("async function") {
        ("async ", rest)
    } else if let Some(rest) = prefix_text.strip_prefix("function") {
        ("", rest)
    } else {
        return None;
    };

    let parameter_start_index = rest.find('(')?;
    let parameters_text = rest[parameter_start_index..].trim_end();
    let replacement = format!("{async_prefix}{parameters_text} => {body_text}");
    let replacement = if candidate.wrap_arrow {
        format!("({replacement})")
    } else {
        replacement
    };

    let edits = ctx
        .edit_builder()
        .replace(candidate.replacement_span, replacement)
        .into_edits();
    if edits.is_empty() {
        return None;
    }

    Some(LintFix::safe("Convert to arrow function").with_edits(edits))
}

/// Return true when one source slice contains comment trivia.
fn source_text_contains_comment(source: &str) -> bool {
    source.contains("//") || source.contains("/*")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Allow callbacks that already use arrow form.
    #[test]
    fn test_allows_arrow_callback() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_allows_arrow_callback.ds",
            r#"
items.map((x) => x + 1)
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    /// Flag plain function expression callbacks.
    #[test]
    fn test_detects_function_callback() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_detects_function_callback.ds",
            r#"
items.map(function(x) { return x + 1 })
"#,
        );
        test.result(result).assert_lint("prefer-arrow-callback");
    }

    /// Rewrite named callbacks to arrow form.
    #[test]
    fn test_flags_named_function() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_flags_named_function.ds",
            r#"
items.map(function increment(x) { return x + 1 })
"#,
        );
        test.result(result)
            .assert_lint("prefer-arrow-callback")
            .assert_safe_fixed(
                r#"
items.map((x) => {
    return x + 1
})
"#,
            );
    }

    /// Allow functions that are not used as callbacks.
    #[test]
    fn test_allows_non_callback_function() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_allows_non_callback_function.ds",
            r#"
function foo() {
    return 42
}
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    /// Rewrite plain function callbacks to arrows.
    #[test]
    fn test_fix_function_to_arrow() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_fix_function_to_arrow.ds",
            r#"
items.map(function(x) { return x + 1 });
"#,
        );
        test.result(result)
            .assert_lint("prefer-arrow-callback")
            .assert_safe_fixed(
                r#"
items.map((x) => {
    return x + 1;
});
"#,
            );
    }

    /// Allow callbacks that use unbound `this` by default.
    #[test]
    fn test_allows_callback_using_this() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_allows_callback_using_this.ds",
            r#"
items.map(function(x) { this.log(x); return x })
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    /// Allow callbacks that use function-local `arguments`.
    #[test]
    fn test_allows_callback_using_arguments() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_allows_callback_using_arguments.ds",
            r#"
items.map(function(x) { return arguments[0] })
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    /// Allow callbacks that use `new.target`.
    #[test]
    fn test_allows_callback_using_new_target() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_allows_callback_using_new_target.ds",
            r#"
items.map(function(x) { return new.target ?? x })
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    /// Allow generator callbacks because arrows cannot express them.
    #[test]
    fn test_allows_generator_callback() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_allows_generator_callback.ds",
            r#"
items.map(function* (x) { yield x })
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    /// Rewrite async callbacks to async arrows.
    #[test]
    fn test_fix_async_function_callback() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_fix_async_function_callback.ds",
            r#"
items.map(async function(x) { return await doWork(x) })
"#,
        );
        test.result(result)
            .assert_lint("prefer-arrow-callback")
            .assert_safe_fixed(
                r#"
items.map(async (x) => {
    return await doWork(x)
})
"#,
            );
    }

    /// Rewrite direct `.bind(this)` callbacks to lexical arrows.
    #[test]
    fn test_fix_bound_this_callback_form() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_fix_bound_this_callback_form.ds",
            r#"
items.map(function(x) { return this.transform(x) }.bind(this))
"#,
        );
        test.result(result)
            .assert_lint("prefer-arrow-callback")
            .assert_safe_fixed(
                r#"
items.map((x) => {
    return this.transform(x)
})
"#,
            );
    }

    /// Allow `.bind(this, extra)` when the callback needs unbound `this`.
    #[test]
    fn test_allows_bound_callback_with_additional_bind_arguments() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_allows_bound_callback_with_additional_bind_arguments.ds",
            r#"
items.map(function(x) { return this.transform(x) }.bind(this, extra))
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    /// Keep non-lexical bind wrappers and rewrite only the inner callback.
    #[test]
    fn test_fix_non_lexical_bind_callback_without_this_usage() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_fix_non_lexical_bind_callback_without_this_usage.ds",
            r#"
items.map(function(x) { return x + 1 }.bind(this, extra))
"#,
        );
        test.result(result)
            .assert_lint("prefer-arrow-callback")
            .assert_safe_fixed(
                r#"
items.map(((x) => {
    return x + 1
}).bind(this, extra))
"#,
            );
    }

    /// Report unbound `this` callbacks without fixing when configured strictly.
    #[test]
    fn test_flags_unbound_this_when_option_disallows_it() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback)
            .with_options(|options| options.style.prefer_arrow_callback_allow_unbound_this = false);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_flags_unbound_this_when_option_disallows_it.ds",
            r#"
items.map(function(x) { return this.transform(x) })
"#,
        );
        test.result(result)
            .assert_lint("prefer-arrow-callback")
            .assert_has_no_fix("prefer-arrow-callback");
    }

    /// Report callbacks with explicit `this` parameters without fixing.
    #[test]
    fn test_flags_this_parameter_callback_without_fix() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback)
            .with_options(|options| options.style.prefer_arrow_callback_allow_unbound_this = false);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_flags_this_parameter_callback_without_fix.ds",
            r#"
items.map(function(this: Service, x) { return this.transform(x) })
"#,
        );
        test.result(result)
            .assert_lint("prefer-arrow-callback")
            .assert_has_no_fix("prefer-arrow-callback");
    }

    /// Allow named callbacks when configured.
    #[test]
    fn test_allows_named_callback_when_option_enabled() {
        let test =
            TestProgram::for_rule_without_prelude(PreferArrowCallback).with_options(|options| {
                options.style.prefer_arrow_callback_allow_named_functions = true
            });
        let result = test.lint_dir(
            "prefer_arrow_callback/test_allows_named_callback_when_option_enabled.ds",
            r#"
items.map(function increment(x) { return x + 1 })
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    /// Allow recursive named callbacks.
    #[test]
    fn test_allows_recursive_named_callback() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_allows_recursive_named_callback.ds",
            r#"
items.map(function loop(x) {
    return x > 1 ? loop(x - 1) : x
})
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    /// Rewrite callbacks inside logical wrappers.
    #[test]
    fn test_fix_callback_inside_logical_wrapper() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_fix_callback_inside_logical_wrapper.ds",
            r#"
items.map(nativeCallback || function(x) { return x + 1 })
"#,
        );
        test.result(result)
            .assert_lint("prefer-arrow-callback")
            .assert_safe_fixed(
                r#"
items.map(nativeCallback || ((x) => {
    return x + 1
}))
"#,
            );
    }

    /// Rewrite callbacks inside ternary wrappers.
    #[test]
    fn test_fix_callback_inside_ternary_wrapper() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_fix_callback_inside_ternary_wrapper.ds",
            r#"
items.map(flag ? function(x) { return x } : function(y) { return y })
"#,
        );
        test.result(result)
            .assert_lint_count("prefer-arrow-callback", 2)
            .assert_safe_fixed(
                r#"
items.map(flag ? (x) => {
    return x
} : (y) => {
    return y
})
"#,
            );
    }

    /// Report outer lexical bind wrappers without fixing nested rewrites.
    #[test]
    fn test_flags_bound_logical_wrapper_without_fix() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_flags_bound_logical_wrapper_without_fix.ds",
            r#"
items.map((nativeCallback || function(x) { return x + 1 }).bind(this))
"#,
        );
        test.result(result)
            .assert_lint("prefer-arrow-callback")
            .assert_has_no_fix("prefer-arrow-callback");
    }

    /// Flag constructor callbacks passed through `new`.
    #[test]
    fn test_flags_new_expression_callback() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_dir(
            "prefer_arrow_callback/test_flags_new_expression_callback.ds",
            r#"
let mapper = new Mapper(function(x) { return x + 1 });
"#,
        );
        test.result(result).assert_lint("prefer-arrow-callback");
    }
}

use destack_ast::{
    self as ast, Argument, BinaryOperator, Declaration, Expression, FunctionCardinality,
    FunctionKind, IfKind, NodeVisitor, Parameter, Pattern, PatternField,
};
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_static_property_access_syntax, expression_unwrap_parenthesized_syntax,
    span_has_comment_trivia,
};
use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer arrow functions for callbacks.
    ///
    /// Arrow functions are more concise and don't bind their own `this`.
    /// Use arrow functions for callbacks unless `this` binding is needed.
    #[lint(
        id = "prefer-arrow-callback",
        code = "LY033",
        category = Style,
        level = Ast,
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
    fn meta(&self) -> &'static crate::LintMeta {
        PreferArrowCallback::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        let options = prefer_arrow_callback_options(ctx);

        // iterate over call and new expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let dynamic_arguments = match ctx.tree.get(node_id) {
                Expression::Call {
                    dynamic_arguments, ..
                }
                | Expression::New {
                    dynamic_arguments, ..
                } => dynamic_arguments,
                _ => continue,
            };

            for argument_id in dynamic_arguments {
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
fn prefer_arrow_callback_options(ctx: &LintModuleAstContext<'_>) -> PreferArrowCallbackOptions {
    PreferArrowCallbackOptions {
        allow_named_functions: ctx.options.allow_named_functions_in_prefer_arrow_callback,
        allow_unbound_this: ctx.options.allow_unbound_this_in_prefer_arrow_callback,
    }
}

/// Check if an argument is a function expression that should be an arrow function.
fn check_callback_argument(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static crate::LintMeta,
    options: PreferArrowCallbackOptions,
    argument_id: ast::LocalNodeId<Argument>,
) {
    let argument = ctx.tree.get(argument_id);

    // get value from the argument (Positional, Named, etc.)
    let value_id = match argument {
        Argument::Positional { value, .. } => *value,
        Argument::Named { value, .. } => *value,
        Argument::Labeled { value, .. } => *value,
        Argument::Spread { .. } => return,
    };

    // resolve callback function candidates in direct and wrapped forms
    let candidates = callback_candidates(ctx, value_id);
    for candidate in candidates {
        let declaration = ctx.tree.get(candidate.declaration_id);
        let Declaration::Function {
            descriptor,
            signature,
            body,
            ..
        } = declaration
        else {
            continue;
        };

        // skip lambda functions because they already use arrow syntax
        if signature.kind == FunctionKind::Lambda {
            continue;
        }

        // skip generators because arrow functions cannot be generators
        if signature.cardinality == FunctionCardinality::Generator {
            continue;
        }

        let function_name = descriptor.name.map(|name| name.string());

        // keep named callbacks when allowNamedFunctions is enabled
        if options.allow_named_functions && function_name.is_some() {
            continue;
        }

        // keep one summary of callback body constraints
        let body_usage = body.map(|body_id| {
            callback_body_usage(
                ctx,
                candidate.declaration_id,
                signature,
                body_id,
                function_name,
            )
        });

        // keep callbacks that use unsupported references
        if body_usage.as_ref().is_some_and(|usage| {
            usage.uses_super || usage.uses_arguments || usage.uses_recursive_name
        }) {
            continue;
        }

        // keep unbound this callbacks when configured
        let uses_this = body_usage.as_ref().is_some_and(|usage| usage.uses_this);
        if uses_this && !candidate.is_lexical_this && options.allow_unbound_this {
            continue;
        }

        let severity = ctx.get_effective_severity(meta, argument_id);
        if !severity.is_enabled() {
            continue;
        }

        // keep no-fix mode for unbound this and unsupported wrapper forms
        let can_fix = candidate.can_fix && !(uses_this && !candidate.is_lexical_this);

        let mut diagnostic = LintDiagnostic::new(
            PREFER_ARROW_CALLBACK.id,
            PREFER_ARROW_CALLBACK.code,
            PREFER_ARROW_CALLBACK.category,
            severity,
            "prefer arrow function for callback",
            ctx.module.file_id,
            candidate.replacement_span,
        )
        .with_label("use `() => { ... }` instead of `function() { ... }`");
        if can_fix && let Some(fix) = callback_fix(ctx, &candidate) {
            diagnostic = diagnostic.with_fix(fix);
        }

        ctx.report(diagnostic);
    }
}

/// One callback function candidate and conversion mode.
struct CallbackCandidate {
    /// The function declaration expression id.
    declaration_id: ast::LocalNodeId<Declaration>,
    /// The source span to replace in autofixes.
    replacement_span: Span,
    /// Whether the callback uses lexical this via `.bind(this)`.
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

/// Resolve callback candidates from an argument value expression.
fn callback_candidates(
    ctx: &LintModuleAstContext<'_>,
    value_expression_id: ast::LocalNodeId<Expression>,
) -> Vec<CallbackCandidate> {
    let mut candidates = Vec::new();
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

/// Collect callback candidates through callback wrapper shapes.
fn collect_callback_candidates(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<Expression>,
    is_lexical_this: bool,
    wrap_arrow: bool,
    fix_mode: CallbackFixMode,
    candidates: &mut Vec<CallbackCandidate>,
) {
    let expression_id = expression_unwrap_parenthesized_syntax(ctx.tree, expression_id);
    let expression = ctx.tree.get(expression_id);

    // direct function expression callback
    if let Expression::Declaration(declaration_id) = expression {
        let declaration = ctx.tree.get(*declaration_id);
        if matches!(declaration, Declaration::Function { .. }) {
            let replacement_span = ctx.tree.get_span(*declaration_id);
            push_callback_candidate(
                candidates,
                CallbackCandidate {
                    declaration_id: *declaration_id,
                    replacement_span,
                    is_lexical_this,
                    can_fix: fix_mode == CallbackFixMode::Allowed,
                    wrap_arrow,
                },
            );
        }
        return;
    }

    // logical wrappers can hold callback expressions in either branch
    if let Expression::Binary {
        left,
        operator: BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce,
        right,
    } = expression
    {
        collect_callback_candidates(ctx, *left, is_lexical_this, true, fix_mode, candidates);
        collect_callback_candidates(ctx, *right, is_lexical_this, true, fix_mode, candidates);
        return;
    }

    // ternary wrappers can hold callback expressions in both branches
    if let Expression::If {
        kind: IfKind::Ternary,
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

    // optional and non null wrappers can hold callback expressions
    if let Expression::Maybe { left, .. } | Expression::Must { left, .. } = expression {
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

    // `.bind(...)` wrappers can adjust this semantics and fix policy
    let Expression::Call {
        left: call_left_id,
        dynamic_arguments,
        ..
    } = expression
    else {
        return;
    };
    let Some(bind_shape) = bind_call_shape(ctx, *call_left_id, dynamic_arguments.as_slice()) else {
        return;
    };
    let bind_target_id = expression_unwrap_parenthesized_syntax(ctx.tree, bind_shape.target_id);
    let bind_target_expression = ctx.tree.get(bind_target_id);

    // remove direct `.bind(this)` wrappers for function literals
    if bind_shape.is_lexical_this
        && let Expression::Declaration(declaration_id) = bind_target_expression
    {
        let declaration = ctx.tree.get(*declaration_id);
        if matches!(declaration, Declaration::Function { .. }) {
            push_callback_candidate(
                candidates,
                CallbackCandidate {
                    declaration_id: *declaration_id,
                    replacement_span: ctx.tree.get_span(expression_id),
                    is_lexical_this: true,
                    can_fix: fix_mode == CallbackFixMode::Allowed,
                    wrap_arrow,
                },
            );
            return;
        }
    }

    // recurse into wrapped bind targets when direct bind removal is not available
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
    if !exists {
        candidates.push(candidate);
    }
}

/// Parsed shape for one `.bind(...)` call wrapper.
struct BindCallShape {
    /// The bound callback target expression.
    target_id: ast::LocalNodeId<Expression>,
    /// Whether this bind call is lexical `.bind(this)`.
    is_lexical_this: bool,
}

/// Parse one `.bind(...)` call wrapper.
fn bind_call_shape(
    ctx: &LintModuleAstContext<'_>,
    call_left_id: ast::LocalNodeId<Expression>,
    dynamic_arguments: &[ast::LocalNodeId<Argument>],
) -> Option<BindCallShape> {
    let bind_name = ctx.strings.intern("bind");
    let call_left_id = expression_unwrap_parenthesized_syntax(ctx.tree, call_left_id);
    let Some((member_left_id, name)) =
        expression_static_property_access_syntax(ctx.tree, call_left_id)
    else {
        return None;
    };
    if name != bind_name {
        return None;
    }

    let is_lexical_this = dynamic_arguments.len() == 1
        && dynamic_arguments.first().is_some_and(|argument_id| {
            argument_is_this_expression(ctx, ctx.tree.get(*argument_id))
        });

    Some(BindCallShape {
        target_id: member_left_id,
        is_lexical_this,
    })
}

/// Return true when one argument is the `this` expression.
fn argument_is_this_expression(ctx: &LintModuleAstContext<'_>, argument: &Argument) -> bool {
    let expression_id = match argument {
        Argument::Positional { value, .. }
        | Argument::Named { value, .. }
        | Argument::Labeled { value, .. } => *value,
        Argument::Spread { .. } => return false,
    };
    let expression_id = expression_unwrap_parenthesized_syntax(ctx.tree, expression_id);

    matches!(ctx.tree.get(expression_id), Expression::This)
}

/// One summary of callback body references relevant to arrow conversion.
#[derive(Debug, Default, Clone, Copy)]
struct CallbackBodyUsage {
    /// Whether the callback body references `this`.
    uses_this: bool,
    /// Whether the callback body references `super`.
    uses_super: bool,
    /// Whether the callback body references function local `arguments`.
    uses_arguments: bool,
    /// Whether the callback body references its own function name.
    uses_recursive_name: bool,
}

/// Analyze callback body references that affect arrow conversion.
fn callback_body_usage(
    ctx: &LintModuleAstContext<'_>,
    function_declaration_id: ast::LocalNodeId<Declaration>,
    signature: &ast::FunctionSignature,
    body_expression_id: ast::LocalNodeId<Expression>,
    function_name: Option<ast::StringId>,
) -> CallbackBodyUsage {
    let arguments_name = ctx.strings.intern("arguments");
    let ignore_arguments_reference =
        signature_contains_parameter_name(ctx.tree, signature, arguments_name);
    let mut visitor = CallbackBodyUsageVisitor {
        options: ast::NodeVisitorOptions::default(),
        target_function_id: function_declaration_id,
        arguments_name,
        function_name,
        ignore_arguments_reference,
        usage: CallbackBodyUsage::default(),
    };
    let body_expression = ctx.tree.get(body_expression_id);
    visitor.visit_expression(ctx.tree, body_expression_id, body_expression);

    visitor.usage
}

/// Visitor that collects callback body references and skips nested functions.
struct CallbackBodyUsageVisitor {
    /// Visitor options.
    options: ast::NodeVisitorOptions,
    /// The callback function being analyzed.
    target_function_id: ast::LocalNodeId<Declaration>,
    /// String id for `arguments`.
    arguments_name: ast::StringId,
    /// Optional function name for recursive reference detection.
    function_name: Option<ast::StringId>,
    /// Whether `arguments` is a parameter and should be ignored.
    ignore_arguments_reference: bool,
    /// Collected body reference usage.
    usage: CallbackBodyUsage,
}

impl ast::NodeVisitor for CallbackBodyUsageVisitor {
    fn options(&self) -> &ast::NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &ast::NodeTree,
        expression_id: ast::LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // skip nested function scopes so we only inspect the callback body itself
        if let Expression::Declaration(declaration_id) = expression
            && *declaration_id != self.target_function_id
        {
            let declaration = tree.get(*declaration_id);
            if matches!(declaration, Declaration::Function { .. }) {
                return;
            }
        }

        // collect relevant reference kinds
        match expression {
            Expression::This => {
                self.usage.uses_this = true;
            }
            Expression::Super => {
                self.usage.uses_super = true;
            }
            Expression::Path { path, .. } if path.segments.len() == 1 => {
                let name = path.segments[0];
                if !self.ignore_arguments_reference && name == self.arguments_name {
                    self.usage.uses_arguments = true;
                }
                if self
                    .function_name
                    .is_some_and(|function_name| name == function_name)
                {
                    self.usage.uses_recursive_name = true;
                }
            }
            _ => {}
        }

        ast::walk_expression(self, tree, expression_id, expression);
    }
}

/// Return true when one function signature includes a parameter name.
fn signature_contains_parameter_name(
    tree: &ast::NodeTree,
    signature: &ast::FunctionSignature,
    name: ast::StringId,
) -> bool {
    if signature
        .this_parameter
        .is_some_and(|parameter_id| parameter_contains_name(tree, parameter_id, name))
    {
        return true;
    }

    signature
        .dynamic_parameters
        .iter()
        .any(|parameter_id| parameter_contains_name(tree, *parameter_id, name))
}

/// Return true when one parameter declares the target name.
fn parameter_contains_name(
    tree: &ast::NodeTree,
    parameter_id: ast::LocalNodeId<Parameter>,
    name: ast::StringId,
) -> bool {
    let parameter = tree.get(parameter_id);
    match parameter {
        Parameter::Named {
            name: parameter_name,
            ..
        }
        | Parameter::VariadicNamed {
            name: parameter_name,
            ..
        } => *parameter_name == name,
        Parameter::Pattern { pattern, .. } | Parameter::VariadicPattern { pattern, .. } => {
            pattern_contains_name(tree, *pattern, name)
        }
    }
}

/// Return true when one pattern binds the target name.
fn pattern_contains_name(
    tree: &ast::NodeTree,
    pattern_id: ast::LocalNodeId<Pattern>,
    name: ast::StringId,
) -> bool {
    let pattern = tree.get(pattern_id);
    match pattern {
        Pattern::Binding {
            name: binding_name,
            pattern,
            ..
        } => {
            *binding_name == name
                || pattern.is_some_and(|inner_id| pattern_contains_name(tree, inner_id, name))
        }
        Pattern::Must(inner_id)
        | Pattern::ReferenceOf {
            right: inner_id, ..
        }
        | Pattern::ValueOf {
            right: inner_id, ..
        } => pattern_contains_name(tree, *inner_id, name),
        Pattern::Tuple { fields }
        | Pattern::Array { fields }
        | Pattern::Object { fields }
        | Pattern::TaggedTuple { fields, .. }
        | Pattern::TaggedObject { fields, .. } => fields
            .iter()
            .any(|field_id| pattern_field_contains_name(tree, *field_id, name)),
        Pattern::Union { patterns } => patterns
            .iter()
            .any(|inner_id| pattern_contains_name(tree, *inner_id, name)),
        _ => false,
    }
}

/// Return true when one pattern field binds the target name.
fn pattern_field_contains_name(
    tree: &ast::NodeTree,
    field_id: ast::LocalNodeId<PatternField>,
    name: ast::StringId,
) -> bool {
    let field = tree.get(field_id);
    match field {
        PatternField::Named {
            name: field_name,
            pattern,
            ..
        } => {
            field_name.string() == name
                || pattern.is_some_and(|pattern_id| pattern_contains_name(tree, pattern_id, name))
        }
        PatternField::Alias { alias, .. } => *alias == name,
        PatternField::Positional { pattern, .. } => pattern_contains_name(tree, *pattern, name),
        PatternField::Spread {
            pattern: Some(pattern_id),
            ..
        } => pattern_contains_name(tree, *pattern_id, name),
        _ => false,
    }
}

/// Build a safe fix for one callback candidate.
fn callback_fix(ctx: &LintModuleAstContext<'_>, candidate: &CallbackCandidate) -> Option<LintFix> {
    let declaration_span = ctx.tree.get_span(candidate.declaration_id);
    if candidate.replacement_span.end > declaration_span.end {
        let suffix_span = Span::new(
            candidate.replacement_span.file,
            declaration_span.end,
            candidate.replacement_span.end,
        );
        if span_has_comment_trivia(ctx.tree, suffix_span) {
            return None;
        }
    }

    let declaration_text = ctx.get_span_text(declaration_span);

    // convert function syntax to arrow syntax
    let (async_prefix, rest) = if let Some(rest) = declaration_text.strip_prefix("async function") {
        ("async ", rest)
    } else if let Some(rest) = declaration_text.strip_prefix("function") {
        ("", rest)
    } else {
        return None;
    };

    // resolve function signature and body slices
    let Some(body_start_index) = rest.find('{') else {
        return None;
    };
    let signature_text = rest[..body_start_index].trim();
    let body_text = &rest[body_start_index..];

    // drop optional function names and keep parameter list with return annotation
    let Some(parameter_start_index) = signature_text.find('(') else {
        return None;
    };
    let parameters_text = signature_text[parameter_start_index..].trim();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_allows_arrow_callback() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_ast(
            "prefer_arrow_callback/test_allows_arrow_callback.ds",
            r#"
items.map((x) => x + 1)
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    #[test]
    fn test_detects_function_callback() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_ast(
            "prefer_arrow_callback/test_detects_function_callback.ds",
            r#"
items.map(function(x) { return x + 1 })
"#,
        );
        test.result(result).assert_lint("prefer-arrow-callback");
    }

    #[test]
    fn test_flags_named_function() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_ast(
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

    #[test]
    fn test_allows_non_callback_function() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        // function declarations not used as callbacks
        let result = test.lint_ast(
            "prefer_arrow_callback/test_allows_non_callback_function.ds",
            r#"
function foo() {
    return 42
}
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    #[test]
    fn test_fix_function_to_arrow() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_ast(
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

    #[test]
    fn test_allows_callback_using_this() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_ast(
            "prefer_arrow_callback/test_allows_callback_using_this.ds",
            r#"
items.map(function(x) { this.log(x); return x })
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    #[test]
    fn test_allows_callback_using_arguments() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_ast(
            "prefer_arrow_callback/test_allows_callback_using_arguments.ds",
            r#"
items.map(function(x) { return arguments[0] })
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    #[test]
    fn test_allows_generator_callback() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_ast(
            "prefer_arrow_callback/test_allows_generator_callback.ds",
            r#"
items.map(function* (x) { yield x })
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    #[test]
    fn test_fix_async_function_callback() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_ast(
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

    #[test]
    fn test_fix_bound_this_callback_form() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_ast(
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

    #[test]
    fn test_allows_bound_callback_with_additional_bind_arguments() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_ast(
            "prefer_arrow_callback/test_allows_bound_callback_with_additional_bind_arguments.ds",
            r#"
items.map(function(x) { return this.transform(x) }.bind(this, extra))
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    #[test]
    fn test_fix_non_lexical_bind_callback_without_this_usage() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_ast(
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

    #[test]
    fn test_flags_unbound_this_when_option_disallows_it() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback)
            .with_options(|options| options.allow_unbound_this_in_prefer_arrow_callback = false);
        let result = test.lint_ast(
            "prefer_arrow_callback/test_flags_unbound_this_when_option_disallows_it.ds",
            r#"
items.map(function(x) { return this.transform(x) })
"#,
        );
        test.result(result)
            .assert_lint("prefer-arrow-callback")
            .assert_has_no_fix("prefer-arrow-callback");
    }

    #[test]
    fn test_allows_named_callback_when_option_enabled() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback)
            .with_options(|options| options.allow_named_functions_in_prefer_arrow_callback = true);
        let result = test.lint_ast(
            "prefer_arrow_callback/test_allows_named_callback_when_option_enabled.ds",
            r#"
items.map(function increment(x) { return x + 1 })
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    #[test]
    fn test_allows_recursive_named_callback() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_ast(
            "prefer_arrow_callback/test_allows_recursive_named_callback.ds",
            r#"
items.map(function loop(x) {
    return x > 1 ? loop(x - 1) : x
})
"#,
        );
        test.result(result).assert_no_lint("prefer-arrow-callback");
    }

    #[test]
    fn test_fix_callback_inside_logical_wrapper() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_ast(
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

    #[test]
    fn test_fix_callback_inside_ternary_wrapper() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_ast(
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

    #[test]
    fn test_flags_bound_logical_wrapper_without_fix() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_ast(
            "prefer_arrow_callback/test_flags_bound_logical_wrapper_without_fix.ds",
            r#"
items.map((nativeCallback || function(x) { return x + 1 }).bind(this))
"#,
        );
        test.result(result)
            .assert_lint("prefer-arrow-callback")
            .assert_has_no_fix("prefer-arrow-callback");
    }

    #[test]
    fn test_flags_new_expression_callback() {
        let test = TestProgram::for_rule_without_prelude(PreferArrowCallback);
        let result = test.lint_ast(
            "prefer_arrow_callback/test_flags_new_expression_callback.ds",
            r#"
let mapper = new Mapper(function(x) { return x + 1 });
"#,
        );
        test.result(result).assert_lint("prefer-arrow-callback");
    }
}

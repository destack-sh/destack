use destack_source::{ModuleId, Span};
use destack_workspace::LintSeverity;
use {destack_ast as ast, destack_dir as dir};

use crate::rules::common::{
    expression_declared_or_inferred_type_id, expression_structural_signature,
    symbol_primary_declaration_for,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow explicit trailing type arguments equal to declared defaults.
    ///
    /// Repeating default generic arguments adds noise without changing behavior.
    #[lint(
        id = "no-unnecessary-type-arguments",
        code = "LC105",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Experimental,
        declarations = Exclude
    )]
    pub NoUnnecessaryTypeArguments,
    "Disallow unnecessary explicit trailing type arguments"
}

impl LintRule for NoUnnecessaryTypeArguments {
    fn meta(&self) -> &'static LintMeta {
        NoUnnecessaryTypeArguments::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // inspect expressions with explicit static arguments
        for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
            let expression = ctx.tree.get(expression_id);
            let Some(static_arguments) = expression.static_arguments() else {
                continue;
            };
            if static_arguments.is_empty() {
                continue;
            }

            let Some(target_symbol) = target_symbol_for_expression(ctx, expression_id, expression)
            else {
                continue;
            };
            let Some(static_parameter_defaults) =
                static_parameter_defaults_for_symbol(ctx, target_symbol)
            else {
                continue;
            };
            if static_parameter_defaults.len() < static_arguments.len() {
                continue;
            }

            let Some(first_redundant_index) = first_redundant_trailing_argument_index(
                ctx,
                static_arguments,
                &static_parameter_defaults,
            ) else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            let redundant_argument_id = static_arguments[first_redundant_index];
            let span = ctx.get_span(redundant_argument_id);
            let mut diagnostic = LintDiagnostic::new(
                NO_UNNECESSARY_TYPE_ARGUMENTS.id,
                NO_UNNECESSARY_TYPE_ARGUMENTS.code,
                NO_UNNECESSARY_TYPE_ARGUMENTS.category,
                severity,
                "unnecessary trailing type arguments",
                ctx.module.file_id,
                span,
            )
            .with_label("these trailing type arguments repeat declared defaults");

            if let Some(fix) = redundant_type_arguments_fix(
                ctx,
                expression_id,
                static_arguments,
                first_redundant_index,
            ) {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Resolve a target symbol for one expression when available.
fn target_symbol_for_expression(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    expression: &dir::Expression,
) -> Option<dir::GlobalSymbolId> {
    // direct expression target first
    if let Some(symbol_id) = expression.target_symbol() {
        return Some(symbol_id);
    }

    // then resolution candidates
    let global_expression_id = expression_id.into_global_any(ctx.module_id());
    let resolution_id = ctx.types.get_resolution_for_node(global_expression_id)?;
    let resolution = ctx.types.get_resolution(resolution_id);

    match resolution {
        dir::Resolution::Static { candidate, .. } => Some(candidate.target_symbol),
        _ => None,
    }
}

/// One static parameter default source.
#[derive(Clone, Copy)]
struct StaticParameterDefault {
    /// The module that owns this default expression.
    module_id: ModuleId,
    /// The default expression when present.
    default_expression: Option<dir::LocalNodeId<dir::Expression>>,
}

/// Resolve static parameter defaults from one declaration or member symbol.
fn static_parameter_defaults_for_symbol(
    ctx: &LintModuleDirContext<'_>,
    symbol_id: dir::GlobalSymbolId,
) -> Option<Vec<StaticParameterDefault>> {
    let declaration_id = symbol_primary_declaration_for(
        &ctx.program,
        ctx.profile_id,
        ctx.module_id(),
        ctx.symbols,
        symbol_id,
    )?;

    if declaration_id.module_id == ctx.module_id() {
        let static_parameters =
            static_parameters_for_declaration(ctx.tree, declaration_id.local_id)?;
        return Some(
            static_parameters
                .into_iter()
                .map(|parameter_id| StaticParameterDefault {
                    module_id: ctx.module_id(),
                    default_expression: parameter_default_expression(ctx.tree, parameter_id),
                })
                .collect(),
        );
    }

    let module_ref = ctx.program.modules.get(declaration_id.module_id);
    let module = module_ref.read();
    let module_dir = module.dir_maybe(ctx.profile_id)?;
    let tree = module_dir.tree.read();
    let static_parameters = static_parameters_for_declaration(&tree, declaration_id.local_id)?;
    Some(
        static_parameters
            .into_iter()
            .map(|parameter_id| StaticParameterDefault {
                module_id: declaration_id.module_id,
                default_expression: parameter_default_expression(&tree, parameter_id),
            })
            .collect(),
    )
}

/// Resolve static parameter ids from one declaration or member node.
fn static_parameters_for_declaration(
    tree: &dir::NodeTree,
    declaration_id: dir::LocalNodeIdAny,
) -> Option<Vec<dir::LocalNodeId<dir::Parameter>>> {
    if declaration_id.ty == dir::NodeType::Declaration {
        let declaration = tree.get(declaration_id.into_typed::<dir::Declaration>());
        return declaration.static_parameters().cloned();
    }

    if declaration_id.ty == dir::NodeType::Member {
        let member = tree.get(declaration_id.into_typed::<dir::Member>());
        return match member {
            dir::Member::Method { signature, .. } => signature
                .generics
                .as_ref()
                .and_then(|generics| generics.static_parameters.clone()),
            dir::Member::Type {
                static_parameters, ..
            } => static_parameters.clone(),
            _ => None,
        };
    }

    None
}

/// Return the first trailing explicit argument index that is redundant.
fn first_redundant_trailing_argument_index(
    ctx: &LintModuleDirContext<'_>,
    static_arguments: &[dir::LocalNodeId<dir::Argument>],
    static_parameter_defaults: &[StaticParameterDefault],
) -> Option<usize> {
    let mut index = static_arguments.len();

    // walk trailing arguments backwards while they match parameter defaults
    while index > 0 {
        let argument_id = static_arguments[index - 1];
        let Some(argument_expression) = argument_value_expression(ctx.tree, argument_id) else {
            break;
        };
        let parameter_default = static_parameter_defaults[index - 1];
        let Some(default_expression) = parameter_default.default_expression else {
            break;
        };

        if !argument_matches_default(
            ctx,
            argument_expression,
            parameter_default.module_id,
            default_expression,
        ) {
            break;
        }

        index -= 1;
    }

    (index < static_arguments.len()).then_some(index)
}

/// Return true when one explicit type argument matches one declared default.
fn argument_matches_default(
    ctx: &LintModuleDirContext<'_>,
    argument_expression: dir::LocalNodeId<dir::Expression>,
    default_module_id: ModuleId,
    default_expression: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // prefer semantic type equality when both sides are in the current module
    if default_module_id == ctx.module_id()
        && let Some(argument_type_id) = local_expression_type_id(ctx, argument_expression)
        && let Some(default_type_id) = local_expression_type_id(ctx, default_expression)
        && dir::are_types_equal(argument_type_id, default_type_id, ctx.types)
    {
        return true;
    }

    // fall back to structural ast comparison across modules
    expression_ast_signature_eq_cross_module(
        ctx,
        ctx.module_id(),
        argument_expression,
        default_module_id,
        default_expression,
    )
}

/// Resolve one expression type id from the current module.
fn local_expression_type_id(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalTypeId> {
    expression_declared_or_inferred_type_id(ctx.module_id(), ctx.tree, ctx.types, expression_id)
        .or_else(|| ctx.expression_type_id(expression_id))
}

/// Compare expressions structurally across modules using AST signatures.
fn expression_ast_signature_eq_cross_module(
    ctx: &LintModuleDirContext<'_>,
    left_module_id: ModuleId,
    left_expression: dir::LocalNodeId<dir::Expression>,
    right_module_id: ModuleId,
    right_expression: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let Some(left_signature) =
        expression_ast_signature_for_module(ctx, left_module_id, left_expression)
    else {
        return false;
    };
    let Some(right_signature) =
        expression_ast_signature_for_module(ctx, right_module_id, right_expression)
    else {
        return false;
    };

    left_signature == right_signature
}

/// Resolve a structural AST signature for one DIR expression.
fn expression_ast_signature_for_module(
    ctx: &LintModuleDirContext<'_>,
    module_id: ModuleId,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<Vec<u64>> {
    if module_id == ctx.module_id() {
        let source_id = ctx.tree.get_source(expression_id.id);
        let ast = ctx.module.ast_maybe()?;
        if ast.tree.get_node_type(source_id) != ast::NodeType::Expression {
            return None;
        }

        let ast_expression_id = ast::LocalNodeId::<ast::Expression>::new(source_id);
        return Some(expression_structural_signature(
            &ast.tree,
            &ast.strings,
            ast_expression_id,
        ));
    }

    let module_ref = ctx.program.modules.get(module_id);
    let module = module_ref.read();
    let module_dir = module.dir_maybe(ctx.profile_id)?;
    let source_id = {
        let tree = module_dir.tree.read();
        tree.get_source(expression_id.id)
    };
    let ast = module.ast_maybe()?;
    if ast.tree.get_node_type(source_id) != ast::NodeType::Expression {
        return None;
    }

    let ast_expression_id = ast::LocalNodeId::<ast::Expression>::new(source_id);
    Some(expression_structural_signature(
        &ast.tree,
        &ast.strings,
        ast_expression_id,
    ))
}

/// Resolve the expression value of one static argument.
fn argument_value_expression(
    tree: &dir::NodeTree,
    argument_id: dir::LocalNodeId<dir::Argument>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let argument = tree.get(argument_id);
    match argument {
        dir::Argument::Named { value, .. }
        | dir::Argument::Labeled { value, .. }
        | dir::Argument::Positional { value, .. }
        | dir::Argument::Spread { value, .. } => Some(*value),
    }
}

/// Resolve the default expression for one static parameter.
fn parameter_default_expression(
    tree: &dir::NodeTree,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let parameter = tree.get(parameter_id);
    match parameter {
        dir::Parameter::Named { default, .. } | dir::Parameter::Pattern { default, .. } => *default,
        dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. } => None,
    }
}

/// Build a safe fix that removes redundant trailing type arguments.
fn redundant_type_arguments_fix(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    static_arguments: &[dir::LocalNodeId<dir::Argument>],
    first_redundant_index: usize,
) -> Option<LintFix> {
    let remove_span = trailing_type_argument_removal_span(
        ctx,
        expression_id,
        static_arguments,
        first_redundant_index,
    )?;
    if remove_span.is_empty() {
        return None;
    }

    let edits = ctx.edit_builder().delete(remove_span).into_edits();
    Some(LintFix::safe("Remove redundant trailing type arguments").with_edits(edits))
}

/// Resolve the source span to delete for redundant trailing type arguments.
fn trailing_type_argument_removal_span(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    static_arguments: &[dir::LocalNodeId<dir::Argument>],
    first_redundant_index: usize,
) -> Option<Span> {
    let last_argument_id = *static_arguments.last()?;
    let last_argument_span = ctx.get_span(last_argument_id);
    let expression_span = ctx.get_span(expression_id);
    let source = ctx.source_text().as_bytes();

    // remove from the comma before first redundant argument to the end of the last argument
    if first_redundant_index > 0 {
        let previous_argument_id = static_arguments[first_redundant_index - 1];
        let previous_argument_span = ctx.get_span(previous_argument_id);
        let first_redundant_argument_span = ctx.get_span(static_arguments[first_redundant_index]);
        if previous_argument_span.end >= last_argument_span.end {
            return None;
        }

        let comma_start = find_comma_between(
            source,
            previous_argument_span.end as usize,
            first_redundant_argument_span.start as usize,
        )?;

        return Some(Span::new(
            previous_argument_span.file,
            comma_start as u32,
            last_argument_span.end,
        ));
    }

    // remove the whole static argument list: `<...>`
    let first_argument_span = ctx.get_span(static_arguments[0]);

    let mut left = first_argument_span.start as usize;
    while left > expression_span.start as usize {
        left -= 1;
        if source[left] == b'<' {
            break;
        }
    }
    if source[left] != b'<' {
        return None;
    }

    let mut right = last_argument_span.end as usize;
    while right < expression_span.end as usize && source[right] != b'>' {
        right += 1;
    }
    if right >= expression_span.end as usize || source[right] != b'>' {
        return None;
    }

    Some(Span::new(
        expression_span.file,
        left as u32,
        right.saturating_add(1) as u32,
    ))
}

/// Find the first comma between two byte offsets.
fn find_comma_between(source: &[u8], start: usize, end: usize) -> Option<usize> {
    if start >= end || end > source.len() {
        return None;
    }

    for (offset, byte) in source[start..end].iter().enumerate() {
        if *byte == b',' {
            return Some(start + offset);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::{TestProgram, test_modules};

    /// Flag explicit type arguments equal to defaults.
    #[test]
    fn test_flags_type_argument_equal_to_default() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeArguments);
        let result = test.lint_dir(
            "no_unnecessary_type_arguments/test_flags_type_argument_equal_to_default.ds",
            r#"
function id<T = string>(value: T): T {
    return value;
}

const value = id<string>("ok");
"#,
        );
        test.result(result)
            .assert_lint("no-unnecessary-type-arguments");
    }

    /// Allow explicit type arguments that differ from defaults.
    #[test]
    fn test_allows_type_argument_different_from_default() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeArguments);
        let result = test.lint_dir(
            "no_unnecessary_type_arguments/test_allows_type_argument_different_from_default.ds",
            r#"
function id<T = string>(value: T): T {
    return value;
}

const value = id<int32>(1);
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-type-arguments");
    }

    /// Allow calls with inferred type arguments.
    #[test]
    fn test_allows_inferred_type_arguments() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeArguments);
        let result = test.lint_dir(
            "no_unnecessary_type_arguments/test_allows_inferred_type_arguments.ds",
            r#"
function id<T = string>(value: T): T {
    return value;
}

const value = id("ok");
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-type-arguments");
    }

    /// Flag only trailing defaults when earlier arguments are required.
    #[test]
    fn test_flags_only_trailing_default_arguments() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeArguments);
        let result = test.lint_dir(
            "no_unnecessary_type_arguments/test_flags_only_trailing_default_arguments.ds",
            r#"
function makePair<T, U = string>(left: T, right: U): (T, U) {
    return (left, right);
}

const pair = makePair<int32, string>(1, "ok");
"#,
        );
        test.result(result)
            .assert_lint("no-unnecessary-type-arguments");
    }

    /// Do not report when only a prefix argument matches defaults.
    #[test]
    fn test_ignores_non_trailing_default_match() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeArguments);
        let result = test.lint_dir(
            "no_unnecessary_type_arguments/test_ignores_non_trailing_default_match.ds",
            r#"
function pair<T = string, U = int32>(left: T, right: U): (T, U) {
    return (left, right);
}

const value = pair<string, float64>("ok", 1.0);
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-type-arguments");
    }

    /// Report redundant defaults for imported generics.
    #[test]
    fn test_flags_cross_module_default_type_argument() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeArguments);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unnecessary_type_arguments/source.ds" => r#"
export function id<T = string>(value: T): T {
    return value;
}
"#,
                "no_unnecessary_type_arguments/consumer.ds" => r#"
import { id } from "./source.ds";

const value = id<string>("ok");
"#,
            },
            "no_unnecessary_type_arguments/consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-unnecessary-type-arguments");
    }

    /// Allow imported generic calls with non default type arguments.
    #[test]
    fn test_allows_cross_module_non_default_type_argument() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeArguments);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unnecessary_type_arguments/source_non_default.ds" => r#"
export function id<T = string>(value: T): T {
    return value;
}
"#,
                "no_unnecessary_type_arguments/consumer_non_default.ds" => r#"
import { id } from "./source_non_default.ds";

const value = id<int32>(1);
"#,
            },
            "no_unnecessary_type_arguments/consumer_non_default.ds",
        );

        test.result(diagnostics)
            .assert_no_lint("no-unnecessary-type-arguments");
    }

    /// Safely remove an unnecessary single explicit type argument.
    #[test]
    fn test_fix_single_redundant_type_argument() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeArguments);
        let result = test.lint_dir(
            "no_unnecessary_type_arguments/test_fix_single_redundant_type_argument.ds",
            r#"
function id<T = string>(value: T): T {
    return value;
}

const value = id<string>("ok");
"#,
        );
        test.result(result)
            .assert_lint("no-unnecessary-type-arguments")
            .assert_has_fix("no-unnecessary-type-arguments")
            .assert_safe_fixed(
                r#"
function id<T = string>(value: T): T {
    return value;
}

const value = id("ok");
"#,
            );
    }

    /// Safely remove only trailing redundant type arguments.
    #[test]
    fn test_fix_trailing_redundant_type_argument() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeArguments);
        let result = test.lint_dir(
            "no_unnecessary_type_arguments/test_fix_trailing_redundant_type_argument.ds",
            r#"
function choose<T, U = string>(left: T, right: U): U {
    return right;
}

const value = choose<int32, string>(1, "ok");
"#,
        );
        test.result(result)
            .assert_lint("no-unnecessary-type-arguments")
            .assert_has_fix("no-unnecessary-type-arguments")
            .assert_safe_fixed(
                r#"
function choose<T, U = string>(left: T, right: U): U {
    return right;
}

const value = choose<int32>(1, "ok");
"#,
            );
    }

    /// Safely remove all explicit type arguments when all are redundant.
    #[test]
    fn test_fix_removes_entire_type_argument_list() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeArguments);
        let result = test.lint_dir(
            "no_unnecessary_type_arguments/test_fix_removes_entire_type_argument_list.ds",
            r#"
function wrap<T = string>(value: T): T {
    return value;
}

const value = wrap< string >("ok");
"#,
        );
        test.result(result)
            .assert_lint("no-unnecessary-type-arguments")
            .assert_has_fix("no-unnecessary-type-arguments")
            .assert_safe_fixed(
                r#"
function wrap<T = string>(value: T): T {
    return value;
}

const value = wrap("ok");
"#,
            );
    }

    /// Safely remove trailing redundant type arguments across lines.
    #[test]
    fn test_fix_multiline_trailing_redundant_type_argument() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeArguments);
        let result = test.lint_dir(
            "no_unnecessary_type_arguments/test_fix_multiline_trailing_redundant_type_argument.ds",
            r#"
function choose<T, U = string>(left: T, right: U): U {
    return right;
}

const value = choose<
    int32,
    string
>(1, "ok");
"#,
        );
        test.result(result)
            .assert_lint("no-unnecessary-type-arguments")
            .assert_has_fix("no-unnecessary-type-arguments")
            .assert_safe_fixed(
                r#"
function choose<T, U = string>(left: T, right: U): U {
    return right;
}

const value = choose<int32>(1, "ok");
"#,
            );
    }

    /// Safely remove redundant defaults for imported generics.
    #[test]
    fn test_fix_cross_module_default_type_argument() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeArguments);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unnecessary_type_arguments/fix_source.ds" => r#"
export function id<T = string>(value: T): T {
    return value;
}
"#,
                "no_unnecessary_type_arguments/fix_consumer.ds" => r#"
import { id } from "./fix_source.ds";

const value = id<string>("ok");
"#,
            },
            "no_unnecessary_type_arguments/fix_consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-unnecessary-type-arguments")
            .assert_has_fix("no-unnecessary-type-arguments")
            .assert_safe_fixed(
                r#"
import { id } from "./fix_source.ds";

const value = id("ok");
"#,
            );
    }
}

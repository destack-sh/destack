use destack_source::ModuleId;
use destack_workspace::LintSeverity;
use {destack_ast as ast, destack_dir as dir};

use crate::rules::common::{
    expression_signature_for_tree, symbol_primary_declaration_for, trailing_argument_removal_span,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow explicit trailing type arguments equal to declared defaults.
    ///
    /// Repeating default generic arguments adds noise without changing behavior.
    #[lint(
        id = "no-unnecessary-type-arguments",
        code = "LC031",
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
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoUnnecessaryTypeArguments::meta()
    }

    /// Check module DIR nodes for redundant trailing type arguments.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // inspect expressions with explicit generic arguments
        for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
            let expression = ctx.tree.get(expression_id);
            let Some(generic_arguments) = expression.generic_arguments() else {
                continue;
            };
            if generic_arguments.is_empty() {
                continue;
            }
            if !generic_arguments_are_plain_positional(ctx.tree, generic_arguments) {
                continue;
            }

            // require optional structure
            let Some(target_symbol) = target_symbol_for_expression(ctx, expression_id, expression)
            else {
                continue;
            };
            let Some(generic_parameter_defaults) =
                generic_parameter_defaults_for_symbol(ctx, target_symbol)
            else {
                continue;
            };
            if generic_parameter_defaults.len() < generic_arguments.len() {
                continue;
            }

            // require optional structure
            let Some(first_redundant_index) = first_redundant_trailing_argument_index(
                ctx,
                generic_arguments,
                &generic_parameter_defaults,
            ) else {
                continue;
            };

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            // resolve redundant argument id
            let redundant_argument_id = generic_arguments[first_redundant_index];
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

            // attach fix when enabled
            if ctx.include_fixes
                && let Some(fix) = redundant_type_arguments_fix(
                    ctx,
                    expression_id,
                    generic_arguments,
                    first_redundant_index,
                )
            {
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

    // extract one target symbol from static resolution results
    match resolution {
        dir::Resolution::Static { candidate, .. } => Some(candidate.target_symbol),
        _ => None,
    }
}

/// One generic parameter default source.
#[derive(Clone, Copy)]
struct GenericParameterDefault {
    /// The module that owns this default expression.
    module_id: ModuleId,
    /// The default value when present.
    default_value: Option<GenericParameterDefaultValue>,
}

/// One generic parameter default value.
#[derive(Clone, Copy)]
enum GenericParameterDefaultValue {
    /// One type-space default.
    Type(dir::LocalNodeId<dir::TypeExpression>),
    /// One value-space default.
    Value(dir::LocalNodeId<dir::Expression>),
}

/// Resolve generic parameter defaults from one declaration or member symbol.
fn generic_parameter_defaults_for_symbol(
    ctx: &LintModuleDirContext<'_>,
    symbol_id: dir::GlobalSymbolId,
) -> Option<Vec<GenericParameterDefault>> {
    let declaration_id = symbol_primary_declaration_for(
        &ctx.repository,
        ctx.revision,
        ctx.profile_id,
        ctx.module_id(),
        ctx.symbols,
        symbol_id,
    )?;

    // resolve defaults from the current module when possible
    if declaration_id.module_id == ctx.module_id() {
        let generic_parameters =
            generic_parameters_for_declaration(ctx.tree, declaration_id.local_id)?;
        return Some(
            generic_parameters
                .into_iter()
                .map(|parameter_id| GenericParameterDefault {
                    module_id: ctx.module_id(),
                    default_value: parameter_default_expression(ctx.tree, parameter_id),
                })
                .collect(),
        );
    }

    // fall back to loading declaration defaults from the owning module
    let module_dir = ctx.analyzed_dir(declaration_id.module_id)?;
    let generic_parameters =
        generic_parameters_for_declaration(&module_dir.tree, declaration_id.local_id)?;
    Some(
        generic_parameters
            .into_iter()
            .map(|parameter_id| GenericParameterDefault {
                module_id: declaration_id.module_id,
                default_value: parameter_default_expression(&module_dir.tree, parameter_id),
            })
            .collect(),
    )
}

/// Resolve generic parameter ids from one declaration or member node.
fn generic_parameters_for_declaration(
    tree: &dir::NodeTree,
    declaration_id: dir::LocalNodeIdAny,
) -> Option<Vec<dir::LocalNodeId<dir::GenericParameter>>> {
    // handle declaration symbols directly
    if declaration_id.ty == dir::NodeType::Declaration {
        let declaration = tree.get(declaration_id.into_typed::<dir::Declaration>());
        return match declaration {
            dir::Declaration::Namespace(declaration) => {
                Some(declaration.generic_parameters.clone())
            }
            dir::Declaration::Type(declaration) => Some(declaration.generic_parameters.clone()),
            dir::Declaration::Struct(declaration) => Some(declaration.generic_parameters.clone()),
            dir::Declaration::Class(declaration) => Some(declaration.generic_parameters.clone()),
            dir::Declaration::Enum(declaration) => Some(declaration.generic_parameters.clone()),
            dir::Declaration::Interface(declaration) => {
                Some(declaration.generic_parameters.clone())
            }
            dir::Declaration::Extension(declaration) => {
                Some(declaration.generic_parameters.clone())
            }
            dir::Declaration::Function(declaration) => {
                Some(declaration.signature.generic_parameters.clone())
            }
            dir::Declaration::Global(_) | dir::Declaration::ImportAlias(_) => None,
        };
    }

    // handle member symbols that can declare static parameters
    if declaration_id.ty == dir::NodeType::Member {
        let member = tree.get(declaration_id.into_typed::<dir::Member>());
        return match member {
            dir::Member::Method { signature, .. } => Some(signature.generic_parameters.clone()),
            dir::Member::AssociatedType {
                generic_parameters, ..
            } => Some(generic_parameters.clone()),
            _ => None,
        };
    }

    None
}

/// Return the first trailing explicit argument index that is redundant.
fn first_redundant_trailing_argument_index(
    ctx: &LintModuleDirContext<'_>,
    generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    generic_parameter_defaults: &[GenericParameterDefault],
) -> Option<usize> {
    let mut index = generic_arguments.len();

    // walk trailing arguments backwards while they match parameter defaults
    while index > 0 {
        let argument_id = generic_arguments[index - 1];
        let Some(argument_expression) = generic_argument_expression_id(ctx.tree, argument_id)
        else {
            break;
        };
        let parameter_default = generic_parameter_defaults[index - 1];
        let Some(default_value) = parameter_default.default_value else {
            break;
        };

        // stop once one trailing argument no longer matches its default
        if !argument_matches_default(
            ctx,
            argument_expression,
            parameter_default.module_id,
            default_value,
        ) {
            break;
        }

        index -= 1;
    }

    (index < generic_arguments.len()).then_some(index)
}

/// Return true when all generic arguments are plain positional arguments.
fn generic_arguments_are_plain_positional(
    tree: &dir::NodeTree,
    generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
) -> bool {
    generic_arguments.iter().all(|argument_id| {
        let argument = tree.get(*argument_id);
        matches!(argument, dir::GenericArgument::Positional { .. })
    })
}

/// Return one value expression id from one generic argument.
fn generic_argument_expression_id(
    tree: &dir::NodeTree,
    argument_id: dir::LocalNodeId<dir::GenericArgument>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let argument = tree.get(argument_id);
    match argument {
        dir::GenericArgument::Positional { value } | dir::GenericArgument::Spread { value } => {
            Some(*value)
        }
        dir::GenericArgument::Error => None,
    }
}

/// Return true when one explicit type argument matches one declared default.
fn argument_matches_default(
    ctx: &LintModuleDirContext<'_>,
    argument_expression: dir::LocalNodeId<dir::Expression>,
    default_module_id: ModuleId,
    default_value: GenericParameterDefaultValue,
) -> bool {
    match default_value {
        GenericParameterDefaultValue::Type(default_type_expression) => {
            type_argument_matches_default(
                ctx,
                argument_expression,
                default_module_id,
                default_type_expression,
            )
        }
        GenericParameterDefaultValue::Value(default_expression) => {
            expression_ast_signature_eq_cross_module(
                ctx,
                ctx.module_id(),
                argument_expression,
                default_module_id,
                default_expression,
            )
        }
    }
}

/// Return true when one explicit type argument matches one declared type default.
fn type_argument_matches_default(
    ctx: &LintModuleDirContext<'_>,
    argument_expression: dir::LocalNodeId<dir::Expression>,
    default_module_id: ModuleId,
    default_type_expression: dir::LocalNodeId<dir::TypeExpression>,
) -> bool {
    let Some(argument_text) =
        type_argument_source_text_for_module(ctx, ctx.module_id(), argument_expression)
    else {
        return false;
    };
    let Some(default_text) =
        type_expression_source_text_for_module(ctx, default_module_id, default_type_expression)
    else {
        return false;
    };

    argument_text == default_text
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
        let ast = ctx.module_ast(module_id)?;
        if ast.tree.get_node_type(source_id) != ast::NodeType::Expression {
            return None;
        }

        // map local source node to an ast expression id
        let ast_expression_id = ast::LocalNodeId::<ast::Expression>::new(source_id);
        return Some(expression_signature_for_tree(
            &ast.tree,
            &ast.strings,
            ast_expression_id,
        ));
    }

    // load the referenced module when the expression comes from another module
    let module_dir = ctx.analyzed_dir(module_id)?;
    let source_id = module_dir.tree.get_source(expression_id.id);
    let ast = ctx.module_ast(module_id)?;
    if ast.tree.get_node_type(source_id) != ast::NodeType::Expression {
        return None;
    }

    // map cross module source node to an ast expression id
    let ast_expression_id = ast::LocalNodeId::<ast::Expression>::new(source_id);
    Some(expression_signature_for_tree(
        &ast.tree,
        &ast.strings,
        ast_expression_id,
    ))
}

/// Resolve the default expression for one generic parameter.
fn parameter_default_expression(
    tree: &dir::NodeTree,
    parameter_id: dir::LocalNodeId<dir::GenericParameter>,
) -> Option<GenericParameterDefaultValue> {
    let parameter = tree.get(parameter_id);
    match parameter {
        dir::GenericParameter::Type { default, .. } => {
            default.map(GenericParameterDefaultValue::Type)
        }
        dir::GenericParameter::Value { default, .. } => {
            default.map(GenericParameterDefaultValue::Value)
        }
        dir::GenericParameter::Error { .. } => None,
    }
}

/// Build a safe fix that removes redundant trailing type arguments.
fn redundant_type_arguments_fix(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    first_redundant_index: usize,
) -> Option<LintFix> {
    // resolve spans for the expression and generic arguments
    let expression_span = ctx.get_span(expression_id);
    let argument_spans: Vec<_> = generic_arguments
        .iter()
        .map(|argument_id| ctx.get_span(*argument_id))
        .collect();

    // resolve the trailing argument removal span
    let remove_span = trailing_argument_removal_span(
        ctx.source_text(),
        expression_span,
        &argument_spans,
        first_redundant_index,
    )?;
    if remove_span.is_empty() {
        return None;
    }

    // delete the redundant trailing type argument segment
    let edits = ctx.edit_builder().delete(remove_span).into_edits();
    Some(LintFix::safe("Remove redundant trailing type arguments").with_edits(edits))
}

/// Resolve the source text for one type argument expression.
fn type_argument_source_text_for_module(
    ctx: &LintModuleDirContext<'_>,
    module_id: ModuleId,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<String> {
    let ast = ctx.module_ast(module_id)?;
    let module = ctx.repository_module(module_id)?;
    let file = ctx.repository_file(module.file_id)?;

    let source_id = if module_id == ctx.module_id() {
        ctx.tree.get_source(expression_id.id)
    } else {
        let module_dir = ctx.analyzed_dir(module_id)?;
        module_dir.tree.get_source(expression_id.id)
    };
    if ast.tree.get_node_type(source_id) != ast::NodeType::Expression {
        return None;
    }

    let ast_expression_id = ast::LocalNodeId::<ast::Expression>::new(source_id);
    let ast_expression = ast.tree.get(ast_expression_id);
    let ast::Expression::Type {
        value: type_expression_id,
    } = ast_expression
    else {
        return None;
    };

    let span = ast.tree.get_span(*type_expression_id);
    Some(file.text()[span.start as usize..span.end as usize].to_string())
}

/// Resolve the source text for one DIR type expression.
fn type_expression_source_text_for_module(
    ctx: &LintModuleDirContext<'_>,
    module_id: ModuleId,
    type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
) -> Option<String> {
    let ast = ctx.module_ast(module_id)?;
    let module = ctx.repository_module(module_id)?;
    let file = ctx.repository_file(module.file_id)?;

    let source_id = if module_id == ctx.module_id() {
        ctx.tree.get_source(type_expression_id.id)
    } else {
        let module_dir = ctx.analyzed_dir(module_id)?;
        module_dir.tree.get_source(type_expression_id.id)
    };
    if ast.tree.get_node_type(source_id) != ast::NodeType::TypeExpression {
        return None;
    }

    let ast_type_expression_id = ast::LocalNodeId::<ast::TypeExpression>::new(source_id);
    let span = ast.tree.get_span(ast_type_expression_id);
    Some(file.text()[span.start as usize..span.end as usize].to_string())
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

    /// Flag redundant explicit generic value arguments equal to defaults.
    #[test]
    fn test_flags_value_argument_equal_to_default() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeArguments);
        let result = test.lint_dir(
            "no_unnecessary_type_arguments/test_flags_value_argument_equal_to_default.ds",
            r#"
function repeat<const Count = 1>(value: int32): int32 {
    return value;
}

const value = repeat<1>(1);
"#,
        );
        test.result(result)
            .assert_lint("no-unnecessary-type-arguments");
    }

    /// Safely remove redundant explicit generic value arguments.
    #[test]
    fn test_fix_value_argument_equal_to_default() {
        let test = TestProgram::for_rule_without_prelude(NoUnnecessaryTypeArguments);
        let result = test.lint_dir(
            "no_unnecessary_type_arguments/test_fix_value_argument_equal_to_default.ds",
            r#"
function repeat<const Count = 1>(value: int32): int32 {
    return value;
}

const value = repeat<1>(1);
"#,
        );
        test.result(result)
            .assert_lint("no-unnecessary-type-arguments")
            .assert_has_fix("no-unnecessary-type-arguments")
            .assert_safe_fixed(
                r#"
function repeat<const Count = 1>(value: int32): int32 {
    return value;
}

const value = repeat(1);
"#,
            );
    }
}

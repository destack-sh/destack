use destack_core::StringId;
use destack_dir as dir;
use destack_source::LabeledSpan;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    argument_expression_id, expression_candidate_symbols, expression_unwrap_parenthesized,
    is_string_type, symbol_declaration_for,
};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow positional arguments that appear swapped by parameter name.
    ///
    /// When two argument names match the opposite parameter names, the call is
    /// often a bug and hard to notice in review.
    #[lint(
        id = "no-arguments-order-mismatch",
        code = "LC046",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoArgumentsOrderMismatch,
    "Disallow swapped positional arguments by parameter name"
}

impl LintRule for NoArgumentsOrderMismatch {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoArgumentsOrderMismatch::meta()
    }

    /// Check module DIR nodes for likely swapped positional arguments.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for (expression_id, expression) in ctx.dir.iter_nodes_of_type::<dir::Expression>() {
            let (callee_expression_id, argument_ids) = match expression {
                dir::Expression::Call {
                    left, arguments, ..
                } => (*left, arguments.as_slice()),
                _ => continue,
            };

            let argument_name_hints = argument_name_hints(ctx.dir.tree(), argument_ids);
            if argument_name_hints.len() < 2 {
                continue;
            }
            if argument_name_hints.iter().all(Option::is_none) {
                continue;
            }

            let Some(parameter_names) =
                stable_parameter_names_for_call_target(ctx, callee_expression_id)
            else {
                continue;
            };
            if parameter_names.len() < 2 {
                continue;
            }

            let Some((first_index, second_index)) =
                swapped_argument_pair(argument_name_hints.as_slice(), parameter_names.as_slice())
            else {
                continue;
            };
            if !swapped_pair_type_compatible(
                ctx,
                callee_expression_id,
                argument_ids,
                first_index,
                second_index,
            ) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            let Some(Some(first_parameter_name)) = parameter_names.get(first_index) else {
                continue;
            };
            let Some(Some(second_parameter_name)) = parameter_names.get(second_index) else {
                continue;
            };
            let first_parameter_name_text = ctx.strings.get(*first_parameter_name).to_string();
            let second_parameter_name_text = ctx.strings.get(*second_parameter_name).to_string();

            let first_argument_span = ctx.get_span(argument_ids[first_index]);
            let second_argument_span = ctx.get_span(argument_ids[second_index]);
            let span = ctx.get_span(expression_id);
            ctx.report(
                LintReport::new(
                    NO_ARGUMENTS_ORDER_MISMATCH.id,
                    NO_ARGUMENTS_ORDER_MISMATCH.code,
                    NO_ARGUMENTS_ORDER_MISMATCH.category,
                    severity,
                    "positional arguments appear to be in the wrong order",
                    span,
                )
                .label("these arguments look swapped for this call")
                .secondary(LabeledSpan::new(
                    first_argument_span,
                    format!("this argument matches parameter `{second_parameter_name_text}`"),
                ))
                .secondary(LabeledSpan::new(
                    second_argument_span,
                    format!("this argument matches parameter `{first_parameter_name_text}`"),
                )),
            );
        }
    }
}

/// Return true when one swapped argument pair is type compatible with swapped parameters.
///
/// When local signature and type information is available, each argument should
/// fit the opposite parameter.
fn swapped_pair_type_compatible(
    ctx: &LintModuleContext<'_>,
    callee_expression_id: dir::LocalNodeId<dir::Expression>,
    argument_ids: &[dir::LocalNodeId<dir::Argument>],
    first_index: usize,
    second_index: usize,
) -> bool {
    let candidate_symbols =
        expression_candidate_symbols(ctx.module_id(), ctx.resolutions, callee_expression_id);
    if candidate_symbols.is_empty() {
        return true;
    }

    // evaluate every local declaration candidate conservatively
    for symbol_id in candidate_symbols {
        let Some(declaration_id) = symbol_declaration_for(ctx, symbol_id) else {
            continue;
        };
        if declaration_id.module_id != ctx.module_id() {
            continue;
        }

        let Some(parameters) = declaration_parameters(ctx.dir.tree(), declaration_id.local_id)
        else {
            continue;
        };
        let Some(first_parameter_id) = parameters.get(first_index).copied() else {
            continue;
        };
        let Some(second_parameter_id) = parameters.get(second_index).copied() else {
            continue;
        };
        let Some(first_argument_expression_id) =
            argument_expression_id(ctx.dir.tree(), argument_ids[first_index])
        else {
            continue;
        };
        let Some(second_argument_expression_id) =
            argument_expression_id(ctx.dir.tree(), argument_ids[second_index])
        else {
            continue;
        };

        // require stable argument and parameter types before rejecting
        let Some(first_argument_type_id) = ctx.expression_type_id(first_argument_expression_id)
        else {
            continue;
        };
        let Some(second_argument_type_id) = ctx.expression_type_id(second_argument_expression_id)
        else {
            continue;
        };
        let Some(first_parameter_type_id) = parameter_type_id(ctx, first_parameter_id) else {
            continue;
        };
        let Some(second_parameter_type_id) = parameter_type_id(ctx, second_parameter_id) else {
            continue;
        };

        let first_mismatch =
            type_pair_has_string_mismatch(ctx, first_argument_type_id, second_parameter_type_id);
        let second_mismatch =
            type_pair_has_string_mismatch(ctx, second_argument_type_id, first_parameter_type_id);
        if first_mismatch || second_mismatch {
            return false;
        }
    }

    true
}

/// Return true when one argument and parameter pair is obviously string incompatible.
fn type_pair_has_string_mismatch(
    ctx: &LintModuleContext<'_>,
    left_type_id: dir::GlobalTypeId,
    right_type_id: dir::GlobalTypeId,
) -> bool {
    let string_symbol = ctx.get_language_item(dir::LanguageItem::String);
    let left_is_string = is_string_type(ctx, left_type_id, string_symbol);
    let right_is_string = is_string_type(ctx, right_type_id, string_symbol);
    left_is_string != right_is_string
}

/// Return argument name hints for one positional argument list.
fn argument_name_hints(
    tree: &dir::Tree,
    argument_ids: &[dir::LocalNodeId<dir::Argument>],
) -> Vec<Option<StringId>> {
    let mut hints = Vec::new();

    for argument_id in argument_ids {
        let argument = tree.get(*argument_id);
        let dir::Argument::Positional { value, .. } = argument else {
            return Vec::new();
        };

        hints.push(expression_name_hint(tree, *value));
    }

    hints
}

/// Return one argument name hint for an expression when available.
fn expression_name_hint(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<StringId> {
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);
    match expression {
        dir::Expression::QualifiedReference { path, .. } => path.last_segment(),
        dir::Expression::Member { name, .. } | dir::Expression::PrivateMember { name, .. } => *name,
        dir::Expression::As {
            expression: value,
            target_type: _,
        }
        | dir::Expression::Satisfies {
            expression: value,
            target_type: _,
        } => expression_name_hint(tree, *value),
        dir::Expression::MoveOf { right, .. } | dir::Expression::BorrowOf { right, .. } => {
            expression_name_hint(tree, *right)
        }
        dir::Expression::Maybe { left, .. } | dir::Expression::Must { left, .. } => {
            expression_name_hint(tree, *left)
        }
        _ => None,
    }
}

/// Return stable parameter names for one call target.
///
/// When multiple resolution candidates disagree about parameter names, this
/// returns none to avoid noisy false positives.
fn stable_parameter_names_for_call_target(
    ctx: &LintModuleContext<'_>,
    callee_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<Vec<Option<StringId>>> {
    let candidate_symbols =
        expression_candidate_symbols(ctx.module_id(), ctx.resolutions, callee_expression_id);
    if candidate_symbols.is_empty() {
        return None;
    }

    let mut candidate_parameter_names = Vec::new();
    for symbol_id in candidate_symbols {
        let Some(parameter_names) = parameter_names_for_symbol(ctx, symbol_id) else {
            continue;
        };

        if !candidate_parameter_names.contains(&parameter_names) {
            candidate_parameter_names.push(parameter_names);
        }
    }

    if candidate_parameter_names.len() != 1 {
        return None;
    }

    candidate_parameter_names.into_iter().next()
}

/// Return parameter names for one symbol declaration.
fn parameter_names_for_symbol(
    ctx: &LintModuleContext<'_>,
    symbol_id: dir::GlobalSymbolId,
) -> Option<Vec<Option<StringId>>> {
    let declaration_id = symbol_declaration_for(ctx, symbol_id)?;

    if declaration_id.module_id == ctx.module_id() {
        return declaration_parameter_names(ctx.dir.tree(), declaration_id.local_id);
    }

    let module_dir = ctx.session.dir_parsed(declaration_id.module_id)?;
    declaration_parameter_names(&module_dir.tree, declaration_id.local_id)
}

/// Return parameter names for one declaration node id.
fn declaration_parameter_names(
    tree: &dir::Tree,
    declaration_id: dir::LocalNodeIdAny,
) -> Option<Vec<Option<StringId>>> {
    let parameters = declaration_parameters(tree, declaration_id)?;
    Some(
        parameters
            .into_iter()
            .map(|parameter_id| parameter_name_for_signature_parameter(tree, parameter_id))
            .collect(),
    )
}

/// Return the effective type for one parameter.
fn parameter_type_id(
    ctx: &LintModuleContext<'_>,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
) -> Option<dir::GlobalTypeId> {
    let global_parameter_id = parameter_id.into_global_any(ctx.module_id());

    // prefer the checked parameter node type
    if let Some(type_id) = ctx.types.get_node_type_id(global_parameter_id) {
        return Some(type_id);
    }

    // fall back to the bound parameter symbol
    let parameter_symbol = ctx.symbol_for_node(parameter_id)?;
    ctx.symbol_type_id(parameter_symbol)
}

/// Return parameter ids for one callable declaration node.
fn declaration_parameters(
    tree: &dir::Tree,
    declaration_id: dir::LocalNodeIdAny,
) -> Option<Vec<dir::LocalNodeId<dir::Parameter>>> {
    if declaration_id.ty == dir::NodeType::Declaration {
        let declaration = tree.get(declaration_id.into_typed::<dir::Declaration>());
        if let dir::Declaration::Function(declaration) = declaration {
            return Some(declaration.signature.parameters.clone());
        }
        if let Some(members) = declaration.member_ids() {
            return constructor_parameters(tree, members);
        }
    }

    if declaration_id.ty == dir::NodeType::Member {
        let member = tree.get(declaration_id.into_typed::<dir::Member>());
        if let dir::Member::Method { signature, .. } = member {
            return Some(signature.parameters.clone());
        }
    }

    if declaration_id.ty == dir::NodeType::Property {
        let property = tree.get(declaration_id.into_typed::<dir::Property>());
        if let dir::Property::Method { signature, .. } = property {
            return Some(signature.parameters.clone());
        }
    }

    None
}

/// Return constructor parameters for a class or struct when stable.
///
/// When multiple constructors disagree on parameter shape, this
/// returns none to avoid noisy false positives.
fn constructor_parameters(
    tree: &dir::Tree,
    members: &[dir::LocalNodeId<dir::Member>],
) -> Option<Vec<dir::LocalNodeId<dir::Parameter>>> {
    let mut constructor_parameters: Option<Vec<dir::LocalNodeId<dir::Parameter>>> = None;

    for member_id in members {
        let member = tree.get(*member_id);
        let dir::Member::Method { signature, .. } = member else {
            continue;
        };
        if signature.role != Some(dir::FunctionRole::Constructor) {
            continue;
        }

        let parameters = signature.parameters.clone();
        if let Some(existing_parameters) = constructor_parameters.as_ref() {
            if *existing_parameters != parameters {
                return None;
            }
        } else {
            constructor_parameters = Some(parameters);
        }
    }

    constructor_parameters
}

/// Return a stable name for one signature parameter.
fn parameter_name_for_signature_parameter(
    tree: &dir::Tree,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
) -> Option<StringId> {
    let parameter = tree.get(parameter_id);
    match parameter {
        dir::Parameter::Named { name, .. } | dir::Parameter::VariadicNamed { name, .. } => {
            Some(*name)
        }
        dir::Parameter::Pattern { pattern, .. }
        | dir::Parameter::VariadicPattern { pattern, .. } => pattern_name_hint(tree, *pattern),
        dir::Parameter::Error => None,
    }
}

/// Return a stable pattern binding name when one exists.
fn pattern_name_hint(
    tree: &dir::Tree,
    pattern_id: dir::LocalNodeId<dir::Pattern>,
) -> Option<StringId> {
    let pattern = tree.get(pattern_id);
    match pattern {
        dir::Pattern::Binding { name, .. } => Some(*name),
        dir::Pattern::Must(inner)
        | dir::Pattern::BorrowOf { right: inner, .. }
        | dir::Pattern::MoveOf { right: inner, .. }
        | dir::Pattern::DereferenceOf { right: inner } => pattern_name_hint(tree, *inner),
        _ => None,
    }
}

/// Return the first pair of indices that look like a swapped argument pair.
fn swapped_argument_pair(
    argument_hints: &[Option<StringId>],
    parameter_names: &[Option<StringId>],
) -> Option<(usize, usize)> {
    let max_index = std::cmp::min(argument_hints.len(), parameter_names.len());

    for first_index in 0..max_index {
        let Some(first_argument_name) = argument_hints[first_index] else {
            continue;
        };
        let Some(first_parameter_name) = parameter_names[first_index] else {
            continue;
        };
        if first_argument_name == first_parameter_name {
            continue;
        }

        for second_index in first_index + 1..max_index {
            let Some(second_argument_name) = argument_hints[second_index] else {
                continue;
            };
            let Some(second_parameter_name) = parameter_names[second_index] else {
                continue;
            };
            if first_parameter_name == second_parameter_name {
                continue;
            }

            let first_matches_second_parameter = first_argument_name == second_parameter_name;
            let second_matches_first_parameter = second_argument_name == first_parameter_name;
            if first_matches_second_parameter && second_matches_first_parameter {
                return Some((first_index, second_index));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::{TestProgram, test_modules};

    /// Flag two positional arguments that appear swapped by parameter names.
    #[test]
    fn test_flags_swapped_positional_arguments() {
        let test = TestProgram::for_rule_without_prelude(NoArgumentsOrderMismatch);
        let result = test.lint_dir(
            "no_arguments_order_mismatch/test_flags_swapped_positional_arguments.ds",
            r#"
function createUser(firstName: string, lastName: string): string {
    return `${firstName} ${lastName}`;
}

const firstName = "Ada";
const lastName = "Lovelace";

createUser(lastName, firstName);
"#,
        );
        test.result(result)
            .assert_lint("no-arguments-order-mismatch");
    }

    /// Allow calls where argument order matches parameter names.
    #[test]
    fn test_allows_correct_positional_argument_order() {
        let test = TestProgram::for_rule_without_prelude(NoArgumentsOrderMismatch);
        let result = test.lint_dir(
            "no_arguments_order_mismatch/test_allows_correct_positional_argument_order.ds",
            r#"
function createUser(firstName: string, lastName: string): string {
    return `${firstName} ${lastName}`;
}

const firstName = "Ada";
const lastName = "Lovelace";

createUser(firstName, lastName);
"#,
        );
        test.result(result)
            .assert_no_lint("no-arguments-order-mismatch");
    }

    /// Allow calls when argument names do not provide a stable signal.
    #[test]
    fn test_allows_uninformative_argument_names() {
        let test = TestProgram::for_rule_without_prelude(NoArgumentsOrderMismatch);
        let result = test.lint_dir(
            "no_arguments_order_mismatch/test_allows_uninformative_argument_names.ds",
            r#"
function configure(hostName: string, port: int32): void {}

const a = "localhost";
const b = 8080;

configure(a, b);
"#,
        );
        test.result(result)
            .assert_no_lint("no-arguments-order-mismatch");
    }

    /// Allow swapped name hints when swapped types do not fit opposite parameters.
    #[test]
    fn test_allows_swapped_name_hints_with_incompatible_swapped_types() {
        let test = TestProgram::for_rule_without_prelude(NoArgumentsOrderMismatch);
        let result = test.lint_dir(
            "no_arguments_order_mismatch/test_allows_swapped_name_hints_with_incompatible_swapped_types.ds",
            r#"
function setDimensions(width: int32, height: string): void {}

const height: int32 = 10;
const width: string = "wide";

setDimensions(height, width);
"#,
        );
        test.result(result)
            .assert_no_lint("no-arguments-order-mismatch");
    }

    /// Flag swapped imported function calls across modules.
    #[test]
    fn test_flags_cross_module_swapped_arguments() {
        let test = TestProgram::for_rule_without_prelude(NoArgumentsOrderMismatch);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_arguments_order_mismatch/api.ds" => r#"
export function pair(leftValue: string, rightValue: string): string {
    return `${leftValue}:${rightValue}`;
}
"#,
                "no_arguments_order_mismatch/use.ds" => r#"
import { pair } from "./api.ds";

const leftValue = "L";
const rightValue = "R";

pair(rightValue, leftValue);
"#,
            },
            "no_arguments_order_mismatch/use.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-arguments-order-mismatch");
    }

    /// Allow calls with non positional arguments to avoid false positives.
    #[test]
    fn test_allows_non_positional_arguments() {
        let test = TestProgram::for_rule_without_prelude(NoArgumentsOrderMismatch);
        let result = test.lint_dir(
            "no_arguments_order_mismatch/test_allows_non_positional_arguments.ds",
            r#"
declare function makeName(firstName: string, lastName: string): string;
const values = ["Ada", "Lovelace"];
makeName(...values);
"#,
        );
        test.result(result)
            .assert_no_lint("no-arguments-order-mismatch");
    }

    /// Flag swapped positional arguments for constructor calls.
    #[test]
    fn test_flags_swapped_constructor_arguments() {
        let test = TestProgram::for_rule_without_prelude(NoArgumentsOrderMismatch);
        let result = test.lint_dir(
            "no_arguments_order_mismatch/test_flags_swapped_constructor_arguments.ds",
            r#"
class Person {
    constructor(firstName: string, lastName: string) {}
}

const firstName = "Ada";
const lastName = "Lovelace";

new Person(lastName, firstName);
"#,
        );
        test.result(result)
            .assert_lint("no-arguments-order-mismatch");
    }

    /// Flag swapped constructor arguments for imported class symbols.
    #[test]
    fn test_flags_cross_module_swapped_constructor_arguments() {
        let test = TestProgram::for_rule_without_prelude(NoArgumentsOrderMismatch);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_arguments_order_mismatch/person.ds" => r#"
export class Person {
    constructor(firstName: string, lastName: string) {}
}
"#,
                "no_arguments_order_mismatch/new_person.ds" => r#"
import { Person } from "./person.ds";

const firstName = "Ada";
const lastName = "Lovelace";

new Person(lastName, firstName);
"#,
            },
            "no_arguments_order_mismatch/new_person.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-arguments-order-mismatch");
    }

    /// Flag swapped positional arguments in method calls.
    #[test]
    fn test_flags_swapped_method_arguments() {
        let test = TestProgram::for_rule_without_prelude(NoArgumentsOrderMismatch);
        let result = test.lint_dir(
            "no_arguments_order_mismatch/test_flags_swapped_method_arguments.ds",
            r#"
class Pairer {
    pair(leftValue: string, rightValue: string): string {
        return `${leftValue}:${rightValue}`;
    }
}

const leftValue = "L";
const rightValue = "R";
const pairer = new Pairer();

pairer.pair(rightValue, leftValue);
"#,
        );
        test.result(result)
            .assert_lint("no-arguments-order-mismatch");
    }

    /// Allow method calls where positional argument order is correct.
    #[test]
    fn test_allows_method_arguments_in_correct_order() {
        let test = TestProgram::for_rule_without_prelude(NoArgumentsOrderMismatch);
        let result = test.lint_dir(
            "no_arguments_order_mismatch/test_allows_method_arguments_in_correct_order.ds",
            r#"
class Pairer {
    pair(leftValue: string, rightValue: string): string {
        return `${leftValue}:${rightValue}`;
    }
}

const leftValue = "L";
const rightValue = "R";
const pairer = new Pairer();

pairer.pair(leftValue, rightValue);
"#,
        );
        test.result(result)
            .assert_no_lint("no-arguments-order-mismatch");
    }
}

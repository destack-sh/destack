use std::collections::HashSet;

use destack_dir::{
    self as dir, GlobalSymbolId, LocalNodeId, LocalSymbolId, Mutability, NodeVisitor,
    NodeVisitorOptions, walk_expression,
};
use destack_source::ModuleId;
use destack_workspace::{LintSeverity, PreferConstDestructuring};

use crate::rules::common::{
    collect_local_symbol_direct_reference_expression_ids, collect_pattern_value_binding_symbols,
    expression_assignment_target, expression_is_standalone_statement, expression_reference_is_read,
    statement_expression_ancestor,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Require `const` declarations for never reassigned variables.
    ///
    /// Using `const` for variables that are never reassigned helps clarify
    /// intent and can catch accidental reassignments.
    #[lint(
        id = "prefer-const",
        code = "LY035",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferConst,
    "Require const for never-reassigned variables"
}

impl LintRule for PreferConst {
    fn meta(&self) -> &'static LintMeta {
        PreferConst::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // collect mutable declaration symbols by declaration site
        let mut collector = LetDeclarationCollector::new(ctx);
        collector.run();
        let let_declarations = collector.let_declarations;

        if let_declarations.is_empty() {
            return;
        }

        // report declarations when one or more bound symbols stay immutable
        for declaration in let_declarations {
            let mut eligible_symbols = Vec::new();

            // collect eligible symbols group by group
            for group in &declaration.binding_groups {
                let group_eligible_symbols = group
                    .symbols
                    .iter()
                    .copied()
                    .filter(|symbol_id| symbol_should_be_const(ctx, &declaration, *symbol_id))
                    .collect::<Vec<_>>();
                let requires_all_symbols = group.is_destructuring
                    && ctx.options.style.prefer_const_destructuring
                        == PreferConstDestructuring::All;
                if requires_all_symbols && group_eligible_symbols.len() != group.symbols.len() {
                    continue;
                }

                eligible_symbols.extend(group_eligible_symbols);
            }
            if eligible_symbols.is_empty() {
                continue;
            }

            let can_fix_declaration = eligible_symbols.len() == declaration.symbols.len()
                && declaration
                    .symbols
                    .iter()
                    .all(|symbol_id| declaration.initialized_symbols.contains(symbol_id));

            for symbol_id in eligible_symbols {
                let severity = ctx.get_effective_severity(meta, declaration.expression_id);
                if !severity.is_enabled() {
                    continue;
                }

                let span = ctx.get_span(declaration.expression_id);
                let symbol_name = ctx
                    .symbols
                    .get_symbol(symbol_id.local_id)
                    .name()
                    .map(|name| ctx.repository.strings.get(name).to_string())
                    .unwrap_or_else(|| "binding".to_string());
                let mut diagnostic = LintDiagnostic::new(
                    PREFER_CONST.id,
                    PREFER_CONST.code,
                    PREFER_CONST.category,
                    severity,
                    format!("`{symbol_name}` is never reassigned, use const instead"),
                    ctx.module.file_id,
                    span,
                )
                .with_label("this binding can be const");

                if can_fix_declaration
                    && ctx.include_fixes
                    && let Some(fix) = build_prefer_const_fix(ctx, declaration.expression_id)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// One mutable declaration and its bound symbols.
struct LetDeclaration {
    /// The declaration expression id.
    expression_id: LocalNodeId<dir::Expression>,
    /// All binding groups declared in the expression.
    binding_groups: Vec<BindingGroup>,
    /// All value symbols declared in the expression.
    symbols: Vec<GlobalSymbolId>,
    /// The symbols initialized at the declaration site.
    initialized_symbols: HashSet<GlobalSymbolId>,
}

/// One binding group within a mutable declaration.
struct BindingGroup {
    /// The declared symbols in the group.
    symbols: Vec<GlobalSymbolId>,
    /// Whether the group is a destructuring pattern.
    is_destructuring: bool,
}

/// Build a safe rewrite from `let` or `var` to `const`.
fn build_prefer_const_fix(
    ctx: &LintModuleDirContext<'_>,
    expression_id: LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    let statement_id = statement_expression_ancestor(ctx.tree, expression_id)?;
    let statement_span = ctx.get_span(statement_id);
    let statement_text = ctx.get_span_text(statement_span);
    if statement_text.contains('{') || statement_text.contains('[') {
        return None;
    }

    let keyword = binding_keyword(statement_text)?;
    let keyword_offset = statement_text.find(keyword)? as u32;
    let replacement_span = destack_source::Span::new(
        statement_span.file,
        statement_span.start + keyword_offset,
        statement_span.start + keyword_offset + keyword.len() as u32,
    );
    let edits = ctx
        .edit_builder()
        .replace(replacement_span, "const")
        .into_edits();
    Some(LintFix::safe("Replace with const declaration").with_edits(edits))
}

/// Return the leading mutable declaration keyword.
fn binding_keyword(expression_text: &str) -> Option<&'static str> {
    if expression_text.starts_with("let") {
        return Some("let");
    }

    if expression_text.starts_with("var") {
        return Some("var");
    }

    None
}

/// Collector for let declarations.
struct LetDeclarationCollector<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The module ID for creating GlobalSymbolIds.
    module_id: ModuleId,
    /// Collected let declarations with all bound symbols.
    let_declarations: Vec<LetDeclaration>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> LetDeclarationCollector<'a, 'b> {
    /// Build a collector for let declarations.
    fn new(ctx: &'a mut LintModuleDirContext<'b>) -> Self {
        let module_id = ctx.module_id();

        Self {
            ctx,
            module_id,
            let_declarations: Vec::new(),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Convert a local symbol ID to a global symbol ID.
    fn to_global(&self, local_id: LocalSymbolId) -> GlobalSymbolId {
        GlobalSymbolId::new(self.module_id, local_id)
    }
}

impl NodeVisitor for LetDeclarationCollector<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check for mutable let declarations
        if let dir::Expression::Let {
            mutability: Mutability::Mutable,
            declarators,
            ..
        } = expression
        {
            // collect value symbols from all declarator patterns
            let mut declaration_symbols = HashSet::new();
            let mut initialized_symbols = HashSet::new();
            let mut binding_groups = Vec::new();
            for declarator_id in declarators {
                let declarator = tree.get(*declarator_id);
                let mut declarator_symbols = HashSet::new();
                collect_pattern_value_binding_symbols(
                    tree,
                    self.ctx.symbols,
                    declarator.pattern,
                    &mut declarator_symbols,
                );
                if declarator.value.is_some() {
                    initialized_symbols.extend(
                        declarator_symbols
                            .iter()
                            .copied()
                            .map(|local_symbol| self.to_global(local_symbol)),
                    );
                }
                if !declarator_symbols.is_empty() {
                    binding_groups.push(BindingGroup {
                        symbols: declarator_symbols
                            .iter()
                            .copied()
                            .map(|local_symbol| self.to_global(local_symbol))
                            .collect(),
                        is_destructuring: pattern_is_destructuring(tree, declarator.pattern),
                    });
                }
                declaration_symbols.extend(declarator_symbols);
            }

            if !declaration_symbols.is_empty() {
                let symbols = declaration_symbols
                    .into_iter()
                    .map(|local_symbol| self.to_global(local_symbol))
                    .collect();
                self.let_declarations.push(LetDeclaration {
                    expression_id: id,
                    binding_groups,
                    symbols,
                    initialized_symbols,
                });
            }
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

/// Return true when one bound symbol should use const.
fn symbol_should_be_const(
    ctx: &LintModuleDirContext<'_>,
    declaration: &LetDeclaration,
    symbol_id: GlobalSymbolId,
) -> bool {
    if declaration.initialized_symbols.contains(&symbol_id) {
        return !symbol_has_write_references(ctx, symbol_id);
    }

    symbol_has_single_const_eligible_assignment(ctx, declaration.expression_id, symbol_id)
}

/// Return true when one symbol has write references after its declaration.
fn symbol_has_write_references(ctx: &LintModuleDirContext<'_>, symbol_id: GlobalSymbolId) -> bool {
    let reference_ids = collect_local_symbol_direct_reference_expression_ids(
        ctx.module_id(),
        ctx.tree,
        symbol_id.local_id,
    );

    reference_ids
        .into_iter()
        .any(|reference_id| reference_write_kind(ctx.tree, reference_id).is_some())
}

/// Return true when one symbol has exactly one const-eligible assignment.
fn symbol_has_single_const_eligible_assignment(
    ctx: &LintModuleDirContext<'_>,
    declaration_expression_id: LocalNodeId<dir::Expression>,
    symbol_id: GlobalSymbolId,
) -> bool {
    let reference_ids = collect_local_symbol_direct_reference_expression_ids(
        ctx.module_id(),
        ctx.tree,
        symbol_id.local_id,
    );
    let mut assignment_expression_id = None;
    let mut is_read_before_assignment = false;

    for reference_id in reference_ids {
        if assignment_expression_id.is_none()
            && expression_reference_is_read(ctx.tree, reference_id)
        {
            is_read_before_assignment = true;
        }

        match reference_write_kind(ctx.tree, reference_id) {
            Some(ReferenceWriteKind::SimpleAssign(parent_id)) => {
                if assignment_expression_id.is_some() {
                    return false;
                }
                assignment_expression_id = Some(parent_id);
            }
            Some(ReferenceWriteKind::OtherWrite) => {
                return false;
            }
            None => {}
        }
    }

    let Some(assignment_expression_id) = assignment_expression_id else {
        return false;
    };
    if ctx.options.style.prefer_const_ignore_read_before_assign && is_read_before_assignment {
        return false;
    }

    assignment_can_become_const_declaration(
        ctx,
        declaration_expression_id,
        assignment_expression_id,
    )
}

/// One write shape for a symbol reference.
enum ReferenceWriteKind {
    /// A plain assignment like `x = value`.
    SimpleAssign(LocalNodeId<dir::Expression>),
    /// Any other write that cannot become a const declaration.
    OtherWrite,
}

/// Return the write kind for one symbol reference.
fn reference_write_kind(
    tree: &dir::Tree,
    reference_id: LocalNodeId<dir::Expression>,
) -> Option<ReferenceWriteKind> {
    let parent = tree.get_parent(reference_id.id)?;
    if parent.ty != dir::NodeType::Expression {
        return None;
    }

    let parent_id = parent.into_typed::<dir::Expression>();
    let parent_expression = tree.get(parent_id);
    let target_id = expression_assignment_target(tree, parent_expression)?;
    if target_id != reference_id {
        return None;
    }

    match parent_expression {
        dir::Expression::Assign { .. } => Some(ReferenceWriteKind::SimpleAssign(parent_id)),
        dir::Expression::AssignBinary { .. } | dir::Expression::Unary { .. } => {
            Some(ReferenceWriteKind::OtherWrite)
        }
        _ => None,
    }
}

/// Return true when one assignment can become a const declaration.
fn assignment_can_become_const_declaration(
    ctx: &LintModuleDirContext<'_>,
    declaration_expression_id: LocalNodeId<dir::Expression>,
    assignment_expression_id: LocalNodeId<dir::Expression>,
) -> bool {
    if !expression_is_standalone_statement(ctx.tree, assignment_expression_id) {
        return false;
    }

    let (declaration_scope_id, _, _) = ctx.symbols.get_scope(declaration_expression_id, ctx.tree);
    let (assignment_scope_id, _, _) = ctx.symbols.get_scope(assignment_expression_id, ctx.tree);
    if declaration_scope_id != assignment_scope_id {
        return false;
    }

    let Some(statement_expression_id) =
        statement_expression_ancestor(ctx.tree, assignment_expression_id)
    else {
        return false;
    };
    let Some(parent) = ctx.tree.get_parent(statement_expression_id.id) else {
        return true;
    };

    matches!(parent.ty, dir::NodeType::Block)
}

/// Return true when one pattern is a destructuring pattern.
fn pattern_is_destructuring(tree: &dir::Tree, pattern_id: LocalNodeId<dir::Pattern>) -> bool {
    !matches!(
        tree.get(pattern_id),
        dir::Pattern::Wildcard
            | dir::Pattern::Must(_)
            | dir::Pattern::ReferenceOf { .. }
            | dir::Pattern::ValueOf { .. }
            | dir::Pattern::Binding { .. }
            | dir::Pattern::Expression { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag let that is never reassigned.
    #[test]
    fn test_flags_never_reassigned_let() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_flags_never_reassigned_let.ds",
            r#"
let x = 1;
let y = x + 1;
"#,
        );
        test.result(result).assert_lint_count("prefer-const", 2);
    }

    /// Safely rewrite `let` bindings that are never reassigned.
    #[test]
    fn test_fix_rewrites_let_to_const() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_fix_rewrites_let_to_const.ds",
            r#"
let value = 1;
"#,
        );
        test.result(result)
            .assert_lint("prefer-const")
            .assert_has_fix("prefer-const")
            .assert_safe_fixed(
                r#"
const value = 1;
"#,
            );
    }

    /// Safely rewrite legacy mutable declarations that are never reassigned.
    #[test]
    fn test_mutation_fix_rewrites_var_to_const() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_mutation_fix_rewrites_var_to_const.ds",
            r#"
var value = 1;
"#,
        );
        test.result(result)
            .assert_lint("prefer-const")
            .assert_has_fix("prefer-const")
            .assert_safe_fixed(
                r#"
const value = 1;
"#,
            );
    }

    /// Allow let that is reassigned.
    #[test]
    fn test_allows_reassigned_let() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_reassigned_let.ds",
            r#"
let x = 1;
x = 2;
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Allow let with compound assignment.
    #[test]
    fn test_allows_compound_assignment() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_compound_assignment.ds",
            r#"
let x = 1;
x += 1;
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Allow let with increment.
    #[test]
    fn test_allows_increment() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_increment.ds",
            r#"
let x = 1;
x++;
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Allow let with pre increment.
    #[test]
    fn test_allows_pre_increment() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_pre_increment.ds",
            r#"
let x = 1;
++x;
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Allow const declarations.
    #[test]
    fn test_allows_const() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_const.ds",
            r#"
const x = 1;
const y = x + 1;
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Mixed let and const.
    #[test]
    fn test_mixed_declarations() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_mixed_declarations.ds",
            r#"
let x = 1;
let y = 2;
y = 3;
const z = x + y;
"#,
        );
        // only x should be flagged (y is reassigned, z is already const)
        test.result(result).assert_lint_count("prefer-const", 1);
    }

    /// Allow let with decrement.
    #[test]
    fn test_allows_decrement() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_decrement.ds",
            r#"
let x = 10;
x--;
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Allow let reassigned in nested scope.
    #[test]
    fn test_allows_reassigned_in_nested_scope() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_reassigned_in_nested_scope.ds",
            r#"
let x = 1;
if (true) {
    x = 2;
}
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Flag let in function that is never reassigned.
    #[test]
    fn test_flags_let_in_function() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_flags_let_in_function.ds",
            r#"
function foo(): number {
    let x = 1;
    return x;
}
"#,
        );
        test.result(result).assert_lint("prefer-const");
    }

    /// Allow let reassigned with other compound operators.
    #[test]
    fn test_allows_other_compound_operators() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_other_compound_operators.ds",
            r#"
let a = 10;
let b = 20;
let c = 5;
a -= 1;
b *= 2;
c /= 1;
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Avoid mixed declaration fixes when one declarator is reassigned.
    #[test]
    fn test_flags_const_eligible_binding_in_mixed_declaration() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_flags_const_eligible_binding_in_mixed_declaration.ds",
            r#"
let a = 1, b = 2;
b = 3;
"#,
        );
        test.result(result)
            .assert_lint_count("prefer-const", 1)
            .assert_has_no_fix("prefer-const");
    }

    /// Flag destructured let declarations that are never reassigned.
    #[test]
    fn test_flags_destructured_let() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_flags_destructured_let.ds",
            r#"
let { a, b } = value;
"#,
        );
        test.result(result)
            .assert_lint("prefer-const")
            .assert_has_no_fix("prefer-const");
    }

    /// Flag one late assignment that can become a const declaration.
    #[test]
    fn test_flags_single_late_assignment() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_flags_single_late_assignment.ds",
            r#"
let value;
value = 1;
consume(value);
"#,
        );
        test.result(result)
            .assert_lint("prefer-const")
            .assert_has_no_fix("prefer-const");
    }

    /// Allow conditional late assignment that cannot become a declaration.
    #[test]
    fn test_allows_conditional_late_assignment() {
        let test = TestProgram::for_rule_with_prelude(PreferConst);
        let result = test.lint_dir(
            "prefer_const/test_allows_conditional_late_assignment.ds",
            r#"
let value;
if (ready) {
    value = 1;
}
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Allow partial destructuring groups when `all` is required.
    #[test]
    fn test_allows_partial_destructuring_group_with_all_mode() {
        let test = TestProgram::for_rule_with_prelude(PreferConst).with_options(|options| {
            options.style.prefer_const_destructuring = PreferConstDestructuring::All;
        });
        let result = test.lint_dir(
            "prefer_const/test_allows_partial_destructuring_group_with_all_mode.ds",
            r#"
let { a, b } = value;
b = 1;
consume(a);
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }

    /// Ignore read-before-assign bindings when configured.
    #[test]
    fn test_allows_read_before_assign_when_ignored() {
        let test = TestProgram::for_rule_with_prelude(PreferConst).with_options(|options| {
            options.style.prefer_const_ignore_read_before_assign = true;
        });
        let result = test.lint_dir(
            "prefer_const/test_allows_read_before_assign_when_ignored.ds",
            r#"
let value;
consume(value);
value = 1;
"#,
        );
        test.result(result).assert_no_lint("prefer-const");
    }
}

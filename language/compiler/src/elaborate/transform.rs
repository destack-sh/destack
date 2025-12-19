use destack_dir::{Expression, NodeTree, SymbolTable, TypeTable};
use destack_source::ModuleId;

use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Transform a module: semantic simplifications for constructs no target supports.
    ///
    /// Transformations (in order):
    /// 1. `if let` → if + explicit binding
    /// 2. `match` → decision trees (if-else chains)
    /// 3. Expressions as values → temp + assignments
    ///
    /// Note: simple destructuring patterns are NOT transformed here.
    /// They remain in DIR for targets to handle (JS emits native destructuring,
    /// native/lower expands to field accesses).
    pub(super) fn elaborate_module_transform(&self, module_id: ModuleId) -> ElaborateResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let mut tree = module.dir.tree.write();
        let symbols = module.dir.symbols.read();
        let types = module.dir.types.read();

        // pass 1: if-let → if + binding
        self.transform_if_let(&mut tree, &symbols, &types)?;

        // pass 2: match → decision trees
        self.transform_match(&mut tree, &symbols, &types)?;

        // pass 3: expressions as values → temp + assignments (must be last)
        self.transform_expression_as_value(&mut tree, &symbols, &types)?;

        Ok(())
    }

    /// Transform `if let` expressions into explicit if + binding.
    ///
    /// ```ds
    /// if let Some(x) = expr { body }
    /// ```
    /// ->
    /// ```ds
    /// { let __m = expr; if (__m is Some) { let x = __m.value; body } }
    /// ```
    fn transform_if_let(
        &self,
        _tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<()> {
        // NOTE #Incomplete: transform if-let expressions
        Ok(())
    }

    /// Transform `match` expressions into decision trees (if-else chains).
    ///
    /// ```ds
    /// match (x) {
    ///     Some(v) if v > 0 => positive(v)
    ///     Some(v) => nonPositive(v)
    ///     None => zero()
    /// }
    /// ```
    /// ->
    /// ```ds
    /// if (x is Some) {
    ///     let v = x.value;
    ///     if (v > 0) { positive(v) }
    ///     else { nonPositive(v) }
    /// } else { zero() }
    /// ```
    fn transform_match(
        &self,
        _tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<()> {
        // NOTE #Incomplete: transform match expressions to decision trees
        Ok(())
    }

    /// Transform expressions used as values into temporaries and assignments.
    ///
    /// Must run last since transform_if_let and transform_match produce if expressions.
    ///
    /// ```ds
    /// const result = if (cond) { a } else { b };
    /// ```
    /// ->
    /// ```ds
    /// let result;
    /// if (cond) { result = a; } else { result = b; }
    /// ```
    fn transform_expression_as_value(
        &self,
        _tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<()> {
        // NOTE #Incomplete: transform expressions as values
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    #[test]
    fn test_transform_match_literal_patterns() {
        // match on literal values transforms to if-else chain
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function check(x: number): string {
    match (x) {
        1 => "one"
        2 => "two"
        _ => "other"
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile();
        test.check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function check(x: number): string {
    if (x == 1) { "one" }
    else if (x == 2) { "two" }
    else { "other" }
}
"#,
        );
    }

    #[test]
    fn test_transform_match_boolean() {
        // match on boolean transforms to if-else
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function check(b: boolean): number {
    match (b) {
        true => 1
        false => 0
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile();
        test.check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function check(b: boolean): number {
    if (b == true) { 1 }
    else { 0 }
}
"#,
        );
    }

    #[test]
    fn test_transform_match_with_guard() {
        // match with guard clause transforms to nested if
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function classify(x: number): string {
    match (x) {
        n if n > 0 => "positive"
        n if n < 0 => "negative"
        _ => "zero"
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile();
        test.check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function classify(x: number): string {
    let n = x;
    if (n > 0) { "positive" }
    else if (n < 0) { "negative" }
    else { "zero" }
}
"#,
        );
    }

    #[test]
    fn test_transform_match_wildcard_only() {
        // match with only wildcard becomes the body directly
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function always(x: number): number {
    match (x) {
        _ => 42
    }
}
"#,
        );
        test.elaborate_module(module_id);
        test.compile();
        test.check_clean();
        test.assert_elaborated(
            module_id,
            r#"
function always(x: number): number {
    42
}
"#,
        );
    }
}

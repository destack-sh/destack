use destack_dir::{NodeTree, SymbolTable, TypeTable};

use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Transform `if let` expressions into explicit if + binding.
    ///
    /// ```ds
    /// if let Some(x) = expr { body }
    /// ```
    /// ->
    /// ```ds
    /// { let __m = expr; if (__m is Some) { let x = __m.value; body } }
    /// ```
    ///
    /// if-let syntax is an If expression with a refutable Let expression as the condition:
    /// 1. Detect If nodes where condition is a Let with a refutable pattern
    /// 2. Extract the pattern and the value expression
    /// 3. Create the temp binding, type check, and inner bindings
    pub(super) fn transform_if_let(
        &self,
        _tree: &mut NodeTree,
        _symbols: &SymbolTable,
        _types: &TypeTable,
    ) -> ElaborateResult<()> {
        // TODO #Incomplete: if-let transform needs parser and binder support
        // the condition of an if-let is an Expression::Let with a refutable pattern
        Ok(())
    }
}

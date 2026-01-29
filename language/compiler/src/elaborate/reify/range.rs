use destack_builtin::LanguageSymbol;
use destack_dir::{
    DynamicKey, Expression, LocalNodeId, NodeTree, NodeType, Path, Property, SymbolBinding,
    SymbolKind, SymbolSpace, SymbolTable, SymbolType,
};
use destack_workspace::ProfileId;

use crate::{Compiler, ElaborateError, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Reify a range expression into a core range struct literal.
    pub(super) fn reify_range_expression(
        &self,
        module_id: destack_source::ModuleId,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        start: LocalNodeId<Expression>,
        end: LocalNodeId<Expression>,
        is_inclusive: bool,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
    ) -> ElaborateResult<()> {
        // resolve the range constructor symbol
        let range_item = if is_inclusive {
            LanguageSymbol::RangeInclusive
        } else {
            LanguageSymbol::Range
        };
        let range_symbol = self
            .get_language_symbol(profile, range_item)
            .ok_or_else(|| ElaborateError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(module_id)
                    .into_anchored(Some(profile)),
            })?;
        let range_name_id = self.program.strings.intern(range_item.export_name());

        // reserve the tagged object type expression
        let scope = tree.get_scope(expression_id);
        let parent_id = Some(expression_id.into_any());
        let ty_id = tree.reserve_from(
            NodeType::Expression,
            expression_id.into_any(),
            scope,
            parent_id,
        );
        let ty_path = Path::from(&[range_name_id][..]);
        let ty_id = tree.insert(
            ty_id,
            Expression::GlobalReference {
                path: ty_path,
                static_arguments: None,
                target_symbol: range_symbol,
            },
        );

        // build the start and end properties
        let start_property =
            self.create_range_property(expression_id, start, "start", tree, symbols);
        let end_property = self.create_range_property(expression_id, end, "end", tree, symbols);

        // replace the range expression with a tagged object expression
        tree.replace(
            expression_id,
            Expression::TaggedObjectExpression {
                ty: ty_id,
                properties: vec![start_property, end_property],
            },
        );

        Ok(())
    }

    /// Create a field property for a range literal.
    fn create_range_property(
        &self,
        expression_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        name: &str,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
    ) -> LocalNodeId<Property> {
        // allocate a property node in the same scope as the range expression
        let scope = tree.get_scope(expression_id);
        let parent_id = Some(expression_id.into_any());
        let property_id = tree.reserve_from(
            NodeType::Property,
            expression_id.into_any(),
            scope,
            parent_id,
        );

        // bind an anonymous property symbol
        let (symbol_id, _) = symbols.insert_symbol(
            SymbolKind::Item,
            SymbolType::Void,
            SymbolSpace::Value,
            SymbolBinding::Runtime,
            None,
            scope,
            None,
        );

        // emit a named field property
        let name_id = self.program.strings.intern(name);
        let property = Property::Field {
            modifiers: None,
            key: Some(DynamicKey::Name(name_id)),
            value: Some(value_id),
            default: None,
            symbol: symbol_id,
        };

        tree.insert(property_id, property)
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    /// Reify an exclusive range expression into a tagged Range literal.
    #[test]
    fn test_reify_range_expression_exclusive() {
        // build a module with a range expression
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
let range = 0..10;
"#,
        );

        // run the elaborate pipeline
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
let range = Range { start: 0, end: 10 };
"#,
        );
    }

    /// Reify an inclusive range expression into a tagged RangeInclusive literal.
    #[test]
    fn test_reify_range_expression_inclusive() {
        // build a module with a range expression
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
let range = 1..=3;
"#,
        );

        // run the elaborate pipeline
        test.elaborate_module(module_id);
        test.compile_check_clean();
        test.assert_elaborated(
            module_id,
            r#"
let range = RangeInclusive { start: 1, end: 3 };
"#,
        );
    }
}

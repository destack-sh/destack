use crate::{Expression, Key, Name};
use destack_ast::{StringId, is_identifier};
use destack_dir::{self as dir, Module, NodeTree, SymbolTable, TypeTable};

use crate::{TranspileResult, TranspileResultExt, Transpiler, TranspilerUnit};

impl Transpiler {
    /// Lower a string to a name.
    pub fn lower_string_to_name(
        &self,
        _module: &Module,
        string_id: StringId,
        unit: &mut TranspilerUnit,
    ) -> Name {
        let string_id = unit.strings.intern_from(&self.program.strings, string_id);
        let string = self.program.strings.get(string_id);
        if is_identifier(string.as_ref()) {
            Name::Identifier(string_id)
        } else {
            Name::String(string_id)
        }
    }

    /// Lower a key from DIR into JS AST.
    pub fn lower_key(
        &self,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        key: dir::DynamicKey,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<Key> {
        let key = match key {
            dir::DynamicKey::Name(name) => {
                let name = self.lower_string_to_name(module, name, unit);
                Key::Name(name)
            }
            dir::DynamicKey::Expression(expression_id) => {
                let expression_id = self
                    .lower_expression(module, tree, symbols, types, expression_id, unit)
                    .expect_node::<Expression>(expression_id.into_global_any(module.id), unit)?;
                Key::Expression(expression_id)
            }
            dir::DynamicKey::NamedExpression { name, key } => {
                let name = self.lower_string_to_name(module, name, unit);
                let key = self
                    .lower_expression(module, tree, symbols, types, key, unit)
                    .expect_node::<Expression>(key.into_global_any(module.id), unit)?;
                Key::NamedExpression { name, key }
            }
        };
        Ok(key)
    }
}

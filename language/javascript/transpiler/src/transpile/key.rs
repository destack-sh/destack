use dyst_ast::{StringId, is_identifier};
use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::{Key, Name};

use crate::{TranspileResult, Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    /// Transpile a string to a name.
    pub fn transpile_string_to_name(
        &self,
        _module: &'a Module,
        string_id: StringId,
        unit: &mut TranspilerUnit,
    ) -> Name {
        let string_id = unit.strings.intern_from(&self.session.strings, string_id);
        let string = self.session.strings.get(string_id);
        if is_identifier(string.as_ref()) {
            Name::Identifier(string_id)
        } else {
            Name::String(string_id)
        }
    }

    /// Transpile a key from DIR into JS AST.
    pub fn transpile_key(
        &self,
        module: &'a Module,
        key: dir::Key,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<Key> {
        let key = match key {
            dir::Key::Name(name) => {
                let name = self.transpile_string_to_name(module, name, unit);
                Key::Name(name)
            }
            dir::Key::Expression(expression) => {
                let expression = self.transpile_expression(module, expression, unit)?;
                Key::Expression(expression)
            }
            dir::Key::NamedExpression { name, key } => {
                let name = self.transpile_string_to_name(module, name, unit);
                let key = self.transpile_expression(module, key, unit)?;
                Key::NamedExpression { name, key }
            }
        };
        Ok(key)
    }
}

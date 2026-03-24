use crate::{CodegenJsResult, CodegenJsResultExt, Expression, Key, ModuleLowerer, Name};
use destack_ast::{StringId, is_identifier};
use destack_dir as dir;

impl ModuleLowerer<'_> {
    /// Lower a string to a name.
    pub fn lower_string_to_name(&mut self, string_id: StringId) -> Name {
        let string = self.source_strings.get(string_id);
        let string_id = self.strings.intern(&string);
        if is_identifier(string.as_ref()) {
            Name::Identifier(string_id)
        } else {
            Name::String(string_id)
        }
    }

    /// Lower a name from DIR into JS AST.
    pub fn lower_name(&mut self, name: dir::Name) -> Name {
        let name_id = self.strings.intern_from(self.source_strings, name.string());
        match name {
            dir::Name::Identifier(_) => Name::Identifier(name_id),
            dir::Name::String(_) => Name::String(name_id),
            dir::Name::Number(_) => Name::String(name_id),
        }
    }

    /// Lower a key from DIR into JS AST.
    pub fn lower_key(&mut self, key: dir::DynamicKey) -> CodegenJsResult<Key> {
        let key = match key {
            dir::DynamicKey::Name(name) => {
                let name = self.lower_string_to_name(name);
                Key::Name(name)
            }
            dir::DynamicKey::Private(name) => {
                let name = self.strings.intern_from(self.source_strings, name);
                Key::Private(name)
            }
            dir::DynamicKey::Number(name) => {
                let name = self.lower_string_to_name(name);
                Key::Name(name)
            }
            dir::DynamicKey::Expression(expression_id) => {
                let expression_id = self
                    .lower_expression(expression_id)
                    .expect_node::<Expression>(
                        expression_id.into_global_any(self.module.id),
                        self,
                    )?;
                Key::Expression(expression_id)
            }
            dir::DynamicKey::NamedExpression { name, key } => {
                let name = self.lower_string_to_name(name);
                let key = self
                    .lower_expression(key)
                    .expect_node::<Expression>(key.into_global_any(self.module.id), self)?;
                Key::NamedExpression { name, key }
            }
        };
        Ok(key)
    }
}

use crate::EmitError;
use destack_dir as dir;
use destack_js as js;

use crate::emit::js::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower a name from DIR into JavaScript.
    pub(crate) fn lower_name(&mut self, name: dir::Name) -> js::Name {
        match name {
            dir::Name::Identifier(name) => js::Name::Identifier(name),
            dir::Name::String(name) => js::Name::String(name),
            dir::Name::Index(index) => {
                let name = self.strings.intern(&index.to_string());

                js::Name::String(name)
            }
        }
    }

    /// Lower a key from DIR into JavaScript.
    pub(crate) fn lower_key(&mut self, key: dir::Key) -> Result<js::Key, EmitError> {
        match key {
            dir::Key::Name(name) => Ok(js::Key::Name(self.lower_name(name))),
            dir::Key::Expression(expression_id) => {
                let expression_id = self.lower_expression_as::<js::Expression>(expression_id)?;

                Ok(js::Key::Expression(expression_id))
            }
        }
    }
}

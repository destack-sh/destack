use tspp_dir as dir;
use tspp_js as js;

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
}

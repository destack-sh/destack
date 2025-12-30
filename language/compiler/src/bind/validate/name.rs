use destack_base::StringId;
use destack_dir::StaticKey;
use destack_workspace::{BUILTIN_PACKAGE_ID, Module};

use crate::{BindError, Compiler};

const RESERVED_IDENTIFIERS: &[&str] = &[
    "implements",
    "interface",
    "let",
    "package",
    "private",
    "protected",
    "public",
    "static",
    "yield",
    "await",
    "eval",
    "arguments",
];

impl Compiler {
    /// Check if an identifier is reserved.
    fn is_reserved_identifier(&self, name: StringId) -> bool {
        let name_str = self.program.strings.get(name);
        RESERVED_IDENTIFIERS.contains(&name_str.as_ref())
    }

    /// Check for reserved identifiers used as binding names.
    pub(super) fn validate_binding_names(&self, module: &Module) {
        let dir = module.dir_base();
        let symbols = dir.symbols.read();
        for scope in symbols.scopes() {
            for (key, symbol_id) in scope.named_symbols.iter() {
                let symbol = symbols.get_symbol(*symbol_id);
                let Some(primary_declaration) = symbol.primary_declaration else {
                    continue;
                };

                // reserved identifiers are not allowed as binding names
                if let StaticKey::Name(name) = key
                    && self.is_reserved_identifier(*name)
                    && module.package_id != BUILTIN_PACKAGE_ID
                {
                    self.error(BindError::ReservedIdentifier {
                        node: primary_declaration,
                        name: *name,
                    });
                }
            }
        }
    }
}

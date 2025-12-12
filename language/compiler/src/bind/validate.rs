use destack_dir::{StaticKey, SymbolKind};
use destack_workspace::Module;

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
    fn is_reserved_identifier(&self, name: destack_source::StringId) -> bool {
        let name_str = self.program.strings.get(name);
        RESERVED_IDENTIFIERS.contains(&name_str.as_ref())
    }

    /// Check for reserved identifiers used as binding names.
    pub(super) fn validate_binding_names(&self, module: &Module) {
        let symbols = module.dir.symbols.read();
        for scope in symbols.scopes() {
            for (key, symbol_id) in scope.named_symbols.iter() {
                let symbol = symbols.get_symbol(*symbol_id);
                let Some(primary_declaration) = symbol.primary_declaration else {
                    continue;
                };
                if let StaticKey::Name(name) = key
                    && self.is_reserved_identifier(*name)
                {
                    self.error(BindError::ReservedIdentifier {
                        node: primary_declaration,
                        name: *name,
                    });
                }
            }
        }
    }

    /// Check for conflicting bindings in module scopes.
    pub(super) fn validate_binding_conflicts(&self, module: &Module) {
        let no_redeclare_locals = self
            .program
            .with_dsconfig_options(module, |opts| opts.compiler.no_redeclared_locals)
            .unwrap_or(false);

        // cross-check all named symbols in all scopes in the module
        let symbols = module.dir.symbols.read();
        for scope in symbols.scopes() {
            for (key, symbol_id) in scope.named_symbols.iter() {
                let symbol = symbols.get_symbol(*symbol_id);
                let Some(primary_declaration) = symbol.primary_declaration else {
                    continue;
                };
                for (other_key, other_symbol_id) in scope.named_symbols.iter() {
                    if *other_key != *key || *other_symbol_id == *symbol_id {
                        continue;
                    }
                    let other_symbol = symbols.get_symbol(*other_symbol_id);

                    // local conflicts are allowed unless configured otherwise (no_redeclared_locals)
                    if symbol.kind == SymbolKind::Local
                        && other_symbol.kind == SymbolKind::Local
                        && !no_redeclare_locals
                    {
                        continue;
                    }

                    // error on conflicting bindings
                    let Some(other_primary_declaration) = other_symbol.primary_declaration else {
                        continue;
                    };
                    let error = if symbol.export.is_some() && other_symbol.export.is_some() {
                        BindError::ConflictingExport {
                            node: primary_declaration,
                            other_node: other_primary_declaration,
                            module: module.id,
                            name: Some(*key),
                        }
                    } else {
                        BindError::ConflictingBinding {
                            node: primary_declaration,
                            other_node: other_primary_declaration,
                            scope: symbol.scope.0.into_global(module.id),
                            name: Some(*key),
                        }
                    };
                    self.error(error);
                }
            }
        }
    }
}

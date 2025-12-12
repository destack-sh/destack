use destack_dir::SymbolKind;
use destack_workspace::Module;

use crate::{BindError, Compiler};

impl Compiler {
    /// Check for conflicting item symbols in module scopes and report errors.
    pub(super) fn bind_validate_scopes(&self, module: &Module) {
        let no_redeclare_locals = self
            .program
            .with_dsconfig_options(module, |opts| opts.compiler.no_redeclared_locals)
            .unwrap_or(false);
        let symbols = module.dir.symbols.read();
        for scope in symbols.scopes() {
            for (key, symbol_id) in scope.named_symbols.iter() {
                let symbol = symbols.get_symbol(*symbol_id);
                let Some(primary_declaration) = symbol.primary_declaration else {
                    continue;
                };
                for (other_key, other_symbol_id) in scope.named_symbols.iter() {
                    if *other_key == *key && *other_symbol_id != *symbol_id {
                        let other_symbol = symbols.get_symbol(*other_symbol_id);

                        // local "conflicts" are allowed if configured
                        if symbol.kind == SymbolKind::Local
                            && other_symbol.kind == SymbolKind::Local
                            && !no_redeclare_locals
                        {
                            continue;
                        }

                        // conflicting binding, error appropriately
                        let Some(other_primary_declaration) = other_symbol.primary_declaration
                        else {
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
}

#[cfg(test)]
mod tests {
    use crate::TestProgram;

    /// By default (no dsconfig), redeclaring locals in `.ds` is allowed.
    #[test]
    fn test_redeclare_locals_allowed_by_default() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module(
            "test.ds",
            r#"
let x = 1;
let x = 2;
"#,
        );
        test.bind_module(module_id);
        test.compile();

        // should have no ConflictingBinding error
        test.check_no_diagnostic_code("EB004");
    }

    /// With dsconfig setting no_redeclared_locals: true, redeclaring locals is an error.
    #[test]
    fn test_redeclare_locals_forbidden_by_dsconfig() {
        let test = TestProgram::memory_sequential();
        test.add_file(
            "dsconfig.json",
            r#"{
                "compilerOptions": {
                    "noRedeclaredLocals": true
                }
            }"#,
        );
        test.add_file(
            "package.json",
            r#"{ "name": "test-pkg" }"#,
        );
        let module_id = test.register_module(
            "test.ds",
            r#"
let x = 1;
let x = 2;
"#,
        );
        test.bind_module(module_id);
        test.compile();

        test.check_has_diagnostic("EB004");
    }
}

use crate::Compiler;
use destack_dir::{
    FloatType, IntType, LocalScopeId, LocalScopeMark, LocalSymbolId, PrimitiveType, Scope,
    SymbolKind, SymbolTable, TypeLiteral,
};

/// Builtin type name resolution.
impl Compiler {
    /// Resolve the `Self` type by walking up the scope chain to find the enclosing type.
    /// Returns the symbol id of the enclosing class/struct/enum, or None if not inside a type.
    pub(super) fn resolve_self_type(
        &self,
        scope: (LocalScopeId, &Scope, LocalScopeMark),
        symbols: &SymbolTable,
    ) -> Option<LocalSymbolId> {
        let mut current_scope = scope;
        loop {
            // check if this scope has an owner that is an Item (class/struct/enum)
            if let Some(owner_id) = current_scope.1.owner_id {
                let owner_symbol = symbols.get_symbol(owner_id);
                // Item kind symbols are types (struct/class/enum), Local is for variables
                if owner_symbol.kind == SymbolKind::Item {
                    return Some(owner_id);
                }
            }
            // go to parent scope
            if let Some((parent_scope_id, parent_mark)) = current_scope.1.parent {
                current_scope = (
                    parent_scope_id,
                    symbols.get_scope_by_id(parent_scope_id),
                    parent_mark,
                );
            } else {
                break;
            }
        }
        None
    }

    /// Whether the token string encodes a type literal with an explicit width.
    fn is_type_with_width(&self, prefix: &'static str, target: &str) -> Option<u16> {
        if let Some(target) = target.strip_prefix(prefix) {
            target.parse::<u16>().ok()
        } else {
            None
        }
    }

    /// Resolve an identifier string to a builtin type literal.
    pub(super) fn resolve_string_to_type(&self, string: &str) -> Option<TypeLiteral> {
        match string {
            // undefined
            "undefined" => Some(TypeLiteral::Undefined),
            // unknown
            "unknown" => Some(TypeLiteral::Unknown),
            // void
            "void" => Some(TypeLiteral::Void),
            // null
            "null" => Some(TypeLiteral::Null),
            // any
            "any" => Some(TypeLiteral::Any),
            // never
            "never" => Some(TypeLiteral::Never),
            // boolean
            "boolean" => Some(TypeLiteral::Primitive(PrimitiveType::Boolean)),
            // character
            "character" => Some(TypeLiteral::Primitive(PrimitiveType::Character)),
            // string
            "string" => Some(TypeLiteral::Primitive(PrimitiveType::String)),
            // bigint
            "bigint" => Some(TypeLiteral::Primitive(PrimitiveType::Bigint)),
            // number
            "number" => Some(TypeLiteral::Primitive(PrimitiveType::Number)),
            // Self - handled specially in resolve_expression, not as a builtin type
            "Self" => None,
            // int (followed by number or nothing)
            "int" => Some(TypeLiteral::Primitive(PrimitiveType::Int(
                IntType::Arbitrary {
                    width: self.options.resolve.default_int_width,
                    is_signed: true,
                }
                .simplify(),
            ))),
            "intp" => Some(TypeLiteral::Primitive(PrimitiveType::Int(IntType::IntP))),
            int_str if let Some(width) = self.is_type_with_width("int", int_str) => {
                Some(TypeLiteral::Primitive(PrimitiveType::Int(
                    IntType::Arbitrary {
                        width,
                        is_signed: true,
                    }
                    .simplify(),
                )))
            }
            // uint (followed by number or nothing)
            "uint" => Some(TypeLiteral::Primitive(PrimitiveType::Int(
                IntType::Arbitrary {
                    width: self.options.resolve.default_int_width,
                    is_signed: false,
                }
                .simplify(),
            ))),
            "uintp" => Some(TypeLiteral::Primitive(PrimitiveType::Int(IntType::UintP))),
            uint_str if let Some(width) = self.is_type_with_width("uint", uint_str) => {
                Some(TypeLiteral::Primitive(PrimitiveType::Int(
                    IntType::Arbitrary {
                        width,
                        is_signed: false,
                    }
                    .simplify(),
                )))
            }
            uint_str if let Some(width) = self.is_type_with_width("u", uint_str) => {
                Some(TypeLiteral::Primitive(PrimitiveType::Int(
                    IntType::Arbitrary {
                        width,
                        is_signed: false,
                    }
                    .simplify(),
                )))
            }
            // float (followed by number or nothing)
            "float" => Some(TypeLiteral::Primitive(PrimitiveType::Float(
                FloatType::Arbitrary {
                    width: self.options.resolve.default_float_width,
                }
                .simplify(),
            ))),
            float_str if let Some(width) = self.is_type_with_width("float", float_str) => {
                Some(TypeLiteral::Primitive(PrimitiveType::Float(
                    FloatType::Arbitrary { width }.simplify(),
                )))
            }
            // composite type
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::{Expression, FloatType, IntType, PrimitiveType, TypeLiteral};
    use destack_source::DiagnosticSeverity;

    use crate::{TestProgram, assert_node};

    /// Test that builtin type names resolve to TypeLiteral expressions in value position.
    #[test]
    fn test_resolve_builtin_types_in_value_position() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module(
            "test.ds",
            r#"
int;
int32;
int64;
int68;
uint;
uint8;
uint16;
float;
float32;
float64;
boolean;
string;
"#,
        );
        test.resolve_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir.tree.read();
        let roots = &module.dir.roots;

        // int (defaults to int32)
        assert_node!(tree, roots[0], Expression::Statement { statement } => {
            assert_node!(tree, *statement, Expression::TypeLiteral { value } => {
                assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))));
            });
        });
        // int32
        assert_node!(tree, roots[1], Expression::Statement { statement } => {
            assert_node!(tree, *statement, Expression::TypeLiteral { value } => {
                assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))));
            });
        });
        // int64
        assert_node!(tree, roots[2], Expression::Statement { statement } => {
            assert_node!(tree, *statement, Expression::TypeLiteral { value } => {
                assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int64))));
            });
        });
        // int68 (arbitrary width)
        assert_node!(tree, roots[3], Expression::Statement { statement } => {
            assert_node!(tree, *statement, Expression::TypeLiteral { value } => {
                assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Int(IntType::Arbitrary { width: 68, is_signed: true }))));
            });
        });
        // uint (defaults to uint32)
        assert_node!(tree, roots[4], Expression::Statement { statement } => {
            assert_node!(tree, *statement, Expression::TypeLiteral { value } => {
                assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Int(IntType::Uint32))));
            });
        });
        // uint8
        assert_node!(tree, roots[5], Expression::Statement { statement } => {
            assert_node!(tree, *statement, Expression::TypeLiteral { value } => {
                assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Int(IntType::Uint8))));
            });
        });
        // uint16
        assert_node!(tree, roots[6], Expression::Statement { statement } => {
            assert_node!(tree, *statement, Expression::TypeLiteral { value } => {
                assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Int(IntType::Uint16))));
            });
        });
        // float (defaults to float64)
        assert_node!(tree, roots[7], Expression::Statement { statement } => {
            assert_node!(tree, *statement, Expression::TypeLiteral { value } => {
                assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Float(FloatType::Float64))));
            });
        });
        // float32
        assert_node!(tree, roots[8], Expression::Statement { statement } => {
            assert_node!(tree, *statement, Expression::TypeLiteral { value } => {
                assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Float(FloatType::Float32))));
            });
        });
        // float64
        assert_node!(tree, roots[9], Expression::Statement { statement } => {
            assert_node!(tree, *statement, Expression::TypeLiteral { value } => {
                assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Float(FloatType::Float64))));
            });
        });
        // boolean
        assert_node!(tree, roots[10], Expression::Statement { statement } => {
            assert_node!(tree, *statement, Expression::TypeLiteral { value } => {
                assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::Boolean)));
            });
        });
        // string
        assert_node!(tree, roots[11], Expression::Statement { statement } => {
            assert_node!(tree, *statement, Expression::TypeLiteral { value } => {
                assert!(matches!(value, TypeLiteral::Primitive(PrimitiveType::String)));
            });
        });
    }

    /// Test that a variable shadows a builtin type name.
    #[test]
    fn test_variable_shadows_builtin_type() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module(
            "test.ds",
            r#"
let string: string = "hello";
string;
"#,
        );
        test.resolve_module(module_id);
        test.compile_dump_clean();

        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir.tree.read();
        let roots = &module.dir.roots;

        let (string_symbol_id, _) = test
            .resolve_to_node::<destack_dir::Pattern>("test.ds", "string")
            .unwrap();
        // second root: `string;` should resolve to the variable, not the builtin
        assert_node!(tree, roots[1], Expression::Statement { statement } => {
            assert_node!(tree, *statement, Expression::ModuleReference { target_symbol, .. } => {
                assert_eq!(*target_symbol, string_symbol_id);
            });
        });
    }

    /// Test that Self resolves to the enclosing struct.
    #[test]
    fn test_self_type_in_struct() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module(
            "test.ds",
            r#"
struct Foo {
    create(): Self {
        return new Self()
    }
}
"#,
        );
        test.resolve_module(module_id);
        test.compile_dump_clean();

        // get the Foo symbol
        let foo_symbol_id = test.resolve_to_symbol("test.ds", "Foo").unwrap();

        // verify that Self in return type resolves to Foo
        let module = test.program.modules.get(module_id);
        let module = module.read();
        let tree = module.dir.tree.read();

        // find the Self reference in the function return type
        let mut found_self_reference = false;
        for expr_id in tree.iter_node_ids_of_type::<Expression>() {
            if let Expression::ModuleReference {
                target_symbol,
                path,
                ..
            } = tree.get(expr_id)
            {
                // check if this is a Self reference that points to Foo
                let first_segment = test.program.strings.get(path.segments[0]);
                if first_segment == "Self" && *target_symbol == foo_symbol_id {
                    found_self_reference = true;
                    break;
                }
            }
        }
        assert!(found_self_reference, "Self should resolve to Foo struct");
    }

    /// Test that Self outside a type context produces an error.
    #[test]
    fn test_self_type_outside_type_errors() {
        let test = TestProgram::memory_sequential();
        let module_id = test.register_module(
            "test.ds",
            r#"
let x: Self = 1;
"#,
        );
        test.resolve_module(module_id);
        test.compile();

        // check that we get an error
        let diagnostics = test.program.diagnostics.collect();
        let has_error = diagnostics
            .highest_severity()
            .map(|s| s >= DiagnosticSeverity::Error)
            .unwrap_or(false);
        assert!(
            has_error,
            "Self outside type context should produce an error"
        );
    }
}

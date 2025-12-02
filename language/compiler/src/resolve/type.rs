use crate::Compiler;
use destack_dir::{FloatType, IntType, PrimitiveType, TypeLiteral};

/// Builtin type name resolution.
impl Compiler {
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
            // Self
            "Self" => panic!("self type can't be resolved"),
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

    use crate::{ImportTask, TestProgram, assert_node};

    /// Test that builtin type names resolve to TypeLiteral expressions in value position.
    #[test]
    fn test_resolve_builtin_types_in_value_position() {
        let test = TestProgram::memory_sequential();
        let file = test.file(
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
        test.enqueue(ImportTask::ImportModuleFromFile { file: file.id });
        test.compile_dump_clean();

        let module = test.module_for_file(&file);
        let module = module.read();
        let tree = module.tree.read();
        let roots = &module.roots;

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
        let file = test.file(
            "test.ds",
            r#"
let string: string = "hello";
string;
"#,
        );
        test.enqueue(ImportTask::ImportModuleFromFile { file: file.id });
        test.compile_dump_clean();

        let module = test.module_for_file(&file);
        let module = module.read();
        let tree = module.tree.read();
        let roots = &module.roots;

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
}

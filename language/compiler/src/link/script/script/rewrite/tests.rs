use destack_codegen_js as js;
use destack_core::StringPool;
use destack_repository::Target;

use super::linker::Rewriter;

/// One small script module builder for minify rewrite tests.
struct TestModuleBuilder {
    /// The JS tree being built.
    tree: js::Tree,
    /// The string pool for the module.
    strings: StringPool,
}

impl TestModuleBuilder {
    /// Create one empty test module builder.
    fn new() -> Self {
        Self {
            tree: js::Tree::new(),
            strings: StringPool::new(),
        }
    }

    /// Finish into one script module.
    fn finish(self) -> js::Module {
        js::Module {
            tree: self.tree,
            roots: Vec::new(),
            strings: self.strings,
        }
    }

    /// Insert one expression.
    fn expression(&mut self, expression: js::Expression) -> js::LocalNodeId<js::Expression> {
        self.tree.insert_generated(expression)
    }

    /// Insert one path expression.
    fn path(&mut self, name: &str) -> js::LocalNodeId<js::Expression> {
        let name = self.strings.intern(name);

        self.expression(js::Expression::Path {
            path: js::Path {
                segments: smallvec::smallvec![name],
            },
            generic_arguments: vec![],
        })
    }

    /// Insert one binding pattern.
    fn binding_pattern(&mut self, name: &str) -> js::LocalNodeId<js::Pattern> {
        let name = self.strings.intern(name);

        self.tree.insert_generated(js::Pattern::Binding {
            mutability: Some(js::Mutability::Mutable),
            name,
        })
    }

    /// Insert one object pattern.
    fn object_pattern(&mut self) -> js::LocalNodeId<js::Pattern> {
        self.tree
            .insert_generated(js::Pattern::Object { fields: Vec::new() })
    }

    /// Insert one string literal expression.
    fn string(&mut self, value: &str) -> js::LocalNodeId<js::Expression> {
        let value = self.strings.intern(value);

        self.expression(js::Expression::ScalarLiteral {
            value: js::ScalarLiteral::String(value),
        })
    }

    /// Insert one undefined literal expression.
    fn undefined(&mut self) -> js::LocalNodeId<js::Expression> {
        self.expression(js::Expression::ScalarLiteral {
            value: js::ScalarLiteral::Undefined,
        })
    }

    /// Insert one null literal expression.
    fn null(&mut self) -> js::LocalNodeId<js::Expression> {
        self.expression(js::Expression::ScalarLiteral {
            value: js::ScalarLiteral::Null,
        })
    }

    /// Insert one unary expression.
    fn unary(
        &mut self,
        operator: js::UnaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> js::LocalNodeId<js::Expression> {
        self.expression(js::Expression::Unary { operator, right })
    }

    /// Insert one binary expression.
    fn binary(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> js::LocalNodeId<js::Expression> {
        self.expression(js::Expression::Binary {
            left,
            operator,
            right,
        })
    }

    /// Insert one member access expression.
    fn member(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        name: &str,
    ) -> js::LocalNodeId<js::Expression> {
        let name = self.strings.intern(name);

        self.expression(js::Expression::Member { left, name })
    }

    /// Insert one index expression.
    fn index(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        right: js::LocalNodeId<js::Expression>,
    ) -> js::LocalNodeId<js::Expression> {
        self.expression(js::Expression::Index {
            position: js::PostfixPosition::Direct,
            left,
            right,
        })
    }

    /// Insert one positional call expression.
    fn call(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        arguments: Vec<js::LocalNodeId<js::Expression>>,
    ) -> js::LocalNodeId<js::Expression> {
        let arguments = arguments
            .into_iter()
            .map(|value| {
                self.tree
                    .insert_generated(js::Argument::Positional { value })
            })
            .collect();

        self.expression(js::Expression::Call {
            position: js::PostfixPosition::Direct,
            left,
            generic_arguments: vec![],
            arguments: arguments,
        })
    }

    /// Insert one block.
    fn block(
        &mut self,
        statements: Vec<js::LocalNodeId<js::Statement>>,
    ) -> js::LocalNodeId<js::Block> {
        self.tree.insert_generated(js::Block { statements })
    }

    /// Insert one return statement.
    fn return_statement(
        &mut self,
        value: Option<js::LocalNodeId<js::Expression>>,
    ) -> js::LocalNodeId<js::Statement> {
        self.tree.insert_generated(js::Statement::Return { value })
    }

    /// Insert one let statement.
    fn let_statement(
        &mut self,
        pattern: js::LocalNodeId<js::Pattern>,
        value: Option<js::LocalNodeId<js::Expression>>,
    ) -> js::LocalNodeId<js::Statement> {
        let declarator = self.tree.insert_generated(js::Declarator {
            pattern,
            ty: None,
            value,
        });

        self.tree.insert_generated(js::Statement::Let {
            export: None,
            is_ambient: true,
            mutability: js::Mutability::Mutable,
            declarators: vec![declarator],
        })
    }

    /// Insert one function declaration.
    fn function_declaration(
        &mut self,
        asynchrony: js::Asynchrony,
        is_generator: bool,
        body: Option<js::LocalNodeId<js::Block>>,
    ) -> js::LocalNodeId<js::Declaration> {
        self.tree
            .insert_generated(js::Declaration::Function(js::FunctionDeclaration {
                name: None,
                export: None,
                is_ambient: true,
                is_abstract: false,
                signature: js::FunctionSignature {
                    asynchrony,
                    role: None,
                    form: js::FunctionForm::Function,
                    generic_parameters: Vec::new(),
                    this_parameter: None,
                    parameters: Vec::new(),
                    return_type: None,
                    is_abstract: false,
                    is_override: false,
                    is_generator,
                },
                body,
            }))
    }
}

/// One real rewriter harness for direct minify helper tests.
struct TestRewriter {
    /// The target policy used by the rewriter.
    target: destack_repository::Target,
    /// The module under test.
    module: js::Module,
}

impl TestRewriter {
    /// Create one test rewriter around one module.
    fn new(module: js::Module) -> Self {
        Self {
            target: Target::native(),
            module,
        }
    }

    /// Create one production rewriter over the current module.
    fn rewriter(&mut self) -> Rewriter<'_, '_> {
        Rewriter::new(&self.target, &mut self.module)
    }

    /// Return the rewritten module.
    fn module(&self) -> &js::Module {
        &self.module
    }

    /// Fold one strict or loose nullish comparison.
    fn fold_loose_nullish_comparison(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        self.rewriter()
            .fold_loose_nullish_comparison(left, operator, right)
    }

    /// Fold one `typeof x == "undefined"` comparison.
    fn fold_typeof_undefined_comparison(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        self.rewriter()
            .fold_typeof_undefined_comparison(left, operator, right)
    }

    /// Fold one nullish coalescing expression.
    fn fold_nullish_coalescing(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        self.rewriter().fold_nullish_coalescing(left, right)
    }

    /// Fold one strict comparison.
    fn fold_strict_comparison(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        self.rewriter()
            .fold_strict_comparison(left, operator, right)
    }

    /// Fold one literal comparison.
    fn fold_literal_comparison(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        self.rewriter()
            .fold_literal_comparison(left, operator, right)
    }

    /// Fold one sequence expression.
    fn fold_sequence_expression(
        &mut self,
        expressions: &[js::LocalNodeId<js::Expression>],
    ) -> Option<(js::Expression, Option<js::ScriptSymbolId>)> {
        self.rewriter().fold_sequence_expression(expressions)
    }

    /// Fold one boolean ternary.
    fn fold_boolean_ternary(
        &mut self,
        condition: js::LocalNodeId<js::Expression>,
        then_expression: js::LocalNodeId<js::Expression>,
        else_expression: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        self.rewriter()
            .fold_boolean_ternary(condition, then_expression, else_expression)
    }

    /// Fold one static unary expression.
    fn fold_static_unary_expression(
        &mut self,
        operator: js::UnaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        self.rewriter()
            .fold_static_unary_expression(operator, right)
    }

    /// Fold one associative binary chain.
    fn fold_associative_binary_chain(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        self.rewriter()
            .fold_associative_binary_chain(left, operator, right)
    }

    /// Fold one ES2020 ternary pattern.
    fn fold_es2020_ternary(
        &mut self,
        condition: js::LocalNodeId<js::Expression>,
        then_expression: js::LocalNodeId<js::Expression>,
        else_expression: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        self.rewriter()
            .fold_es2020_ternary(condition, then_expression, else_expression)
    }

    /// Fold one nullish disjunction.
    fn fold_nullish_disjunction(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        self.rewriter()
            .fold_nullish_disjunction(left, operator, right)
    }

    /// Fold one nullish conjunction.
    fn fold_nullish_conjunction(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        operator: js::BinaryOperator,
        right: js::LocalNodeId<js::Expression>,
    ) -> Option<js::Expression> {
        self.rewriter()
            .fold_nullish_conjunction(left, operator, right)
    }
}

/// Rewrite `value == undefined` to `value == null`.
#[test]
fn test_rewrites_loose_undefined_comparison_to_null() {
    let mut builder = TestModuleBuilder::new();
    let value = builder.path("value");
    let undefined = builder.undefined();
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_loose_nullish_comparison(value, js::BinaryOperator::Equal, undefined)
        .expect("expected loose nullish comparison rewrite");

    let js::Expression::Binary {
        left,
        operator,
        right,
    } = rewritten
    else {
        panic!("expected rewritten binary expression");
    };

    assert_eq!(left, value);
    assert_eq!(operator, js::BinaryOperator::Equal);
    assert!(Rewriter::is_null_expression(rewriter.module(), right));
}

/// Rewrite `typeof value === "undefined"` to `typeof value > "u"`.
#[test]
fn test_rewrites_strict_typeof_undefined_comparison_to_string_order() {
    let mut builder = TestModuleBuilder::new();
    let value = builder.path("value");
    let typeof_value = builder.unary(js::UnaryOperator::Typeof, value);
    let undefined = builder.string("undefined");
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_strict_comparison(typeof_value, js::BinaryOperator::EqualStrict, undefined)
        .expect("expected typeof rewrite");

    let js::Expression::Binary {
        left,
        operator,
        right,
    } = rewritten
    else {
        panic!("expected rewritten binary expression");
    };

    assert_eq!(left, typeof_value);
    assert_eq!(operator, js::BinaryOperator::GreaterThan);
    assert!(matches!(
        rewriter.module().tree.get(right),
        js::Expression::ScalarLiteral {
            value: js::ScalarLiteral::String(value),
        } if rewriter.module().strings.get(*value) == "u"
    ));
}

/// Rewrite `"undefined" === typeof value` to `typeof value > "u"`.
#[test]
fn test_rewrites_reversed_strict_typeof_undefined_comparison_to_string_order() {
    let mut builder = TestModuleBuilder::new();
    let value = builder.path("value");
    let typeof_value = builder.unary(js::UnaryOperator::Typeof, value);
    let undefined = builder.string("undefined");
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_strict_comparison(undefined, js::BinaryOperator::EqualStrict, typeof_value)
        .expect("expected reversed typeof rewrite");

    let js::Expression::Binary {
        left,
        operator,
        right,
    } = rewritten
    else {
        panic!("expected rewritten binary expression");
    };

    assert_eq!(left, typeof_value);
    assert_eq!(operator, js::BinaryOperator::GreaterThan);
    assert!(matches!(
        rewriter.module().tree.get(right),
        js::Expression::ScalarLiteral {
            value: js::ScalarLiteral::String(value),
        } if rewriter.module().strings.get(*value) == "u"
    ));
}

/// Rewrite `typeof value != "undefined"` to `typeof value < "u"`.
#[test]
fn test_rewrites_loose_typeof_undefined_inequality_to_string_order() {
    let mut builder = TestModuleBuilder::new();
    let value = builder.path("value");
    let typeof_value = builder.unary(js::UnaryOperator::Typeof, value);
    let undefined = builder.string("undefined");
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_typeof_undefined_comparison(typeof_value, js::BinaryOperator::NotEqual, undefined)
        .expect("expected typeof inequality rewrite");

    let js::Expression::Binary {
        left,
        operator,
        right,
    } = rewritten
    else {
        panic!("expected rewritten binary expression");
    };

    assert_eq!(left, typeof_value);
    assert_eq!(operator, js::BinaryOperator::LessThan);
    assert!(matches!(
        rewriter.module().tree.get(right),
        js::Expression::ScalarLiteral {
            value: js::ScalarLiteral::String(value),
        } if rewriter.module().strings.get(*value) == "u"
    ));
}

/// Rewrite `typeof value == "undefined"` to `typeof value > "u"`.
#[test]
fn test_rewrites_loose_typeof_undefined_equality_to_string_order() {
    let mut builder = TestModuleBuilder::new();
    let value = builder.path("value");
    let typeof_value = builder.unary(js::UnaryOperator::Typeof, value);
    let undefined = builder.string("undefined");
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_typeof_undefined_comparison(typeof_value, js::BinaryOperator::Equal, undefined)
        .expect("expected typeof equality rewrite");

    let js::Expression::Binary {
        left,
        operator,
        right,
    } = rewritten
    else {
        panic!("expected rewritten binary expression");
    };

    assert_eq!(left, typeof_value);
    assert_eq!(operator, js::BinaryOperator::GreaterThan);
    assert!(matches!(
        rewriter.module().tree.get(right),
        js::Expression::ScalarLiteral {
            value: js::ScalarLiteral::String(value),
        } if rewriter.module().strings.get(*value) == "u"
    ));
}

/// Rewrite `a === null || a === undefined` to `a == null`.
#[test]
fn test_rewrites_nullish_disjunction_to_loose_null() {
    let mut builder = TestModuleBuilder::new();
    let left_value = builder.path("value");
    let left_null = builder.null();
    let left_comparison = builder.binary(left_value, js::BinaryOperator::EqualStrict, left_null);
    let right_value = builder.path("value");
    let right_undefined = builder.undefined();
    let right_comparison = builder.binary(
        right_value,
        js::BinaryOperator::EqualStrict,
        right_undefined,
    );
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_nullish_disjunction(left_comparison, js::BinaryOperator::Or, right_comparison)
        .expect("expected nullish disjunction rewrite");

    let js::Expression::Binary {
        left,
        operator,
        right,
    } = rewritten
    else {
        panic!("expected rewritten binary expression");
    };

    assert_eq!(operator, js::BinaryOperator::Equal);
    assert!(Rewriter::same_expression_value(
        rewriter.module(),
        left,
        left_value
    ));
    assert!(Rewriter::is_null_expression(rewriter.module(), right));
}

/// Rewrite `a !== null && a !== undefined` to `a != null`.
#[test]
fn test_rewrites_nullish_conjunction_to_loose_null() {
    let mut builder = TestModuleBuilder::new();
    let left_value = builder.path("value");
    let left_null = builder.null();
    let left_comparison = builder.binary(left_value, js::BinaryOperator::NotEqualStrict, left_null);
    let right_value = builder.path("value");
    let right_undefined = builder.undefined();
    let right_comparison = builder.binary(
        right_value,
        js::BinaryOperator::NotEqualStrict,
        right_undefined,
    );
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_nullish_conjunction(left_comparison, js::BinaryOperator::And, right_comparison)
        .expect("expected nullish conjunction rewrite");

    let js::Expression::Binary {
        left,
        operator,
        right,
    } = rewritten
    else {
        panic!("expected rewritten binary expression");
    };

    assert_eq!(operator, js::BinaryOperator::NotEqual);
    assert!(Rewriter::same_expression_value(
        rewriter.module(),
        left,
        left_value
    ));
    assert!(Rewriter::is_null_expression(rewriter.module(), right));
}

/// Rewrite `undefined == value` to `value == null`.
#[test]
fn test_rewrites_reversed_loose_undefined_comparison_to_null() {
    let mut builder = TestModuleBuilder::new();
    let undefined = builder.undefined();
    let value = builder.path("value");
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_loose_nullish_comparison(undefined, js::BinaryOperator::Equal, value)
        .expect("expected reversed loose nullish comparison rewrite");

    let js::Expression::Binary {
        left,
        operator,
        right,
    } = rewritten
    else {
        panic!("expected rewritten binary expression");
    };

    assert_eq!(left, value);
    assert_eq!(operator, js::BinaryOperator::Equal);
    assert!(Rewriter::is_null_expression(rewriter.module(), right));
}

/// Simplify `null ?? fallback` to the fallback expression.
#[test]
fn test_rewrites_nullish_coalescing_with_null_left() {
    let mut builder = TestModuleBuilder::new();
    let left = builder.null();
    let right = builder.path("fallback");
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_nullish_coalescing(left, right)
        .expect("expected nullish coalescing rewrite");

    let js::Expression::Path { path, .. } = rewritten else {
        panic!("expected fallback path");
    };

    assert_eq!(rewriter.module().strings.get(path.segments[0]), "fallback");
}

/// Simplify `0 ?? fallback` to the left expression.
#[test]
fn test_rewrites_nullish_coalescing_with_non_nullish_left() {
    let mut builder = TestModuleBuilder::new();
    let left = builder.expression(js::Expression::ScalarLiteral {
        value: js::ScalarLiteral::Number(0.0),
    });
    let right = builder.path("fallback");
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_nullish_coalescing(left, right)
        .expect("expected nullish coalescing rewrite");

    let js::Expression::ScalarLiteral {
        value: js::ScalarLiteral::Number(value),
    } = rewritten
    else {
        panic!("expected number literal");
    };

    assert_eq!(value, 0.0);
}

/// Fold one primitive literal comparison to its boolean result.
#[test]
fn test_rewrites_literal_string_comparison_to_boolean() {
    let mut builder = TestModuleBuilder::new();
    let left = builder.string("a");
    let right = builder.string("b");
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_literal_comparison(left, js::BinaryOperator::LessThan, right)
        .expect("expected literal comparison rewrite");

    let js::Expression::ScalarLiteral {
        value: js::ScalarLiteral::Boolean(value),
    } = rewritten
    else {
        panic!("expected boolean literal");
    };

    assert!(value);
}

/// Fold `Infinity === Infinity` to `true`.
#[test]
fn test_rewrites_global_infinity_comparison_to_boolean() {
    let mut builder = TestModuleBuilder::new();
    let left = builder.path("Infinity");
    let right = builder.path("Infinity");
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_literal_comparison(left, js::BinaryOperator::EqualStrict, right)
        .expect("expected global infinity comparison rewrite");

    let js::Expression::ScalarLiteral {
        value: js::ScalarLiteral::Boolean(value),
    } = rewritten
    else {
        panic!("expected boolean literal");
    };

    assert!(value);
}

/// Fold `NaN === NaN` to `false`.
#[test]
fn test_rewrites_global_nan_comparison_to_boolean() {
    let mut builder = TestModuleBuilder::new();
    let left = builder.path("NaN");
    let right = builder.path("NaN");
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_literal_comparison(left, js::BinaryOperator::EqualStrict, right)
        .expect("expected global NaN comparison rewrite");

    let js::Expression::ScalarLiteral {
        value: js::ScalarLiteral::Boolean(value),
    } = rewritten
    else {
        panic!("expected boolean literal");
    };

    assert!(!value);
}

/// Simplify `undefined ?? fallback` to the fallback expression.
#[test]
fn test_rewrites_global_undefined_coalescing_to_fallback() {
    let mut builder = TestModuleBuilder::new();
    let left = builder.path("undefined");
    let right = builder.path("fallback");
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_nullish_coalescing(left, right)
        .expect("expected global undefined coalescing rewrite");

    let js::Expression::Path { path, .. } = rewritten else {
        panic!("expected fallback path");
    };

    assert_eq!(rewriter.module().strings.get(path.segments[0]), "fallback");
}

/// Drop removable comma prefixes and keep the live tail expression.
#[test]
fn test_rewrites_sequence_expression_prefix() {
    let mut builder = TestModuleBuilder::new();
    let zero = builder.expression(js::Expression::ScalarLiteral {
        value: js::ScalarLiteral::Number(0.0),
    });
    let string = builder.string("prefix");
    let value = builder.path("value");
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_sequence_expression(&[zero, string, value])
        .expect("expected sequence rewrite");

    let (rewritten_expression, rewritten_symbol_id) = rewritten;

    let js::Expression::Path { path, .. } = rewritten_expression else {
        panic!("expected live tail path");
    };

    assert_eq!(rewriter.module().strings.get(path.segments[0]), "value");
    assert_eq!(rewritten_symbol_id, rewriter.module().tree.symbol(value));
}

/// Rewrite `typeof value === "string"` to one loose string comparison.
#[test]
fn test_rewrites_strict_typeof_string_comparison_to_loose() {
    let mut builder = TestModuleBuilder::new();
    let value = builder.path("value");
    let typeof_value = builder.unary(js::UnaryOperator::Typeof, value);
    let string = builder.string("string");
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_strict_comparison(typeof_value, js::BinaryOperator::EqualStrict, string)
        .expect("expected typeof string rewrite");

    let js::Expression::Binary {
        left,
        operator,
        right,
    } = rewritten
    else {
        panic!("expected rewritten binary expression");
    };

    assert_eq!(left, typeof_value);
    assert_eq!(operator, js::BinaryOperator::Equal);
    assert_eq!(right, string);
}

/// Fold `typeof null === "object"` to `true`.
#[test]
fn test_rewrites_null_typeof_comparison_to_boolean() {
    let mut builder = TestModuleBuilder::new();
    let null = builder.null();
    let typeof_null = builder.unary(js::UnaryOperator::Typeof, null);
    let object = builder.string("object");
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_literal_comparison(typeof_null, js::BinaryOperator::EqualStrict, object)
        .expect("expected null typeof comparison rewrite");

    let js::Expression::ScalarLiteral {
        value: js::ScalarLiteral::Boolean(value),
    } = rewritten
    else {
        panic!("expected boolean literal");
    };

    assert!(value);
}

/// Rewrite `value == void 0` to `value == null`.
#[test]
fn test_rewrites_loose_void_zero_comparison_to_null() {
    let mut builder = TestModuleBuilder::new();
    let value = builder.path("value");
    let zero = builder.expression(js::Expression::ScalarLiteral {
        value: js::ScalarLiteral::Number(0.0),
    });
    let undefined = builder.unary(js::UnaryOperator::Void, zero);
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_loose_nullish_comparison(value, js::BinaryOperator::Equal, undefined)
        .expect("expected loose void comparison rewrite");

    let js::Expression::Binary {
        left,
        operator,
        right,
    } = rewritten
    else {
        panic!("expected rewritten binary expression");
    };

    assert_eq!(left, value);
    assert_eq!(operator, js::BinaryOperator::Equal);
    assert!(Rewriter::is_null_expression(rewriter.module(), right));
}

/// Rewrite one boolean ternary to one direct boolean expression.
#[test]
fn test_rewrites_boolean_ternary_to_not() {
    let mut builder = TestModuleBuilder::new();
    let left = builder.path("left");
    let right = builder.path("right");
    let condition = builder.binary(left, js::BinaryOperator::EqualStrict, right);
    let then_expression = builder.expression(js::Expression::ScalarLiteral {
        value: js::ScalarLiteral::Boolean(false),
    });
    let else_expression = builder.expression(js::Expression::ScalarLiteral {
        value: js::ScalarLiteral::Boolean(true),
    });
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_boolean_ternary(condition, then_expression, else_expression)
        .expect("expected boolean ternary rewrite");

    let js::Expression::Unary {
        operator: js::UnaryOperator::Not,
        right,
    } = rewritten
    else {
        panic!("expected unary not");
    };

    assert_eq!(right, condition);
}

/// Rewrite `void` over one removable literal to plain undefined.
#[test]
fn test_rewrites_void_literal_to_undefined() {
    let mut builder = TestModuleBuilder::new();
    let right = builder.expression(js::Expression::ScalarLiteral {
        value: js::ScalarLiteral::Number(1.0),
    });
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_static_unary_expression(js::UnaryOperator::Void, right)
        .expect("expected void literal rewrite");

    let js::Expression::ScalarLiteral {
        value: js::ScalarLiteral::Undefined,
    } = rewritten
    else {
        panic!("expected undefined literal");
    };
}

/// Rewrite `a || (b || c)` to one flatter chain.
#[test]
fn test_rewrites_right_nested_or_chain() {
    let mut builder = TestModuleBuilder::new();
    let a = builder.path("a");
    let b = builder.path("b");
    let c = builder.path("c");
    let right = builder.binary(b, js::BinaryOperator::Or, c);
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_associative_binary_chain(a, js::BinaryOperator::Or, right)
        .expect("expected right nested or rewrite");

    let js::Expression::Binary {
        left,
        operator,
        right,
    } = rewritten
    else {
        panic!("expected rewritten binary expression");
    };

    let js::Expression::Binary {
        left: left_left,
        operator: left_operator,
        right: left_right,
    } = rewriter.module().tree.get(left)
    else {
        panic!("expected nested left chain");
    };

    assert_eq!(operator, js::BinaryOperator::Or);
    assert_eq!(*left_left, a);
    assert_eq!(*left_right, b);
    assert_eq!(*left_operator, js::BinaryOperator::Or);
    assert_eq!(right, c);
}

/// Rewrite `a && (b && c)` to one flatter chain.
#[test]
fn test_rewrites_right_nested_and_chain() {
    let mut builder = TestModuleBuilder::new();
    let a = builder.path("a");
    let b = builder.path("b");
    let c = builder.path("c");
    let right = builder.binary(b, js::BinaryOperator::And, c);
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_associative_binary_chain(a, js::BinaryOperator::And, right)
        .expect("expected right nested and rewrite");

    let js::Expression::Binary {
        left,
        operator,
        right,
    } = rewritten
    else {
        panic!("expected rewritten binary expression");
    };

    let js::Expression::Binary {
        left: left_left,
        operator: left_operator,
        right: left_right,
    } = rewriter.module().tree.get(left)
    else {
        panic!("expected nested left chain");
    };

    assert_eq!(operator, js::BinaryOperator::And);
    assert_eq!(*left_left, a);
    assert_eq!(*left_right, b);
    assert_eq!(*left_operator, js::BinaryOperator::And);
    assert_eq!(right, c);
}

/// Rewrite `value != null ? value : fallback` to `value ?? fallback`.
#[test]
fn test_rewrites_es2020_nullish_coalescing_ternary() {
    let mut builder = TestModuleBuilder::new();
    let value = builder.path("value");
    let null = builder.null();
    let condition = builder.binary(value, js::BinaryOperator::NotEqual, null);
    let fallback = builder.path("fallback");
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_es2020_ternary(condition, value, fallback)
        .expect("expected es2020 nullish ternary rewrite");

    let js::Expression::Binary {
        left,
        operator,
        right,
    } = rewritten
    else {
        panic!("expected rewritten binary expression");
    };

    assert_eq!(left, value);
    assert_eq!(operator, js::BinaryOperator::Coalesce);
    assert_eq!(right, fallback);
}

/// Rewrite `value == null ? undefined : value.plain` to `value?.plain`.
#[test]
fn test_rewrites_es2020_optional_member_ternary() {
    let mut builder = TestModuleBuilder::new();
    let value = builder.path("value");
    let null = builder.null();
    let condition = builder.binary(value, js::BinaryOperator::Equal, null);
    let undefined = builder.undefined();
    let member = builder.member(value, "plain");
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_es2020_ternary(condition, undefined, member)
        .expect("expected es2020 optional member rewrite");

    let js::Expression::Member { left, name, .. } = rewritten else {
        panic!("expected rewritten member expression");
    };
    let js::Expression::Maybe { left: receiver, .. } = rewriter.module().tree.get(left) else {
        panic!("expected optional receiver");
    };

    assert_eq!(*receiver, value);
    assert_eq!(rewriter.module().strings.get(name), "plain");
}

/// Rewrite `value != null ? value[key] : undefined` to `value?.[key]`.
#[test]
fn test_rewrites_es2020_optional_index_ternary() {
    let mut builder = TestModuleBuilder::new();
    let value = builder.path("value");
    let null = builder.null();
    let condition = builder.binary(value, js::BinaryOperator::NotEqual, null);
    let key = builder.path("key");
    let index = builder.index(value, key);
    let undefined = builder.undefined();
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_es2020_ternary(condition, index, undefined)
        .expect("expected es2020 optional index rewrite");

    let js::Expression::Index {
        position,
        left,
        right,
    } = rewritten
    else {
        panic!("expected rewritten index expression");
    };
    let js::Expression::Maybe { left: receiver, .. } = rewriter.module().tree.get(left) else {
        panic!("expected optional receiver");
    };

    assert_eq!(position, js::PostfixPosition::Indirect);
    assert_eq!(*receiver, value);
    assert_eq!(right, key);
}

/// Rewrite `value != null ? value(arg) : undefined` to `value?.(arg)`.
#[test]
fn test_rewrites_es2020_optional_call_ternary() {
    let mut builder = TestModuleBuilder::new();
    let value = builder.path("value");
    let null = builder.null();
    let condition = builder.binary(value, js::BinaryOperator::NotEqual, null);
    let argument = builder.path("argument");
    let call = builder.call(value, vec![argument]);
    let undefined = builder.undefined();
    let mut rewriter = TestRewriter::new(builder.finish());

    let rewritten = rewriter
        .fold_es2020_ternary(condition, call, undefined)
        .expect("expected es2020 optional call rewrite");

    let js::Expression::Call {
        position,
        left,
        arguments,
        ..
    } = rewritten
    else {
        panic!("expected rewritten call expression");
    };
    let js::Expression::Maybe { left: receiver, .. } = rewriter.module().tree.get(left) else {
        panic!("expected optional receiver");
    };
    let js::Argument::Positional {
        value: rewritten_arg,
    } = rewriter.module().tree.get(arguments[0])
    else {
        panic!("expected positional argument");
    };

    assert_eq!(position, js::PostfixPosition::Indirect);
    assert_eq!(*receiver, value);
    assert_eq!(*rewritten_arg, argument);
}

/// Keep `return undefined` inside async generators.
#[test]
fn test_keeps_undefined_return_in_async_generator() {
    let mut builder = TestModuleBuilder::new();
    let undefined_value = builder.undefined();
    let statement = builder.return_statement(Some(undefined_value));
    let body = builder.block(vec![statement]);
    builder.function_declaration(js::Asynchrony::Async, true, Some(body));
    let mut module = builder.finish();

    Rewriter::elide_undefined_returns(&mut module);

    let js::Statement::Return { value } = module.tree.get(statement) else {
        panic!("expected return statement");
    };

    assert_eq!(*value, Some(undefined_value));
}

/// Elide `return undefined` inside plain functions.
#[test]
fn test_elides_undefined_return_in_plain_function() {
    let mut builder = TestModuleBuilder::new();
    let undefined_value = builder.undefined();
    let statement = builder.return_statement(Some(undefined_value));
    let body = builder.block(vec![statement]);
    builder.function_declaration(js::Asynchrony::Sync, false, Some(body));
    let mut module = builder.finish();

    Rewriter::elide_undefined_returns(&mut module);

    let js::Statement::Return { value } = module.tree.get(statement) else {
        panic!("expected return statement");
    };

    assert_eq!(*value, None);
}

/// Only elide undefined initializers for simple `let` bindings.
#[test]
fn test_keeps_undefined_initializer_for_destructuring_let() {
    let mut builder = TestModuleBuilder::new();
    let pattern = builder.object_pattern();
    let value = builder.undefined();
    let statement = builder.let_statement(pattern, Some(value));
    let mut module = builder.finish();

    Rewriter::elide_undefined_let_initializers(&mut module);

    let js::Statement::Let { declarators, .. } = module.tree.get(statement) else {
        panic!("expected let statement");
    };
    let declarator = module.tree.get(declarators[0]);

    assert_eq!(declarator.value, Some(value));
}

/// Elide undefined initializers for simple `let` bindings.
#[test]
fn test_elides_undefined_initializer_for_simple_let() {
    let mut builder = TestModuleBuilder::new();
    let pattern = builder.binding_pattern("value");
    let value = builder.undefined();
    let statement = builder.let_statement(pattern, Some(value));
    let mut module = builder.finish();

    Rewriter::elide_undefined_let_initializers(&mut module);

    let js::Statement::Let { declarators, .. } = module.tree.get(statement) else {
        panic!("expected let statement");
    };
    let declarator = module.tree.get(declarators[0]);

    assert_eq!(declarator.value, None);
}

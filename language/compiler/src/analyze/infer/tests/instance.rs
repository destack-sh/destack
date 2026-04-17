use super::*;
use destack_dir::{Instance, Key, LocalInstanceId, Member, Node, Resolution};

#[derive(Debug, Clone, PartialEq)]
struct ExpectedInstanceShape {
    generic_parameter_symbols: Vec<GlobalSymbolId>,
    inherited_static_argument_count: usize,
    generic_argument_primitives: Vec<PrimitiveType>,
}

impl TestModuleView<'_> {
    /// Resolve one declaration member symbol by owner and name.
    fn expect_member_symbol_for_owner(
        &self,
        owner_symbol: GlobalSymbolId,
        member_name: StringId,
    ) -> GlobalSymbolId {
        // resolve the owner declaration
        let owner_entry = self.symbols().get_symbol(owner_symbol.local_id);
        let declaration_id = owner_entry
            .primary_declaration
            .expect("expected owner declaration")
            .into_local_typed::<Declaration>();
        let declaration = self.tree().get(declaration_id);
        let members = declaration.member_ids().expect("expected owner members");

        // find the requested member
        for member_id in members {
            let member = self.tree().get(*member_id);
            let key_name = member.key().and_then(|key| match key {
                Key::Name(name) => Some(name.string()),
                Key::Private(_) => None,
                Key::Expression(_) => None,
            });
            if key_name.is_some_and(|name| name == member_name) {
                return member.symbol().into_global(self.module_id);
            }
        }

        panic!("expected member symbol");
    }

    /// Resolve one instance attached to one node.
    fn expect_instance_for_node<N: Node>(
        &self,
        node_id: LocalNodeId<N>,
    ) -> (GlobalSymbolId, Vec<StaticArgument>) {
        // load the attached instance
        let instance_id = self
            .types()
            .get_instance_for_node(node_id.into_global_any(self.module_id))
            .expect("expected instance for node");
        let instance = self.types().get_instance(instance_id);

        (instance.symbol_id, instance.generic_arguments.clone())
    }

    /// Resolve one committed instance id attached to one expression node.
    fn expect_instance_id_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalInstanceId {
        self.types()
            .get_instance_for_node(expression_id.into_global_any(self.module_id))
            .expect("expected instance id for expression")
    }

    /// Resolve declaration-ordered static parameter symbols for one declaration or member symbol.
    fn generic_parameter_symbols_for_symbol(&self, symbol: GlobalSymbolId) -> Vec<GlobalSymbolId> {
        for declaration_id in self.tree().iter_node_ids_of_type::<Declaration>() {
            let declaration = self.tree().get(declaration_id);
            let declaration_symbol = declaration.symbol().into_global(self.module_id);
            let declaration_parameters = declaration
                .generic_parameters()
                .unwrap_or_default()
                .iter()
                .map(|parameter_id| {
                    self.tree()
                        .get(*parameter_id)
                        .symbol()
                        .into_global(self.module_id)
                })
                .collect::<Vec<_>>();

            if declaration_symbol == symbol {
                return declaration_parameters;
            }

            let Some(member_ids) = declaration.member_ids() else {
                continue;
            };

            for member_id in member_ids {
                let member = self.tree().get(*member_id);
                let member_symbol = member.symbol().into_global(self.module_id);
                if member_symbol != symbol {
                    continue;
                }

                let member_parameters = match member {
                    Member::AssociatedType {
                        generic_parameters, ..
                    } => generic_parameters
                        .iter()
                        .map(|parameter_id| {
                            self.tree()
                                .get(*parameter_id)
                                .symbol()
                                .into_global(self.module_id)
                        })
                        .collect::<Vec<_>>(),
                    Member::Method { signature, .. } => signature
                        .generic_parameters
                        .iter()
                        .map(|parameter_id| {
                            self.tree()
                                .get(*parameter_id)
                                .symbol()
                                .into_global(self.module_id)
                        })
                        .collect::<Vec<_>>(),
                    _ => Vec::new(),
                };

                let mut parameters = declaration_parameters.clone();
                parameters.extend(member_parameters);
                return parameters;
            }
        }

        panic!("expected static parameter symbols for {symbol:?}")
    }

    /// Assert the full committed instance set for one symbol.
    fn assert_instances_for_symbol(
        &self,
        symbol: GlobalSymbolId,
        expected: &[ExpectedInstanceShape],
    ) {
        let actual = self
            .types()
            .iter_instances()
            .filter_map(|(_, instance)| {
                if instance.symbol_id != symbol {
                    return None;
                }

                let generic_argument_primitives = instance
                    .generic_arguments
                    .iter()
                    .map(|argument| self.primitive_for_static_argument(argument))
                    .collect::<Vec<_>>();
                Some(ExpectedInstanceShape {
                    generic_parameter_symbols: instance.generic_parameter_symbols.clone(),
                    inherited_static_argument_count: instance.inherited_static_argument_count,
                    generic_argument_primitives,
                })
            })
            .collect::<Vec<_>>();
        let mut remaining = actual.clone();

        assert_eq!(
            actual.len(),
            expected.len(),
            "unexpected number of instances for symbol {symbol:?}: {actual:#?}"
        );

        for expected_shape in expected {
            let found_index = remaining.iter().position(|actual| actual == expected_shape);
            if let Some(index) = found_index {
                remaining.swap_remove(index);
                continue;
            }

            panic!(
                "missing expected instance shape for {symbol:?}: {expected_shape:#?}, actual: {actual:#?}",
            );
        }
    }

    /// Assert one node has no committed instance.
    fn expect_no_instance_for_node(&self, node_id: GlobalNodeIdAny) {
        assert!(
            self.types().get_instance_for_node(node_id).is_none(),
            "expected no instance for node {node_id:?}"
        );
    }

    /// Resolve one static resolution target and its optional instance for one expression node.
    fn expect_static_resolution_target_and_instance(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> (GlobalSymbolId, Option<LocalInstanceId>) {
        let resolution_id = self
            .types()
            .get_resolution_for_node(expression_id.into_global_any(self.module_id))
            .expect("expected resolution for expression");
        let resolution = self.types().get_resolution(resolution_id);
        match resolution {
            Resolution::Static { candidate, .. } => (candidate.target_symbol, candidate.instance),
            other => panic!("expected static resolution, got {other:?}"),
        }
    }

    /// Resolve dynamic resolution targets and instances for one expression node.
    fn expect_dynamic_resolution_targets_and_instances(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Vec<(GlobalSymbolId, Option<LocalInstanceId>)> {
        let resolution_id = self
            .types()
            .get_resolution_for_node(expression_id.into_global_any(self.module_id))
            .expect("expected resolution for expression");
        let resolution = self.types().get_resolution(resolution_id);
        match resolution {
            Resolution::Dynamic { candidates, .. } => candidates
                .iter()
                .map(|candidate| (candidate.target_symbol, candidate.instance))
                .collect(),
            other => panic!("expected dynamic resolution, got {other:?}"),
        }
    }

    /// Extract one primitive type literal from one static argument.
    fn primitive_for_static_argument(&self, argument: &StaticArgument) -> PrimitiveType {
        let StaticArgument::Evaluated { value, .. } = argument else {
            panic!("expected evaluated static argument");
        };
        let StaticExpression::Type { ty } = value else {
            panic!("expected type static argument");
        };

        assert_type!(
            self.types(),
            *ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(expected_primitive)
            } => {
                *expected_primitive
            }
        )
    }

    /// Assert static arguments match one ordered primitive sequence.
    fn assert_static_argument_primitive_sequence(
        &self,
        arguments: &[StaticArgument],
        expected: &[PrimitiveType],
    ) {
        // check argument list width first
        assert_eq!(
            arguments.len(),
            expected.len(),
            "expected {} arguments, got {}: {arguments:?}",
            expected.len(),
            arguments.len(),
        );

        // assert each slot matches the expected primitive type
        let actual = arguments
            .iter()
            .map(|argument| self.primitive_for_static_argument(argument))
            .collect::<Vec<_>>();
        assert_eq!(actual.as_slice(), expected);
    }
}

/// Verify type alias annotations commit node instances for static arguments.
#[test]
fn test_instance_records_type_alias_annotation_instantiation() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
type Wrap<T> = T;

let value: Wrap<number> = 1;
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let declarator_id = test.expect_nth_let_declarator(module_id, 0);
    let declarator = view.tree().get(declarator_id);
    let type_expression_id = declarator.ty.expect("expected annotation");

    // Wrap<number>
    let wrap_symbol = test
        .resolve_to_symbol("test.ds", "Wrap")
        .expect("expected Wrap symbol");
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(type_expression_id);
    // Wrap
    assert_eq!(instance_symbol, wrap_symbol);
    // <number>
    view.assert_static_argument_primitive_sequence(&generic_arguments, &[PrimitiveType::Number]);
}

/// Verify annotation instances attach to the type-expression node and not declaration nodes.
#[test]
fn test_instance_annotation_attaches_to_type_expression_node_only() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Box<T> {
    value: T
}

declare let value: Box<number>;
"#,
    );

    // analyze
    test.analyze_module(module_id);
    test.compile();
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = view.expect_let_declarator(value_name);
    let declarator = view.tree().get(declarator_id);
    let type_expression_id = declarator.ty.expect("expected annotation");

    // Box<number>
    let box_symbol = test
        .resolve_to_symbol("test.ds", "Box")
        .expect("expected Box symbol");
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(type_expression_id);
    // Box
    assert_eq!(instance_symbol, box_symbol);
    // <number>
    view.assert_static_argument_primitive_sequence(&generic_arguments, &[PrimitiveType::Number]);

    // declare let value: Box<number>
    view.expect_no_instance_for_node(declarator_id.into_global(module_id).into_any());

    // value
    view.expect_no_instance_for_node(declarator.pattern.into_global(module_id).into_any());
}

/// Verify interface annotations commit node instances for static arguments.
#[test]
fn test_instance_records_interface_annotation_instantiation() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Container<T> {
    value: T
}

declare let value: Container<string>;
"#,
    );

    // analyze without the clean assertion so we can inspect the published state
    test.analyze_module(module_id);
    test.compile();
    let view = test.view(module_id);

    // read
    let declarator_id = test.expect_nth_let_declarator(module_id, 0);
    let declarator = view.tree().get(declarator_id);
    let type_expression_id = declarator.ty.expect("expected annotation");

    // Container<string>
    let container_symbol = test
        .resolve_to_symbol("test.ds", "Container")
        .expect("expected Container symbol");
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(type_expression_id);
    // Container
    assert_eq!(instance_symbol, container_symbol);
    // <string>
    view.assert_static_argument_primitive_sequence(&generic_arguments, &[PrimitiveType::String]);
}

/// Verify struct annotations commit node instances and keep non-generic calls instance-free.
#[test]
fn test_instance_records_struct_annotation_instantiation_only() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Box<T> {
    value: T
}

declare function makeBox(): Box<number>;

let boxed: Box<number> = makeBox();
"#,
    );

    // analyze
    test.analyze_module(module_id);
    test.compile();
    let view = test.view(module_id);

    // read
    let declarator_id = test.expect_nth_let_declarator(module_id, 0);
    let declarator = view.tree().get(declarator_id);
    let type_expression_id = declarator.ty.expect("expected annotation");
    let value_id = declarator.value.expect("expected initializer");

    // Box<number>
    let box_symbol = test
        .resolve_to_symbol("test.ds", "Box")
        .expect("expected Box symbol");
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(type_expression_id);
    // Box
    assert_eq!(instance_symbol, box_symbol);
    // <number>
    view.assert_static_argument_primitive_sequence(&generic_arguments, &[PrimitiveType::Number]);

    // makeBox()
    assert!(
        view.types()
            .get_instance_for_node(value_id.into_global_any(module_id))
            .is_none()
    );
}

/// Verify new expressions on generic classes commit class instances.
#[test]
fn test_instance_records_class_constructor_instantiation_on_new_expression() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Box<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

let boxed = new Box<string>("hi");
"#,
    );

    // analyze
    test.analyze_module(module_id);
    test.compile();
    let view = test.view(module_id);

    // read
    let declarator_id = test.expect_nth_let_declarator(module_id, 0);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    // new Box<string>("hi")
    let box_symbol = test
        .resolve_to_symbol("test.ds", "Box")
        .expect("expected Box symbol");
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(value_id);

    // Box
    assert_eq!(instance_symbol, box_symbol);

    // <string>
    view.assert_static_argument_primitive_sequence(&generic_arguments, &[PrimitiveType::String]);
}

/// Keep class constructor shapes available after implicit-any field diagnostics.
#[test]
fn test_instance_keeps_constructor_shape_for_implicit_any_field() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", Some(r#""noImplicitAny": true"#));
    let module_id = test.add_module(
        "test.ds",
        r#"
class Box {
    value;
}

let boxed = new Box(1);
"#,
    );

    // analyze with the expected implicit-any diagnostic
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostic("EA107");

    // keep the constructor shape available for the class value type
    let view = test.view(module_id);
    let box_symbol = test.canonical_symbol_for_path("test.ds", "Box");
    let box_value_id = view.expect_value_type_id(box_symbol);
    let mut shape = ObjectShape::default();
    let mut extras = Vec::new();
    let mut visited = Vec::new();
    test.compiler.collect_value_shape_from_type(
        box_value_id,
        view.types(),
        &mut shape,
        &mut extras,
        &mut visited,
    );

    assert_eq!(shape.construct_signatures.len(), 1);
}

/// Verify class member calls commit method instances with inherited and explicit arguments.
#[test]
fn test_instance_records_class_method_instantiation_with_inherited_arguments() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare class Box<T> {
    map<U>(value: T): U;
}

declare let box: Box<number>;

let value = box.map<string>(1);
"#,
    );

    // analyze
    test.analyze_module(module_id);
    test.compile();
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = view.expect_let_declarator(value_name);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    let box_symbol = test
        .resolve_to_symbol("test.ds", "Box")
        .expect("expected Box symbol");
    let map_name = test.program.strings.intern("map");
    let map_symbol = view.expect_member_symbol_for_owner(box_symbol, map_name);

    // box.map<string>(1)
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(value_id);

    // Box.map
    assert_eq!(instance_symbol, map_symbol);

    // <number, string>
    view.assert_static_argument_primitive_sequence(
        &generic_arguments,
        &[PrimitiveType::Number, PrimitiveType::String],
    );
}

/// Verify class member calls commit method instances with inherited and inferred arguments.
#[test]
fn test_instance_records_class_method_instantiation_with_inferred_arguments() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare class Box<T> {
    map<U>(value: U): U;
}

declare let box: Box<number>;
declare let text: string;

let value = box.map(text);
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = view.expect_let_declarator(value_name);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    let box_symbol = test
        .resolve_to_symbol("test.ds", "Box")
        .expect("expected Box symbol");
    let map_name = test.program.strings.intern("map");
    let map_symbol = view.expect_member_symbol_for_owner(box_symbol, map_name);

    // box.map(text)
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(value_id);

    // Box.map
    assert_eq!(instance_symbol, map_symbol);

    // <number, string>
    view.assert_static_argument_primitive_sequence(
        &generic_arguments,
        &[PrimitiveType::Number, PrimitiveType::String],
    );

    // Box.map
    // Box<number>.map<string>
    view.assert_instances_for_symbol(
        map_symbol,
        &[ExpectedInstanceShape {
            generic_parameter_symbols: view.generic_parameter_symbols_for_symbol(map_symbol),
            inherited_static_argument_count: 1,
            generic_argument_primitives: vec![PrimitiveType::Number, PrimitiveType::String],
        }],
    );
}

/// Verify flow narrowing feeds inferred member static arguments at call sites.
#[test]
fn test_instance_records_class_method_instantiation_with_flow_narrowed_argument() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare class Box<T> {
    map<U>(value: U): U;
}

declare let box: Box<number>;
declare let valueOrCount: string | number;

let narrowed = typeof valueOrCount == "string" ? valueOrCount : "fallback";
let value = box.map(narrowed);
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = view.expect_let_declarator(value_name);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    let box_symbol = test
        .resolve_to_symbol("test.ds", "Box")
        .expect("expected Box symbol");
    let map_name = test.program.strings.intern("map");
    let map_symbol = view.expect_member_symbol_for_owner(box_symbol, map_name);

    // box.map(valueOrCount)
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(value_id);

    // Box.map
    assert_eq!(instance_symbol, map_symbol);

    // <number, string>
    view.assert_static_argument_primitive_sequence(
        &generic_arguments,
        &[PrimitiveType::Number, PrimitiveType::String],
    );
}

/// Verify generic function calls commit instances with inferred arguments.
#[test]
fn test_instance_records_function_call_instantiation_with_inferred_arguments() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare function identity<T>(value: T): T;

declare let text: string;

let value = identity(text);
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = view.expect_let_declarator(value_name);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    let identity_symbol = test
        .resolve_to_symbol("test.ds", "identity")
        .expect("expected identity symbol");

    // identity(text)
    let instance_id = view.expect_instance_id_for_expression(value_id);
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(value_id);

    // identity
    assert_eq!(instance_symbol, identity_symbol);

    // <string>
    view.assert_static_argument_primitive_sequence(&generic_arguments, &[PrimitiveType::String]);
    assert_eq!(view.types().instance_count(), 1);

    // identity
    // identity<string>
    view.assert_instances_for_symbol(
        identity_symbol,
        &[ExpectedInstanceShape {
            generic_parameter_symbols: view.generic_parameter_symbols_for_symbol(identity_symbol),
            inherited_static_argument_count: 0,
            generic_argument_primitives: vec![PrimitiveType::String],
        }],
    );

    // identity(text) resolution
    let (resolution_symbol, resolution_instance_id) =
        view.expect_static_resolution_target_and_instance(value_id);
    assert_eq!(resolution_symbol, identity_symbol);
    assert_eq!(resolution_instance_id, Some(instance_id));

    // identity
    let Expression::Call { left, .. } = view.tree().get(value_id) else {
        panic!("expected call expression");
    };
    view.expect_no_instance_for_node(left.into_global_any(module_id));
}

/// Verify generic callback instantiation preserves const tuple literal return precision.
#[test]
fn test_instance_records_generic_callback_const_tuple_literal_return_precision() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare function mapOne<T, U>(value: T, callback: (input: T) => U): U;

const tuple = [1, 2] as const;
let head = mapOne(tuple, input => input[0]);
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let head_name = test.program.strings.intern("head");
    let declarator_id = view.expect_let_declarator(head_name);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    // mapOne(tuple, input => input[0])
    let map_one_symbol = test
        .resolve_to_symbol("test.ds", "mapOne")
        .expect("expected mapOne symbol");
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(value_id);
    assert_eq!(instance_symbol, map_one_symbol);
    assert_eq!(generic_arguments.len(), 2);

    // mapOne<T, U>
    let StaticArgument::Evaluated {
        value: StaticExpression::Type { ty: t_ty_id },
        ..
    } = generic_arguments[0]
    else {
        panic!("expected evaluated type argument for T");
    };
    let StaticArgument::Evaluated {
        value: StaticExpression::Type { ty: u_ty_id },
        ..
    } = generic_arguments[1]
    else {
        panic!("expected evaluated type argument for U");
    };

    // T
    assert_type!(
        view.types(),
        t_ty_id,
        Type::Tuple {
            elements,
            is_readonly: true
        } => {
            assert_eq!(elements.len(), 2, "expected tuple arity for T");
            assert_type!(
                view.types(),
                elements[0].ty,
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1))
                }
            );
            assert_type!(
                view.types(),
                elements[1].ty,
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(2))
                }
            );
        }
    );

    // U
    assert_type!(
        view.types(),
        u_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1))
        }
    );
}

/// Verify flow narrowing feeds inferred function static arguments at call sites.
#[test]
fn test_instance_records_function_call_instantiation_with_flow_narrowed_argument() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare function identity<T>(value: T): T;
declare let valueOrCount: string | number;

let narrowed = typeof valueOrCount == "string" ? valueOrCount : "fallback";
let value = identity(narrowed);
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = view.expect_let_declarator(value_name);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    // identity(valueOrCount)
    let identity_symbol = test
        .resolve_to_symbol("test.ds", "identity")
        .expect("expected identity symbol");
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(value_id);

    // identity
    assert_eq!(instance_symbol, identity_symbol);

    // <string>
    view.assert_static_argument_primitive_sequence(&generic_arguments, &[PrimitiveType::String]);
}

/// Verify non-generic calls commit no instance and keep resolution instance empty.
#[test]
fn test_instance_non_generic_call_commits_no_instance() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare function plain(value: number): number;

let value = plain(1);
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = view.expect_let_declarator(value_name);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    // plain(1)
    view.expect_no_instance_for_node(value_id.into_global_any(module_id));

    // plain(1) resolution
    let plain_symbol = test
        .resolve_to_symbol("test.ds", "plain")
        .expect("expected plain symbol");
    let (resolution_symbol, resolution_instance_id) =
        view.expect_static_resolution_target_and_instance(value_id);
    assert_eq!(resolution_symbol, plain_symbol);
    assert!(resolution_instance_id.is_none());
}

/// Verify interface member calls commit method instances with inherited and explicit arguments.
#[test]
fn test_instance_records_interface_method_instantiation_with_inherited_arguments() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare interface Container<T> {
    map<U>(value: T): U;
}

declare let container: Container<number>;

let value = container.map<string>(1);
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = view.expect_let_declarator(value_name);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    let container_symbol = test
        .resolve_to_symbol("test.ds", "Container")
        .expect("expected Container symbol");
    let map_name = test.program.strings.intern("map");
    let map_symbol = view.expect_member_symbol_for_owner(container_symbol, map_name);

    // container.map<string>(1)
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(value_id);

    // Container.map
    assert_eq!(instance_symbol, map_symbol);

    // <number, string>
    view.assert_static_argument_primitive_sequence(
        &generic_arguments,
        &[PrimitiveType::Number, PrimitiveType::String],
    );
}

/// Verify interface member calls commit method instances with inherited and inferred arguments.
#[test]
fn test_instance_records_interface_method_instantiation_with_inferred_arguments() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare interface Container<T> {
    map<U>(value: U): U;
}

declare let container: Container<number>;
declare let text: string;

let value = container.map(text);
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = view.expect_let_declarator(value_name);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    let container_symbol = test
        .resolve_to_symbol("test.ds", "Container")
        .expect("expected Container symbol");
    let map_name = test.program.strings.intern("map");
    let map_symbol = view.expect_member_symbol_for_owner(container_symbol, map_name);

    // container.map(text)
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(value_id);

    // Container.map
    assert_eq!(instance_symbol, map_symbol);

    // <number, string>
    view.assert_static_argument_primitive_sequence(
        &generic_arguments,
        &[PrimitiveType::Number, PrimitiveType::String],
    );
}

/// Verify inherited class member calls commit base-member instances with inherited arguments.
#[test]
fn test_instance_records_inherited_class_method_instantiation_from_base_member() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare class Base<T> {
    map<U>(): U;
}

declare class Derived<T> extends Base<T> {}

declare let derived: Derived<number>;

let value = derived.map<string>();
"#,
    );

    // analyze
    test.analyze_module(module_id);
    test.compile();
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = view.expect_let_declarator(value_name);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    let base_symbol = test
        .resolve_to_symbol("test.ds", "Base")
        .expect("expected Base symbol");
    let map_name = test.program.strings.intern("map");
    let base_map_symbol = view.expect_member_symbol_for_owner(base_symbol, map_name);

    // derived.map<string>()
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(value_id);

    // Base.map
    assert_eq!(instance_symbol, base_map_symbol);

    // <number, string>
    view.assert_static_argument_primitive_sequence(
        &generic_arguments,
        &[PrimitiveType::Number, PrimitiveType::String],
    );
}

/// Verify inherited class member calls commit base-member instances with inherited and inferred arguments.
#[test]
fn test_instance_records_inherited_class_method_instantiation_with_inferred_arguments() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare class Base<T> {
    map<U>(value: U): U;
}

declare class Derived<T> extends Base<T> {}

declare let derived: Derived<number>;
declare let text: string;

let value = derived.map(text);
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = view.expect_let_declarator(value_name);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    let base_symbol = test
        .resolve_to_symbol("test.ds", "Base")
        .expect("expected Base symbol");
    let map_name = test.program.strings.intern("map");
    let base_map_symbol = view.expect_member_symbol_for_owner(base_symbol, map_name);

    // derived.map(text)
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(value_id);

    // Base.map
    assert_eq!(instance_symbol, base_map_symbol);

    // <number, string>
    view.assert_static_argument_primitive_sequence(
        &generic_arguments,
        &[PrimitiveType::Number, PrimitiveType::String],
    );
}

/// Verify the committed instance set is exact for repeated and distinct member instantiations.
#[test]
fn test_instance_set_is_exact_for_repeated_and_distinct_member_instantiations() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare class Box<T> {
    map<U>(value: U): U;
}

declare let boxString: Box<string>;
declare let boxNumber: Box<number>;
declare let flag: boolean;

let first = boxString.map<boolean>(flag);
let second = boxString.map(flag);
let third = boxNumber.map(flag);
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let box_symbol = test
        .resolve_to_symbol("test.ds", "Box")
        .expect("expected Box symbol");
    let map_name = test.program.strings.intern("map");
    let map_symbol = view.expect_member_symbol_for_owner(box_symbol, map_name);

    let box_string_name = test.program.strings.intern("boxString");
    let box_number_name = test.program.strings.intern("boxNumber");
    let box_string_declarator_id = view.expect_let_declarator(box_string_name);
    let box_number_declarator_id = view.expect_let_declarator(box_number_name);
    let box_string_declarator = view.tree().get(box_string_declarator_id);
    let box_number_declarator = view.tree().get(box_number_declarator_id);
    let box_string_ty = box_string_declarator
        .ty
        .expect("expected boxString annotation");
    let box_number_ty = box_number_declarator
        .ty
        .expect("expected boxNumber annotation");

    // Box<string>
    let (box_string_instance_symbol, box_string_arguments) =
        view.expect_instance_for_node(box_string_ty);

    // Box<number>
    let (box_number_instance_symbol, box_number_arguments) =
        view.expect_instance_for_node(box_number_ty);

    // Box
    assert_eq!(box_string_instance_symbol, box_symbol);
    assert_eq!(box_number_instance_symbol, box_symbol);

    // <string>
    view.assert_static_argument_primitive_sequence(&box_string_arguments, &[PrimitiveType::String]);

    // <number>
    view.assert_static_argument_primitive_sequence(&box_number_arguments, &[PrimitiveType::Number]);

    let first_name = test.program.strings.intern("first");
    let second_name = test.program.strings.intern("second");
    let third_name = test.program.strings.intern("third");
    let first_declarator = view.tree().get(view.expect_let_declarator(first_name));
    let second_declarator = view.tree().get(view.expect_let_declarator(second_name));
    let third_declarator = view.tree().get(view.expect_let_declarator(third_name));
    let first_value_id = first_declarator.value.expect("expected first initializer");
    let second_value_id = second_declarator
        .value
        .expect("expected second initializer");
    let third_value_id = third_declarator.value.expect("expected third initializer");

    let (first_instance_symbol, first_arguments) = view.expect_instance_for_node(first_value_id);
    let (second_instance_symbol, second_arguments) = view.expect_instance_for_node(second_value_id);
    let (third_instance_symbol, third_arguments) = view.expect_instance_for_node(third_value_id);

    // Box.map
    assert_eq!(first_instance_symbol, map_symbol);
    assert_eq!(second_instance_symbol, map_symbol);
    assert_eq!(third_instance_symbol, map_symbol);

    // boxString.map<boolean>(flag): Box<string>.map<boolean>
    view.assert_static_argument_primitive_sequence(
        &first_arguments,
        &[PrimitiveType::String, PrimitiveType::Boolean],
    );

    // boxString.map(flag): Box<string>.map<boolean>
    view.assert_static_argument_primitive_sequence(
        &second_arguments,
        &[PrimitiveType::String, PrimitiveType::Boolean],
    );

    // boxNumber.map(flag): Box<number>.map<boolean>
    view.assert_static_argument_primitive_sequence(
        &third_arguments,
        &[PrimitiveType::Number, PrimitiveType::Boolean],
    );

    // first, second, third
    let first_instance_id = view
        .types()
        .get_instance_for_node(first_value_id.into_global_any(module_id))
        .expect("expected first instance id");
    let second_instance_id = view
        .types()
        .get_instance_for_node(second_value_id.into_global_any(module_id))
        .expect("expected second instance id");
    let third_instance_id = view
        .types()
        .get_instance_for_node(third_value_id.into_global_any(module_id))
        .expect("expected third instance id");

    // boxString.map<boolean>(flag), boxString.map(flag): Box<string>.map<boolean>
    assert_eq!(first_instance_id, second_instance_id);
    assert_ne!(first_instance_id, third_instance_id);
    assert_ne!(second_instance_id, third_instance_id);

    // instances
    let expected_unique_instance_ids = HashSet::from([
        first_instance_id,
        second_instance_id,
        third_instance_id,
        view.types()
            .get_instance_for_node(box_string_ty.into_global_any(module_id))
            .expect("expected boxString instance id"),
        view.types()
            .get_instance_for_node(box_number_ty.into_global_any(module_id))
            .expect("expected boxNumber instance id"),
    ]);
    assert_eq!(
        view.types().instance_count(),
        expected_unique_instance_ids.len()
    );
}

/// Verify cross-module inherited class member calls commit base-member instances.
#[test]
fn test_instance_records_cross_module_inherited_class_method_instantiation_from_base_member() {
    let test = TestProgram::memory_sequential();
    let lib_id = test.add_module(
        "lib.ds",
        r#"
export declare class Base<T> {
    map<U>(): U;
}

export declare class Derived<T> extends Base<T> {}
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import { Derived } from "./lib.ds";

declare let derived: Derived<number>;

let value = derived.map<string>();
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(main_id);
    let main_view = test.view(main_id);
    let lib_view = test.declared_view(lib_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = main_view.expect_let_declarator(value_name);
    let declarator = main_view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    let base_symbol = test
        .resolve_to_symbol("lib.ds", "Base")
        .expect("expected Base symbol");
    let map_name = test.program.strings.intern("map");
    let base_map_symbol = lib_view.expect_member_symbol_for_owner(base_symbol, map_name);

    // derived.map<string>()
    let (instance_symbol, generic_arguments) = main_view.expect_instance_for_node(value_id);

    // Base.map
    assert_eq!(instance_symbol, base_map_symbol);

    // <number, string>
    main_view.assert_static_argument_primitive_sequence(
        &generic_arguments,
        &[PrimitiveType::Number, PrimitiveType::String],
    );
}

/// Verify re-exported inherited class member calls still commit base-member instances.
#[test]
fn test_instance_records_reexported_cross_module_inherited_class_method_instantiation() {
    let test = TestProgram::memory_sequential();
    let lib_id = test.add_module(
        "lib.ds",
        r#"
export declare class Base<T> {
    map<U>(value: U): [T, U];
}
"#,
    );
    test.add_module(
        "relay.ds",
        r#"
import { Base } from "./lib.ds";

export class Derived<T> extends Base<T> {}
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import { Derived } from "./relay.ds";

declare let derived: Derived<number>;
declare let text: string;

let value = derived.map(text);
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(main_id);
    let main_view = test.view(main_id);
    let lib_view = test.view(lib_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = main_view.expect_let_declarator(value_name);
    let declarator = main_view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    let base_symbol = test
        .resolve_to_symbol("lib.ds", "Base")
        .expect("expected Base symbol");
    let map_name = test.program.strings.intern("map");
    let base_map_symbol = lib_view.expect_member_symbol_for_owner(base_symbol, map_name);

    // derived.map(text)
    let (instance_symbol, generic_arguments) = main_view.expect_instance_for_node(value_id);

    // Base.map
    assert_eq!(instance_symbol, base_map_symbol);

    // <number, string>
    main_view.assert_static_argument_primitive_sequence(
        &generic_arguments,
        &[PrimitiveType::Number, PrimitiveType::String],
    );
}

/// Verify imported generic function calls commit instances in the consumer module.
#[test]
fn test_instance_records_cross_module_function_call_instantiation_with_inferred_arguments() {
    let test = TestProgram::memory_sequential();
    let lib_id = test.add_module(
        "lib.ds",
        r#"
export declare function identity<T>(value: T): T;
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import { identity } from "./lib.ds";

declare let text: string;

let value = identity(text);
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(main_id);
    let main_view = test.view(main_id);
    let lib_view = test.view(lib_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = main_view.expect_let_declarator(value_name);
    let declarator = main_view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    let identity_symbol = test
        .resolve_to_symbol("lib.ds", "identity")
        .expect("expected identity symbol");

    // identity(text)
    let (instance_symbol, generic_arguments) = main_view.expect_instance_for_node(value_id);
    // identity
    assert_eq!(instance_symbol, identity_symbol);
    // <string>
    main_view
        .assert_static_argument_primitive_sequence(&generic_arguments, &[PrimitiveType::String]);

    // identity(text) resolution
    let (resolution_symbol, resolution_instance_id) =
        main_view.expect_static_resolution_target_and_instance(value_id);
    assert_eq!(resolution_symbol, identity_symbol);
    assert!(resolution_instance_id.is_some());

    // identity<string>
    let committed = main_view
        .types()
        .iter_instances()
        .filter(|(_, instance)| instance.symbol_id == identity_symbol)
        .collect::<Vec<_>>();
    assert_eq!(
        committed.len(),
        1,
        "expected one committed identity instance"
    );
    let committed_instance = committed[0].1;
    assert_eq!(committed_instance.inherited_static_argument_count, 0);
    main_view.assert_static_argument_primitive_sequence(
        &committed_instance.generic_arguments,
        &[PrimitiveType::String],
    );

    // lib module declaration-only analysis should not commit consumer-call instances
    assert_eq!(lib_view.types().instance_count(), 0);
}

/// Verify equal imported instantiations commit independently per consumer module.
#[test]
fn test_instance_commits_are_module_local_for_same_imported_symbol_instantiation() {
    let test = TestProgram::memory_sequential();
    let lib_id = test.add_module(
        "lib.ds",
        r#"
export declare function identity<T>(value: T): T;
"#,
    );
    let main_a_id = test.add_module(
        "main_a.ds",
        r#"
import { identity } from "./lib.ds";

declare let text: string;

let value = identity(text);
"#,
    );
    let main_b_id = test.add_module(
        "main_b.ds",
        r#"
import { identity } from "./lib.ds";

declare let text: string;

let value = identity(text);
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(main_a_id);
    test.analyze_module_and_check_clean(main_b_id);
    let lib_view = test.view(lib_id);
    let main_a_view = test.view(main_a_id);
    let main_b_view = test.view(main_b_id);

    // read
    let identity_symbol = test
        .resolve_to_symbol("lib.ds", "identity")
        .expect("expected identity symbol");
    let value_name = test.program.strings.intern("value");
    let value_a = main_a_view
        .tree()
        .get(main_a_view.expect_let_declarator(value_name))
        .value
        .expect("expected main_a value initializer");
    let value_b = main_b_view
        .tree()
        .get(main_b_view.expect_let_declarator(value_name))
        .value
        .expect("expected main_b value initializer");

    // main_a identity(text)
    let (instance_symbol_a, arguments_a) = main_a_view.expect_instance_for_node(value_a);
    assert_eq!(instance_symbol_a, identity_symbol);
    main_a_view.assert_static_argument_primitive_sequence(&arguments_a, &[PrimitiveType::String]);

    // main_b identity(text)
    let (instance_symbol_b, arguments_b) = main_b_view.expect_instance_for_node(value_b);
    assert_eq!(instance_symbol_b, identity_symbol);
    main_b_view.assert_static_argument_primitive_sequence(&arguments_b, &[PrimitiveType::String]);

    // per-module committed instance sets
    let committed_a = main_a_view
        .types()
        .iter_instances()
        .filter(|(_, instance)| instance.symbol_id == identity_symbol)
        .count();
    let committed_b = main_b_view
        .types()
        .iter_instances()
        .filter(|(_, instance)| instance.symbol_id == identity_symbol)
        .count();
    assert_eq!(committed_a, 1, "expected one identity instance in main_a");
    assert_eq!(committed_b, 1, "expected one identity instance in main_b");

    // lib module remains declaration-only for this scenario
    assert_eq!(lib_view.types().instance_count(), 0);
}

/// Verify inherited interface member calls commit base-member instances with inherited arguments.
#[test]
fn test_instance_records_inherited_interface_method_instantiation_from_base_member() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare interface Base<T> {
    map<U>(): U;
}

declare interface Derived<T> extends Base<T> {}

declare let derived: Derived<number>;

let value = derived.map<string>();
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = view.expect_let_declarator(value_name);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    let base_symbol = test
        .resolve_to_symbol("test.ds", "Base")
        .expect("expected Base symbol");
    let map_name = test.program.strings.intern("map");
    let base_map_symbol = view.expect_member_symbol_for_owner(base_symbol, map_name);

    // derived.map<string>()
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(value_id);

    // Base.map
    assert_eq!(instance_symbol, base_map_symbol);

    // <number, string>
    view.assert_static_argument_primitive_sequence(
        &generic_arguments,
        &[PrimitiveType::Number, PrimitiveType::String],
    );
}

/// Verify inherited member instantiation expressions preserve explicit arguments across calls.
#[test]
fn test_instance_reuses_inherited_member_instantiation_arguments_for_call() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare class Base<T> {
    map<U>(): U;
}

declare class Derived<T> extends Base<T> {}

declare let derived: Derived<number>;

let value = (derived.map<string>)();
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = view.expect_let_declarator(value_name);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    let base_symbol = test
        .resolve_to_symbol("test.ds", "Base")
        .expect("expected Base symbol");
    let map_name = test.program.strings.intern("map");
    let base_map_symbol = view.expect_member_symbol_for_owner(base_symbol, map_name);

    // (derived.map<string>)()
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(value_id);

    // Base.map
    assert_eq!(instance_symbol, base_map_symbol);

    // <number, string>
    view.assert_static_argument_primitive_sequence(
        &generic_arguments,
        &[PrimitiveType::Number, PrimitiveType::String],
    );
}

/// Verify extension member instances follow extension parameter order then member signature order.
#[test]
fn test_instance_records_extension_member_instantiation_with_target_reordering() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Pair<A, B> {
    left: A;
    right: B;
}

extension PairOps<Left, Right> for Pair<Right, Left> {
    map<U>(value: U): U {
        return value;
    }
}

declare let pair: Pair<number, string>;

let value = pair.map<boolean>(true);
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = view.expect_let_declarator(value_name);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    let extension_symbol = test
        .resolve_to_symbol("test.ds", "PairOps")
        .expect("expected PairOps symbol");
    let map_name = test.program.strings.intern("map");
    let map_symbol = view.expect_member_symbol_for_owner(extension_symbol, map_name);

    // pair.map<boolean>(true)
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(value_id);

    // PairOps.map
    assert_eq!(instance_symbol, map_symbol);

    // <string, number, boolean>
    view.assert_static_argument_primitive_sequence(
        &generic_arguments,
        &[
            PrimitiveType::String,
            PrimitiveType::Number,
            PrimitiveType::Boolean,
        ],
    );

    // PairOps.map
    // PairOps<string, number>.map<boolean>
    view.assert_instances_for_symbol(
        map_symbol,
        &[ExpectedInstanceShape {
            generic_parameter_symbols: view.generic_parameter_symbols_for_symbol(map_symbol),
            inherited_static_argument_count: 2,
            generic_argument_primitives: vec![
                PrimitiveType::String,
                PrimitiveType::Number,
                PrimitiveType::Boolean,
            ],
        }],
    );
}

/// Verify union member calls commit one instance per dynamic target member.
#[test]
fn test_instance_records_union_member_call_instances_for_each_dynamic_target() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare interface Left<T> {
    map<U>(): [T, U];
}

declare interface Right<T> {
    map<U>(): [T, U];
}

declare let cond: boolean;
declare let left: Left<string>;
declare let right: Right<number>;

let both: Left<string> | Right<number> = cond ? left : right;
let value = both.map<boolean>();
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("value");
    let declarator_id = view.expect_let_declarator(value_name);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    let left_symbol = test
        .resolve_to_symbol("test.ds", "Left")
        .expect("expected Left symbol");
    let right_symbol = test
        .resolve_to_symbol("test.ds", "Right")
        .expect("expected Right symbol");
    let map_name = test.program.strings.intern("map");
    let left_map_symbol = view.expect_member_symbol_for_owner(left_symbol, map_name);
    let right_map_symbol = view.expect_member_symbol_for_owner(right_symbol, map_name);

    // both.map<boolean>()
    let candidates = view.expect_dynamic_resolution_targets_and_instances(value_id);
    assert_eq!(candidates.len(), 2, "expected two dynamic candidates");

    // Left.map
    let left_candidate_instance_id = candidates
        .iter()
        .find_map(|(target_symbol, instance_id)| {
            (*target_symbol == left_map_symbol)
                .then(|| instance_id.expect("expected Left.map candidate instance"))
        })
        .expect("expected Left.map candidate");

    // Right.map
    let right_candidate_instance_id = candidates
        .iter()
        .find_map(|(target_symbol, instance_id)| {
            (*target_symbol == right_map_symbol)
                .then(|| instance_id.expect("expected Right.map candidate instance"))
        })
        .expect("expected Right.map candidate");

    // Left<string>.map<boolean>
    let left_candidate_instance = view.types().get_instance(left_candidate_instance_id);
    view.assert_static_argument_primitive_sequence(
        &left_candidate_instance.generic_arguments,
        &[PrimitiveType::String, PrimitiveType::Boolean],
    );

    // Right<number>.map<boolean>
    let right_candidate_instance = view.types().get_instance(right_candidate_instance_id);
    view.assert_static_argument_primitive_sequence(
        &right_candidate_instance.generic_arguments,
        &[PrimitiveType::Number, PrimitiveType::Boolean],
    );

    // Left.map
    // Left<string>.map<boolean>
    view.assert_instances_for_symbol(
        left_map_symbol,
        &[ExpectedInstanceShape {
            generic_parameter_symbols: view.generic_parameter_symbols_for_symbol(left_map_symbol),
            inherited_static_argument_count: 1,
            generic_argument_primitives: vec![PrimitiveType::String, PrimitiveType::Boolean],
        }],
    );

    // Right.map
    // Right<number>.map<boolean>
    view.assert_instances_for_symbol(
        right_map_symbol,
        &[ExpectedInstanceShape {
            generic_parameter_symbols: view.generic_parameter_symbols_for_symbol(right_map_symbol),
            inherited_static_argument_count: 1,
            generic_argument_primitives: vec![PrimitiveType::Number, PrimitiveType::Boolean],
        }],
    );
}

/// Verify imported function instantiations dedupe to one canonical instance in one consumer module.
#[test]
fn test_instance_dedupes_imported_function_instantiations_in_consumer_module() {
    let test = TestProgram::memory_sequential();
    let lib_id = test.add_module(
        "lib.ds",
        r#"
export declare function identity<T>(value: T): T;
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import { identity } from "./lib.ds";

declare let text: string;

let explicit = identity<string>(text);
let inferred = identity(text);
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(main_id);
    let main_view = test.view(main_id);
    let lib_view = test.view(lib_id);

    // read
    let explicit_name = test.program.strings.intern("explicit");
    let inferred_name = test.program.strings.intern("inferred");
    let explicit_id = main_view
        .tree()
        .get(main_view.expect_let_declarator(explicit_name))
        .value
        .expect("expected explicit initializer");
    let inferred_id = main_view
        .tree()
        .get(main_view.expect_let_declarator(inferred_name))
        .value
        .expect("expected inferred initializer");
    let identity_symbol = test
        .resolve_to_symbol("lib.ds", "identity")
        .expect("expected identity symbol");

    // identity<string>(text), identity(text)
    let explicit_instance_id = main_view.expect_instance_id_for_expression(explicit_id);
    let inferred_instance_id = main_view.expect_instance_id_for_expression(inferred_id);

    // identity<string>
    assert_eq!(explicit_instance_id, inferred_instance_id);
    main_view.assert_instances_for_symbol(
        identity_symbol,
        &[ExpectedInstanceShape {
            generic_parameter_symbols: lib_view
                .generic_parameter_symbols_for_symbol(identity_symbol),
            inherited_static_argument_count: 0,
            generic_argument_primitives: vec![PrimitiveType::String],
        }],
    );
}

/// Verify unspecialized generic function references do not commit instances.
#[test]
fn test_instance_skips_unsolved_function_reference_instance_commit() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare function identity<T>(value: T): T;

let fn_ref = identity;
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("fn_ref");
    let value_id = view
        .tree()
        .get(view.expect_let_declarator(value_name))
        .value
        .expect("expected value initializer");

    // identity
    view.expect_no_instance_for_node(value_id.into_global_any(module_id));
    assert_eq!(view.types().instance_count(), 0);
}

/// Verify member references with unsolved method arguments do not commit instances.
#[test]
fn test_instance_skips_unsolved_member_reference_instance_commit() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare interface Container<T> {
    map<U>(value: U): U;
}

declare let container: Container<number>;

let mapper = container.map;
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("mapper");
    let declarator_id = view.expect_let_declarator(value_name);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected mapper initializer");

    // container.map
    view.expect_no_instance_for_node(value_id.into_global_any(module_id));
    let (_, resolution_instance_id) = view.expect_static_resolution_target_and_instance(value_id);
    assert!(resolution_instance_id.is_none());
}

/// Verify associated alias projection substitution preserves inherited method instance arguments.
#[test]
fn test_instance_records_associated_projection_member_instantiation() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Box<T> {
    type Item = T;

    map<U>(value: this.Item, next: U): U {
        return next;
    }
}

declare let box: Box<number>;

let value = box.map<boolean>(1, true);
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let value_name = test.program.strings.intern("value");
    let value_id = view
        .tree()
        .get(view.expect_let_declarator(value_name))
        .value
        .expect("expected value initializer");
    let box_symbol = test
        .resolve_to_symbol("test.ds", "Box")
        .expect("expected Box symbol");
    let map_name = test.program.strings.intern("map");
    let map_symbol = view.expect_member_symbol_for_owner(box_symbol, map_name);

    // box.map<boolean>(1, true)
    let (instance_symbol, generic_arguments) = view.expect_instance_for_node(value_id);

    // Box.map
    assert_eq!(instance_symbol, map_symbol);

    // Box<number>.map<boolean>
    view.assert_static_argument_primitive_sequence(
        &generic_arguments,
        &[PrimitiveType::Number, PrimitiveType::Boolean],
    );
}

/// Verify dynamic-resolution candidate attachments remap to canonical committed instances.
#[test]
fn test_instance_keeps_dynamic_candidate_instances_canonical_after_finalize() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare interface Left<T> {
    map<U>(value: U): [T, U];
}

declare interface Right<T> {
    map<U>(value: U): [T, U];
}

declare let cond: boolean;
declare let left: Left<string>;
declare let right: Right<number>;
declare let flag: boolean;

let both: Left<string> | Right<number> = cond ? left : right;
let first = both.map<boolean>(flag);
let second = both.map(flag);
"#,
    );

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let left_symbol = test
        .resolve_to_symbol("test.ds", "Left")
        .expect("expected Left symbol");
    let right_symbol = test
        .resolve_to_symbol("test.ds", "Right")
        .expect("expected Right symbol");
    let map_name = test.program.strings.intern("map");
    let left_map_symbol = view.expect_member_symbol_for_owner(left_symbol, map_name);
    let right_map_symbol = view.expect_member_symbol_for_owner(right_symbol, map_name);
    let first_name = test.program.strings.intern("first");
    let second_name = test.program.strings.intern("second");
    let first_id = view
        .tree()
        .get(view.expect_let_declarator(first_name))
        .value
        .expect("expected first initializer");
    let second_id = view
        .tree()
        .get(view.expect_let_declarator(second_name))
        .value
        .expect("expected second initializer");

    let first_candidates = view.expect_dynamic_resolution_targets_and_instances(first_id);
    let second_candidates = view.expect_dynamic_resolution_targets_and_instances(second_id);
    let first_instance_by_symbol = first_candidates
        .into_iter()
        .map(|(symbol, instance_id)| {
            (
                symbol,
                instance_id.expect("expected first dynamic candidate instance"),
            )
        })
        .collect::<HashMap<_, _>>();
    let second_instance_by_symbol = second_candidates
        .into_iter()
        .map(|(symbol, instance_id)| {
            (
                symbol,
                instance_id.expect("expected second dynamic candidate instance"),
            )
        })
        .collect::<HashMap<_, _>>();

    // Left<string>.map<boolean>
    assert_eq!(
        first_instance_by_symbol.get(&left_map_symbol),
        second_instance_by_symbol.get(&left_map_symbol),
    );

    // Right<number>.map<boolean>
    assert_eq!(
        first_instance_by_symbol.get(&right_map_symbol),
        second_instance_by_symbol.get(&right_map_symbol),
    );

    // Left.map
    // Left<string>.map<boolean>
    view.assert_instances_for_symbol(
        left_map_symbol,
        &[ExpectedInstanceShape {
            generic_parameter_symbols: view.generic_parameter_symbols_for_symbol(left_map_symbol),
            inherited_static_argument_count: 1,
            generic_argument_primitives: vec![PrimitiveType::String, PrimitiveType::Boolean],
        }],
    );

    // Right.map
    // Right<number>.map<boolean>
    view.assert_instances_for_symbol(
        right_map_symbol,
        &[ExpectedInstanceShape {
            generic_parameter_symbols: view.generic_parameter_symbols_for_symbol(right_map_symbol),
            inherited_static_argument_count: 1,
            generic_argument_primitives: vec![PrimitiveType::Number, PrimitiveType::Boolean],
        }],
    );
}

/// Verify repeated analysis produces an identical committed instance table.
#[test]
fn test_instance_commit_table_is_idempotent_across_reanalysis() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
declare class Box<T> {
    map<U>(value: U): U;
}

declare let box: Box<number>;
declare let flag: boolean;

let first = box.map<boolean>(flag);
let second = box.map(flag);
"#,
    );

    // analyze
    test.analyze_module(module_id);
    test.compile();
    let first_view = test.view(module_id);
    // read
    let mut first_snapshot = first_view
        .types()
        .iter_instances()
        .map(|(instance_id, instance)| (instance_id, instance.clone()))
        .collect::<Vec<(LocalInstanceId, Instance)>>();
    first_snapshot.sort_by_key(|(instance_id, _)| *instance_id);
    // analyze
    test.analyze_module_and_check_clean(module_id);
    let second_view = test.view(module_id);

    // read
    let mut second_snapshot = second_view
        .types()
        .iter_instances()
        .map(|(instance_id, instance)| (instance_id, instance.clone()))
        .collect::<Vec<(LocalInstanceId, Instance)>>();
    second_snapshot.sort_by_key(|(instance_id, _)| *instance_id);

    assert_eq!(first_snapshot, second_snapshot);
}

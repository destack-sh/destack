use super::*;
use destack_dir::{DynamicKey, Resolution};

#[derive(Debug, Clone, PartialEq)]
struct ExpectedInstanceShape {
    static_parameter_symbols: Vec<GlobalSymbolId>,
    inherited_static_argument_count: usize,
    static_argument_primitives: Vec<PrimitiveType>,
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
                DynamicKey::Name(name) => Some(name),
                DynamicKey::Number(name) => Some(name),
                DynamicKey::Private(_) => None,
                DynamicKey::Expression(_) | DynamicKey::NamedExpression { .. } => None,
            });
            if key_name.is_some_and(|name| *name == member_name) {
                return member.symbol().into_global(self.module_id);
            }
        }

        panic!("expected member symbol");
    }

    /// Resolve one instance fact attached to one expression node.
    fn expect_instance_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> (GlobalSymbolId, Vec<StaticArgument>) {
        // load the attached instance
        let instance_id = self
            .types()
            .get_instance_for_node(expression_id.into_global_any(self.module_id))
            .expect("expected instance for expression");
        let instance = self.types().get_instance(instance_id);

        (instance.symbol_id, instance.static_arguments.clone())
    }

    /// Resolve one committed instance id attached to one expression node.
    fn expect_instance_id_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> destack_dir::LocalInstanceId {
        self.types()
            .get_instance_for_node(expression_id.into_global_any(self.module_id))
            .expect("expected instance id for expression")
    }

    /// Resolve declaration-ordered static parameter symbols for one declaration or member symbol.
    fn static_parameter_symbols_for_symbol(&self, symbol: GlobalSymbolId) -> Vec<GlobalSymbolId> {
        for declaration_id in self.tree().iter_node_ids_of_type::<Declaration>() {
            let declaration = self.tree().get(declaration_id);
            let declaration_symbol = declaration.symbol().into_global(self.module_id);
            let declaration_parameters = declaration
                .static_parameters()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|parameter_id| {
                    self.tree()
                        .get(parameter_id)
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
                    destack_dir::Member::Type {
                        static_parameters, ..
                    } => static_parameters
                        .clone()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|parameter_id| {
                            self.tree()
                                .get(parameter_id)
                                .symbol()
                                .into_global(self.module_id)
                        })
                        .collect::<Vec<_>>(),
                    destack_dir::Member::Method { signature, .. } => signature
                        .generics
                        .as_ref()
                        .and_then(|generics| generics.static_parameters.as_ref())
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|parameter_id| {
                            self.tree()
                                .get(parameter_id)
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

        Vec::new()
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

                let static_argument_primitives = instance
                    .static_arguments
                    .iter()
                    .map(|argument| self.primitive_for_static_argument(argument))
                    .collect::<Vec<_>>();
                Some(ExpectedInstanceShape {
                    static_parameter_symbols: instance.static_parameter_symbols.clone(),
                    inherited_static_argument_count: instance.inherited_static_argument_count,
                    static_argument_primitives,
                })
            })
            .collect::<Vec<_>>();

        assert_eq!(
            actual.len(),
            expected.len(),
            "unexpected number of instances for symbol {symbol:?}: {actual:#?}"
        );

        for expected_shape in expected {
            assert!(
                actual.contains(expected_shape),
                "missing expected instance shape for {symbol:?}: {expected_shape:#?}, actual: {actual:#?}",
            );
        }
    }

    /// Assert one node has no committed instance fact.
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
    ) -> (GlobalSymbolId, Option<destack_dir::LocalInstanceId>) {
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
    let (instance_symbol, static_arguments) =
        view.expect_instance_for_expression(type_expression_id);
    // Wrap
    assert_eq!(instance_symbol, wrap_symbol);
    // <number>
    view.assert_static_argument_primitive_sequence(&static_arguments, &[PrimitiveType::Number]);
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
    test.analyze_module_and_check_clean(module_id);
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
    let (instance_symbol, static_arguments) =
        view.expect_instance_for_expression(type_expression_id);
    // Box
    assert_eq!(instance_symbol, box_symbol);
    // <number>
    view.assert_static_argument_primitive_sequence(&static_arguments, &[PrimitiveType::Number]);

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

    // analyze
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let declarator_id = test.expect_nth_let_declarator(module_id, 0);
    let declarator = view.tree().get(declarator_id);
    let type_expression_id = declarator.ty.expect("expected annotation");

    // Container<string>
    let container_symbol = test
        .resolve_to_symbol("test.ds", "Container")
        .expect("expected Container symbol");
    let (instance_symbol, static_arguments) =
        view.expect_instance_for_expression(type_expression_id);
    // Container
    assert_eq!(instance_symbol, container_symbol);
    // <string>
    view.assert_static_argument_primitive_sequence(&static_arguments, &[PrimitiveType::String]);
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
    test.analyze_module_and_check_clean(module_id);
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
    let (instance_symbol, static_arguments) =
        view.expect_instance_for_expression(type_expression_id);
    // Box
    assert_eq!(instance_symbol, box_symbol);
    // <number>
    view.assert_static_argument_primitive_sequence(&static_arguments, &[PrimitiveType::Number]);

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
    test.analyze_module_and_check_clean(module_id);
    let view = test.view(module_id);

    // read
    let declarator_id = test.expect_nth_let_declarator(module_id, 0);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer");

    // new Box<string>("hi")
    let box_symbol = test
        .resolve_to_symbol("test.ds", "Box")
        .expect("expected Box symbol");
    let (instance_symbol, static_arguments) = view.expect_instance_for_expression(value_id);

    // Box
    assert_eq!(instance_symbol, box_symbol);

    // <string>
    view.assert_static_argument_primitive_sequence(&static_arguments, &[PrimitiveType::String]);
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

    // box.map<string>(1)
    let (instance_symbol, static_arguments) = view.expect_instance_for_expression(value_id);

    // Box.map
    assert_eq!(instance_symbol, map_symbol);

    // <number, string>
    view.assert_static_argument_primitive_sequence(
        &static_arguments,
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
    let (instance_symbol, static_arguments) = view.expect_instance_for_expression(value_id);

    // Box.map
    assert_eq!(instance_symbol, map_symbol);

    // <number, string>
    view.assert_static_argument_primitive_sequence(
        &static_arguments,
        &[PrimitiveType::Number, PrimitiveType::String],
    );

    // Box.map
    // Box.map<number, string>
    view.assert_instances_for_symbol(
        map_symbol,
        &[ExpectedInstanceShape {
            static_parameter_symbols: view.static_parameter_symbols_for_symbol(map_symbol),
            inherited_static_argument_count: 1,
            static_argument_primitives: vec![PrimitiveType::Number, PrimitiveType::String],
        }],
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
    let (instance_symbol, static_arguments) = view.expect_instance_for_expression(value_id);

    // identity
    assert_eq!(instance_symbol, identity_symbol);

    // <string>
    view.assert_static_argument_primitive_sequence(&static_arguments, &[PrimitiveType::String]);
    assert_eq!(view.types().instance_count(), 1);

    // identity
    // identity<string>
    view.assert_instances_for_symbol(
        identity_symbol,
        &[ExpectedInstanceShape {
            static_parameter_symbols: view.static_parameter_symbols_for_symbol(identity_symbol),
            inherited_static_argument_count: 0,
            static_argument_primitives: vec![PrimitiveType::String],
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

/// Verify non-generic calls commit no instance and keep resolution instance empty.
#[test]
fn test_instance_non_generic_call_has_no_instance_fact() {
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
    let (instance_symbol, static_arguments) = view.expect_instance_for_expression(value_id);

    // Container.map
    assert_eq!(instance_symbol, map_symbol);

    // <number, string>
    view.assert_static_argument_primitive_sequence(
        &static_arguments,
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
    let (instance_symbol, static_arguments) = view.expect_instance_for_expression(value_id);

    // Container.map
    assert_eq!(instance_symbol, map_symbol);

    // <number, string>
    view.assert_static_argument_primitive_sequence(
        &static_arguments,
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
    let (instance_symbol, static_arguments) = view.expect_instance_for_expression(value_id);

    // Base.map
    assert_eq!(instance_symbol, base_map_symbol);

    // <number, string>
    view.assert_static_argument_primitive_sequence(
        &static_arguments,
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
    let (instance_symbol, static_arguments) = view.expect_instance_for_expression(value_id);

    // Base.map
    assert_eq!(instance_symbol, base_map_symbol);

    // <number, string>
    view.assert_static_argument_primitive_sequence(
        &static_arguments,
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
        view.expect_instance_for_expression(box_string_ty);

    // Box<number>
    let (box_number_instance_symbol, box_number_arguments) =
        view.expect_instance_for_expression(box_number_ty);

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

    let (first_instance_symbol, first_arguments) =
        view.expect_instance_for_expression(first_value_id);
    let (second_instance_symbol, second_arguments) =
        view.expect_instance_for_expression(second_value_id);
    let (third_instance_symbol, third_arguments) =
        view.expect_instance_for_expression(third_value_id);

    // Box.map
    assert_eq!(first_instance_symbol, map_symbol);
    assert_eq!(second_instance_symbol, map_symbol);
    assert_eq!(third_instance_symbol, map_symbol);

    // boxString.map<boolean>(flag): <string, boolean>
    view.assert_static_argument_primitive_sequence(
        &first_arguments,
        &[PrimitiveType::String, PrimitiveType::Boolean],
    );

    // boxString.map(flag): <string, boolean>
    view.assert_static_argument_primitive_sequence(
        &second_arguments,
        &[PrimitiveType::String, PrimitiveType::Boolean],
    );

    // boxNumber.map(flag): <number, boolean>
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

    // derived.map<string>()
    let (instance_symbol, static_arguments) = main_view.expect_instance_for_expression(value_id);

    // Base.map
    assert_eq!(instance_symbol, base_map_symbol);

    // <number, string>
    main_view.assert_static_argument_primitive_sequence(
        &static_arguments,
        &[PrimitiveType::Number, PrimitiveType::String],
    );
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
    let (instance_symbol, static_arguments) = view.expect_instance_for_expression(value_id);

    // Base.map
    assert_eq!(instance_symbol, base_map_symbol);

    // <number, string>
    view.assert_static_argument_primitive_sequence(
        &static_arguments,
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
    let (instance_symbol, static_arguments) = view.expect_instance_for_expression(value_id);

    // Base.map
    assert_eq!(instance_symbol, base_map_symbol);

    // <number, string>
    view.assert_static_argument_primitive_sequence(
        &static_arguments,
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
    let (instance_symbol, static_arguments) = view.expect_instance_for_expression(value_id);

    // PairOps.map
    assert_eq!(instance_symbol, map_symbol);

    // <string, number, boolean>
    view.assert_static_argument_primitive_sequence(
        &static_arguments,
        &[
            PrimitiveType::String,
            PrimitiveType::Number,
            PrimitiveType::Boolean,
        ],
    );

    // PairOps.map
    // PairOps.map<string, number, boolean>
    view.assert_instances_for_symbol(
        map_symbol,
        &[ExpectedInstanceShape {
            static_parameter_symbols: view.static_parameter_symbols_for_symbol(map_symbol),
            inherited_static_argument_count: 2,
            static_argument_primitives: vec![
                PrimitiveType::String,
                PrimitiveType::Number,
                PrimitiveType::Boolean,
            ],
        }],
    );
}

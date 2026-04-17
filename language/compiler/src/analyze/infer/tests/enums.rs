use super::*;

/// Analyze enum member shapes.
#[test]
fn test_analyze_enum_member_shapes() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let source = r#"
enum Status {
    Active = 1
    Inactive = 2

    isActive(): boolean {
        true
    }

    static Default = Status.Active;
}
"#;
    let module_id = test.add_module("test.ds", source);
    test.analyze_module(module_id);
    test.compile_check_clean();

    // resolve enum symbol and load module types
    let enum_symbol = test.canonical_symbol_for_path("test.ds", "Status");
    let view = test.view(module_id);

    // check instance shape for instance methods
    let instance_id = view.expect_instance_type_id(enum_symbol);
    let instance_ty = view.types().get_type(instance_id);
    let Type::Object { fields, .. } = instance_ty else {
        panic!("expected enum instance object type, found {instance_ty:?}");
    };
    let is_active_key = StaticKey::Name(test.program.strings.intern("isActive"));
    assert!(
        fields.iter().any(|field| field.key.matches(&is_active_key)),
        "expected isActive in enum instance fields, found {fields:?}",
    );

    // check value shape for static fields
    let value_id = view.expect_value_type_id(enum_symbol);
    let mut shape = ObjectShape::default();
    let mut extras = Vec::new();
    let mut visited = Vec::new();
    test.compiler.collect_value_shape_from_type(
        value_id,
        view.types(),
        &mut shape,
        &mut extras,
        &mut visited,
    );
    let default_key = StaticKey::Name(test.program.strings.intern("Default"));
    assert!(
        shape
            .fields
            .iter()
            .any(|field| field.key.matches(&default_key)),
        "expected Default in enum value fields, found {:?}",
        shape.fields,
    );
}

/// Enum values are not implicitly assignable to backing types.
#[test]
fn test_analyze_enum_backing_assignability() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let source = r#"
enum Status {
    Active = 1
}

const status = Status.Active;
const raw: int32 = status;
"#;
    // allow expected diagnostics from invalid assignments
    let module_id = test.add_module("test.ds", source);
    test.analyze_module(module_id);
    test.compile();

    // load typed module data
    let view = test.view(module_id);
    let options = test.analyze_context_options_for_module(module_id);
    let module = test.program.module_descriptor(module_id);
    let module = module.as_ref();
    let profile = view.profile_id();

    // resolve the binding symbols
    let status_name = test.program.strings.intern("status");
    let raw_name = test.program.strings.intern("raw");
    let status_symbol = view.expect_binding_symbol(status_name);
    let raw_declarator_id = view.expect_let_declarator(raw_name);
    let status_initializer_id = view.expect_initializer(status_name);

    // read declared and inferred types
    let raw_declared_id =
        view.expect_declared_type_id(raw_declarator_id.into_global_any(module_id));
    let status_value_id = view.expect_value_type_id(status_symbol);
    let enum_symbol = test.canonical_symbol_for_path("test.ds", "Status");
    let enum_value_id = view.expect_value_type_id(enum_symbol);

    // ensure the enum value type is resolved
    if let Type::InferVar { .. } = view.types().get_type(enum_value_id) {
        panic!("unexpected enum value type inferred as infer var");
    }

    // resolve the enum field value type
    let active_name = test.program.strings.intern("Active");
    let enum_field_symbol = view.expect_enum_field_symbol(active_name);
    let enum_field_value_id = view.expect_value_type_id(enum_field_symbol);
    match view.types().get_type(enum_field_value_id) {
        Type::Reference { symbol, .. } => {
            assert_eq!(
                *symbol, enum_symbol,
                "expected enum field value to be nominal enum type",
            );
        }
        other => panic!("expected enum field value reference, got {other:?}"),
    }

    // confirm the inferred member type is nominal
    match view.types().get_type(status_value_id) {
        Type::Reference { symbol, .. } => {
            assert_eq!(
                *symbol, enum_symbol,
                "expected Status.Active to infer nominal enum type",
            );
        }
        other => panic!("expected Status.Active to infer enum reference, got {other:?}"),
    }

    // confirm the initializer resolves to a member on Status
    let (left_id, member_name) = view.expect_member_expression(status_initializer_id);
    assert_eq!(member_name, active_name);

    let left_symbol = view.expect_reference_symbol(left_id);
    let left_symbol = canonical_symbol_id(
        &test.compiler,
        module,
        view.symbols(),
        profile,
        left_symbol,
        CanonicalSymbolMode::FollowAliases,
    );
    assert_eq!(left_symbol, enum_symbol);

    // validate assignability
    let symbols = view.symbols().clone();
    let mut types = view.types().clone();
    let assignability = is_type_assignable(
        &test.compiler,
        module,
        profile,
        view.tree(),
        &symbols,
        raw_declared_id,
        status_value_id,
        &mut types,
        &options,
    );
    assert_eq!(
        assignability,
        Assignability::NotAssignable,
        "expected enum value to be non-assignable to backing type, target={:?}, source={:?}",
        view.types().get_type(raw_declared_id),
        view.types().get_type(status_value_id),
    );
}

/// Enum fields should resolve implicit integer values.
#[test]
fn test_analyze_enum_field_values_integer() {
    let test = TestProgram::memory_sequential();
    let source = r#"
enum Status {
    Active
    Inactive
    Pending = 10
    Disabled
}
"#;
    let module_id = test.add_module("test.ds", source);
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();

    let view = test.view(module_id);
    let strings = &test.program.strings;
    let active_symbol = view.expect_enum_field_symbol(strings.intern("Active"));
    let inactive_symbol = view.expect_enum_field_symbol(strings.intern("Inactive"));
    let pending_symbol = view.expect_enum_field_symbol(strings.intern("Pending"));
    let disabled_symbol = view.expect_enum_field_symbol(strings.intern("Disabled"));

    let active_value = view
        .types()
        .get_enum_field_value(active_symbol)
        .expect("expected Active enum value");
    let inactive_value = view
        .types()
        .get_enum_field_value(inactive_symbol)
        .expect("expected Inactive enum value");
    let pending_value = view
        .types()
        .get_enum_field_value(pending_symbol)
        .expect("expected Pending enum value");
    let disabled_value = view
        .types()
        .get_enum_field_value(disabled_symbol)
        .expect("expected Disabled enum value");

    assert_eq!(active_value, EnumFieldValue::Int(0));
    assert_eq!(inactive_value, EnumFieldValue::Int(1));
    assert_eq!(pending_value, EnumFieldValue::Int(10));
    assert_eq!(disabled_value, EnumFieldValue::Int(11));
}

/// String-backed enum fields require explicit values.
#[test]
fn test_analyze_enum_field_values_string_requires_explicit() {
    let test = TestProgram::memory_sequential();
    let source = r#"
enum Status {
    Active = "active"
    Inactive
}
"#;
    let module_id = test.add_module("test.ds", source);
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostics(&["EA105"]);
}

/// Enum field values may use constant expressions.
#[test]
fn test_analyze_enum_field_values_const_expression() {
    let test = TestProgram::memory_sequential();
    let source = r#"
enum Status {
    Active = 1 + 2
    Inactive
}
"#;
    let module_id = test.add_module("test.ds", source);
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();

    let view = test.view(module_id);
    let strings = &test.program.strings;
    let active_symbol = view.expect_enum_field_symbol(strings.intern("Active"));
    let inactive_symbol = view.expect_enum_field_symbol(strings.intern("Inactive"));

    let active_value = view
        .types()
        .get_enum_field_value(active_symbol)
        .expect("expected Active enum value");
    let inactive_value = view
        .types()
        .get_enum_field_value(inactive_symbol)
        .expect("expected Inactive enum value");

    assert_eq!(active_value, EnumFieldValue::Int(3));
    assert_eq!(inactive_value, EnumFieldValue::Int(4));
}

/// Enum field values can reference earlier enum members.
#[test]
fn test_analyze_enum_field_values_reference_earlier_member() {
    let test = TestProgram::memory_sequential();
    let source = r#"
enum Status {
    Active = 1
    Inactive = Active + 2
    Pending = Inactive
}
"#;
    let module_id = test.add_module("test.ds", source);
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();

    let view = test.view(module_id);
    let strings = &test.program.strings;
    let active_symbol = view.expect_enum_field_symbol(strings.intern("Active"));
    let inactive_symbol = view.expect_enum_field_symbol(strings.intern("Inactive"));
    let pending_symbol = view.expect_enum_field_symbol(strings.intern("Pending"));

    let active_value = view
        .types()
        .get_enum_field_value(active_symbol)
        .expect("expected Active enum value");
    let inactive_value = view
        .types()
        .get_enum_field_value(inactive_symbol)
        .expect("expected Inactive enum value");
    let pending_value = view
        .types()
        .get_enum_field_value(pending_symbol)
        .expect("expected Pending enum value");

    assert_eq!(active_value, EnumFieldValue::Int(1));
    assert_eq!(inactive_value, EnumFieldValue::Int(3));
    assert_eq!(pending_value, EnumFieldValue::Int(3));
}

/// Enum field values cannot reference non-enum values.
#[ignore]
#[test]
fn test_analyze_enum_field_values_require_enum_reference() {
    let test = TestProgram::memory_sequential();
    let source = r#"
const base = 1;

enum Status {
    Active = base
    Inactive
}
"#;
    let module_id = test.add_module("test.ds", source);
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostics(&["EA105"]);
}

/// Enum field values should be inferred as nominal enum types.
#[test]
fn test_analyze_enum_field_nominal_type() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_module_with_source(
        "test.ds",
        r#"
enum Status {
    Active = 1
}

const status = Status.Active;
"#,
    );

    // load typed module data
    let view = test.view(module_id);

    // confirm the initializer is a member expression
    let status_name = test.program.strings.intern("status");
    let status_symbol = view.expect_binding_symbol(status_name);
    let value_id = view.expect_initializer(status_name);
    let (left_id, member_name) = view.expect_member_expression(value_id);
    let _left_symbol = view.expect_reference_symbol(left_id);
    let active_name = test.program.strings.intern("Active");
    assert_eq!(member_name, active_name);

    // resolve the enum value type
    let enum_symbol = test.canonical_symbol_for_path("test.ds", "Status");
    let enum_module = test.module("test.ds");
    let enum_module = enum_module.as_ref();

    // confirm the binding value type is the nominal enum reference
    let status_symbol = canonical_symbol_id(
        &test.compiler,
        enum_module,
        view.symbols(),
        view.profile_id(),
        status_symbol,
        CanonicalSymbolMode::FollowAliases,
    );
    let status_ty_id = view.expect_value_type_id(status_symbol);
    assert_type!(
        view.types(),
        status_ty_id,
        Type::Reference {
            symbol,
            generic_arguments
        } => {
            assert_eq!(*symbol, enum_symbol);
            assert!(generic_arguments.is_none());
        }
    );
    let enum_value_ty_id = view.expect_value_type_id(enum_symbol);

    // collect the Active field type from the enum value object
    let object_fields = view.object_fields_for_type(enum_value_ty_id);
    let active_key = StaticKey::Name(test.program.strings.intern("Active"));
    let active_field = object_fields
        .iter()
        .find(|field| field.key.matches(&active_key))
        .expect("expected Active field");
    assert_type!(
        view.types(),
        active_field.ty,
        Type::Reference {
            symbol,
            generic_arguments
        } => {
            assert_eq!(*symbol, enum_symbol);
            assert!(generic_arguments.is_none());
        }
    );
    let active_field_ty = active_field.ty;

    // confirm enums are not implicitly assignable to their backing type
    let options = test.analyze_context_options_for_module(module_id);
    let symbols = view.symbols().clone();
    let mut types = view.types().clone();
    let int32_ty_id = types.insert_type_from_any(
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32)),
        },
        value_id.into_any(),
    );
    let assignable = is_type_assignable(
        &test.compiler,
        enum_module,
        view.profile_id(),
        view.tree(),
        &symbols,
        int32_ty_id,
        active_field_ty,
        &mut types,
        &options,
    );
    assert!(!assignable.is_assignable());
}

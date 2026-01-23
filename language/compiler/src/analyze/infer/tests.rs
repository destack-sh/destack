use crate::analyze::common::CanonicalSymbolMode;
use crate::{
    AnalyzeOptions, InferContext, TestProgram, assert_string, assert_type,
    expect_let_declarator_by_name, root_expression_id,
};
use destack_base::StringId;
use destack_dir::{
    BinaryOperator, Declaration, Declarator, EnumFieldValue, Expression, ExtensionKind,
    FlowEdgeKind, FlowGraphBuilder, GlobalNodeIdAny, GlobalSymbolId, IfCondition, IfKind,
    InferTable, IntType, LocalNodeId, LocalScopeMark, LocalTypeId, NodeTree, NormalizationMode,
    Pattern, PrimitiveType, ScalarLiteral, StaticArgument, StaticExpression, StaticKey, SymbolKind,
    SymbolSpace, SymbolTable, SymbolType, Type, TypeField, TypeLiteral, TypeTable,
    TypeUnaryOperator,
};
use destack_source::ModuleId;
use destack_workspace::DsConfigCompilerOptions;

/// Cached view of module tables for tests.
struct TestModuleView<'a> {
    /// The owning test program.
    test: &'a TestProgram,
    /// The module id under test.
    module_id: ModuleId,
    /// The module root expressions.
    roots: Vec<LocalNodeId<Expression>>,
    /// The module node tree.
    tree: NodeTree,
    /// The module symbol table.
    symbols: SymbolTable,
    /// The module type table.
    types: TypeTable,
}

impl TestProgram {
    /// Add, analyze, and check a module in one step.
    fn analyze_module_with_source(&self, name: &str, source: &str) -> ModuleId {
        // register the module
        let module_id = self.add_module(name, source);

        // analyze and check diagnostics
        self.analyze_module_and_check_clean(module_id);

        module_id
    }

    /// Resolve a canonical symbol id for a module path.
    fn canonical_symbol_for_path(&self, module_uri: &str, path: &str) -> GlobalSymbolId {
        // resolve the module symbol
        let symbol = self
            .resolve_to_symbol(module_uri, path)
            .unwrap_or_else(|| panic!("expected symbol for {module_uri}:{path}"));

        // resolve the module state
        let module = self.module(module_uri);
        let module = module.read();
        let profile = self.default_profile_id(module.id);
        let symbols = module.dir(profile).symbols.read();

        // resolve the canonical symbol id
        self.compiler.canonical_symbol_id(
            &module,
            &symbols,
            profile,
            symbol,
            CanonicalSymbolMode::FollowAliases,
        )
    }

    /// Create a cached module view for tests.
    fn view(&self, module_id: ModuleId) -> TestModuleView<'_> {
        // load module state
        let profile = self.default_profile_id(module_id);
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);

        // clone module dir data
        let roots = dir.roots.clone();
        let tree = dir.tree.read().clone();
        let symbols = dir.symbols.read().clone();
        let types = dir.types.read().clone();

        TestModuleView {
            test: self,
            module_id,
            roots,
            tree,
            symbols,
            types,
        }
    }
}

impl<'a> TestModuleView<'a> {
    /// Read the roots for this view.
    fn roots(&self) -> &[LocalNodeId<Expression>] {
        &self.roots
    }

    /// Read the node tree for this view.
    fn tree(&self) -> &NodeTree {
        &self.tree
    }

    /// Read the symbol table for this view.
    fn symbols(&self) -> &SymbolTable {
        &self.symbols
    }

    /// Read the type table for this view.
    fn types(&self) -> &TypeTable {
        &self.types
    }

    /// Resolve the default profile id for this module.
    fn profile_id(&self) -> destack_workspace::ProfileId {
        self.test.default_profile_id(self.module_id)
    }

    /// Resolve the root expression for the view.
    fn root_expression_id(&self, index: usize) -> LocalNodeId<Expression> {
        root_expression_id(&self.roots, &self.tree, index)
    }

    /// Resolve an inferred type for a local expression.
    fn expect_inferred_type(&self, expression_id: LocalNodeId<Expression>) -> &Type {
        // resolve inferred type
        self.types
            .get_inferred_type(expression_id.into_global_any(self.module_id))
            .expect("expected inferred type")
    }

    /// Resolve an inferred type id for a local expression.
    fn expect_inferred_type_id(&self, expression_id: LocalNodeId<Expression>) -> LocalTypeId {
        // resolve inferred type id
        self.types
            .get_inferred_type_id(expression_id.into_global_any(self.module_id))
            .expect("expected inferred type id")
    }

    /// Resolve the declarator for a binding name.
    fn expect_let_declarator(&self, name: StringId) -> LocalNodeId<Declarator> {
        expect_let_declarator_by_name(&self.roots, &self.tree, name)
    }

    /// Resolve a binding symbol for a let declarator by name.
    fn expect_binding_symbol(&self, name: StringId) -> GlobalSymbolId {
        // resolve the declarator
        let declarator_id = self.expect_let_declarator(name);
        let declarator = self.tree.get(declarator_id);

        // extract the binding symbol
        match self.tree.get(declarator.pattern) {
            Pattern::Binding {
                symbol, pattern, ..
            } => {
                assert!(pattern.is_none());
                symbol.into_global(self.module_id)
            }
            other => panic!("expected binding pattern, got {other:?}"),
        }
    }

    /// Resolve the initializer value for a let declarator by name.
    fn expect_initializer(&self, name: StringId) -> LocalNodeId<Expression> {
        // resolve the declarator
        let declarator_id = self.expect_let_declarator(name);
        let declarator = self.tree.get(declarator_id);

        // return the initializer
        declarator
            .value
            .unwrap_or_else(|| panic!("expected initializer for {name:?}"))
    }

    /// Resolve the enum field symbol for a member name.
    fn expect_enum_field_symbol(&self, name: StringId) -> GlobalSymbolId {
        // scan enum declarations for the field
        for declaration_id in self.tree.iter_node_ids_of_type::<Declaration>() {
            let Declaration::Enum { fields, .. } = self.tree.get(declaration_id) else {
                continue;
            };
            for field_id in fields {
                let field = self.tree.get(*field_id);
                if field.name == name {
                    return field.symbol.into_global(self.module_id);
                }
            }
        }

        panic!("expected enum field for {name:?}");
    }

    /// Read a declared type id for a node.
    fn expect_declared_type_id(&self, node_id: GlobalNodeIdAny) -> LocalTypeId {
        self.types
            .get_declared_type_id(node_id)
            .expect("expected declared type id")
    }

    /// Read a value type id for a symbol.
    fn expect_value_type_id(&self, symbol: GlobalSymbolId) -> LocalTypeId {
        self.types
            .get_value_type_id(symbol)
            .expect("expected value type id")
    }

    /// Read an instance type id for a symbol.
    fn expect_instance_type_id(&self, symbol: GlobalSymbolId) -> LocalTypeId {
        self.types
            .get_instance_type_id(symbol)
            .expect("expected instance type id")
    }

    /// Resolve a member expression into its receiver and member key.
    fn expect_member_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> (LocalNodeId<Expression>, StringId) {
        match self.tree.get(expression_id) {
            Expression::Member { left, name, .. } => (*left, *name),
            other => panic!("expected member expression, got {other:?}"),
        }
    }

    /// Resolve a reference symbol from a reference expression.
    fn expect_reference_symbol(&self, expression_id: LocalNodeId<Expression>) -> GlobalSymbolId {
        match self.tree.get(expression_id) {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => *target_symbol,
            other => panic!("expected reference expression, got {other:?}"),
        }
    }

    /// Collect the first object field list available for a type.
    fn object_fields_for_type(&self, ty_id: LocalTypeId) -> Vec<TypeField> {
        // prefer direct object types
        if let Type::Object { fields, .. } = self.types.get_type(ty_id) {
            return fields.to_vec();
        }

        // scan intersection elements for an object type
        if let Type::Intersection { elements } = self.types.get_type(ty_id) {
            for element_id in elements {
                if let Type::Object { fields, .. } = self.types.get_type(*element_id) {
                    return fields.to_vec();
                }
            }
        }

        panic!("expected object fields for type");
    }
}

/// Collect extension kinds for a target symbol.
fn extension_kinds_for_target(
    types: &TypeTable,
    target_symbol: GlobalSymbolId,
) -> Vec<ExtensionKind> {
    // collect extension ids for the target symbol
    let Some(extension_ids) = types.get_extensions_for_target(target_symbol) else {
        return Vec::new();
    };

    // map extension ids to kinds
    extension_ids
        .iter()
        .map(|extension_id| types.get_extension(*extension_id).kind)
        .collect()
}

/// Analyze number literal.
#[test]
fn test_analyze_number_literal() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_module_with_source("test.ds", "42");

    // load typed module data
    let view = test.view(module_id);

    // select the root expression
    let expression_id = view.root_expression_id(0);

    // read inferred type
    let ty = view.expect_inferred_type(expression_id);

    // integer literals now have literal types, not widened primitive types
    assert_eq!(
        *ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(42))
        }
    );
}

/// Analyze string literal.
#[test]
fn test_analyze_string_literal() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_module_with_source("test.ds", r#""hello""#);

    // load typed module data
    let view = test.view(module_id);

    // select the root expression
    let expression_id = view.root_expression_id(0);

    // read inferred type
    let ty = view.expect_inferred_type(expression_id);

    // string literal has literal type (e.g., "hello" has type "hello")
    assert!(matches!(
        ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
        }
    ));
}

/// Analyze boolean literal.
#[test]
fn test_analyze_boolean_literal() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_module_with_source("test.ds", "true");

    // load typed module data
    let view = test.view(module_id);

    // select the root expression
    let expression_id = view.root_expression_id(0);

    // read inferred type
    let ty = view.expect_inferred_type(expression_id);

    // boolean literal has literal type (e.g., true has type true)
    assert_eq!(
        *ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(true))
        }
    );
}

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
    let mut shape = crate::analyze::common::ObjectShape::default();
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
    let options = test.compiler.analyze_context_options_for_module(module_id);
    let module = test.program.modules.get(module_id);
    let module = module.read();
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
    let left_symbol = test.compiler.canonical_symbol_id(
        &module,
        view.symbols(),
        profile,
        left_symbol,
        CanonicalSymbolMode::FollowAliases,
    );
    assert_eq!(left_symbol, enum_symbol);

    // validate assignability
    let symbols = view.symbols().clone();
    let mut types = view.types().clone();
    let assignability = test.compiler.is_type_assignable(
        &module,
        profile,
        &symbols,
        raw_declared_id,
        status_value_id,
        &mut types,
        &options,
    );
    assert_eq!(
        assignability,
        crate::analyze::infer::assign::Assignability::NotAssignable,
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
// FUGU #Broken: phase 9 enum field reference validation
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

/// Merge global interface members across imported modules.
#[test]
fn test_merge_global_declarations_across_imports() {
    // arrange test modules
    let test = TestProgram::memory_sequential();
    test.add_module(
        "a.ds",
        r#"
declare global {
    interface GlobalThing {
        value: number
    }
}
"#,
    );
    test.add_module(
        "b.ds",
        r#"
declare global {
    interface GlobalThing {
        label: string
    }
}
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import "./a.ds";
import "./b.ds";

const thing: GlobalThing = { value: 1, label: "ok" };
"#,
    );

    // analyze the entry module
    test.analyze_module_and_check_clean(main_id);

    // load module data for inspection
    let view = test.view(main_id);
    let module = test.program.modules.get(main_id);
    let module = module.read();
    let profile = view.profile_id();
    let dir = module.dir(profile);
    let thing_name = test.program.strings.intern("thing");
    let global_key = StaticKey::Name(test.program.strings.intern("GlobalThing"));
    let global_group = test
        .compiler
        .get_global_symbol_group(main_id, profile, global_key, SymbolSpace::Type)
        .expect("missing global group for GlobalThing");
    assert_eq!(global_group.len(), 2);

    for global_symbol in &global_group {
        let remote_profile = test.default_profile_id(global_symbol.module_id);
        let remote_module = test.program.modules.get(global_symbol.module_id);
        let remote_module = remote_module.read();
        let remote_types = remote_module.dir(remote_profile).types.read();
        assert!(
            remote_types.get_instance_type_id(*global_symbol).is_some(),
            "expected instance type for GlobalThing in module {:?}",
            global_symbol.module_id,
        );
    }

    // locate the bound symbol for thing in the module scope
    let namespace_scope = view.symbols().get_scope_by_id(dir.namespace_scope);
    let thing_symbol = view
        .symbols()
        .active_named_symbols(namespace_scope)
        .find_map(|(key, symbol_id)| match key {
            StaticKey::Name(name_id) if name_id == thing_name => Some(symbol_id),
            _ => None,
        })
        .expect("missing symbol for thing")
        .into_global(main_id);
    let thing_entry = view.symbols().get_symbol(thing_symbol.local_id);
    assert!(
        thing_entry.target_symbol.is_none(),
        "unexpected target_symbol on thing binding",
    );
    assert!(
        thing_entry.canonical_symbol.is_none(),
        "unexpected canonical_symbol on thing binding",
    );
    let canonical_symbol = test.compiler.canonical_symbol_id(
        &module,
        view.symbols(),
        profile,
        thing_symbol,
        CanonicalSymbolMode::FollowAliases,
    );
    assert_eq!(canonical_symbol, thing_symbol);

    // assert the binding uses a nominal reference type
    let value_ty_id = view
        .types()
        .get_value_type_id(thing_symbol)
        .expect("missing value type for thing");
    let value_ty = view.types().get_type(value_ty_id);
    let Type::Reference {
        symbol: global_thing,
        ..
    } = value_ty
    else {
        panic!("expected reference type for thing");
    };
    assert_eq!(global_thing.ty(), SymbolType::Interface);

    // assert the merged instance type includes both fields
    let instance_ty_id = view
        .types()
        .get_instance_type_id(*global_thing)
        .expect("missing instance type for GlobalThing");
    let instance_ty = view.types().get_type(instance_ty_id);
    let Type::Object { fields, .. } = instance_ty else {
        panic!("expected object instance type for GlobalThing");
    };
    let value_key = StaticKey::Name(test.program.strings.intern("value"));
    let label_key = StaticKey::Name(test.program.strings.intern("label"));
    assert!(fields.iter().any(|field| field.key.matches(&value_key)));
    assert!(fields.iter().any(|field| field.key.matches(&label_key)));
}

/// Analyze binary number operation.
#[test]
fn test_analyze_binary_number_operation() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "1 + 2");

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // select the root expression
    let expression_id = view.root_expression_id(0);

    // read inferred type
    let ty = view.expect_inferred_type(expression_id);

    // constant folding: 1 + 2 evaluates to literal type 3
    assert_eq!(
        *ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(3))
        }
    );
}

/// Narrow nullish types in an if guard.
#[test]
fn test_narrowing_nullish_if_guard() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
const value: string | null = null;
if (value != null) {
    const narrowed: string = value;
} else {
    0;
}
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the if expression
    let (_, if_expression) = view
        .tree()
        .iter_nodes_of_type::<Expression>()
        .find(|(_, expression)| matches!(expression, Expression::If { .. }))
        .expect("expected if expression");
    let Expression::If {
        then_expression, ..
    } = if_expression
    else {
        panic!("expected if expression");
    };

    // locate the narrowed declaration in the then block
    let then_expression_id = match view.tree().get(*then_expression) {
        Expression::Block { block } => {
            let block = view.tree().get(*block);
            *block
                .expressions
                .first()
                .expect("expected then block expression")
        }
        Expression::Statement { statement } => *statement,
        _ => *then_expression,
    };

    let then_expression_id = match view.tree().get(then_expression_id) {
        Expression::Statement { statement } => *statement,
        _ => then_expression_id,
    };

    let Expression::Let { declarators, .. } = view.tree().get(then_expression_id) else {
        panic!("expected let expression");
    };

    let declarator_id = declarators.first().expect("expected declarator");
    let declarator = view.tree().get(*declarator_id);
    let value_expression_id = declarator
        .value
        .expect("expected value expression in declarator");

    // assert nullish types are stripped in the then branch
    let left_type_id = view.expect_inferred_type_id(value_expression_id);
    let left_type = view.types().get_type(left_type_id);

    let is_nullish = match left_type {
        Type::Union { elements } => elements.iter().any(|element_id| {
            matches!(
                view.types().get_type(*element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Null | TypeLiteral::Undefined
                }
            )
        }),
        Type::TypeLiteral {
            value: TypeLiteral::Null | TypeLiteral::Undefined,
        } => true,
        _ => false,
    };

    assert!(!is_nullish, "expected nullish to be stripped");
}

/// Analyze binary number comparison.
#[test]
fn test_analyze_binary_number_comparison() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "1 < 2");

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // select the root expression
    let expression_id = view.root_expression_id(0);

    // read inferred type
    let ty = view.expect_inferred_type(expression_id);

    // constant folding: 1 < 2 evaluates to literal true
    assert_eq!(
        *ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(true))
        }
    );
}

/// Analyze union discriminant comparisons as boolean expressions.
#[test]
fn test_analyze_union_discriminant_comparison() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function select(value: { kind: 0, value: int32 } | { kind: 1, value: int32 }): int32 {
    return value.kind == 0 ? 1 : 2;
}
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the ternary condition
    let condition_id = view
        .tree()
        .iter_node_ids_of_type::<Expression>()
        .iter()
        .find_map(|expression_id| match view.tree().get(*expression_id) {
            Expression::If {
                kind: IfKind::Ternary,
                condition: IfCondition::Expression { condition },
                ..
            } => Some(*condition),
            _ => None,
        })
        .expect("expected ternary condition expression");

    // read inferred type
    let ty = view.expect_inferred_type(condition_id);

    assert_eq!(
        *ty,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Boolean)
        }
    );
}

/// Analyze let expression infer type.
#[test]
fn test_analyze_let_expression_infer_type() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x = 42");

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the let expression
    let let_expr_id = view.root_expression_id(0);
    let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();

    // no declared type
    assert!(
        view.types()
            .get_declared_type(let_expr_id.into_global_any(module_id))
            .is_none()
    );

    // value_type[x] = literal 42
    let x_ty = view.types().get_value_type(x_symbol).unwrap();
    assert_eq!(
        *x_ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(42))
        }
    );

    // instance_type[x] = undefined
    assert!(view.types().get_instance_type(x_symbol).is_none());
}

/// Analyze let expression declare type.
#[test]
fn test_analyze_let_expression_declare_type() {
    // use compatible types: string annotation with string value
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", r#"let x: string = "hello""#);

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // select the root expression
    let expression_id = view.root_expression_id(0);
    let let_expression = view.tree().get(expression_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();
    let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();

    // declared_type[declarator] = string
    let declared = view
        .types()
        .get_declared_type(declarator_id.into_global(module_id).into())
        .unwrap();
    assert_eq!(
        *declared,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        }
    );

    // value_type[x] = string
    let x_ty = view.types().get_value_type(x_symbol).unwrap();
    assert_eq!(
        *x_ty,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        }
    );

    // instance_type[x] = undefined
    assert!(view.types().get_instance_type(x_symbol).is_none());
}

/// Analyze let expression infer tuple type with pattern.
#[test]
fn test_analyze_let_expression_infer_tuple_type_with_pattern() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
let (x, y, ...rest, z) = (123, 'abc', true, 456);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // resolve binding symbols
    let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();
    let y_symbol = test.resolve_to_symbol("test.ds", "y").unwrap();
    let rest_symbol = test.resolve_to_symbol("test.ds", "rest").unwrap();
    let z_symbol = test.resolve_to_symbol("test.ds", "z").unwrap();

    // value_type[x] = literal 123
    let x_ty_id = view.types().get_value_type_id(x_symbol).unwrap();

    assert_type!(
        view.types(),
        x_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(123))
        }
    );

    // value_type[y] = literal 'abc'
    let y_ty_id = view.types().get_value_type_id(y_symbol).unwrap();
    assert_type!(
        view.types(),
        y_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
        }
    );

    // value_type[rest] = (true,)
    let rest_ty_id = view.types().get_value_type_id(rest_symbol).unwrap();
    assert_type!(view.types(), rest_ty_id, Type::Tuple { elements, is_readonly: _ } => {
        assert_eq!(elements.len(), 1);
        assert_type!(view.types(), elements[0].ty, Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(true))
        });
    });

    // value_type[z] = literal 456
    let z_ty_id = view.types().get_value_type_id(z_symbol).unwrap();
    assert_type!(
        view.types(),
        z_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(456))
        }
    );
}

/// Analyze cross module type import.
#[test]
fn test_analyze_cross_module_type_import() {
    // import a value from another module and verify its type is correctly imported
    let test = TestProgram::memory_sequential();
    test.add_file(
        "lib.ds",
        r#"
export let value = 42;
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { value } from "./lib.ds";
let x = value;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // x should have literal type 42 (imported from lib.ds)
    let x_symbol = test.resolve_to_symbol("main.ds", "x").unwrap();
    let x_ty_id = view.types().get_value_type_id(x_symbol).unwrap();

    assert_type!(
        view.types(),
        x_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(42))
        }
    );
}

/// Analyze cross module array type import.
#[test]
fn test_analyze_cross_module_array_type_import() {
    // import an array value from another module
    let test = TestProgram::memory_sequential();
    test.add_file(
        "lib.ds",
        r#"
export let items = [1, 2, 3];
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { items } from "./lib.ds";
let x = items;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // x should have an array element union from the imported literal values
    let x_symbol = test.resolve_to_symbol("main.ds", "x").unwrap();
    let x_ty_id = view.types().get_value_type_id(x_symbol).unwrap();

    assert_type!(view.types(), x_ty_id, Type::Array { element: Some(element_id), .. } => {
        assert_type!(view.types(), *element_id, Type::Union { elements } => {
            // union has three literal elements
            assert_eq!(elements.len(), 3);

            // each element stays a literal integer
            for element_id in elements {
                assert_type!(view.types(), *element_id, Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_))
                });
            }
        });
    });
}

/// Analyze cross module string type import.
#[test]
fn test_analyze_cross_module_string_type_import() {
    // import a string value from another module
    let test = TestProgram::memory_sequential();
    test.add_file(
        "lib.ds",
        r#"
export let greeting = "hello";
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { greeting } from "./lib.ds";
let x = greeting;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // x should have literal string type (imported from lib.ds)
    let x_symbol = test.resolve_to_symbol("main.ds", "x").unwrap();
    let x_ty_id = view.types().get_value_type_id(x_symbol).unwrap();

    assert_type!(
        view.types(),
        x_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
        }
    );
}

/// Analyze cross module member access using declared shapes.
#[test]
fn test_analyze_cross_module_member_access_declared_shape() {
    // arrange test modules
    let test = TestProgram::memory_sequential();
    test.add_file(
        "lib.ds",
        r#"
export struct Box {
    value: number
}

export let boxed: Box = { value: 1 };
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { boxed } from "./lib.ds";

let value = boxed.value;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // value should be number from the declared field type
    let value_symbol = test.resolve_to_symbol("main.ds", "value").unwrap();
    let value_ty_id = view.types().get_value_type_id(value_symbol).unwrap();

    assert_type!(
        view.types(),
        value_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

/// Analyze circular imports with a declared export anchor.
#[test]
fn test_analyze_cross_module_circular_imports_with_declared_anchor() {
    // arrange circular modules with a declared anchor
    let test = TestProgram::memory_sequential();
    let b_module_id = test.add_module(
        "b.ds",
        r#"
import { a } from "./a.ds";

export let b = a;
"#,
    );
    let a_module_id = test.add_module(
        "a.ds",
        r#"
import { b } from "./b.ds";

export let a: number = b;
"#,
    );

    // run analyze pipeline for the anchor module
    test.analyze_module_and_check_clean(a_module_id);

    // load typed module data
    let view = test.view(b_module_id);

    // b should pick up the declared number type from a
    let b_symbol = test.resolve_to_symbol("b.ds", "b").unwrap();
    let b_ty_id = view.types().get_value_type_id(b_symbol).unwrap();

    assert_type!(
        view.types(),
        b_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

/// Analyze export chain surface inference.
#[test]
fn test_analyze_export_chain_surface_inference() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_module_with_source(
        "test.ds",
        r#"
export let a = 1;
export let b = a;
"#,
    );

    // load typed module data
    let view = test.view(module_id);

    // locate exported declarators
    let a_name = test.program.strings.intern("a");
    let b_name = test.program.strings.intern("b");
    let a_declarator_id = view.expect_let_declarator(a_name);
    let b_declarator_id = view.expect_let_declarator(b_name);

    // resolve binding symbols
    let a_pattern = view.tree().get(view.tree().get(a_declarator_id).pattern);
    let b_pattern = view.tree().get(view.tree().get(b_declarator_id).pattern);
    let Pattern::Binding {
        symbol: a_symbol, ..
    } = a_pattern
    else {
        panic!("expected binding");
    };
    let Pattern::Binding {
        symbol: b_symbol, ..
    } = b_pattern
    else {
        panic!("expected binding");
    };

    // read value types
    let a_ty_id = view
        .types()
        .get_value_type_id(a_symbol.into_global(module_id))
        .expect("expected a type");
    let b_ty_id = view
        .types()
        .get_value_type_id(b_symbol.into_global(module_id))
        .expect("expected b type");

    // both exports resolve to the literal number type
    assert_type!(
        view.types(),
        a_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1))
        }
    );
    assert_type!(
        view.types(),
        b_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1))
        }
    );
}

/// Resolve member access on object literal to field type.
#[test]
fn test_analyze_member_access_object_field() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
let obj = { x: 42, y: "hello" };
let a = obj.x;
let b = obj.y;
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile();
}

/// Resolve member access across multiple fields.
#[test]
fn test_analyze_member_access_multiple_fields() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
let obj = { x: 42, y: "hello", z: true };
let a = obj.x;
let b = obj.y;
let c = obj.z;
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// Resolve chained member access on nested objects.
#[test]
fn test_analyze_member_access_chained() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
let obj = { inner: { value: 42 } };
let a = obj.inner.value;
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// Substitute `this` types for member calls.
#[test]
fn test_analyze_this_type_member_call() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Builder { value: int32 }

extension for Builder {
    combine(other: this): this { return other; }
}

let builder = Builder { value: 0 };
let other = Builder { value: 1 };
let result = builder.combine(other);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // resolve symbols
    let builder_symbol = test.resolve_to_symbol("test.ds", "Builder").unwrap();
    let result_symbol = test.resolve_to_symbol("test.ds", "result").unwrap();
    let result_ty_id = view
        .types()
        .get_value_type_id(result_symbol)
        .expect("expected result type");

    assert_type!(view.types(), result_ty_id, Type::Reference { symbol, .. } => {
        assert_eq!(*symbol, builder_symbol);
    });
}

/// Substitute `this` types inside static type arguments.
#[test]
fn test_analyze_this_type_static_argument() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Box<T> { value: T }

struct Builder { value: int32 }

extension for Builder {
    box(): Box<this> { return { value: this }; }
}

let builder = Builder { value: 0 };
let boxed = builder.box();
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // resolve symbols
    let box_symbol = test.resolve_to_symbol("test.ds", "Box").unwrap();
    let builder_symbol = test.resolve_to_symbol("test.ds", "Builder").unwrap();
    let boxed_symbol = test.resolve_to_symbol("test.ds", "boxed").unwrap();
    let boxed_ty_id = view
        .types()
        .get_value_type_id(boxed_symbol)
        .expect("expected boxed type");

    let boxed_ty = view.types().get_type(boxed_ty_id).clone();
    let (boxed_symbol, boxed_arguments) = match boxed_ty {
        Type::Value { value } => match view.types().get_type(value).clone() {
            Type::Reference {
                symbol,
                static_arguments,
            } => (symbol, static_arguments),
            other => panic!("expected boxed reference type, got {other:?}"),
        },
        Type::Reference {
            symbol,
            static_arguments,
        } => (symbol, static_arguments),
        other => panic!("expected boxed value type, got {other:?}"),
    };

    assert_eq!(boxed_symbol, box_symbol);
    let boxed_arguments = boxed_arguments.expect("expected static arguments");
    assert_eq!(boxed_arguments.len(), 1);
    let StaticArgument::Evaluated { value, .. } = &boxed_arguments[0] else {
        panic!("expected evaluated static argument");
    };
    let StaticExpression::Type { ty } = value else {
        panic!("expected static type argument");
    };
    let inner_ty = view.types().get_type(*ty).clone();
    let builder_reference = match inner_ty {
        Type::Reference { symbol, .. } => symbol,
        Type::Value { value } => match view.types().get_type(value).clone() {
            Type::Reference { symbol, .. } => symbol,
            other => panic!("expected builder reference type, got {other:?}"),
        },
        other => panic!("expected builder value type, got {other:?}"),
    };
    assert_eq!(builder_reference, builder_symbol);
}

/// Infer an inherent extension.
#[test]
fn test_analyze_inherent_extension() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Point { 
    x: number, 
    y: number,
}

extension for Point {
    magnitude(): number { 
        return 0; 
    }
}

extension for Point {
    distance(other: Point): number { 
        return 0; 
    }
}

let origin = Point { x: 0, y: 0 };
let magnitude = origin.magnitude();
let distance = origin.distance(origin);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // resolve the canonical target symbol
    let point_symbol = test.canonical_symbol_for_path("test.ds", "Point");

    // verify extension kinds
    let extension_kinds = extension_kinds_for_target(view.types(), point_symbol);
    assert_eq!(extension_kinds.len(), 2);
    assert!(
        extension_kinds
            .iter()
            .all(|kind| *kind == ExtensionKind::Inherent)
    );

    // verify extension method return types
    let magnitude_symbol = test.resolve_to_symbol("test.ds", "magnitude").unwrap();
    let distance_symbol = test.resolve_to_symbol("test.ds", "distance").unwrap();
    let magnitude_ty_id = view
        .types()
        .get_value_type_id(magnitude_symbol)
        .expect("expected magnitude type");
    let distance_ty_id = view
        .types()
        .get_value_type_id(distance_symbol)
        .expect("expected distance type");

    assert_type!(
        view.types(),
        magnitude_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
    assert_type!(
        view.types(),
        distance_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

/// Infer a local extension (on a foreign type).
#[test]
fn test_analyze_local_extension() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    test.add_file(
        "point.ds",
        r#"
export struct Point { 
x: number, 
y: number,
}
"#,
    );
    let module_id = test.add_module(
        "test.ds",
        r#"
import { Point } from "./point.ds";

// local extension on foreign type
extension for Point {
    distance(other: Point): number { return 0; }
}

let origin = Point { x: 0, y: 0 };
let distance = origin.distance(origin);
"#,
    );
    let consumer_id = test.add_module(
        "consumer.ds",
        r#"
import { Point } from "./point.ds";

let origin = Point { x: 0, y: 0 };
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);
    test.analyze_module_and_check_clean(consumer_id);

    // load typed module data
    let view = test.view(module_id);
    let consumer_view = test.view(consumer_id);

    // resolve the canonical target symbol
    let point_symbol = test.canonical_symbol_for_path("test.ds", "Point");
    let consumer_point_symbol = test.canonical_symbol_for_path("consumer.ds", "Point");

    // verify extension kinds in the defining module
    let extension_kinds = extension_kinds_for_target(view.types(), point_symbol);
    assert!(!extension_kinds.is_empty());
    assert!(
        extension_kinds
            .iter()
            .all(|kind| *kind == ExtensionKind::Local)
    );

    // verify extension does not leak into other modules
    let consumer_kinds = extension_kinds_for_target(consumer_view.types(), consumer_point_symbol);
    assert!(consumer_kinds.is_empty());

    // verify extension method return type
    let distance_symbol = test.resolve_to_symbol("test.ds", "distance").unwrap();
    let distance_ty_id = view
        .types()
        .get_value_type_id(distance_symbol)
        .expect("expected distance type");
    assert_type!(
        view.types(),
        distance_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

/// Infer a named extension (on a foreign type, from a foreign extension).
#[test]
fn test_analyze_named_extension() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    test.add_file(
        "point.ds",
        r#"
export struct Point { 
    x: number, 
    y: number,
}
"#,
    );
    test.add_file(
        "extensions.ds",
        r#"
import { Point } from "./point.ds";

export extension PointHelpers for Point {
    distance(): number { return 0; }
}
"#,
    );
    let module_id = test.add_module(
        "test.ds",
        r#"
import { PointHelpers } from "./extensions.ds";
import { Point } from "./point.ds";

declare function getPoint(): Point;

let origin = getPoint();
let distance = origin.distance();
"#,
    );
    let consumer_id = test.add_module(
        "consumer.ds",
        r#"
import { Point } from "./point.ds";

let origin = Point { x: 0, y: 0 };
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);
    test.analyze_module_and_check_clean(consumer_id);

    // load typed module data
    let view = test.view(module_id);
    let consumer_view = test.view(consumer_id);

    // resolve the canonical target symbol
    let point_symbol = test.canonical_symbol_for_path("test.ds", "Point");
    let consumer_point_symbol = test.canonical_symbol_for_path("consumer.ds", "Point");

    // verify extension kinds in the importing module
    let extension_kinds = extension_kinds_for_target(view.types(), point_symbol);
    assert!(!extension_kinds.is_empty());
    assert!(
        extension_kinds
            .iter()
            .all(|kind| *kind == ExtensionKind::Nominal)
    );

    // verify extension does not appear without an import
    let consumer_kinds = extension_kinds_for_target(consumer_view.types(), consumer_point_symbol);
    assert!(consumer_kinds.is_empty());

    // verify extension method return type
    let distance_symbol = test.resolve_to_symbol("test.ds", "distance").unwrap();
    let distance_ty_id = view
        .types()
        .get_value_type_id(distance_symbol)
        .expect("expected distance type");
    assert_type!(
        view.types(),
        distance_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

/// Analyze infer parameter types from call arguments.
#[test]
fn test_analyze_infer_parameter_types_from_call_arguments() {
    // infers parameter types from call arguments
    let test = TestProgram::memory_sequential();
    test.add_package("test", Some(r#""noImplicitAny": false"#));
    let module_id = test.add_module(
        "test.ds",
        r#"
function add(a, b) {
    return a + b
}
add(1, 2)
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the function type
    let fn_symbol = test.expect_first_function_symbol(module_id);
    let fn_ty_id = view
        .types()
        .get_value_type_id(fn_symbol)
        .expect("expected function type");

    // function type uses literal argument types and a number return type
    assert_type!(view.types(), fn_ty_id, Type::Function { dynamic_parameters, return_type, .. } => {
        // two parameters inferred from call arguments
        assert_eq!(dynamic_parameters.len(), 2);

        // first parameter matches literal 1
        assert_type!(view.types(), dynamic_parameters[0], Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1))
        });

        // second parameter matches literal 2
        assert_type!(view.types(), dynamic_parameters[1], Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(2))
        });

        // return type is number from arithmetic
        let return_type = return_type.expect("expected return type");
        assert_type!(view.types(), return_type, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });
    });
}

/// Analyze infer parameter type from default.
#[test]
fn test_analyze_infer_parameter_type_from_default() {
    // infers parameter type from default value
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function greet(name = "hi") {
    return name
}
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the function type
    let fn_symbol = test.expect_first_function_symbol(module_id);
    let fn_ty_id = view
        .types()
        .get_value_type_id(fn_symbol)
        .expect("expected function type");

    // function type uses default value literal and returns a string
    assert_type!(view.types(), fn_ty_id, Type::Function { dynamic_parameters, return_type, .. } => {
        // one parameter inferred from default value
        assert_eq!(dynamic_parameters.len(), 1);

        // parameter uses the default string literal type
        assert_type!(view.types(), dynamic_parameters[0], Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
        });

        // return type follows the parameter type
        let return_type = return_type.expect("expected return type");
        assert_type!(view.types(), return_type, Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
        });
    });
}

/// Analyze static type arguments on call.
#[test]
fn test_analyze_static_type_arguments_on_call() {
    // resolves explicit static type arguments at call sites
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function identity<T>(value: T): T {
    return value
}
identity<number>(1);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the call expression
    let call_expression_id = view.root_expression_id(1);
    let call_ty_id = view.expect_inferred_type_id(call_expression_id);

    // call expression uses explicit number type argument
    assert_type!(
        view.types(),
        call_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

/// Analyze static type arguments inferred.
#[test]
fn test_analyze_static_type_arguments_inferred() {
    // infers static type arguments from call arguments
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function identity<T>(value: T): T {
    return value
}
let one = identity(1);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the initializer expression
    let expression_id = view.root_expression_id(1);
    let let_expression = view.tree().get(expression_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();
    let declarator = view.tree().get(*declarator_id);
    let value_id = declarator.value.expect("expected initializer value");
    let value_ty_id = view.expect_inferred_type_id(value_id);

    // identity returns the literal type of the inferred argument
    assert_type!(
        view.types(),
        value_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1))
        }
    );
}

/// Analyze static type arguments on reference.
#[test]
fn test_analyze_static_type_arguments_on_reference() {
    // resolves explicit static arguments on function references
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function identity<T>(value: T): T {
    return value
}
let as_number = identity<number>;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the initializer expression
    let expression_id = view.root_expression_id(1);
    let let_expression = view.tree().get(expression_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();
    let declarator = view.tree().get(*declarator_id);
    let value_id = declarator.value.expect("expected initializer value");
    let value_ty_id = view.expect_inferred_type_id(value_id);

    // specialized function reference uses number parameter and return type
    assert_type!(view.types(), value_ty_id, Type::Function { dynamic_parameters, return_type, .. } => {
        // single parameter is number
        assert_eq!(dynamic_parameters.len(), 1);

        // parameter uses the explicit number type argument
        assert_type!(view.types(), dynamic_parameters[0], Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });

        // return type matches the parameter
        let return_type = return_type.expect("expected return type");
        assert_type!(view.types(), return_type, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });
    });
}

/// Analyze type reference instance.
#[test]
fn test_analyze_type_reference_instance() {
    // registers instances for type reference annotations
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Box<T> {
    value: T
}

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the type annotation
    let declarator_id = test.expect_first_let_declarator(module_id);
    let declarator = view.tree().get(declarator_id);
    let type_expression_id = declarator.ty.expect("expected type annotation");

    // resolve the instance
    let instance_id = view
        .types()
        .get_instance_for_node(type_expression_id.into_global_any(module_id))
        .expect("expected instance");
    let instance = view.types().get_instance(instance_id);

    let box_symbol = test.resolve_to_symbol("test.ds", "Box").unwrap();

    // instance targets Box with a single static type argument
    assert_eq!(instance.symbol_id, box_symbol);
    assert_eq!(instance.static_arguments.len(), 1);

    // static argument is the explicit number type
    match &instance.static_arguments[0] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::Type { ty } => {
                assert_type!(
                    view.types(),
                    *ty,
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::Number)
                    }
                );
            }
            _ => panic!("expected type argument"),
        },
        _ => panic!("expected evaluated argument"),
    }
}

/// Analyze type reference default static value.
#[test]
fn test_analyze_type_reference_default_static_value() {
    // applies default static value arguments for type references
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Buffer<T, comptime N: number = 4> {
    value: T
}

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
"#,
    );
    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the type annotation
    let declarator_id = test.expect_first_let_declarator(module_id);
    let declarator = view.tree().get(declarator_id);
    let type_expression_id = declarator.ty.expect("expected type annotation");

    // resolve the instance
    let instance_id = view
        .types()
        .get_instance_for_node(type_expression_id.into_global_any(module_id))
        .expect("expected instance");
    let instance = view.types().get_instance(instance_id);

    let buffer_symbol = test.resolve_to_symbol("test.ds", "Buffer").unwrap();

    // instance targets Buffer with explicit T and default N value
    assert_eq!(instance.symbol_id, buffer_symbol);
    assert_eq!(instance.static_arguments.len(), 2);

    // first static argument is the explicit string type
    match &instance.static_arguments[0] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::Type { ty } => {
                assert_type!(
                    view.types(),
                    *ty,
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::String)
                    }
                );
            }
            _ => panic!("expected type argument"),
        },
        _ => panic!("expected evaluated argument"),
    }

    // second static argument is the default value N = 4
    match &instance.static_arguments[1] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::ScalarLiteral { value } => {
                assert_eq!(*value, ScalarLiteral::Integer(4));
            }
            _ => panic!("expected scalar literal argument"),
        },
        _ => panic!("expected evaluated argument"),
    }
}

/// Analyze static type arguments that are type parameters.
#[test]
fn test_analyze_static_type_argument_parameter_constraint() {
    // type parameter arguments satisfy parameter bounds
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Node {}
interface Box<T extends Node> {
    value: T
}

type Use<T extends Node> = Box<T>;

declare let value: Use<Node>;
let boxed: Box<Node> = value;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);
    let profile = view.profile_id();

    // locate declarators by name
    let value_name = test.program.strings.intern("value");
    let boxed_name = test.program.strings.intern("boxed");
    let value_declarator_id = view.expect_let_declarator(value_name);
    let boxed_declarator_id = view.expect_let_declarator(boxed_name);

    // inspect instance arguments for Use<Node>
    let value_declarator = view.tree().get(value_declarator_id);
    let value_type_expression_id = value_declarator.ty.expect("expected value type annotation");
    let use_instance_id = view
        .types()
        .get_instance_for_node(value_type_expression_id.into_global_any(module_id))
        .expect("expected Use instance");
    let use_instance = view.types().get_instance(use_instance_id);
    let use_symbol = test.resolve_to_symbol("test.ds", "Use").unwrap();
    let node_symbol = test.resolve_to_symbol("test.ds", "Node").unwrap();

    // instance targets Use with Node as the static argument
    assert_eq!(use_instance.symbol_id, use_symbol);
    assert_eq!(use_instance.static_arguments.len(), 1);
    match &use_instance.static_arguments[0] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::Type { ty } => {
                assert_type!(
                    view.types(),
                    *ty,
                    Type::Reference {
                        symbol,
                        static_arguments
                    } => {
                        assert_eq!(*symbol, node_symbol);
                        assert!(static_arguments.is_none());
                    }
                );
            }
            _ => panic!("expected type argument"),
        },
        _ => panic!("expected evaluated argument"),
    }

    // verify Use<Node> is assignable to Box<Node>
    let value_type_id = view
        .types()
        .get_declared_type_id(value_declarator_id.into_global(module_id).into())
        .expect("expected value declared type");
    let boxed_type_id = view
        .types()
        .get_declared_type_id(boxed_declarator_id.into_global(module_id).into())
        .expect("expected boxed declared type");

    // compare assignability using the module profile
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let options = test.compiler.analyze_context_options_for_module(module.id);
    let symbols = view.symbols().clone();
    let mut types = view.types().clone();
    let assignable = test.compiler.is_type_assignable(
        &module,
        profile,
        &symbols,
        boxed_type_id,
        value_type_id,
        &mut types,
        &options,
    );
    assert!(assignable.is_assignable());
}

/// Analyze index access on constrained type parameters.
#[test]
fn test_analyze_index_access_uses_parameter_constraint() {
    // index access respects the parameter constraint type
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Map {
    foo: number
}

type Pick<K extends keyof Map> = Map[K];

declare let value: Pick<"foo">;
let number_value: number = value;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);
    let profile = view.profile_id();

    // locate declarators by name
    let value_name = test.program.strings.intern("value");
    let number_value_name = test.program.strings.intern("number_value");
    let value_declarator_id = view.expect_let_declarator(value_name);
    let number_declarator_id = view.expect_let_declarator(number_value_name);

    // inspect instance arguments for Pick<"foo">
    let value_declarator = view.tree().get(value_declarator_id);
    let value_type_expression_id = value_declarator.ty.expect("expected value type annotation");
    let pick_instance_id = view
        .types()
        .get_instance_for_node(value_type_expression_id.into_global_any(module_id))
        .expect("expected Pick instance");
    let pick_instance = view.types().get_instance(pick_instance_id);
    let pick_symbol = test.resolve_to_symbol("test.ds", "Pick").unwrap();
    let foo_name = test.program.strings.intern("foo");

    // instance targets Pick with a string literal argument
    assert_eq!(pick_instance.symbol_id, pick_symbol);
    assert_eq!(pick_instance.static_arguments.len(), 1);
    match &pick_instance.static_arguments[0] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::ScalarLiteral { value } => {
                assert_eq!(*value, ScalarLiteral::String(foo_name));
            }
            StaticExpression::Type { ty } => {
                assert_type!(
                    view.types(),
                    *ty,
                    Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(name))
                    } => {
                        assert_eq!(*name, foo_name);
                    }
                );
            }
            _ => panic!("expected scalar literal argument"),
        },
        _ => panic!("expected evaluated argument"),
    }

    // verify Pick<"foo"> is assignable to number
    let value_type_id = view
        .types()
        .get_declared_type_id(value_declarator_id.into_global(module_id).into())
        .expect("expected value declared type");
    let number_type_id = view
        .types()
        .get_declared_type_id(number_declarator_id.into_global(module_id).into())
        .expect("expected number_value declared type");

    // compare assignability using the module profile
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let options = test.compiler.analyze_context_options_for_module(module.id);
    let symbols = view.symbols().clone();
    let mut types = view.types().clone();
    let assignable = test.compiler.is_type_assignable(
        &module,
        profile,
        &symbols,
        number_type_id,
        value_type_id,
        &mut types,
        &options,
    );
    assert!(assignable.is_assignable());
}

/// Analyze tuple index access on constrained type parameters.
#[test]
fn test_analyze_tuple_index_access_uses_parameter_constraint() {
    // tuple index access respects the parameter constraint type
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
type Element<Types extends [boolean, boolean]> = Types[0];

declare let value: Element<[true, false]>;
let ok: boolean = value;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);
    let profile = view.profile_id();

    // locate declarators by name
    let value_name = test.program.strings.intern("value");
    let ok_name = test.program.strings.intern("ok");
    let value_declarator_id = view.expect_let_declarator(value_name);
    let ok_declarator_id = view.expect_let_declarator(ok_name);

    // compare assignability using the module profile
    let value_type_id = view
        .types()
        .get_declared_type_id(value_declarator_id.into_global(module_id).into())
        .expect("expected value declared type");
    let ok_type_id = view
        .types()
        .get_declared_type_id(ok_declarator_id.into_global(module_id).into())
        .expect("expected ok declared type");
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let options = test.compiler.analyze_context_options_for_module(module.id);
    let symbols = view.symbols().clone();
    let mut types = view.types().clone();
    let assignable = test.compiler.is_type_assignable(
        &module,
        profile,
        &symbols,
        ok_type_id,
        value_type_id,
        &mut types,
        &options,
    );
    assert!(assignable.is_assignable());
}

/// Analyze builtin Pick mapped types.
#[test]
fn test_analyze_builtin_pick_optional_shape() {
    // Pick preserves optional fields for literal keys.
    let test = TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&["es5"]);
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Person {
    name: string
    age?: number
}

type AgeOnly = Pick<Person, "age">;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);
    let profile = view.profile_id();
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let age_symbol = test
        .resolve_to_symbol("test.ds", "AgeOnly")
        .expect("expected AgeOnly symbol");
    let instance_id = view
        .types()
        .get_instance_type_id(age_symbol)
        .expect("expected AgeOnly instance type");
    let pick_symbol = match view.types().get_type(instance_id) {
        Type::Reference { symbol, .. } => *symbol,
        other => panic!("expected Pick reference, got {other:?}"),
    };

    // verify the builtin alias target is declared
    let pick_module = test.program.modules.get(pick_symbol.module_id);
    let pick_module = pick_module.read();
    let pick_types = pick_module.dir(profile).types.read();
    let _ = pick_types
        .get_alias_target_type_id(pick_symbol)
        .expect("expected Pick alias target type");

    // normalize the alias instance into an object shape
    let mut types = view.types().clone();
    let normalized = test.compiler.normalize_type(
        &module,
        profile,
        instance_id,
        view.symbols(),
        &mut types,
        NormalizationMode::Assign,
    );
    // verify the mapped field preserves optionality
    let age_name = test.program.strings.intern("age");
    assert_type!(types, normalized, Type::Object { fields, .. } => {
        let age_field = fields
            .iter()
            .find(|field| field.key.matches(&StaticKey::Name(age_name)))
            .expect("expected age field");
        assert!(age_field.is_optional, "expected optional age field");
    });
}

/// Analyze builtin Omit mapped types.
// FUGU #Broken: phase 4 mapped types and utility types
#[ignore]
#[test]
fn test_analyze_builtin_omit_shape() {
    // Omit removes the specified keys from the source shape.
    let test = TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&["es5"]);
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Person {
    name: string
    age: number
}

type WithoutAge = Omit<Person, "age">;
type ExcludedKeys = Exclude<keyof Person, "age">;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);
    let profile = view.profile_id();
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let mut types = view.types().clone();
    let without_age_symbol = test
        .resolve_to_symbol("test.ds", "WithoutAge")
        .expect("expected WithoutAge symbol");
    let instance_id = view
        .types()
        .get_instance_type_id(without_age_symbol)
        .expect("expected WithoutAge instance type");
    let excluded_keys_symbol = test
        .resolve_to_symbol("test.ds", "ExcludedKeys")
        .expect("expected ExcludedKeys symbol");
    let excluded_keys_id = view
        .types()
        .get_instance_type_id(excluded_keys_symbol)
        .expect("expected ExcludedKeys instance type");
    let exclude_arguments = match types.get_type(excluded_keys_id) {
        Type::Reference {
            static_arguments: Some(arguments),
            ..
        } => arguments.clone(),
        _ => panic!("expected ExcludedKeys to be a reference"),
    };
    assert_eq!(exclude_arguments.len(), 2, "expected Exclude arguments");
    let exclude_left = match &exclude_arguments[0] {
        StaticArgument::Evaluated {
            value: StaticExpression::Type { ty },
            ..
        } => *ty,
        _ => panic!("expected Exclude left argument to be a type"),
    };
    assert_type!(types, exclude_left, Type::Unary {
        operator: TypeUnaryOperator::Keyof,
        right,
    } => {
        assert_type!(types, *right, Type::Reference { .. });
    });
    let exclude_right = match &exclude_arguments[1] {
        StaticArgument::Evaluated {
            value: StaticExpression::Type { ty },
            ..
        } => *ty,
        _ => panic!("expected Exclude right argument to be a type"),
    };
    assert_type!(
        types,
        exclude_right,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_)),
        }
    );
    let es5_module = test.module("builtin://lib/es/es5/index.d.ts");
    let es5_module = es5_module.read();
    let es5_profile = test.default_profile_id(es5_module.id);
    let es5_dir = es5_module.dir(es5_profile);
    let es5_symbols = es5_dir.symbols.read();
    let es5_types = es5_dir.types.read();
    let exclude_definition_symbol = test
        .resolve_to_symbol("builtin://lib/es/es5/index.d.ts", "Exclude")
        .expect("expected Exclude symbol");
    let exclude_definition_symbol = test.compiler.typed_symbol_id(
        &es5_module,
        es5_profile,
        exclude_definition_symbol,
        &es5_symbols,
    );
    let exclude_alias = es5_types
        .get_alias_target_type_id(exclude_definition_symbol)
        .expect("expected Exclude alias target");
    assert_type!(es5_types, exclude_alias, Type::Conditional { distributive_symbol, .. } => {
        assert!(
            distributive_symbol.is_some(),
            "expected Exclude conditional to be distributive"
        );
    });

    // normalize the alias instance into an object shape
    let normalized = test.compiler.normalize_type(
        &module,
        profile,
        instance_id,
        view.symbols(),
        &mut types,
        NormalizationMode::Assign,
    );
    let excluded_keys_normalized = test.compiler.normalize_type(
        &module,
        profile,
        excluded_keys_id,
        view.symbols(),
        &mut types,
        NormalizationMode::Assign,
    );

    // verify the omitted key is removed
    let name_id = test.program.strings.intern("name");
    let age_id = test.program.strings.intern("age");
    // collect excluded keys for assertion
    let mut keys = Vec::new();
    match types.get_type(excluded_keys_normalized) {
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(string_id)),
        } => {
            let key = test.program.strings.get(*string_id);
            keys.push(key.to_string());
        }
        Type::Union { elements } => {
            for element_id in elements {
                assert_type!(types, *element_id, Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(string_id)),
                } => {
                    let key = test.program.strings.get(*string_id);
                    keys.push(key.to_string());
                });
            }
        }
        other => panic!("expected excluded keys to be string literal(s), got {other:?}"),
    }
    keys.sort();
    assert_eq!(keys, vec!["name"], "expected excluded keys to omit age");
    assert_type!(types, normalized, Type::Object { fields, .. } => {
        let has_name = fields.iter().any(|field| field.key.matches(&StaticKey::Name(name_id)));
        let has_age = fields.iter().any(|field| field.key.matches(&StaticKey::Name(age_id)));
        assert!(has_name, "expected name field");
        assert!(!has_age, "expected age field to be omitted");
    });
}

/// Analyze member instance inherited static arguments.
#[test]
fn test_analyze_member_instance_inherited_static_arguments() {
    // registers inherited static arguments for member instances
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Container<T> {
    map<U>(value: T): U
}

declare function getContainer(): Container<number>;

let result = getContainer().map<string>(1);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the initializer expression
    let declarator_id = test.expect_first_let_declarator(module_id);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer value");

    // resolve the member instance
    let instance_id = view
        .types()
        .get_instance_for_node(value_id.into_global_any(module_id))
        .expect("expected instance");
    let instance = view.types().get_instance(instance_id);

    // resolve the member symbol
    let map_name = test.program.strings.intern("map");
    let container_symbol = test.resolve_to_symbol("test.ds", "Container").unwrap();
    let map_symbol = test.expect_interface_member_symbol(module_id, container_symbol, map_name);

    // instance targets Container.map with inherited T and explicit U
    assert_eq!(instance.symbol_id, map_symbol);
    assert_eq!(instance.static_arguments.len(), 2);

    // first static argument is inherited Container T = number
    match &instance.static_arguments[0] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::Type { ty } => {
                assert_type!(
                    view.types(),
                    *ty,
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::Number)
                    }
                );
            }
            _ => panic!("expected type argument"),
        },
        _ => panic!("expected evaluated argument"),
    }

    // second static argument is explicit U = string
    match &instance.static_arguments[1] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::Type { ty } => {
                assert_type!(
                    view.types(),
                    *ty,
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::String)
                    }
                );
            }
            _ => panic!("expected type argument"),
        },
        _ => panic!("expected evaluated argument"),
    }
}

/// Analyze member instance from member expression.
#[test]
fn test_analyze_member_instance_from_member_expression() {
    // registers static arguments on member expressions
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Container<T> {
    map<U>(value: T): U
}

declare function getContainer(): Container<number>;

let mapper = getContainer().map<string>;
mapper(1);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the member expression initializer
    let declarator_id = test.expect_first_let_declarator(module_id);
    let declarator = view.tree().get(declarator_id);
    let value_id = declarator.value.expect("expected initializer value");

    // resolve the member instance
    let instance_id = view
        .types()
        .get_instance_for_node(value_id.into_global_any(module_id))
        .expect("expected instance");
    let instance = view.types().get_instance(instance_id);

    // resolve the member symbol
    let map_name = test.program.strings.intern("map");
    let container_symbol = test.resolve_to_symbol("test.ds", "Container").unwrap();
    let map_symbol = test.expect_interface_member_symbol(module_id, container_symbol, map_name);

    // instance targets Container.map with inherited T and explicit U
    assert_eq!(instance.symbol_id, map_symbol);
    assert_eq!(instance.static_arguments.len(), 2);

    // first static argument is inherited Container T = number
    match &instance.static_arguments[0] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::Type { ty } => {
                assert_type!(
                    view.types(),
                    *ty,
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::Number)
                    }
                );
            }
            _ => panic!("expected type argument"),
        },
        _ => panic!("expected evaluated argument"),
    }

    // second static argument is explicit U = string
    match &instance.static_arguments[1] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::Type { ty } => {
                assert_type!(
                    view.types(),
                    *ty,
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::String)
                    }
                );
            }
            _ => panic!("expected type argument"),
        },
        _ => panic!("expected evaluated argument"),
    }
}

/// Analyze substitute type reference arguments.
#[test]
fn test_analyze_substitute_type_reference_arguments() {
    // substitutes type arguments inside type references during inference
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Box<T> { value: T }

declare function wrap<T>(value: T): Box<T>;

let result = wrap(1);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // resolve the result symbol
    let result_symbol = test.resolve_to_symbol("test.ds", "result").unwrap();

    // load typed module data
    let view = test.view(module_id);

    // resolve the result type
    let result_ty_id = view
        .types()
        .get_value_type_id(result_symbol)
        .expect("expected result type");
    let box_symbol = test.resolve_to_symbol("test.ds", "Box").unwrap();

    // result type is Box with a literal type argument
    assert_type!(
        view.types(),
        result_ty_id,
        Type::Reference {
            symbol,
            static_arguments,
        } => {
            // type reference targets Box
            assert_eq!(*symbol, box_symbol);
            let static_arguments = static_arguments.as_ref().expect("expected arguments");

            // type reference carries one static argument
            assert_eq!(static_arguments.len(), 1);

            // static argument is the inferred literal type
            match &static_arguments[0] {
                StaticArgument::Evaluated { value, .. } => match value {
                    StaticExpression::Type { ty } => {
                        assert_type!(
                            view.types(),
                            *ty,
                            Type::TypeLiteral {
                                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1))
                            }
                        );
                    }
                    _ => panic!("expected type argument"),
                },
                _ => panic!("expected evaluated argument"),
            }
        }
    );
}

/// Analyze contextual lambda from annotation.
#[test]
fn test_analyze_contextual_lambda_from_annotation() {
    // infers lambda signature from contextual function type
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
const add: (a: number, b: number) => number = (a, b) => a + b;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // select the root expression
    let expression_id = view.root_expression_id(0);
    let let_expression = view.tree().get(expression_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();
    let declarator = view.tree().get(*declarator_id);
    let value_id = declarator.value.expect("expected function value");
    let value_ty_id = view.expect_inferred_type_id(value_id);

    // contextual annotation yields number, number to number
    assert_type!(view.types(), value_ty_id, Type::Function { dynamic_parameters, return_type, .. } => {
        // two parameters inherited from annotation
        assert_eq!(dynamic_parameters.len(), 2);

        // first parameter is number
        assert_type!(view.types(), dynamic_parameters[0], Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });

        // second parameter is number
        assert_type!(view.types(), dynamic_parameters[1], Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });

        // return type is number
        let return_type = return_type.expect("expected return type");
        assert_type!(view.types(), return_type, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });
    });
}

/// Analyze contextual lambda from argument.
#[test]
fn test_analyze_contextual_lambda_from_argument() {
    // infers lambda signature from argument type
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function apply(fn: (a: number) => number): number {
    return fn(1);
}
apply((a) => a + 1);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the call expression
    let call_expression_id = view.root_expression_id(1);
    let call_expression = view.tree().get(call_expression_id);
    let Expression::Call {
        dynamic_arguments, ..
    } = call_expression
    else {
        panic!("expected call expression");
    };

    // resolve the argument type
    let argument_id = dynamic_arguments.first().expect("expected argument");
    let argument = view.tree().get(*argument_id);
    let argument_value_id = argument.value();
    let argument_ty_id = view.expect_inferred_type_id(argument_value_id);

    // contextual argument yields number to number function type
    assert_type!(view.types(), argument_ty_id, Type::Function { dynamic_parameters, return_type, .. } => {
        // one parameter inherited from argument context
        assert_eq!(dynamic_parameters.len(), 1);

        // parameter is number
        assert_type!(view.types(), dynamic_parameters[0], Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });

        // return type is number
        let return_type = return_type.expect("expected return type");
        assert_type!(view.types(), return_type, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });
    });
}

/// Analyze contextual array literal.
#[test]
fn test_analyze_contextual_array_literal() {
    // infers array literal element types from contextual type
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
const numbers: number[] = [1, 2];
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // select the root expression
    let expression_id = view.root_expression_id(0);
    let let_expression = view.tree().get(expression_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();
    let declarator = view.tree().get(*declarator_id);
    let value_id = declarator.value.expect("expected array value");
    let value_ty_id = view.expect_inferred_type_id(value_id);

    // array element type is number from number[] context
    assert_type!(view.types(), value_ty_id, Type::Array { element: Some(element_id), .. } => {
        assert_type!(view.types(), *element_id, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });
    });
}

// Binds mapped type parameters for use in value type.
// FUGU #Broken: phase 4 mapped types parameter scope
#[ignore]
#[test]
fn test_analyze_type_mapped_parameter_scope() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "type Map<T> = { [K in keyof T]: T[K] };");

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // resolve the mapped type symbol
    let map_symbol = test
        .resolve_to_symbol("test.ds", "Map")
        .expect("expected Map symbol");
    let map_instance_id = view
        .types()
        .get_instance_type_id(map_symbol)
        .expect("expected Map instance type");

    assert_type!(view.types(), map_instance_id, Type::Mapped { parameter, value, .. } => {
        assert_string!(test.program, parameter.name, "K");
        assert_type!(view.types(), parameter.constraint, Type::Unary { operator: TypeUnaryOperator::Keyof, right } => {
            assert_type!(view.types(), *right, Type::Reference { symbol, static_arguments } => {
                assert!(static_arguments.is_none());
                let symbol = view.symbols().get_symbol(symbol.into_local());
                assert_string!(test.program, symbol.name().expect("expected symbol name"), "T");
            });
        });
        assert_type!(view.types(), *value, Type::Index { left, index } => {
            assert_type!(view.types(), *left, Type::Reference { symbol, .. } => {
                let symbol = view.symbols().get_symbol(symbol.into_local());
                assert_string!(test.program, symbol.name().expect("expected symbol name"), "T");
            });
            assert_type!(view.types(), *index, Type::Reference { symbol, .. } => {
                let symbol = view.symbols().get_symbol(symbol.into_local());
                assert_eq!(symbol.kind, SymbolKind::Local);
                assert_string!(test.program, symbol.name().expect("expected symbol name"), "K");
            });
        });
    });
}

/// Bind infer variables for use in conditional true branch.
// FUGU #Broken: phase 7 conditional infer scope
#[ignore]
#[test]
fn test_analyze_type_infer_scope() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "type Foo<T> = T extends infer U ? U : never;");

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // resolve the conditional type symbol
    let foo_symbol = test
        .resolve_to_symbol("test.ds", "Foo")
        .expect("expected Foo symbol");
    let foo_instance_id = view
        .types()
        .get_instance_type_id(foo_symbol)
        .expect("expected Foo instance type");

    assert_type!(
        view.types(),
        foo_instance_id,
        Type::Conditional {
            distributive_symbol,
            left,
            right,
            then_type,
            else_type
        } => {
        assert!(distributive_symbol.is_some());
        assert_type!(view.types(), *left, Type::Reference { symbol, .. } => {
            let symbol = view.symbols().get_symbol(symbol.into_local());
            assert_string!(test.program, symbol.name().expect("expected symbol name"), "T");
        });
        assert_type!(view.types(), *right, Type::Infer { name, constraint } => {
            assert_string!(test.program, *name, "U");
            assert!(constraint.is_none());
        });
        assert_type!(view.types(), *then_type, Type::Reference { symbol, .. } => {
            let symbol = view.symbols().get_symbol(symbol.into_local());
            assert_eq!(symbol.kind, SymbolKind::Local);
            assert_string!(test.program, symbol.name().expect("expected symbol name"), "U");
        });
        assert_type!(view.types(), *else_type, Type::TypeLiteral { value: TypeLiteral::Never });
    });
}

/// Build a flow graph with true and false branches for if expressions.
#[test]
fn test_build_flow_graph_if_expression() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        "const value = true; if (value) { 1 } else { 2 };",
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load tree data
    let view = test.view(module_id);

    // locate the if expression
    let if_expression_id = view
        .roots()
        .iter()
        .find_map(|root_id| match view.tree().get(*root_id) {
            Expression::If { .. } => Some(*root_id),
            Expression::Statement { statement } => match view.tree().get(*statement) {
                Expression::If { .. } => Some(*statement),
                _ => None,
            },
            _ => None,
        })
        .expect("expected if expression");

    // build the flow graph
    let graph = FlowGraphBuilder::new(module_id, view.tree()).build(if_expression_id);

    let mut has_true_edge = false;
    let mut has_false_edge = false;
    let mut has_join_block = false;
    for block in &graph.blocks {
        if block.predecessors.len() >= 2 {
            has_join_block = true;
        }
        for edge in &block.successors {
            if edge.kind == FlowEdgeKind::True {
                has_true_edge = true;
            }
            if edge.kind == FlowEdgeKind::False {
                has_false_edge = true;
            }
        }
    }

    assert!(has_true_edge);
    assert!(has_false_edge);
    assert!(has_join_block);
}

/// Build a flow graph that narrows the right side of short circuit guards.
#[test]
fn test_build_flow_graph_short_circuit_guard() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
        const value: string | null = null;
        const accepts_string = (input: string): boolean => true;
        if (value !== null && accepts_string(value)) { };
        "#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load tree data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let profile = test.default_profile_id(module_id);
    let dir = module.dir(profile);
    let tree = dir.tree.read();
    let symbols = dir.symbols.read();
    let mut types = dir.types.write();

    // locate the if expression
    let if_expression_id = dir
        .roots
        .iter()
        .find_map(|root_id| match tree.get(*root_id) {
            Expression::If { .. } => Some(*root_id),
            Expression::Statement { statement } => match tree.get(*statement) {
                Expression::If { .. } => Some(*statement),
                _ => None,
            },
            _ => None,
        })
        .expect("expected if expression");

    // locate the right side of the condition
    let Expression::If { condition, .. } = tree.get(if_expression_id) else {
        panic!("expected if expression");
    };
    let condition_id = match condition {
        IfCondition::Expression { condition } => *condition,
        IfCondition::Let { .. } => panic!("expected expression condition"),
    };
    let Expression::Binary {
        left,
        operator,
        right,
    } = tree.get(condition_id)
    else {
        panic!("expected binary condition");
    };
    assert_eq!(*operator, BinaryOperator::And);

    let Expression::Binary { left, .. } = tree.get(*left) else {
        panic!("expected binary left guard");
    };

    let Expression::Call {
        dynamic_arguments, ..
    } = tree.get(*right)
    else {
        panic!("expected call expression");
    };
    let argument_id = dynamic_arguments
        .first()
        .copied()
        .expect("expected call argument");
    let argument = tree.get(argument_id);
    let argument_value_id = argument.value();

    // ensure the right side reference uses a distinct node id
    assert_ne!(*left, argument_value_id);

    // build flow data for the condition expression
    // build the flow graph
    let graph = FlowGraphBuilder::new(module.id, &tree).build(condition_id);
    let mut infer = InferTable::default();
    let context = InferContext::new(
        profile,
        AnalyzeOptions::from(&DsConfigCompilerOptions::default()),
    );
    let flow = test
        .compiler
        .compute_flow_table_for_graph(
            &module, &graph, &tree, &symbols, &mut types, &mut infer, &context,
        )
        .expect("expected flow table");

    // read the flow environment for the right side argument
    let block_id = graph
        .block_by_node
        .get(&argument_value_id.into_any())
        .copied()
        .expect("expected block for argument");
    let environment_id = flow
        .entry_environment_for_block(block_id)
        .expect("expected entry environment");
    let environment = flow
        .environment(environment_id)
        .expect("expected environment");

    // assert that the value symbol is narrowed to a non null type
    let value_symbol = test
        .resolve_to_symbol("test.ds", "value")
        .expect("expected value symbol");
    let narrowed_type_id = environment
        .bindings
        .get(&value_symbol)
        .copied()
        .expect("expected narrowing for value");
    let is_null = match types.get_type(narrowed_type_id) {
        Type::TypeLiteral {
            value: TypeLiteral::Null,
        } => true,
        Type::Union { elements } => elements.iter().any(|element_id| {
            matches!(
                types.get_type(*element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Null
                }
            )
        }),
        _ => false,
    };
    assert!(!is_null);
}

/// Build a flow graph with a back edge for for loops.
#[test]
fn test_build_flow_graph_for_loop() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
        function run() {
            for (let i: number = 0; i < 3; i = i + 1) {
                i = i + 1;
            }
        }
        "#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load tree data
    let view = test.view(module_id);

    // locate the first function body
    let function_body_id = view
        .roots()
        .iter()
        .find_map(|root_id| match view.tree().get(*root_id) {
            Expression::Declaration { declaration } => match view.tree().get(*declaration) {
                Declaration::Function {
                    body: Some(body), ..
                } => Some(*body),
                _ => None,
            },
            Expression::Statement { statement } => match view.tree().get(*statement) {
                Expression::Declaration { declaration } => match view.tree().get(*declaration) {
                    Declaration::Function {
                        body: Some(body), ..
                    } => Some(*body),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        })
        .expect("expected function body");

    // build the flow graph
    let graph = FlowGraphBuilder::new(module_id, view.tree()).build(function_body_id);

    let mut has_true_edge = false;
    let mut has_false_edge = false;
    let mut has_back_edge = false;
    for block in &graph.blocks {
        for edge in &block.successors {
            if edge.kind == FlowEdgeKind::True {
                has_true_edge = true;
            }
            if edge.kind == FlowEdgeKind::False {
                has_false_edge = true;
            }
            if edge.target.0 < block.id.0 {
                has_back_edge = true;
            }
        }
    }

    assert!(has_true_edge);
    assert!(has_false_edge);
    assert!(has_back_edge);
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
    let enum_module = enum_module.read();

    // confirm the binding value type is the nominal enum reference
    let status_symbol = test.compiler.canonical_symbol_id(
        &enum_module,
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
            static_arguments
        } => {
            assert_eq!(*symbol, enum_symbol);
            assert!(static_arguments.is_none());
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
            static_arguments
        } => {
            assert_eq!(*symbol, enum_symbol);
            assert!(static_arguments.is_none());
        }
    );
    let active_field_ty = active_field.ty;

    // confirm enums are not implicitly assignable to their backing type
    let options = test.compiler.analyze_context_options_for_module(module_id);
    let symbols = view.symbols().clone();
    let mut types = view.types().clone();
    let int32_ty_id = types.insert_type_from_any(
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32)),
        },
        value_id.into_any(),
    );
    let assignable = test.compiler.is_type_assignable(
        &enum_module,
        view.profile_id(),
        &symbols,
        int32_ty_id,
        active_field_ty,
        &mut types,
        &options,
    );
    assert!(!assignable.is_assignable());
}

// static value arguments accept imported constants
#[test]
fn test_analyze_static_value_argument_imported_constant() {
    let test = TestProgram::memory_sequential();
    let _utils_id = test.add_module("utils.ds", "export const SIZE = 4;");
    let main_id = test.add_module(
        "main.ds",
        r#"
import { SIZE } from "./utils.ds";

type Buffer<comptime N: number> = uint8[N];

declare let value: Buffer<SIZE>;
"#,
    );

    test.analyze_module(main_id);
    test.compile();

    test.with_dir_types_mut(main_id, |module, profile, dir, tree, symbols, types| {
        let key = StaticKey::Name(test.program.strings.intern("SIZE"));
        let namespace_scope = symbols.get_scope_by_id(dir.namespace_scope);
        let symbol_id = symbols
            .find_active_symbol_up_to(namespace_scope, key, LocalScopeMark::end())
            .map(|symbol| symbol.into_global(module.id))
            .expect("expected imported SIZE symbol");

        let mut expression_id = None;
        for argument_id in tree.iter_node_ids_of_type::<destack_dir::Argument>() {
            let argument = tree.get(argument_id);
            let value_id = argument.value();
            let value_expression = tree.get(value_id);
            let is_reference = match value_expression {
                Expression::LocalReference { target_symbol, .. }
                | Expression::ModuleReference { target_symbol, .. }
                | Expression::GlobalReference { target_symbol, .. } => *target_symbol == symbol_id,
                _ => false,
            };
            if is_reference {
                expression_id = Some(value_id);
                break;
            }
        }
        let expression_id = expression_id.expect("expected SIZE reference");
        let value = test
            .compiler
            .evaluate_static_expression_value(
                module,
                profile,
                expression_id,
                tree,
                symbols,
                types,
                None,
            )
            .expect("static evaluation failed")
            .expect("expected SIZE reference to evaluate");

        match value {
            StaticExpression::ScalarLiteral {
                value: ScalarLiteral::Integer(value),
            } => assert_eq!(value, 4),
            other => panic!("expected reference literal, got {other:?}"),
        }
    });
}

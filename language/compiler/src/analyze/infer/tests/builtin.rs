use super::*;
use crate::analyze::common::{TypeContext, TypeView};

/// Analyze builtin Pick mapped types.
#[test]
fn test_analyze_builtin_pick_optional_shape() {
    // Pick preserves optional fields for literal keys.
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
type Pick<T, K extends keyof T> = {
    [P in K]: T[P]
};

interface Person {
    name: string
    age?: number
}

type AgeOnly = Pick<Person, "age">;
const ok: AgeOnly = {};
const ok2: AgeOnly = { age: 42 };
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);
}

/// Builtin Pick from libs expands to object shapes.
#[test]
fn test_analyze_builtin_pick_libs_shape() {
    // pick from lib definitions preserves optionality and rejects extra fields
    let test = TestProgram::memory_sequential_with_prelude_and_libs()
        .with_lib("es5")
        .with_profile_libs(&["es5"]);
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Person {
    name: string
    age?: number
}

type AgeOnly = Pick<Person, "age">;
const ok: AgeOnly = {};
const bad: AgeOnly = { name: "Ada" };
"#,
    );

    // run analyze pipeline
    test.resolve_language_environment();
    test.resolve_libs();
    test.analyze_module(module_id);
    test.compile();
    // load module data for inspection
    let view = test.view(module_id);
    let profile = view.profile_id();
    let resolved_symbol = test.resolve_to_symbol("test.ds", "AgeOnly").unwrap();
    let age_only_entry = view.symbols().get_symbol(resolved_symbol.into_local());
    let age_only_symbol = GlobalSymbolId::new(
        resolved_symbol.module_id,
        resolved_symbol.local_id.with_type(age_only_entry.ty),
    );
    let alias_target_id = view
        .types()
        .get_alias_target_type_id(age_only_symbol)
        .expect("expected AgeOnly alias target type");

    // normalize to the object shape and ensure it only contains the picked key
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let dir = test.artifact_dir(module_id, profile);
    let tree = dir.tree.read();
    let options = test.compiler.analyze_context_options_for_module(module.id);
    let symbols = view.symbols().clone();
    let mut types = view.types().clone();
    let mut ctx = TypeContext::new(&module, profile, &options, &tree, &symbols, &mut types);
    let normalized = test.compiler.normalize_type(
        &mut ctx.reborrow(),
        alias_target_id,
        NormalizationMode::Assign,
    );

    assert_type!(types, normalized, Type::Object { fields, index_signatures, .. } => {
        assert!(index_signatures.is_empty());
        let age_key = StaticKey::Name(test.program.strings.intern("age"));
        let age_field = fields.iter().find(|field| field.key.matches(&age_key));
        assert!(age_field.is_some());
        assert!(age_field.expect("expected age field").is_optional);
    });

    test.check_has_diagnostic("EA208");
}

/// Preserve builtin Pick constraints until substitution.
#[test]
fn test_preserve_pick_keyof_constraint_until_substitution() {
    // load libs so we can inspect builtin Pick symbols
    let test = TestProgram::memory_sequential_with_prelude_and_libs()
        .with_lib("es5")
        .with_profile_libs(&["es5"]);
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Person {
    name: string
    age: number
}

type Alias = Pick<Person, "name">;
"#,
    );

    // run analyze pipeline to populate ctx
    test.resolve_language_environment();
    test.resolve_libs();
    test.analyze_module(module_id);
    test.compile();

    // load local module state for constraint evaluation
    let view = test.view(module_id);
    let profile = view.profile_id();
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let symbols = view.symbols().clone();
    let mut types = view.types().clone();

    // locate the es5 lib module that owns Pick
    let builtins = test.program.builtins.as_ref().expect("expected builtins");
    let es5_module_id = test
        .program
        .modules
        .iter()
        .find_map(|module| {
            let module = module.read();
            if builtins.lib_name_for_module(module.id) != Some("es5") {
                return None;
            }

            test.resolve_to_symbol(module.uri.as_ref(), "Pick")
                .map(|_| module.id)
        })
        .expect("expected es5 module exporting Pick");

    // resolve Pick and its static parameter symbols from the lib module
    let es5_module = test.program.modules.get(es5_module_id);
    let es5_module = es5_module.read();
    let es5_profile = test.default_profile_id(es5_module_id);
    let es5_dir = test.artifact_dir(es5_module_id, es5_profile);
    let es5_tree = es5_dir.tree.read();
    let es5_symbols = es5_dir.symbols.read();
    let es5_uri = es5_module.uri.to_string();
    let pick_symbol = test
        .resolve_to_symbol(&es5_uri, "Pick")
        .expect("expected Pick symbol");
    let parameter_symbols = test
        .compiler
        .collect_static_parameter_symbols(
            TypeView::new(&es5_module, es5_profile, &es5_tree, &es5_symbols, &types),
            pick_symbol,
        )
        .expect("expected Pick static parameters");

    // pick out the T and K symbols by name
    let t_name = test.program.strings.intern("T");
    let k_name = test.program.strings.intern("K");
    let mut t_symbol = None;
    let mut k_symbol = None;
    for symbol_id in parameter_symbols {
        let entry = es5_symbols.get_symbol(symbol_id.local_id);
        match entry.name() {
            Some(name) if name == t_name => t_symbol = Some(symbol_id),
            Some(name) if name == k_name => k_symbol = Some(symbol_id),
            _ => {}
        }
    }
    let t_symbol = t_symbol.expect("expected Pick<T, ..> symbol");
    let k_symbol = k_symbol.expect("expected Pick<.., K> symbol");

    // use the Person declaration as a stable local source anchor
    let person_symbol = test
        .resolve_to_symbol("test.ds", "Person")
        .expect("expected Person symbol");
    let person_entry = symbols.get_symbol(person_symbol.into_local());
    let person_declaration = person_entry
        .primary_declaration
        .expect("expected Person declaration");
    let source_id = person_declaration.local_id;

    let dir = test.artifact_dir(module_id, profile);
    let tree = dir.tree.read();
    let options = test.compiler.analyze_context_options_for_module(module.id);

    // import the K constraint into the local type table
    let k_constraint_id = {
        let mut ctx = TypeContext::new(&module, profile, &options, &tree, &symbols, &mut types);
        test.compiler
            .static_parameter_constraint_type(&mut ctx, k_symbol, source_id)
            .expect("expected K constraint type")
    };

    // the raw constraint should still reference static parameters
    let mut visited = HashSet::new();
    assert!(test.compiler.type_contains_static_parameters(
        TypeView::new(&module, profile, &tree, &symbols, &types),
        k_constraint_id,
        &mut visited,
    ));

    // substitute T with Person and ensure the bound becomes concrete
    let person_ref_id = types.insert_type_from_any(
        Type::Reference {
            symbol: person_symbol,
            static_arguments: None,
        },
        source_id,
    );
    let mut substitutions = HashMap::new();
    substitutions.insert(t_symbol, person_ref_id);
    let mut cache = HashMap::new();
    let substituted_constraint_id = test.compiler.substitute_static_parameters(
        k_constraint_id,
        &substitutions,
        &mut types,
        &mut cache,
    );
    let mut ctx = TypeContext::new(&module, profile, &options, &tree, &symbols, &mut types);
    let normalized_constraint_id = test.compiler.normalize_type(
        &mut ctx.reborrow(),
        substituted_constraint_id,
        NormalizationMode::Assign,
    );
    assert!(
        !matches!(
            types.get_type(normalized_constraint_id),
            Type::TypeLiteral {
                value: TypeLiteral::Unknown | TypeLiteral::Any,
            }
        ),
        "expected concrete keyof constraint after substitution"
    );
}

/// Reject unknown keys passed to builtin Pick.
#[test]
fn test_pick_rejects_unknown_keys_bound_validation() {
    // load libs and build a bad Pick instantiation
    let test = TestProgram::memory_sequential_with_prelude_and_libs()
        .with_lib("es5")
        .with_profile_libs(&["es5"]);
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Person {
    name: string
    age: number
}

type Bad = Pick<Person, "missing">;
"#,
    );

    // run analyze pipeline and expect a not assignable diagnostic
    test.resolve_language_environment();
    test.resolve_libs();
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostic("EA101");
}

/// Destack overloads merge into a callable value type.
#[test]
fn test_analyze_destack_function_overload_merge() {
    // overload implementations share a merged value shape
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function parse(value: string): string {
    return value;
}

function parse(value: int32): int32 {
    return value + 1;
}
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load module data for inspection
    let view = test.view(module_id);
    let parse_symbol = test.resolve_to_symbol("test.ds", "parse").unwrap();
    let value_ty_id = view
        .types()
        .get_value_type_id(parse_symbol)
        .expect("expected parse value type");
    let value_ty_id = match view.types().get_type(value_ty_id) {
        Type::Value { value } => *value,
        _ => value_ty_id,
    };

    match view.types().get_type(value_ty_id) {
        Type::Object {
            call_signatures, ..
        } => {
            assert_eq!(call_signatures.len(), 2);

            let mut has_string = false;
            let mut has_int32 = false;
            for signature_id in call_signatures {
                let Type::Function {
                    dynamic_parameters, ..
                } = view.types().get_type(*signature_id)
                else {
                    panic!("expected function signature");
                };
                let param_ty_id = *dynamic_parameters.first().expect("expected parameter type");
                match view.types().get_type(param_ty_id) {
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::String),
                    } => has_string = true,
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32)),
                    } => has_int32 = true,
                    _ => {}
                }
            }

            assert!(has_string);
            assert!(has_int32);
        }
        Type::Function { .. } => {
            panic!("expected overload set, found single signature");
        }
        _ => {
            panic!("expected callable value type");
        }
    }
}

/// Analyze builtin Omit mapped types.
#[test]
fn test_analyze_builtin_omit_shape() {
    // Omit removes the specified keys from the source shape.
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
type Pick<T, K extends keyof T> = {
    [P in K]: T[P]
};
type Exclude<T, U> = T extends U ? never : T;
type Omit<T, K extends keyof T> = Pick<T, Exclude<keyof T, K>>;

interface Person {
    name: string
    age: number
}

type WithoutAge = Omit<Person, "age">;
const ok: WithoutAge = { name: "Ada" };
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);
}

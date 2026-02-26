use crate::TestProgram;
use destack_builtin::LanguageSymbol;
use destack_dir::{CaptureKind, CapturePolicy};

#[test]
fn test_capture_defaults_const_and_let() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function make() {
  const a = 1;
  let b = 2;
  return () => a + b;
}
"#,
    );

    test.analyze_module_and_check_clean(module_id);

    // resolve capture set for the lambda
    let lambda_symbol = test.expect_nth_lambda_symbol(module_id, 0);
    let capture_set = test.capture_set_for_symbol(module_id, lambda_symbol);

    // check capture order and kinds
    let capture_names = test.capture_names_and_kinds(module_id, &capture_set);

    assert_eq!(
        capture_names,
        vec![
            ("a".to_string(), CaptureKind::ByValue),
            ("b".to_string(), CaptureKind::ByReference),
        ]
    );

    // check address taken locals for the owner function
    let make_symbol = test
        .resolve_to_symbol("test.ds", "make")
        .unwrap_or_else(|| panic!("expected make symbol"));
    let reference_local_names = test.reference_local_names(module_id, make_symbol);

    assert_eq!(reference_local_names, vec!["b".to_string()]);
}

#[test]
fn test_capture_decorator_resolves_marker_symbol() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
@capture("byValue")
function inner() {
  return 1;
}
"#,
    );

    test.analyze_module_and_check_clean(module_id);

    let decorator = test.decorator_info_for_declaration("test.ds", "inner");
    assert!(decorator.first_argument_is_string_literal);

    let capture_symbol = test
        .compiler
        .language_symbol(test.default_profile_id(module_id), LanguageSymbol::Capture);
    assert_eq!(decorator.target_symbol, capture_symbol);

    let profile = test.default_profile_id(module_id);
    let target_symbol = decorator.target_symbol;
    let decorator_map = test.compiler.collect_well_known_decorators(profile);
    assert!(
        decorator_map.contains_key(&target_symbol),
        "expected capture symbol in well known decorator map"
    );
}

#[test]
fn test_capture_preserves_first_use_order() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function make() {
    const a = 1;
    let b = 2;

    return () => b + a;
}
"#,
    );

    test.analyze_module_and_check_clean(module_id);

    // capture b, a
    let lambda_symbol = test.expect_nth_lambda_symbol(module_id, 0);
    let capture_set = test.capture_set_for_symbol(module_id, lambda_symbol);
    let capture_names = test.capture_names(module_id, &capture_set);
    assert_eq!(capture_names, vec!["b".to_string(), "a".to_string()]);
}

#[test]
fn test_capture_handles_nested_functions() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function make() {
    const a = 1;
    function inner() {
        return a;
    }
    return inner();
}
"#,
    );

    test.analyze_module_and_check_clean(module_id);

    let capture_set = test.capture_set_for_function_name("test.ds", "inner");
    let capture_names = test.capture_names_and_kinds(module_id, &capture_set);
    assert_eq!(capture_names, vec![("a".to_string(), CaptureKind::ByValue)]);
}

#[test]
fn test_capture_skips_module_scope_bindings() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
const moduleValue = 1;

function make() {
  return () => moduleValue;
}
"#,
    );

    test.analyze_module_and_check_clean(module_id);

    let lambda_symbol = test.expect_nth_lambda_symbol(module_id, 0);
    let capture_set = test.capture_set_for_symbol(module_id, lambda_symbol);
    assert!(capture_set.captures.is_empty());
}

#[test]
fn test_capture_ignores_shadowed_bindings() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function make() {
    const a = 1;
    
    return () => {
        const a = 2;
        return a;
    };
}
"#,
    );

    test.analyze_module_and_check_clean(module_id);

    let lambda_symbol = test.expect_nth_lambda_symbol(module_id, 0);
    let capture_set = test.capture_set_for_symbol(module_id, lambda_symbol);
    assert!(capture_set.captures.is_empty());
}

#[test]
fn test_capture_resolves_this_in_nested_function() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Widget {
    value: int32 = 0;

    method(): int32 {
        const read = () => this.value;
        return read();
    }
}
"#,
    );

    test.analyze_module_and_check_clean(module_id);

    let lambda_symbol = test.expect_nth_lambda_symbol(module_id, 0);
    let capture_set = test.capture_set_for_symbol(module_id, lambda_symbol);
    let this_symbol = capture_set
        .this_symbol
        .unwrap_or_else(|| panic!("expected captured this symbol"));
    let capture_symbols = capture_set
        .captures
        .iter()
        .map(|binding| (binding.symbol, binding.kind))
        .collect::<Vec<_>>();
    assert_eq!(capture_symbols, vec![(this_symbol, CaptureKind::ByValue)]);
}

#[test]
fn test_capture_directive_policy_by_value() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function make() {
    const a = 1;
    let b = 2;

    @capture("byValue")
    function inner() {
        return a + b;
    }

    return inner();
}
"#,
    );

    test.analyze_module_and_check_clean(module_id);

    let capture_set = test.capture_set_for_function_name("test.ds", "inner");
    assert_eq!(capture_set.directive.policy, CapturePolicy::ByValue);
    let capture_names = test.capture_names_and_kinds(module_id, &capture_set);
    assert_eq!(
        capture_names,
        vec![
            ("a".to_string(), CaptureKind::ByValue),
            ("b".to_string(), CaptureKind::ByValue),
        ]
    );
}

#[test]
fn test_capture_directive_policy_by_move() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function make() {
    const a = 1;
    let b = 2;

    @capture("byMove")
    function inner() {
        return a + b;
    }
    
    return inner();
}
"#,
    );

    test.analyze_module_and_check_clean(module_id);

    let capture_set = test.capture_set_for_function_name("test.ds", "inner");
    assert_eq!(capture_set.directive.policy, CapturePolicy::ByMove);

    let capture_names = test.capture_names_and_kinds(module_id, &capture_set);
    assert_eq!(
        capture_names,
        vec![
            ("a".to_string(), CaptureKind::ByMove),
            ("b".to_string(), CaptureKind::ByMove),
        ]
    );
}

#[test]
fn test_capture_directive_overrides_named_binding() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function make() {
    const a = 1;
    let b = 2;
    
    @capture({ default: "byReference", a: "byValue" })
    function inner() {
        return a + b;
    }
    
    return inner();
}
"#,
    );

    test.analyze_module_and_check_clean(module_id);

    let capture_set = test.capture_set_for_function_name("test.ds", "inner");
    assert_eq!(capture_set.directive.policy, CapturePolicy::ByReference);

    let capture_names = test.capture_names_and_kinds(module_id, &capture_set);
    assert_eq!(
        capture_names,
        vec![
            ("a".to_string(), CaptureKind::ByValue),
            ("b".to_string(), CaptureKind::ByReference),
        ]
    );
}

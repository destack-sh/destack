use crate::TestProgram;
use destack_builtin::LanguageSymbol;
use destack_dir::{Annotation, CaptureKind, Declaration, Expression, FunctionKind};

// FUGU #Cleanup: capture tests are _ugly_

fn first_lambda_symbol(
    module_id: destack_source::ModuleId,
    tree: &destack_dir::NodeTree,
) -> destack_dir::GlobalSymbolId {
    tree.iter_nodes_of_type::<Declaration>()
        .find_map(|(_, declaration)| match declaration {
            Declaration::Function {
                descriptor,
                signature,
                ..
            } if signature.kind == FunctionKind::Lambda => {
                Some(descriptor.symbol.into_global(module_id))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected a lambda declaration"))
}

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

    // load module state
    let profile = test.default_profile_id(module_id);
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let dir = module.dir(profile);
    let tree = dir.tree.read();
    let symbols = dir.symbols.read();
    let captures = dir.captures.read();

    // resolve capture set for the lambda
    let lambda_symbol = first_lambda_symbol(module_id, &tree);
    let capture_set = captures
        .capture_set(lambda_symbol)
        .unwrap_or_else(|| panic!("expected capture set for lambda"));

    // check capture order and kinds
    let capture_names = capture_set
        .captures
        .iter()
        .map(|binding| {
            let name = symbols
                .get_symbol(binding.symbol.into_local())
                .name()
                .unwrap_or_else(|| panic!("expected named capture"));
            let name = test.program.strings.get(name);
            (name.to_string(), binding.kind)
        })
        .collect::<Vec<_>>();

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
    let reference_locals = captures.reference_locals(make_symbol).unwrap_or(&[]);
    let reference_local_names = reference_locals
        .iter()
        .map(|symbol| {
            let name = symbols
                .get_symbol(symbol.into_local())
                .name()
                .unwrap_or_else(|| panic!("expected named symbol"));
            test.program.strings.get(name).to_string()
        })
        .collect::<Vec<_>>();

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

    let profile = test.default_profile_id(module_id);
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let dir = module.dir(profile);
    let tree = dir.tree.read();
    let symbols = dir.symbols.read();

    let inner_symbol = test
        .function_symbol_by_name("test.ds", "inner")
        .unwrap_or_else(|| panic!("expected inner symbol"));
    let inner_entry = symbols.get_symbol(inner_symbol.into_local());
    let Some(declaration) = inner_entry.primary_declaration else {
        panic!("expected declaration for inner");
    };
    let mut annotations = tree.get_annotations(declaration.local_id.id);
    if annotations.is_empty() {
        let wrapper_id = tree
            .iter_nodes_of_type::<Expression>()
            .find_map(|(expression_id, expression)| match expression {
                Expression::Declaration { declaration: inner } if inner.id == declaration.local_id.id => {
                    Some(expression_id)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected declaration wrapper for inner"));
        annotations = tree.get_annotations(wrapper_id.id);
    }
    let decorator_id = annotations
        .iter()
        .find(|annotation_id| matches!(tree.get(**annotation_id), Annotation::Decorator { .. }))
        .copied()
        .unwrap_or_else(|| panic!("expected decorator annotation"));
    let Annotation::Decorator { left, arguments, .. } = tree.get(decorator_id) else {
        panic!("expected decorator annotation");
    };
    let Some(arguments) = arguments else {
        panic!("expected decorator arguments");
    };
    let argument_id = arguments
        .first()
        .unwrap_or_else(|| panic!("expected decorator argument"));
    let argument = tree.get(*argument_id);
    let value_id = match argument {
        destack_dir::Argument::Positional { value, .. }
        | destack_dir::Argument::Named { value, .. }
        | destack_dir::Argument::Labeled { value, .. } => *value,
        destack_dir::Argument::Spread { .. } => {
            panic!("unexpected spread decorator argument");
        }
    };
    let value_expr = tree.get(value_id);
    assert!(
        matches!(
            value_expr,
            Expression::ScalarLiteral {
                value: destack_dir::ScalarLiteral::String(_)
            }
        ),
        "expected string literal decorator argument, got {value_expr:?}"
    );
    let target_symbol = match tree.get(*left) {
        Expression::LocalReference { target_symbol, .. }
        | Expression::ModuleReference { target_symbol, .. }
        | Expression::GlobalReference { target_symbol, .. } => *target_symbol,
        other => panic!("expected resolved decorator reference, got {other:?}"),
    };

    let capture_symbol = test.compiler.language_symbol(profile, LanguageSymbol::Capture);
    assert_eq!(target_symbol, capture_symbol);

    let decorator_map = test.compiler.collect_well_known_decorators(profile);
    let resolved_symbol = test.compiler.typed_symbol_id(
        &module,
        profile,
        target_symbol,
        &symbols,
    );
    assert!(
        decorator_map.contains_key(&resolved_symbol),
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

    // load module state
    let profile = test.default_profile_id(module_id);
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let dir = module.dir(profile);
    let tree = dir.tree.read();
    let symbols = dir.symbols.read();
    let captures = dir.captures.read();

    // resolve capture set for the lambda
    let lambda_symbol = first_lambda_symbol(module_id, &tree);
    let capture_set = captures
        .capture_set(lambda_symbol)
        .unwrap_or_else(|| panic!("expected capture set for lambda"));

    // check capture ordering
    let capture_names = capture_set
        .captures
        .iter()
        .map(|binding| {
            let name = symbols
                .get_symbol(binding.symbol.into_local())
                .name()
                .unwrap_or_else(|| panic!("expected named capture"));
            test.program.strings.get(name).to_string()
        })
        .collect::<Vec<_>>();

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

    // load module state
    let profile = test.default_profile_id(module_id);
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let dir = module.dir(profile);
    let symbols = dir.symbols.read();
    let captures = dir.captures.read();

    // resolve capture set for the nested function
    let inner_symbol = test
        .function_symbol_by_name("test.ds", "inner")
        .unwrap_or_else(|| panic!("expected inner symbol"));
    let capture_set = captures
        .capture_set(inner_symbol)
        .unwrap_or_else(|| panic!("expected capture set for inner"));
    // verify capture kind for the const binding
    let capture_names = capture_set
        .captures
        .iter()
        .map(|binding| {
            let name = symbols
                .get_symbol(binding.symbol.into_local())
                .name()
                .unwrap_or_else(|| panic!("expected named capture"));
            let name = test.program.strings.get(name);
            (name.to_string(), binding.kind)
        })
        .collect::<Vec<_>>();

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

    // load module state
    let profile = test.default_profile_id(module_id);
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let dir = module.dir(profile);
    let tree = dir.tree.read();
    let captures = dir.captures.read();

    // resolve capture set for the lambda
    let lambda_symbol = first_lambda_symbol(module_id, &tree);
    let capture_set = captures
        .capture_set(lambda_symbol)
        .unwrap_or_else(|| panic!("expected capture set for lambda"));

    // verify module scope bindings are not captured
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

    // load module state
    let profile = test.default_profile_id(module_id);
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let dir = module.dir(profile);
    let tree = dir.tree.read();
    let captures = dir.captures.read();

    // resolve capture set for the lambda
    let lambda_symbol = first_lambda_symbol(module_id, &tree);
    let capture_set = captures
        .capture_set(lambda_symbol)
        .unwrap_or_else(|| panic!("expected capture set for lambda"));

    // verify shadowed bindings are not captured
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

    // load module state
    let profile = test.default_profile_id(module_id);
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let dir = module.dir(profile);
    let tree = dir.tree.read();
    let symbols = dir.symbols.read();
    let captures = dir.captures.read();

    // resolve capture set for the lambda
    let lambda_symbol = first_lambda_symbol(module_id, &tree);
    let capture_set = captures
        .capture_set(lambda_symbol)
        .unwrap_or_else(|| panic!("expected capture set for lambda"));

    // verify 'this' is captured
    let capture_names = capture_set
        .captures
        .iter()
        .map(|binding| {
            let name = symbols
                .get_symbol(binding.symbol.into_local())
                .name()
                .unwrap_or_else(|| panic!("expected named capture"));
            let name = test.program.strings.get(name);
            (name.to_string(), binding.kind)
        })
        .collect::<Vec<_>>();

    assert_eq!(
        capture_names,
        vec![("this".to_string(), CaptureKind::ByReference)]
    );
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

    // load module state
    let profile = test.default_profile_id(module_id);
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let dir = module.dir(profile);
    let symbols = dir.symbols.read();
    let captures = dir.captures.read();

    // resolve capture set for the nested function
    let inner_symbol = test
        .function_symbol_by_name("test.ds", "inner")
        .unwrap_or_else(|| panic!("expected inner symbol"));
    let capture_set = captures
        .capture_set(inner_symbol)
        .unwrap_or_else(|| panic!("expected capture set for inner"));

    // verify capture directive
    assert_eq!(
        capture_set.directive.policy,
        destack_dir::CapturePolicy::ByValue
    );

    // verify policy overrides to by value
    let capture_names = capture_set
        .captures
        .iter()
        .map(|binding| {
            let name = symbols
                .get_symbol(binding.symbol.into_local())
                .name()
                .unwrap_or_else(|| panic!("expected named capture"));
            let name = test.program.strings.get(name);
            (name.to_string(), binding.kind)
        })
        .collect::<Vec<_>>();

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

    // load module state
    let profile = test.default_profile_id(module_id);
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let dir = module.dir(profile);
    let symbols = dir.symbols.read();
    let captures = dir.captures.read();

    // resolve capture set for the nested function
    let inner_symbol = test
        .function_symbol_by_name("test.ds", "inner")
        .unwrap_or_else(|| panic!("expected inner symbol"));
    let capture_set = captures
        .capture_set(inner_symbol)
        .unwrap_or_else(|| panic!("expected capture set for inner"));

    // verify capture directive
    assert_eq!(
        capture_set.directive.policy,
        destack_dir::CapturePolicy::ByMove
    );

    // verify policy overrides to by move
    let capture_names = capture_set
        .captures
        .iter()
        .map(|binding| {
            let name = symbols
                .get_symbol(binding.symbol.into_local())
                .name()
                .unwrap_or_else(|| panic!("expected named capture"));
            let name = test.program.strings.get(name);
            (name.to_string(), binding.kind)
        })
        .collect::<Vec<_>>();

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

    // load module state
    let profile = test.default_profile_id(module_id);
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let dir = module.dir(profile);
    let symbols = dir.symbols.read();
    let captures = dir.captures.read();

    // resolve capture set for the nested function
    let inner_symbol = test
        .function_symbol_by_name("test.ds", "inner")
        .unwrap_or_else(|| panic!("expected inner symbol"));
    let capture_set = captures
        .capture_set(inner_symbol)
        .unwrap_or_else(|| panic!("expected capture set for inner"));

    // verify capture directive
    assert_eq!(
        capture_set.directive.policy,
        destack_dir::CapturePolicy::ByReference
    );

    // verify rule override for named binding
    let capture_names = capture_set
        .captures
        .iter()
        .map(|binding| {
            let name = symbols
                .get_symbol(binding.symbol.into_local())
                .name()
                .unwrap_or_else(|| panic!("expected named capture"));
            let name = test.program.strings.get(name);
            (name.to_string(), binding.kind)
        })
        .collect::<Vec<_>>();

    assert_eq!(
        capture_names,
        vec![
            ("a".to_string(), CaptureKind::ByValue),
            ("b".to_string(), CaptureKind::ByReference),
        ]
    );
}

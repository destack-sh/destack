use destack_artifact::{EmitFormat, Platform};
use destack_dir::{FunctionMode, Member};

use crate::tests::TestProgram;

fn test_program_js() -> TestProgram {
    let mut test = TestProgram::memory_sequential();
    let default_profile = test.profile(test.default_profile_id_for_root());
    let mut key = default_profile.key.clone();
    key.emit = EmitFormat::Js;
    let profile_id = test.program.profile_id(key);
    test.default_profile_override = Some(profile_id);
    test
}

fn test_program_js_for_platform(platform: Platform) -> TestProgram {
    let mut test = TestProgram::memory_sequential();
    let default_profile = test.profile(test.default_profile_id_for_root());
    let mut key = default_profile.key.clone();
    key.emit = EmitFormat::Js;
    key.platform = platform;
    let profile_id = test.program.profile_id(key);
    test.default_profile_override = Some(profile_id);
    test
}

/// Gate declarations based on static if conditions.
#[test]
fn test_static_if_gates_declaration() {
    // build the test program
    let test = test_program_js();
    let module_id = test.add_module(
        "test.ds",
        r#"
@if(import.meta.emit == "native")
struct Hidden {
    value: number;
}

struct Visible {}
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile_check_clean();

    // confirm the filtered declaration symbols
    assert!(test.resolve_to_symbol("test.ds", "Hidden").is_none());
    assert!(test.resolve_to_symbol("test.ds", "Visible").is_some());
}

/// Gate module statements based on static if conditions.
#[test]
fn test_static_if_gates_statement() {
    // build the test program
    let test = test_program_js();
    let module_id = test.add_module(
        "test.ds",
        r#"
@if(import.meta.emit == "native")
missing_symbol();

const value = 1;
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile_check_clean();

    // confirm other declarations still resolve
    assert!(test.resolve_to_symbol("test.ds", "value").is_some());
}

/// Gate block statements based on static if conditions.
#[test]
fn test_static_if_gates_block_statement() {
    // build the test program
    let test = test_program_js();
    let module_id = test.add_module(
        "test.ds",
        r#"
function demo(): number {
    @if(import.meta.emit == "native")
    missing_symbol();

    return 1;
}
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile_check_clean();
}

/// Report errors when a static if condition is true.
#[test]
fn test_static_if_errors_when_true() {
    // build the test program
    let test = test_program_js();
    let module_id = test.add_module(
        "test.ds",
        r#"
@if(import.meta.emit == "js")
missing_symbol();
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile();

    // confirm the missing symbol diagnostic
    test.check_has_diagnostic("ER101");
}

/// Reject non boolean static if conditions.
#[test]
fn test_static_if_requires_boolean() {
    // build the test program
    let test = test_program_js();
    let module_id = test.add_module(
        "test.ds",
        r#"
@if(1)
const value = 1;
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile();

    // confirm the static if diagnostic
    test.check_has_diagnostic("ER901");
}

/// Reject static if decorators without arguments.
#[test]
fn test_static_if_requires_argument() {
    // build the test program
    let test = test_program_js();
    let module_id = test.add_module(
        "test.ds",
        r#"
@if
const value = 1;
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile();

    // confirm the static if diagnostic
    test.check_has_diagnostic("ER901");
}

/// Gate class members based on static if conditions.
#[test]
fn test_static_if_gates_members() {
    // build the test program
    let test = test_program_js();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Box {
    @if(import.meta.emit == "native")
    missing: MissingType;

    value: number;
}
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile_check_clean();
}

/// Combine multiple static if decorators on a declaration.
#[test]
fn test_static_if_combines_conditions() {
    // build the test program
    let test = test_program_js();
    let module_id = test.add_module(
        "test.ds",
        r#"
@if(import.meta.emit == "js")
@if(import.meta.emit == "native")
const value = missing_symbol();
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile_check_clean();
}

/// Attach static if decorators to accessor members.
#[test]
fn test_static_if_attaches_to_accessor_members() {
    // build the test program
    let test = test_program_js();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Box {
    @if(import.meta.emit == "js" && import.meta.emit == "native")
    get value(): MissingType {
        return missingSymbol;
    }

    @if(import.meta.emit == "js" && import.meta.emit == "native")
    set value(next: MissingType) {
        missingSymbol;
    }
}
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile();

    // locate accessor members in the dir
    test.with_dir_read(
        module_id,
        |_module, _profile, _dir, tree, _symbols, _types| {
            // initialize accessor ids
            let mut getter_id = None;
            let mut setter_id = None;

            // scan members for accessor nodes
            for member_id in tree.iter_node_ids_of_type::<Member>() {
                let Member::Method { signature, .. } = tree.get(member_id) else {
                    continue;
                };
                // record getter and setter members
                match signature.mode {
                    Some(FunctionMode::Getter) => getter_id = Some(member_id),
                    Some(FunctionMode::Setter) => setter_id = Some(member_id),
                    _ => {}
                }
            }

            // confirm both accessors were parsed
            let getter_id = getter_id.expect("expected getter member");
            let setter_id = setter_id.expect("expected setter member");

            // confirm @if annotations were attached
            assert!(
                !tree.get_decorators(getter_id.id).is_empty(),
                "expected getter annotations"
            );
            assert!(
                !tree.get_decorators(setter_id.id).is_empty(),
                "expected setter annotations"
            );

            // confirm accessors were gated out
            assert!(tree.is_inactive(getter_id.id), "expected getter inactive");
            assert!(tree.is_inactive(setter_id.id), "expected setter inactive");
        },
    );

    // confirm no diagnostics after gating
    test.check_clean();
}

/// Treat missing import.meta.env keys as undefined values.
#[test]
fn test_static_if_missing_env_key_is_undefined() {
    // build the test program
    let test = test_program_js();
    let module_id = test.add_module(
        "test.ds",
        r#"
@if(import.meta.env.NOT_DEFINED == undefined)
const visible = 1;

@if(import.meta.env.NOT_DEFINED == "set")
const hidden = missing_symbol();
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile_check_clean();

    // confirm the expected symbol visibility
    assert!(test.resolve_to_symbol("test.ds", "visible").is_some());
}

/// Reject non-string import.meta.env index lookups in static if conditions.
#[test]
fn test_static_if_env_index_requires_string_key() {
    // build the test program
    let test = test_program_js();
    let module_id = test.add_module(
        "test.ds",
        r#"
@if(import.meta.env[1] == "x")
const value = 1;
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile();

    // confirm the static if diagnostic
    test.check_has_diagnostic("ER901");
}

/// Gate declarations based on target-family static if conditions.
#[test]
fn test_static_if_gates_declaration_with_target_family() {
    // build the test program
    let test = test_program_js_for_platform(Platform::Linux);
    let module_id = test.add_module(
        "test.ds",
        r#"
@if(import.meta.target.family == "unix")
const visible = 1;

@if(import.meta.target.family == "windows")
const hidden = missing_symbol();
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile_check_clean();

    // confirm the expected symbol visibility
    assert!(test.resolve_to_symbol("test.ds", "visible").is_some());
}

/// Gate declarations based on target-vendor static if conditions.
#[test]
fn test_static_if_gates_declaration_with_target_vendor() {
    // build the test program
    let test = test_program_js_for_platform(Platform::MacOS);
    let module_id = test.add_module(
        "test.ds",
        r#"
@if(import.meta.target.vendor == "apple")
const visible = 1;

@if(import.meta.target.vendor == "pc")
const hidden = missing_symbol();
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile_check_clean();

    // confirm the expected symbol visibility
    assert!(test.resolve_to_symbol("test.ds", "visible").is_some());
}

/// Treat missing import.meta.target env as undefined values.
#[test]
fn test_static_if_missing_target_env_is_undefined() {
    // build the test program
    let test = test_program_js_for_platform(Platform::Web);
    let module_id = test.add_module(
        "test.ds",
        r#"
@if(import.meta.target.env == undefined)
const visible = 1;

@if(import.meta.target.env == "gnu")
const hidden = missing_symbol();
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile_check_clean();

    // confirm the expected symbol visibility
    assert!(test.resolve_to_symbol("test.ds", "visible").is_some());
}

/// Resolve import.meta.target index lookups in static if conditions.
#[test]
fn test_static_if_target_index_lookup() {
    // build the test program
    let test = test_program_js_for_platform(Platform::Linux);
    let module_id = test.add_module(
        "test.ds",
        r#"
@if(import.meta.target["family"] == "unix")
const visible = 1;

@if(import.meta.target["family"] == "windows")
const hidden = missing_symbol();
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile_check_clean();

    // confirm the expected symbol visibility
    assert!(test.resolve_to_symbol("test.ds", "visible").is_some());
}

/// Reject non-string import.meta.target index lookups in static if conditions.
#[test]
fn test_static_if_target_index_requires_string_key() {
    // build the test program
    let test = test_program_js();
    let module_id = test.add_module(
        "test.ds",
        r#"
@if(import.meta.target[1] == "x")
const value = 1;
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile();

    // confirm the static if diagnostic
    test.check_has_diagnostic("ER901");
}

/// Reject unknown import.meta.target member lookups in static if conditions.
#[test]
fn test_static_if_target_member_rejects_unknown_key() {
    // build the test program
    let test = test_program_js();
    let module_id = test.add_module(
        "test.ds",
        r#"
@if(import.meta.target.not_defined == undefined)
const value = 1;
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile();

    // confirm the static if diagnostic
    test.check_has_diagnostic("ER901");
}

/// Reject unknown import.meta.target index lookups in static if conditions.
#[test]
fn test_static_if_target_index_rejects_unknown_key() {
    // build the test program
    let test = test_program_js();
    let module_id = test.add_module(
        "test.ds",
        r#"
@if(import.meta.target["not_defined"] == undefined)
const value = 1;
"#,
    );

    // resolve the module
    test.resolve_module(module_id);
    test.compile();

    // confirm the static if diagnostic
    test.check_has_diagnostic("ER901");
}

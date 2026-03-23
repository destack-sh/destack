use crate::tests::TestProgram;
use destack_artifact::EmitFormat;

#[test]
fn test_normalize_if_in_let_with_blocks() {
    // if expression with block branches in let binding becomes uninitialized let plus if
    // (uses multi-statement branches to prevent ternary optimization)
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function choose(flag: boolean, a: int32, b: int32): int32 {
    let x = if (flag) { let t = a; t } else { let t = b; t };
    return x;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function choose(flag, a, b): int32 {
    let x;
    if (flag) {
        let t = a;
        x = t;
    } else {
        let t = b;
        x = t;
    }
    return x;
}
"#,
    );
}

#[test]
fn test_normalize_if_in_return_with_blocks() {
    // if expression with block branches in return becomes if with returns
    // (uses multi-statement branches to prevent ternary optimization)
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function choose(flag: boolean, a: int32, b: int32): int32 {
    return if (flag) { let t = a; t } else { let t = b; t };
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function choose(flag, a, b): int32 {
    if (flag) {
        let t = a;
        return t;
    } else {
        let t = b;
        return t;
    }
}
"#,
    );
}

#[test]
fn test_normalize_block_in_let() {
    // block values in let bindings become uninitialized let plus assignment in block
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function choose(): int32 {
    let x = do { let y = 3; y };
    x
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function choose(): int32 {
    let x;
    {
        let y = 3;
        x = y;
    }
    return x;
}
"#,
    );
}

#[test]
fn test_transform_explicit_return_wraps_ternary_expression() {
    // ternary value positions become explicit return statements
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function choose(flag: boolean, a: int32, b: int32): int32 {
    if (flag) a else b
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function choose(flag, a, b): int32 {
    return flag ? a : b;
}
"#,
    );
}

#[test]
fn test_normalize_sequence_in_let() {
    // ts comma expressions in let bindings become statements plus final let
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ts",
        r#"
function touch(value: number): number {
    return value;
}

function ordered(a: number, b: number): number {
    let value = (touch(a), touch(b), b + 1);
    return value;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function touch(value): number {
    return value;
}

function ordered(a, b): number {
    touch(a);
    touch(b);
    let value = b + (1 as number);
    return value;
}
"#,
    );
}

#[test]
fn test_normalize_return_if_else_if_chain() {
    // return if else if chains normalize every branch into explicit returns
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function choose(a: boolean, b: boolean): int32 {
    return if (a) { let x = 1; x } else if (b) { let y = 2; y } else { let z = 3; z };
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function choose(a, b): int32 {
    if (a) {
        let x = 1;
        return x;
    } else if (b) {
        let y = 2;
        return y;
    } else {
        let z = 3;
        return z;
    }
}
"#,
    );
}

#[test]
fn test_normalize_nullish_coalesce_in_let_native() {
    // native profiles normalize nullish coalesce into explicit control flow
    let test = TestProgram::memory_sequential().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function choose(name: string | undefined, fallback: string): string {
    let value = name ?? fallback;
    return value;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function choose(name, fallback): string {
    let value;
    const __coalesce_left9 = name;
    if (__coalesce_left9 == null ||
        __coalesce_left9 == undefined) value = fallback else value = (__coalesce_left9 as string)
    return value;
}
"#,
    );
}

#[test]
fn test_keep_nullish_coalesce_in_return_js() {
    // js profiles retain nullish coalesce syntax
    let test = TestProgram::memory_sequential().with_profile_emit(EmitFormat::Js);
    let module_id = test.add_module(
        "test.ds",
        r#"
function choose(name: string | undefined, fallback: string): string {
    return name ?? fallback;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function choose(name, fallback): string {
    return name ?? fallback;
}
"#,
    );
}

#[test]
fn test_keep_nullish_coalesce_in_return_ts() {
    // ts profiles retain nullish coalesce syntax
    let test = TestProgram::memory_sequential().with_profile_emit(EmitFormat::Ts);
    let module_id = test.add_module(
        "test.ds",
        r#"
function choose(name: string | undefined, fallback: string): string {
    return name ?? fallback;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function choose(name, fallback): string {
    return name ?? fallback;
}
"#,
    );
}

#[test]
fn test_normalize_nullish_coalesce_in_assignment_native() {
    // native profiles normalize nullish coalesce in assignment value positions
    let test = TestProgram::memory_sequential().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function choose(name: string | undefined, fallback: string): string {
    let value = fallback;
    value = name ?? fallback;
    return value;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function choose(name, fallback): string {
    let value = fallback;
    value =
        {
            const __coalesce_left9 = name;
            if (__coalesce_left9 == null ||
                __coalesce_left9 == undefined) fallback else __coalesce_left9
        };
    return value;
}
"#,
    );
}

#[test]
fn test_normalize_nullish_coalesce_in_call_argument_native() {
    // native profiles normalize nullish coalesce in eager call argument positions
    let test = TestProgram::memory_sequential().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function wrap(value: string): string {
    return value;
}

function choose(name: string | undefined, fallback: string): string {
    return wrap(name ?? fallback);
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function wrap(value): string {
    return value;
}

function choose(name, fallback): string {
    return wrap(
        {
            const __coalesce_left10 = name;
            if (__coalesce_left10 == null ||
                __coalesce_left10 == undefined) fallback else __coalesce_left10
        },
    );
}
"#,
    );
}

#[test]
fn test_normalize_nullish_coalesce_in_ternary_arm_native() {
    // native profiles normalize nullish coalesce inside ternary branches without hoisting
    let test = TestProgram::memory_sequential().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function choose(flag: boolean, name: string | undefined, fallback: string): string {
    return flag ? (name ?? fallback) : fallback;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function choose(flag, name, fallback): string {
    return flag
        ? {
            const __coalesce_left8 = name;
            if (__coalesce_left8 == null ||
                __coalesce_left8 == undefined) fallback else __coalesce_left8
        }
        : fallback;
}
"#,
    );
}

#[test]
fn test_normalize_nullish_coalesce_in_short_circuit_rhs_native() {
    // native profiles normalize nullish coalesce in short-circuit rhs without eager hoisting
    let test = TestProgram::memory_sequential().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function isFallback(flag: boolean, name: string | undefined, fallback: string): boolean {
    return flag && ((name ?? fallback) == fallback);
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function isFallback(flag, name, fallback): boolean {
    return flag &&
        {
            const __coalesce_left8 = name;
            if (__coalesce_left8 == null ||
                __coalesce_left8 == undefined) fallback else __coalesce_left8
        } ==
            fallback;
}
"#,
    );
}

#[test]
fn test_normalize_nullish_coalesce_in_object_property_native() {
    // native profiles normalize nullish coalesce inside object property values
    let test = TestProgram::memory_sequential().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function choose(name: string | undefined, fallback: string): { value: string } {
    return { value: name ?? fallback };
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function choose(name, fallback): { value: string } {
    return {
        value: {
            const __coalesce_left9 = name;
            if (__coalesce_left9 == null ||
                __coalesce_left9 == undefined) fallback else __coalesce_left9
        },
    };
}
"#,
    );
}

#[test]
fn test_normalize_nullish_coalesce_in_array_element_native() {
    // native profiles normalize nullish coalesce inside array expression elements
    let test = TestProgram::memory_sequential().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function choose(name: string | undefined, fallback: string): string[] {
    return [name ?? fallback];
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function choose(name, fallback): string[] {
    return [
        {
            const __coalesce_left7 = name;
            if (__coalesce_left7 == null ||
                __coalesce_left7 == undefined) fallback else __coalesce_left7
        },
    ];
}
"#,
    );
}

#[test]
fn test_keep_nullish_coalesce_in_call_argument_js() {
    // js profiles retain nested nullish coalesce syntax
    let test = TestProgram::memory_sequential().with_profile_emit(EmitFormat::Js);
    let module_id = test.add_module(
        "test.ds",
        r#"
function wrap(value: string): string {
    return value;
}

function choose(name: string | undefined, fallback: string): string {
    return wrap(name ?? fallback);
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function wrap(value): string {
    return value;
}

function choose(name, fallback): string {
    return wrap(name ?? fallback);
}
"#,
    );
}

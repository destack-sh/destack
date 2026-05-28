use crate::tests::TestProgram;

#[test]
fn test_transform_match_literal_patterns() {
    // match on literal values transforms to nested if else with proper blocks
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function check(x: number): string {
    match (x) {
        1 => "one"
        2 => "two"
        _ => "other"
    }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile();
    test.check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function check(x): string {
    if (x == 1) {
        return "one";
    } else if (x == 2) {
        return "two";
    } else {
        return "other";
    }
}
"#,
    );
}

#[test]
fn test_transform_match_boolean() {
    // match on boolean: exhaustive optimization skips last check
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function check(b: boolean): number {
    match (b) {
        true => 1
        false => 0
    }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile();
    test.check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function check(b): number {
    if (b == true) {
        return 1;
    } else {
        return 0;
    }
}
"#,
    );
}

#[test]
fn test_transform_match_with_guard() {
    // match with guard clause transforms to nested if else
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function classify(x: number): string {
    match (x) {
        n if n > 0 => "positive"
        n if n < 0 => "negative"
        _ => "zero"
    }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function classify(x): string {
    if ({
        const n = x;
        n > (0 as number)
    }) {
        const n = x;
        return "positive";
    } else if ({
        const n = x;
        n < (0 as number)
    }) {
        const n = x;
        return "negative";
    } else {
        return "zero";
    }
}
"#,
    );
}

#[test]
fn test_transform_match_wildcard_only() {
    // match with only wildcard becomes the body in a block with explicit return
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function always(x: number): number {
    match (x) {
        _ => 42
    }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function always(x): number {
    return 42;
}
"#,
    );
}

#[test]
fn test_transform_match_union_pattern() {
    // union patterns transform to OR checks with proper blocks
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function isWeekend(day: number): boolean {
    match (day) {
        0 | 6 => true
        _ => false
    }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function isWeekend(day): boolean {
    if (day == 0 || day == 6) {
        return true;
    } else {
        return false;
    }
}
"#,
    );
}

#[test]
fn test_transform_match_to_if_else() {
    // match generates if else with proper blocks
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function sign(x: number): number {
    match (x) {
        0 => 0
        _ => 1
    }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function sign(x): number {
    if (x == 0) {
        return 0;
    } else {
        return 1;
    }
}
"#,
    );
}

#[test]
fn test_transform_match_tuple_pattern_bindings() {
    // tuple patterns bind fields through index accesses
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function project(point: (int32, int32)): int32 {
    match (point) {
        (x, y) => x - y
    }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function project(point): int32 {
    {
        const x = point[0];
        const y = point[1];
        return x - y;
    }
}
"#,
    );
}

#[test]
fn test_transform_match_object_pattern_bindings() {
    // object patterns bind fields through member accesses
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
type Pair = { left: int32; right: int32 };

function sumPair(pair: Pair): int32 {
    match (pair) {
        { left, right } => left + right
    }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
type Pair = { left: int32; right: int32 };
function sumPair(pair): int32 {
    {
        const left = pair.left;
        const right = pair.right;
        return left + right;
    }
}
"#,
    );
}

#[test]
fn test_transform_match_tagged_tuple_pattern_literal_and_binding() {
    // newtype patterns type check and bind matched fields
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
newtype Range = (int32, int32);

function width(value: Range): int32 {
    match (value) {
        Range(0, upper) => upper
        _ => 0
    }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
newtype Range = (int32, int32,);
function width(value): int32 {
    if (value is Range && value[0] == 0) {
        const upper = value[1];
        return upper;
    } else {
        return 0;
    }
}
"#,
    );
}

#[test]
fn test_transform_match_tagged_object_pattern_alias_bindings() {
    // nominal object patterns support alias bindings
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct User {
    age: int32
    score: int32
}

function summarize(user: User): int32 {
    match (user) {
        User { age: years, score } => years + score
        _ => 0
    }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
struct User {
    age: int32;
    score: int32;
}
function summarize(user): int32 {
    if (user is User) {
        const years = user.age;
        const score = user.score;
        return years + score;
    } else {
        return 0;
    }
}
"#,
    );
}

#[test]
fn test_transform_match_tuple_pattern_guard_uses_bound_values() {
    // tuple pattern guards evaluate with pattern bindings in scope
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function classify(point: (int32, int32)): int32 {
    match (point) {
        (x, y) if x > y => x - y
        _ => 0
    }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function classify(point): int32 {
    if ({
        const x = point[0];
        const y = point[1];
        x > y
    }) {
        const x = point[0];
        const y = point[1];
        return x - y;
    } else {
        return 0;
    }
}
"#,
    );
}

#[test]
fn test_transform_match_union_tuple_patterns_emit_slot_checks() {
    // union tuple patterns should emit per-slot checks, not fallback unsupported paths
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function classify(point: (int32, int32)): int32 {
    match (point) {
        (0, _) | (1, _) => 1
        _ => 0
    }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function classify(point): int32 {
    if (point[0] == 0 || point[0] == 1) {
        return 1;
    } else {
        return 0;
    }
}
"#,
    );
}

#[test]
fn test_transform_match_union_tagged_tuple_patterns_emit_full_checks() {
    // union newtype patterns should include both type and literal slot checks
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
newtype Range = (int32, int32);

function classify(value: Range): int32 {
    match (value) {
        Range(0, 1) | Range(2, 3) => 1
        _ => 0
    }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
newtype Range = (int32, int32,);
function classify(value): int32 {
    if ((value is Range && value[0] == 0 && value[1] == 1) ||
        (value is Range && value[0] == 2 && value[1] == 3)) {
        return 1;
    } else {
        return 0;
    }
}
"#,
    );
}

#[test]
fn test_transform_match_union_wrapper_patterns_emit_literal_checks() {
    // union wrapper patterns should lower to the same literal checks as plain patterns
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function isWeekend(day: number): boolean {
    match (day) {
        ^0 | 6 => true
        _ => false
    }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function isWeekend(day): boolean {
    if (day == 0 || day == 6) {
        return true;
    } else {
        return false;
    }
}
"#,
    );
}

#[test]
fn test_transform_match_tuple_wrapper_binding_extracts_slots() {
    // tuple field wrappers around bindings should still bind by slot access
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function project(point: (int32, int32)): int32 {
    match (point) {
        (^x, y) => x - y
        _ => 0
    }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function project(point): int32 {
    {
        const x = point[0];
        const y = point[1];
        return x - y;
    }
}
"#,
    );
}

#[test]
fn test_transform_match_object_wrapper_binding_extracts_members() {
    // object field wrappers around bindings should still bind by member access
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
type Pair = { left: int32; right: int32 };

function sumPair(pair: Pair): int32 {
    match (pair) {
        { left: &left, right } => left + right
        _ => 0
    }
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
type Pair = { left: int32; right: int32 };
function sumPair(pair): int32 {
    {
        const left = pair.left;
        const right = pair.right;
        return left + right;
    }
}
"#,
    );
}

use crate::tests::TestProgram;

/// A parameter bounded by Copy loads twice through one reference.
#[test]
fn test_copy_a_parameter_bounded_by_copy() {
    let mut program = TestProgram::mir(
        r#"
@languageItem("memory.Copy")
type Copy { }

function twice<T: Copy, 'a>(v0: ref<T, borrowed, 'a, readonly>): T {
entry(v0: ref<T, borrowed, 'a, readonly>):
    v1: T = load (*v0)
    v2: T = load (*v0)
    return v1
}
"#,
    );
    program.assert_verified();
}

/// A parameter without a Copy bound moves out of a reference on load.
#[test]
fn test_reject_a_load_of_an_unbounded_parameter_through_a_reference() {
    let mut program = TestProgram::mir(
        r#"
function take<T, 'a>(v0: ref<T, borrowed, 'a, readonly>): T {
entry(v0: ref<T, borrowed, 'a, readonly>):
    v1: T = load (*v0)
    return v1
}
"#,
    );
    program.assert_verify_errors(
        r#"
error[move-out-of-reference]: cannot move out through a reference
 ──▶ <test.tsppm>:4:5
  │
2 │ function take<T, 'a>(v0: ref<T, borrowed, 'a, readonly>): T {
3 │ entry(v0: ref<T, borrowed, 'a, readonly>):
4 │     v1: T = load (*v0)
  │     ^^^^^^^^^^^^^^^^^^
5 │     return v1
6 │ }
  │

for more information about an error, run `tspp explain move-out-of-reference`
"#,
    );
}

/// A parameter reaches Copy through the heritage of its bound.
#[test]
fn test_copy_a_parameter_through_its_bound_heritage() {
    let mut program = TestProgram::mir(
        r#"
@languageItem("memory.Copy")
type Copy { }

@languageItem("math.Integer")
type Integer extends Copy { }

function twice<T: Integer, 'a>(v0: ref<T, borrowed, 'a, readonly>): T {
entry(v0: ref<T, borrowed, 'a, readonly>):
    v1: T = load (*v0)
    v2: T = load (*v0)
    return v1
}
"#,
    );
    program.assert_verified();
}

/// A template struct copies once its parameter copies.
#[test]
fn test_copy_a_template_struct_whose_parameter_copies() {
    let mut program = TestProgram::mir(
        r#"
@languageItem("memory.Copy")
type Copy { }

type Slot<T> {
    value: T;
}

function twice<T: Copy, 'a>(v0: ref<Slot<T>, borrowed, 'a, readonly>): Slot<T> {
entry(v0: ref<Slot<T>, borrowed, 'a, readonly>):
    v1: Slot<T> = load (*v0)
    v2: Slot<T> = load (*v0)
    return v1
}
"#,
    );
    program.assert_verified();
}

/// A template struct moves while its parameter stays unbounded.
#[test]
fn test_reject_a_load_of_a_template_struct_over_an_unbounded_parameter() {
    let mut program = TestProgram::mir(
        r#"
type Slot<T> {
    value: T;
}

function take<T, 'a>(v0: ref<Slot<T>, borrowed, 'a, readonly>): Slot<T> {
entry(v0: ref<Slot<T>, borrowed, 'a, readonly>):
    v1: Slot<T> = load (*v0)
    return v1
}
"#,
    );
    program.assert_verify_errors(
        r#"
error[move-out-of-reference]: cannot move out through a reference
  ──▶ <test.tsppm>:8:5
   │
 6 │ function take<T, 'a>(v0: ref<Slot<T>, borrowed, 'a, readonly>): Slot<T> {
 7 │ entry(v0: ref<Slot<T>, borrowed, 'a, readonly>):
 8 │     v1: Slot<T> = load (*v0)
   │     ^^^^^^^^^^^^^^^^^^^^^^^^
 9 │     return v1
10 │ }
   │

for more information about an error, run `tspp explain move-out-of-reference`
"#,
    );
}

/// A representation copies by its arguments: an int32 slot copies, a unique slot moves.
#[test]
fn test_copy_a_representation_by_its_arguments() {
    let mut program = TestProgram::mir(
        r#"
type Slot<T> {
    value: T;
}

function copies<'a>(v0: ref<Slot<int32>, borrowed, 'a, readonly>): Slot<int32> {
entry(v0: ref<Slot<int32>, borrowed, 'a, readonly>):
    v1: Slot<int32> = load (*v0)
    v2: Slot<int32> = load (*v0)
    return v1
}

function moves<'a>(v0: ref<Slot<ref<int32, unique, mutable>>, borrowed, 'a, readonly>): Slot<ref<int32, unique, mutable>> {
entry(v0: ref<Slot<ref<int32, unique, mutable>>, borrowed, 'a, readonly>):
    v1: Slot<ref<int32, unique, mutable>> = load (*v0)
    return v1
}
"#,
    );
    program.assert_verify_errors(
        r#"
error[move-out-of-reference]: cannot move out through a reference
  ──▶ <test.tsppm>:15:5
   │
13 │ function moves<'a>(v0: ref<Slot<ref<int32, unique, mutable>>, borrowed, 'a, readonly>): Slot<ref<int··
14 │ entry(v0: ref<Slot<ref<int32, unique, mutable>>, borrowed, 'a, readonly>):
15 │     v1: Slot<ref<int32, unique, mutable>> = load (*v0)
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
16 │     return v1
17 │ }
   │

for more information about an error, run `tspp explain move-out-of-reference`
"#,
    );
}

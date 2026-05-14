use crate::TestProgram;
/// Lower Vector<T, N> types into MIR vector types.
#[test]
fn test_lower_vector_color_signature() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function toneMap(color: Vector<float32, 4>): Vector<float32, 4> {
    let energy = reduceAdd<float32, 4>(color);
    return splat<float32, 4>(energy);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function toneMap(value0: vector<float32, 4>): vector<float32, 4> {
entry0(value0: vector<float32, 4>):
    value1: float32 = vector.reduce add, value0
    value2: vector<float32, 4> = vector.splat value1
    return value2
}
"#,
    );
}

/// Lower vector types in struct fields.
#[test]
fn test_lower_vector_struct_field() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Particle { velocity: Vector<float32, 3> }

function readVelocity(value: Particle): Vector<float32, 3> {
    return value.velocity;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
type Particle {
    velocity: vector<float32, 3>;
}

function readVelocity(value0: Particle): vector<float32, 3> {
entry0(value0: Particle):
    value1: vector<float32, 3> = field.get value0, 0
    return value1
}
"#,
    );
}

/// Lower vector types inside tuple signatures.
#[test]
fn test_lower_vector_tuple_type() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function passStereoFrame(value: (Vector<int16, 8>, Vector<int16, 8>)): (Vector<int16, 8>, Vector<int16, 8>) {
    return value;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
type passStereoFrame.value#tuple = (vector<int16, 8>, vector<int16, 8>);

function passStereoFrame(value0: passStereoFrame.value#tuple): passStereoFrame.value#tuple {
entry0(value0: passStereoFrame.value#tuple):
    return value0
}
"#,
    );
}

/// Lower vector aliases to the underlying MIR vector type.
#[test]
fn test_lower_vector_alias_type() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
type Rgba = Vector<float32, 4>;

function applyTint(color: Rgba): Rgba {
    return color;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function applyTint(value0: vector<float32, 4>): vector<float32, 4> {
entry0(value0: vector<float32, 4>):
    return value0
}
"#,
    );
}

/// Lower vector splat intrinsic to MIR vector.splat.
#[test]
fn test_lower_vector_splat_intrinsic() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function sumSplat(value: float32): float32 {
    let lanes = splat<float32, 4>(value);
    return reduceAdd<float32, 4>(lanes);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function sumSplat(value0: float32): float32 {
entry0(value0: float32):
    value1: vector<float32, 4> = vector.splat value0
    value2: float32 = vector.reduce add, value1
    return value2
}
"#,
    );
}

/// Lower vector select intrinsic to MIR vector.select.
#[test]
fn test_lower_vector_select_intrinsic() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function sumSelected(
    mask: Vector<boolean, 4>,
    a: Vector<float32, 4>,
    b: Vector<float32, 4>
): float32 {
    let chosen = select<float32, 4>(mask, a, b);
    return reduceAdd<float32, 4>(chosen);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function sumSelected(value0: vector<boolean, 4>, value1: vector<float32, 4>, value2: vector<float32, 4>): float32 {
entry0(value0: vector<boolean, 4>, value1: vector<float32, 4>, value2: vector<float32, 4>):
    value3: vector<float32, 4> = vector.select value0, value1, value2
    value4: float32 = vector.reduce add, value3
    return value4
}
"#,
    );
}

/// Lower vector reduce intrinsics to MIR vector.reduce.
#[test]
fn test_lower_vector_reduce_intrinsic() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function sumLanes(value: Vector<float32, 4>, bias: float32): float32 {
    let bias = splat<float32, 4>(bias);
    let sum = reduceAdd<float32, 4>(value);
    let bias_sum = reduceAdd<float32, 4>(bias);
    return sum + bias_sum;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
function sumLanes(value0: vector<float32, 4>, value1: float32): float32 {
entry0(value0: vector<float32, 4>, value1: float32):
    value2: vector<float32, 4> = vector.splat value1
    value3: float32 = vector.reduce add, value0
    value4: float32 = vector.reduce add, value2
    value5: float32 = float.add value3, value4
    return value5
}
"#,
    );
}

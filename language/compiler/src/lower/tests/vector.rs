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
function toneMap(v0: vector<float32, 4>): vector<float32, 4> {
entry(v0: vector<float32, 4>):
    v1: float32 = vector.reduce add, v0
    v2: vector<float32, 4> = vector.splat v1
    return v2
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

function readVelocity(v0: Particle): vector<float32, 3> {
entry(v0: Particle):
    v1: vector<float32, 3> = field.get v0, 0
    return v1
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

function passStereoFrame(v0: passStereoFrame.value#tuple): passStereoFrame.value#tuple {
entry(v0: passStereoFrame.value#tuple):
    return v0
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
function applyTint(v0: vector<float32, 4>): vector<float32, 4> {
entry(v0: vector<float32, 4>):
    return v0
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
function sumSplat(v0: float32): float32 {
entry(v0: float32):
    v1: vector<float32, 4> = vector.splat v0
    v2: float32 = vector.reduce add, v1
    return v2
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
function sumSelected(v0: vector<boolean, 4>, v1: vector<float32, 4>, v2: vector<float32, 4>): float32 {
entry(v0: vector<boolean, 4>, v1: vector<float32, 4>, v2: vector<float32, 4>):
    v3: vector<float32, 4> = vector.select v0, v1, v2
    v4: float32 = vector.reduce add, v3
    return v4
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
function sumLanes(v0: vector<float32, 4>, v1: float32): float32 {
entry(v0: vector<float32, 4>, v1: float32):
    v2: vector<float32, 4> = vector.splat v1
    v3: float32 = vector.reduce add, v0
    v4: float32 = vector.reduce add, v2
    v5: float32 = float.add v3, v4
    return v5
}
"#,
    );
}

use destack_mir::{FloatType, Space};
use destack_program::{ScalarFormat, Word};

use super::{TestMachine, TestProgram};

/// Execute tensor allocation, elementwise arithmetic, and scalar extraction.
#[test]
fn test_execute_tensor_arithmetic() {
    let first = TestProgram::tensor_allocation(0, 0, Space::Local, 0);
    let second = TestProgram::tensor_allocation(0, 1, Space::Local, 0);
    let sum = TestProgram::tensor_allocation(0, 2, Space::Local, 0);
    let test = TestProgram::words()
        .tensor(0, 1, ScalarFormat::int(32, true), Space::Local, [2, 2])
        .allocations([first, second, sum]);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    tensor.splat r4, r0, a0
    tensor.splat r5, r1, a1
    tensor.element r6, [(r4, l0), (r5, l0)], int.add, a2
    tensor.extract r7, (r6, l0), [r2, r3]
    return r7
}
"#,
        test,
    );

    let value = machine.complete(
        0,
        &[
            Word::int32(13),
            Word::int32(29),
            Word::uint64(1),
            Word::uint64(0),
        ],
    );

    assert_eq!(value, vec![Word::int32(42)]);
}

/// Execute a tensor reshape while preserving logical element order.
#[test]
fn test_execute_tensor_reshape() {
    let matrix = TestProgram::tensor_allocation(0, 0, Space::Local, 0);
    let row = TestProgram::tensor_allocation(0, 1, Space::Local, 1);
    let test = TestProgram::words()
        .tensor(0, 2, ScalarFormat::int(32, true), Space::Local, [2, 2])
        .tensor(1, 2, ScalarFormat::int(32, true), Space::Local, [1, 4])
        .allocations([matrix, row]);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    tensor.splat r5, r0, a0
    tensor.reshape r6, (r5, l0), [r1, r2], a1
    tensor.extract r7, (r6, l1), [r3, r4]
    return r7
}
"#,
        test,
    );

    let value = machine.complete(
        0,
        &[
            Word::int32(37),
            Word::uint64(1),
            Word::uint64(4),
            Word::uint64(0),
            Word::uint64(3),
        ],
    );

    assert_eq!(value, vec![Word::int32(37)]);
}

/// Transform, slice, and pad tensors while preserving axis order and values.
#[test]
fn test_execute_tensor_shape_pipeline() {
    let first = TestProgram::tensor_allocation(0, 0, Space::Local, 0);
    let second = TestProgram::tensor_allocation(0, 1, Space::Local, 0);
    let matrix = TestProgram::tensor_allocation(0, 2, Space::Local, 1);
    let transposed = TestProgram::tensor_allocation(0, 3, Space::Local, 1);
    let column = TestProgram::tensor_allocation(0, 4, Space::Local, 2);
    let padded = TestProgram::tensor_allocation(0, 5, Space::Local, 3);
    let test = TestProgram::words()
        .tensor(0, 4, ScalarFormat::int(32, true), Space::Local, [1, 2])
        .tensor(1, 4, ScalarFormat::int(32, true), Space::Local, [2, 2])
        .tensor(2, 4, ScalarFormat::int(32, true), Space::Local, [2, 1])
        .tensor(3, 4, ScalarFormat::int(32, true), Space::Local, [2, 2])
        .allocations([first, second, matrix, transposed, column, padded]);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    tensor.splat r6, r0, a0
    tensor.splat r7, r1, a1
    tensor.concat r8, [(r6, l0), (r7, l0)], 0, a2
    tensor.transpose r9, (r8, l1), [1, 0], a3
    tensor.slice r10, (r9, l1), [r3, r4], [r5, r4], [r4, r4], a4
    tensor.pad r11, (r10, l2), r2, [r3, r4], [r3, r3], [r3, r3], a5
    tensor.extract r12, (r11, l3), [r4, r4]
    tensor.extract r13, (r11, l3), [r3, r3]
    return r12:r13
}
"#,
        test,
    );

    let value = machine.complete(
        0,
        &[
            Word::int32(7),
            Word::int32(9),
            Word::int32(0),
            Word::uint64(0),
            Word::uint64(1),
            Word::uint64(2),
        ],
    );

    assert_eq!(value, vec![Word::int32(9), Word::int32(0)]);
}

/// Reduce tensor values and return the selected extremum index.
#[test]
fn test_execute_tensor_reductions() {
    let first = TestProgram::tensor_allocation(0, 0, Space::Local, 0);
    let second = TestProgram::tensor_allocation(0, 1, Space::Local, 0);
    let matrix = TestProgram::tensor_allocation(0, 2, Space::Local, 1);
    let sums = TestProgram::tensor_allocation(0, 3, Space::Local, 2);
    let indices = TestProgram::tensor_allocation(0, 4, Space::Local, 3);
    let test = TestProgram::words()
        .tensor(0, 4, ScalarFormat::int(32, true), Space::Local, [1, 2])
        .tensor(1, 4, ScalarFormat::int(32, true), Space::Local, [2, 2])
        .tensor(2, 4, ScalarFormat::int(32, true), Space::Local, [2])
        .tensor(3, 5, ScalarFormat::int(64, false), Space::Local, [2])
        .allocations([first, second, matrix, sums, indices]);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    tensor.splat r4, r0, a0
    tensor.splat r5, r1, a1
    tensor.concat r6, [(r4, l0), (r5, l0)], 0, a2
    tensor.reduce r7, (r6, l1), r2, add, [0], a3
    tensor.indexReduce r8, (r6, l1), max, 0, 0, a4
    tensor.extract r9, (r7, l2), [r3]
    tensor.extract r10, (r8, l3), [r3]
    return r9:r10
}
"#,
        test,
    );

    let value = machine.complete(
        0,
        &[
            Word::int32(3),
            Word::int32(7),
            Word::int32(0),
            Word::uint64(1),
        ],
    );

    assert_eq!(value, vec![Word::int32(10), Word::uint64(1)]);
}

/// Compare, select, and convert tensor elements through scalar operation families.
#[test]
fn test_execute_tensor_element_pipeline() {
    let first = TestProgram::tensor_allocation(0, 0, Space::Local, 0);
    let second = TestProgram::tensor_allocation(0, 1, Space::Local, 0);
    let compared = TestProgram::tensor_allocation(0, 2, Space::Local, 1);
    let selected = TestProgram::tensor_allocation(0, 3, Space::Local, 0);
    let converted = TestProgram::tensor_allocation(0, 4, Space::Local, 2);
    let test = TestProgram::words()
        .tensor(0, 3, ScalarFormat::int(32, true), Space::Local, [2, 2])
        .tensor(1, 4, ScalarFormat::Boolean, Space::Local, [2, 2])
        .tensor(
            2,
            5,
            ScalarFormat::float(FloatType::Float32),
            Space::Local,
            [2, 2],
        )
        .allocations([first, second, compared, selected, converted]);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    tensor.splat r4, r0, a0
    tensor.splat r5, r1, a1
    tensor.compare r6, [(r4, l0), (r5, l0)], int.lt, a2
    tensor.select r7, [(r6, l1), (r5, l0), (r4, l0)], a3
    tensor.convert r8, (r7, l0), exact, a4
    tensor.extract r9, (r8, l2), [r2, r3]
    return r9
}
"#,
        test,
    );

    let value = machine.complete(
        0,
        &[
            Word::int32(4),
            Word::int32(9),
            Word::uint64(1),
            Word::uint64(0),
        ],
    );

    assert_eq!(value, vec![Word::float32(9.0)]);
}

/// Execute tensor-view stores and loads through one stable local heap edge.
#[test]
fn test_execute_tensor_view_memory() {
    let matrix = TestProgram::tensor_allocation(0, 0, Space::Local, 0);
    let test = TestProgram::words()
        .tensor(0, 2, ScalarFormat::int(32, true), Space::Local, [2, 2])
        .tensor_view(1, 2, ScalarFormat::int(32, true), Space::Local, [2, 2])
        .allocations([matrix]);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    tensor.splat r1, r0, a0
    return r1
}

function f1 {
    tensor.store (r0:r5, l1), [r6, r7], r8
    tensor.load r9, (r0:r5, l1), [r6, r7]
    return r9
}
"#,
        test,
    );

    // allocate one managed tensor through bytecode
    let value = machine.complete(0, &[Word::int32(0)]);

    // describe its complete row-major payload as one borrowed view
    let edge = value[0];
    let view = [
        edge,
        Word::uint64((2 * Word::BYTE_LEN) as u64),
        Word::uint64(2),
        Word::uint64(2),
        Word::uint64(2 * size_of::<i32>() as u64),
        Word::uint64(size_of::<i32>() as u64),
        Word::uint64(1),
        Word::uint64(0),
        Word::int32(73),
    ];
    let value = machine.complete(1, &view);

    assert_eq!(value, vec![Word::int32(73)]);
}

/// Allocate and execute one managed tensor in shared storage.
#[test]
fn test_execute_shared_tensor() {
    let matrix = TestProgram::tensor_allocation(0, 0, Space::Shared, 0);
    let test = TestProgram::words()
        .tensor(0, 1, ScalarFormat::int(32, true), Space::Shared, [2, 2])
        .allocations([matrix]);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    tensor.splat r3, r0, a0
    tensor.extract r4, (r3, l0), [r1, r2]
    return r4
}
"#,
        test,
    );

    let value = machine.complete(0, &[Word::int32(91), Word::uint64(1), Word::uint64(1)]);

    assert_eq!(value, vec![Word::int32(91)]);
}

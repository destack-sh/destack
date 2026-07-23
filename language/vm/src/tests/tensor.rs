use destack_mir::{FloatType, Space};
use destack_program::{ScalarFormat, Word};

use super::{TestMachine, TestProgram};

/// Execute tensor allocation, elementwise arithmetic, and scalar extraction.
#[test]
fn test_execute_tensor_arithmetic() {
    let first = TestMachine::tensor_allocation(0, 0, Space::Local, 0);
    let second = TestMachine::tensor_allocation(0, 1, Space::Local, 0);
    let sum = TestMachine::tensor_allocation(0, 2, Space::Local, 0);
    let test = TestProgram::new()
        .tensor(0, 1, ScalarFormat::int(32, true), Space::Local, [2, 2])
        .allocations([first, second, sum]);
    let mut machine = TestMachine::parse(
        r#"
type Matrix
type Element

export function add(
    r0: int32,
    r1: int32,
    r2: uint64,
    r3: uint64,
): int32 {
    r4: tensor<int32, Matrix, space(local)> = tensor.splat r0
    r5: tensor<int32, Matrix, space(local)> = tensor.splat r1
    r6: tensor<int32, Matrix, space(local)> = int.add r4, r5
    r7: int32 = tensor.extract r6, [r2, r3]
    return r7
}
"#,
        test,
    );

    let value = machine.complete(
        "add",
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
    let matrix = TestMachine::tensor_allocation(0, 0, Space::Local, 0);
    let row = TestMachine::tensor_allocation(0, 1, Space::Local, 1);
    let test = TestProgram::new()
        .tensor(0, 2, ScalarFormat::int(32, true), Space::Local, [2, 2])
        .tensor(1, 2, ScalarFormat::int(32, true), Space::Local, [1, 4])
        .allocations([matrix, row]);
    let mut machine = TestMachine::parse(
        r#"
type Matrix
type Row
type Element

export function reshape(
    r0: int32,
    r1: uint64,
    r2: uint64,
    r3: uint64,
    r4: uint64,
): int32 {
    r5: tensor<int32, Matrix, space(local)> = tensor.splat r0
    r6: tensor<int32, Row, space(local)> = tensor.reshape r5, shape(r1, r2)
    r7: int32 = tensor.extract r6, [r3, r4]
    return r7
}
"#,
        test,
    );

    let value = machine.complete(
        "reshape",
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
    let first = TestMachine::tensor_allocation(0, 0, Space::Local, 0);
    let second = TestMachine::tensor_allocation(0, 1, Space::Local, 0);
    let matrix = TestMachine::tensor_allocation(0, 2, Space::Local, 1);
    let transposed = TestMachine::tensor_allocation(0, 3, Space::Local, 1);
    let column = TestMachine::tensor_allocation(0, 4, Space::Local, 2);
    let padded = TestMachine::tensor_allocation(0, 5, Space::Local, 3);
    let test = TestProgram::new()
        .tensor(0, 4, ScalarFormat::int(32, true), Space::Local, [1, 2])
        .tensor(1, 4, ScalarFormat::int(32, true), Space::Local, [2, 2])
        .tensor(2, 4, ScalarFormat::int(32, true), Space::Local, [2, 1])
        .tensor(3, 4, ScalarFormat::int(32, true), Space::Local, [2, 2])
        .allocations([first, second, matrix, transposed, column, padded]);
    let mut machine = TestMachine::parse(
        r#"
type Row
type Matrix
type Column
type Padded
type Element

export function transform(
    r0: int32,
    r1: int32,
    r2: int32,
    r3: uint64,
    r4: uint64,
    r5: uint64,
): (int32, int32) {
    r6: tensor<int32, Row, space(local)> = tensor.splat r0
    r7: tensor<int32, Row, space(local)> = tensor.splat r1
    r8: tensor<int32, Matrix, space(local)> = tensor.concat tensors(r6, r7), axis(0)
    r9: tensor<int32, Matrix, space(local)> = tensor.transpose r8, permutation(1, 0)
    r10: tensor<int32, Column, space(local)> = tensor.slice r9,
        offsets(r3, r4),
        sizes(r5, r4),
        strides(r4, r4)
    r11: tensor<int32, Padded, space(local)> = tensor.pad r10,
        value(r2),
        low(r3, r4),
        high(r3, r3),
        interior(r3, r3)
    r12: int32 = tensor.extract r11, [r4, r4]
    r13: int32 = tensor.extract r11, [r3, r3]
    return r12, r13
}
"#,
        test,
    );

    let value = machine.complete(
        "transform",
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
    let first = TestMachine::tensor_allocation(0, 0, Space::Local, 0);
    let second = TestMachine::tensor_allocation(0, 1, Space::Local, 0);
    let matrix = TestMachine::tensor_allocation(0, 2, Space::Local, 1);
    let sums = TestMachine::tensor_allocation(0, 3, Space::Local, 2);
    let indices = TestMachine::tensor_allocation(0, 4, Space::Local, 3);
    let test = TestProgram::new()
        .tensor(0, 4, ScalarFormat::int(32, true), Space::Local, [1, 2])
        .tensor(1, 4, ScalarFormat::int(32, true), Space::Local, [2, 2])
        .tensor(2, 4, ScalarFormat::int(32, true), Space::Local, [2])
        .tensor(3, 5, ScalarFormat::int(64, false), Space::Local, [2])
        .allocations([first, second, matrix, sums, indices]);
    let mut machine = TestMachine::parse(
        r#"
type Row
type Matrix
type Reduced
type Indices
type IntElement
type IndexElement

export function reduce(
    r0: int32,
    r1: int32,
    r2: int32,
    r3: uint64,
): (int32, uint64) {
    r4: tensor<int32, Row, space(local)> = tensor.splat r0
    r5: tensor<int32, Row, space(local)> = tensor.splat r1
    r6: tensor<int32, Matrix, space(local)> = tensor.concat tensors(r4, r5), axis(0)
    r7: tensor<int32, Reduced, space(local)> = tensor.reduce add, r6, r2, axes(0)
    r8: tensor<uint64, Indices, space(local)> = tensor.indexReduce max, r6,
        axis(0),
        tieBreak(first)
    r9: int32 = tensor.extract r7, [r3]
    r10: uint64 = tensor.extract r8, [r3]
    return r9, r10
}
"#,
        test,
    );

    let value = machine.complete(
        "reduce",
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
    let first = TestMachine::tensor_allocation(0, 0, Space::Local, 0);
    let second = TestMachine::tensor_allocation(0, 1, Space::Local, 0);
    let compared = TestMachine::tensor_allocation(0, 2, Space::Local, 1);
    let selected = TestMachine::tensor_allocation(0, 3, Space::Local, 0);
    let converted = TestMachine::tensor_allocation(0, 4, Space::Local, 2);
    let test = TestProgram::new()
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
type IntMatrix
type BooleanMatrix
type FloatMatrix
type IntElement
type BooleanElement
type FloatElement

export function choose(
    r0: int32,
    r1: int32,
    r2: uint64,
    r3: uint64,
): float32 {
    r4: tensor<int32, IntMatrix, space(local)> = tensor.splat r0
    r5: tensor<int32, IntMatrix, space(local)> = tensor.splat r1
    r6: tensor<boolean, BooleanMatrix, space(local)> = tensor.compare int.lt, r4, r5
    r7: tensor<int32, IntMatrix, space(local)> = tensor.select r6, r5, r4
    r8: tensor<float32, FloatMatrix, space(local)> = tensor.convert exact, r7
    r9: float32 = tensor.extract r8, [r2, r3]
    return r9
}
"#,
        test,
    );

    let value = machine.complete(
        "choose",
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
    let matrix = TestMachine::tensor_allocation(0, 0, Space::Local, 0);
    let test = TestProgram::new()
        .tensor(0, 2, ScalarFormat::int(32, true), Space::Local, [2, 2])
        .tensor_view(1, 2, ScalarFormat::int(32, true), Space::Local, [2, 2])
        .allocations([matrix]);
    let mut machine = TestMachine::parse(
        r#"
type Matrix
type View
type Element

export function allocate(r0: int32): tensor<int32, Matrix, space(local)> {
    r1: tensor<int32, Matrix, space(local)> = tensor.splat r0
    return r1
}

export function access(
    r0: tensorView<int32, View, borrowed, space(local), 6>,
    r6: uint64,
    r7: uint64,
    r8: int32,
): int32 {
    tensor.store r0, [r6, r7], r8
    r9: int32 = tensor.load r0, [r6, r7]
    return r9
}
"#,
        test,
    );

    // allocate one owning tensor through bytecode
    let value = machine.complete("allocate", &[Word::int32(0)]);

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
    let value = machine.complete("access", &view);

    assert_eq!(value, vec![Word::int32(73)]);
}

/// Allocate and execute one owning tensor in shared storage.
#[test]
fn test_execute_shared_tensor() {
    let matrix = TestMachine::tensor_allocation(0, 0, Space::Shared, 0);
    let test = TestProgram::new()
        .tensor(0, 1, ScalarFormat::int(32, true), Space::Shared, [2, 2])
        .allocations([matrix]);
    let mut machine = TestMachine::parse(
        r#"
type Matrix
type Element

export function splat(
    r0: int32,
    r1: uint64,
    r2: uint64,
): int32 {
    r3: tensor<int32, Matrix, space(shared)> = tensor.splat r0
    r4: int32 = tensor.extract r3, [r1, r2]
    return r4
}
"#,
        test,
    );

    let value = machine.complete(
        "splat",
        &[Word::int32(91), Word::uint64(1), Word::uint64(1)],
    );

    assert_eq!(value, vec![Word::int32(91)]);
}

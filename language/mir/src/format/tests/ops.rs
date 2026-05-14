use super::assert_format;

/// Formats vector and tensor operations canonically.
#[test]
fn test_format_vector_tensor_ops() {
    assert_format(
        r#"
function vectorTensorOps(value0: vector<int32, 4>, value1: int32, value2: tensor<int32, (2, 2)>, value3: tensorView<int32, borrowed, (2, 2)>): tensor<int32, (2, 2)> {
entry0(value0: vector<int32, 4>, value1: int32, value2: tensor<int32, (2, 2)>, value3: tensorView<int32, borrowed, (2, 2)>):
    value4: vector<int32, 4> = vector.splat value1
    value5: int32 = vector.extract value4, value1
    value6: vector<int32, 4> = vector.insert value4, value1, value1
    value7: vector<int32, 4> = vector.shuffle value4, value6, [0, 1, 2, 3]
    value8: int32 = vector.reduce add, value7
    value9: vector<boolean, 4> = vector.compare int.eq, value4, value6
    value10: vector<int32, 4> = vector.convert exact, value4
    value11: int32 = 0int32
    value12: int32 = 1int32
    value13: int32 = tensor.load value3, [value11, value12]
    tensor.store value3, [value12, value11], value13
    tensor.fill value3, value11
    tensor.copy value3, value3
    value14: tensor<int32, (2, 2)> = tensor.reshape value2, shape(value11, value12)
    value15: tensor<int32, (2, 2)> = tensor.broadcast value2, dimensions(0, 1)
    value16: tensor<int32, (2, 2)> = tensor.transpose value2, permutation(1, 0)
    value17: tensor<int32, (2, 2)> = tensor.cast value2
    value18: tensorView<int32, borrowed, (2, 2)> = tensor.view value3, offsets(value11, value11), sizes(value12, value12), strides(value12, value12)
    value19: tensor<int32, (2, 2)> = tensor.slice value2, offsets(value11, value11), sizes(value12, value12), strides(value12, value12)
    value20: tensor<int32, (2, 2)> = tensor.pad value2, value(value11), low(value11, value11), high(value11, value11), interior(value11, value11)
    value21: tensor<int32, (2, 2)> = tensor.concat tensors(value2, value2), axis(0)
    value22: tensor<boolean, (2, 2)> = tensor.compare int.eq, value2, value2
    value23: tensor<int32, (2, 2)> = tensor.reduce add, value2, value11, axes(0)
    value24: tensor<uint64, (2, 2)> = tensor.indexReduce min, value2, axis(0), tieBreak(first)
    value25: tensor<int32, (2, 2)> = tensor.dot value2, value2, dims(lhsBatch(), rhsBatch(), lhsContract(1), rhsContract(0))
    value26: tensor<int32, (2, 2)> = tensor.convolution value2, value2, dims(inputBatch(0), inputFeature(1), inputSpatial(2, 3), kernelInputFeature(0), kernelOutputFeature(1), kernelSpatial(2, 3), outputBatch(0), outputFeature(1), outputSpatial(2, 3)), window(strides(1, 1), paddingLow(0, 0), paddingHigh(0, 0), lhsDilation(1, 1), rhsDilation(1, 1), windowReversal(false, false)), groups(feature(1), batch(1))
    value27: tensor<int32, (2, 2)> = tensor.gather value2, value2, dims(offsetDims(0), collapsedSliceDims(1), startIndexMap(0), indexVectorDim(1)), sliceSizes(1, 1)
    value28: tensor<int32, (2, 2)> = tensor.scatter value2, value2, value2, dims(updateWindowDims(0), insertedWindowDims(1), scatterDimsToOperandDims(0), indexVectorDim(1)), mode(replace)
    value29: tensor<float32, (2, 2)> = tensor.convert exact, value2
    value30: tensor<int32, (2, 2)> = tensor.splat value1
    return value14
}
"#,
    );
}

/// Formats check terminators and assume instructions canonically.
#[test]
fn test_format_check_and_assume() {
    assert_format(
        r#"
function guard(value0: uint32, value1: uint32, value2: int32[4]): int32 {
entry0(value0: uint32, value1: uint32, value2: int32[4]):
    value3: boolean = int.lt.u value0, value1
    assume value3
    check bounds.u value0, value1, value2 -> block1(value0), block2

block1(value4: uint32):
    value5: int32 = 0int32
    return value5

block2:
    unreachable
}
"#,
    );
}

/// Formats direct and indirect calls canonically.
#[test]
fn test_format_calls() {
    assert_format(
        r#"
extern function callee(int32, int32): int32

function caller(): int32 {
entry0:
    value0: int32 = 1int32
    value1: int32 = 2int32
    value2: int32 = call callee(value0, value1): (int32, int32) -> int32
    value3: (int32, int32) -> int32 = function.address callee
    value4: int32 = call.indirect value3(value0, value1): (int32, int32) -> int32
    return value4
}
"#,
    );
}

/// Formats void calls with callable type arguments canonically.
#[test]
fn test_format_void_call_with_callable_argument() {
    assert_format(
        r#"
extern function consume(() -> int32): void

function caller(value0: () -> int32): void {
entry0(value0: () -> int32):
    call consume(value0): (() -> int32) -> void
    return
}
"#,
    );
}

/// Formats scalar instruction families canonically.
#[test]
fn test_format_scalar_instruction_families() {
    assert_format(
        r#"
function scalarOps(value0: int32, value1: int32, value2: boolean, value3: float64): int64 {
entry0(value0: int32, value1: int32, value2: boolean, value3: float64):
    value4: boolean = int.lt.s value0, value1
    value5: int32 = select value2, value0, value1
    value6: int32 = int.negate value5
    value7: int32 = int.not value6
    value8: float64 = float.negate value3
    value9: int64 = cast.extend.s value7 -> int64
    value10: float64 = intrinsic.math.float.sqrt(value3)
    value11: float64 = intrinsic.math.float.min(value8, value3)
    value12: float64 = intrinsic.math.float.fma(value8, value3, value11)
    value13: (int32, boolean) = intrinsic.math.arithmetic.add.overflow(value0, value1)
    return value9
}
"#,
    );
}

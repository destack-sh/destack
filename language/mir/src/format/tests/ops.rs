use super::assert_format;

/// Formats vector and tensor operations canonically.
#[test]
fn test_format_vector_tensor_ops() {
    assert_format(
        r#"
function vectorTensorOps(v0: vector<int32, 4>, v1: int32, v2: tensor<int32, local, (2, 2)>, v3: tensorView<int32, borrowed, mutable, (2, 2)>): tensor<int32, local, (2, 2)> {
entry(v0: vector<int32, 4>, v1: int32, v2: tensor<int32, local, (2, 2)>, v3: tensorView<int32, borrowed, mutable, (2, 2)>):
    v4: vector<int32, 4> = vector.splat v1
    v5: int32 = vector.extract v4, v1
    v6: vector<int32, 4> = vector.insert v4, v1, v1
    v7: vector<int32, 4> = vector.shuffle v4, v6, [0, 1, 2, 3]
    v8: int32 = vector.reduce add, v7
    v9: vector<boolean, 4> = vector.compare int.eq, v4, v6
    v10: vector<int32, 4> = vector.convert exact, v4
    v11: int32 = 0
    v12: int32 = 1
    v13: int32 = tensor.load v3, [v11, v12]
    tensor.store v3, [v12, v11], v13
    tensor.fill v3, v11
    tensor.copy v3, v3
    v14: tensor<int32, local, (2, 2)> = tensor.reshape v2, shape(v11, v12)
    v15: tensor<int32, local, (2, 2)> = tensor.broadcast v2, dimensions(0, 1)
    v16: tensor<int32, local, (2, 2)> = tensor.transpose v2, permutation(1, 0)
    v17: tensor<int32, local, (2, 2)> = tensor.cast v2
    v18: tensorView<int32, borrowed, mutable, (2, 2)> = tensor.view v3, offsets(v11, v11), sizes(v12, v12), strides(v12, v12)
    v19: tensor<int32, local, (2, 2)> = tensor.slice v2, offsets(v11, v11), sizes(v12, v12), strides(v12, v12)
    v20: tensor<int32, local, (2, 2)> = tensor.pad v2, value(v11), low(v11, v11), high(v11, v11), interior(v11, v11)
    v21: tensor<int32, local, (2, 2)> = tensor.concat tensors(v2, v2), axis(0)
    v22: tensor<boolean, local, (2, 2)> = tensor.compare int.eq, v2, v2
    v23: tensor<int32, local, (2, 2)> = tensor.reduce add, v2, v11, axes(0)
    v24: tensor<uint64, local, (2, 2)> = tensor.indexReduce min, v2, axis(0), tieBreak(first)
    v25: tensor<int32, local, (2, 2)> = tensor.dot v2, v2, dims(lhsBatch(), rhsBatch(), lhsContract(1), rhsContract(0))
    v26: tensor<int32, local, (2, 2)> = tensor.convolution v2, v2, dims(inputBatch(0), inputFeature(1), inputSpatial(2, 3), kernelInputFeature(0), kernelOutputFeature(1), kernelSpatial(2, 3), outputBatch(0), outputFeature(1), outputSpatial(2, 3)), window(strides(1, 1), paddingLow(0, 0), paddingHigh(0, 0), lhsDilation(1, 1), rhsDilation(1, 1), windowReversal(false, false)), groups(feature(1), batch(1))
    v27: tensor<int32, local, (2, 2)> = tensor.gather v2, v2, dims(offsetDims(0), collapsedSliceDims(1), startIndexMap(0), indexVectorDim(1)), sliceSizes(1, 1)
    v28: tensor<int32, local, (2, 2)> = tensor.scatter v2, v2, v2, dims(updateWindowDims(0), insertedWindowDims(1), scatterDimsToOperandDims(0), indexVectorDim(1)), mode(replace)
    v29: tensor<float32, local, (2, 2)> = tensor.convert exact, v2
    v30: tensor<int32, local, (2, 2)> = tensor.splat v1
    return v14
}
"#,
    );
}

/// Formats check terminators and marker instructions canonically.
#[test]
fn test_format_check_and_assume() {
    assert_format(
        r#"
function guard(v0: uint32, v1: uint32, v2: [int32; 4]): int32 {
entry(v0: uint32, v1: uint32, v2: [int32; 4]):
    v3: boolean = int.lt.u v0, v1
    assume v3
    breakpoint
    profile.increment counter(0)
    profile.sample sampler(1), v3
    check bounds.u v0, v1, v2 => b1(v0), b2

b1(v4: uint32):
    v5: int32 = 0
    return v5

b2:
    unreachable
}
"#,
    );
}

/// Formats every instruction call dispatch canonically.
#[test]
fn test_format_calls() {
    assert_format(
        r#"
external function callee(int32, int32): int32

function caller(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = call callee(v0, v1): (int32, int32) => int32
    v3: fn(int32, int32) => int32 = function.address callee
    v4: int32 = call.indirect v3(v0, v1): (int32, int32) => int32
    v5: int32 = call.virtual v0, int32, 0(v0, v1): (int32, int32) => int32
    v6: int32 = call.dynamic v0, int32, 0(v0, v1): (int32, int32) => int32
    return v6
}
"#,
    );
}

/// Formats void calls with callable type arguments canonically.
#[test]
fn test_format_void_call_with_callable_argument() {
    assert_format(
        r#"
external function consume(() => int32): void

function caller(v0: () => int32): void {
entry(v0: () => int32):
    call consume(v0): (() => int32) => void
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
function scalarOps(v0: int32, v1: int32, v2: boolean, v3: float64): int64 {
entry(v0: int32, v1: int32, v2: boolean, v3: float64):
    v4: boolean = int.lt.s v0, v1
    v5: int32 = select v2, v0, v1
    v6: int32 = int.negate v5
    v7: int32 = int.not v6
    v8: float64 = float.negate v3
    v9: int64 = cast.extend.s v7 -> int64
    v10: float64 = intrinsic.math.float.sqrt(v3)
    v11: float64 = intrinsic.math.float.min(v8, v3)
    v12: float64 = intrinsic.math.float.fma(v8, v3, v11)
    v13: (int32, boolean) = intrinsic.math.arithmetic.overflowing.add(v0, v1)
    return v9
}
"#,
    );
}

/// Infinite and NaN float literals round-trip through the formatter.
#[test]
fn test_format_float_infinity_and_nan() {
    assert_format(
        r#"
function floatLimits(): float32 {
entry:
    v0: float32 = inf
    v1: float32 = -inf
    v2: float32 = NaN
    return v1
}
"#,
    );
}

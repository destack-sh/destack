use crate::Value;
use crate::tests::{TestIsolate, assert_materialized_plain, run_mir_expect, run_mir_with_ok};

/// Create a tensor aggregate value from the provided elements.
fn tensor_from_values(
    isolate: &mut TestIsolate,
    function: &str,
    index: usize,
    values: &[i32],
) -> Value {
    let ty = isolate.parameter_type(function, index);
    let elements = values.iter().copied().map(Value::int32).collect();

    isolate.materialize_value_for_type(ty, elements)
}

/// Create a tensor aggregate value from the provided float elements.
fn tensor_from_f64_values(
    isolate: &mut TestIsolate,
    function: &str,
    index: usize,
    values: &[f64],
) -> Value {
    let ty = isolate.parameter_type(function, index);
    let elements = values.iter().copied().map(Value::float64).collect();

    isolate.materialize_value_for_type(ty, elements)
}

/// Run one tensor MIR function and assert its scalar result.
fn run_tensor_expect<F>(mir_text: &str, function: &str, setup: F, expected: Value)
where
    F: FnOnce(&mut TestIsolate) -> Vec<Value>,
{
    let output = run_mir_with_ok(mir_text, function, setup);

    assert_eq!(assert_materialized_plain(&output.value), expected);
}

/// Tensor splat broadcasts a scalar to every element.
#[test]
fn test_tensor_splat() {
    let mir = r#"
function tensorSplat(): int32 {
b0:
    v0: int32 = 7int32
    v1: tensor<int32, (2, 2)> = tensor.splat v0
    v2: int32 = 1int32
    v3: int32 = 0int32
    v4: int32 = tensor.extract v1, [v2, v3]
    return v4
}"#;
    run_mir_expect(mir, "tensorSplat", &[], Value::int32(7));
}

/// Tensor load and store operate on tensor references.
#[test]
fn test_tensor_load_store() {
    let mir = r#"
function tensorLoadStore(): int32 {
b0:
    v0: ref<int32[4], raw, space(stack)> = stack.alloc int32[4]
    v1: tensorView<int32, raw, space(stack), (2, 2)> = cast.bit v0 -> tensorView<int32, raw, space(stack), (2, 2)>
    v2: int32 = 42int32
    v3: int32 = 1int32
    v4: int32 = 0int32
    tensor.store v1, [v3, v4], v2
    v5: int32 = tensor.load v1, [v3, v4]
    return v5
}"#;
    run_mir_expect(mir, "tensorLoadStore", &[], Value::int32(42));
}

/// Tensor fill and copy write through tensor references.
#[test]
fn test_tensor_fill_copy() {
    let mir = r#"
function tensorFillCopy(): int32 {
b0:
    v0: ref<int32[4], raw, space(stack)> = stack.alloc int32[4]
    v1: ref<int32[4], raw, space(stack)> = stack.alloc int32[4]
    v2: tensorView<int32, raw, space(stack), (2, 2)> = cast.bit v0 -> tensorView<int32, raw, space(stack), (2, 2)>
    v3: tensorView<int32, raw, space(stack), (2, 2)> = cast.bit v1 -> tensorView<int32, raw, space(stack), (2, 2)>
    v4: int32 = 5int32
    tensor.fill v2, v4
    tensor.copy v3, v2
    v5: int32 = 1int32
    v6: int32 = 1int32
    v7: int32 = tensor.load v3, [v5, v6]
    return v7
}"#;
    run_mir_expect(mir, "tensorFillCopy", &[], Value::int32(5));
}

/// Tensor reshape preserves element order.
#[test]
fn test_tensor_reshape() {
    let mir = r#"
function tensorReshape(v0: tensor<int32, (2, 2)>): int32 {
b0(v0: tensor<int32, (2, 2)>):
    v1: int32 = 4int32
    v2: int32 = 1int32
    v3: tensor<int32, (4, 1)> = tensor.reshape v0, shape(v1, v2)
    v4: int64 = 3int64
    v5: int64 = 0int64
    v6: int32 = tensor.extract v3, [v4, v5]
    return v6
}"#;
    run_tensor_expect(
        mir,
        "tensorReshape",
        |interp| {
            vec![tensor_from_values(
                interp,
                "tensorReshape",
                0,
                &[1, 2, 3, 4],
            )]
        },
        Value::int32(4),
    );
}

/// Tensor broadcast duplicates the input across expanded dimensions.
#[test]
fn test_tensor_broadcast() {
    let mir = r#"
function tensorBroadcast(v0: tensor<int32, (2, 2)>): int32 {
b0(v0: tensor<int32, (2, 2)>):
    v1: tensor<int32, (2, 2)> = tensor.broadcast v0, dimensions(0, 1)
    v2: int64 = 1int64
    v3: int32 = tensor.extract v1, [v2, v2]
    return v3
}"#;
    run_tensor_expect(
        mir,
        "tensorBroadcast",
        |interp| {
            vec![tensor_from_values(
                interp,
                "tensorBroadcast",
                0,
                &[1, 2, 3, 4],
            )]
        },
        Value::int32(4),
    );
}

/// Tensor transpose swaps axes as requested.
#[test]
fn test_tensor_transpose() {
    let mir = r#"
function tensorTranspose(v0: tensor<int32, (2, 2)>): int32 {
b0(v0: tensor<int32, (2, 2)>):
    v1: tensor<int32, (2, 2)> = tensor.transpose v0, permutation(1, 0)
    v2: int64 = 0int64
    v3: int64 = 1int64
    v4: int32 = tensor.extract v1, [v2, v3]
    return v4
}"#;
    run_tensor_expect(
        mir,
        "tensorTranspose",
        |interp| {
            vec![tensor_from_values(
                interp,
                "tensorTranspose",
                0,
                &[1, 2, 3, 4],
            )]
        },
        Value::int32(3),
    );
}

/// Tensor slice extracts the requested window.
#[test]
fn test_tensor_slice() {
    let mir = r#"
function tensorSlice(v0: tensor<int32, (2, 2)>): int32 {
b0(v0: tensor<int32, (2, 2)>):
    v1: int32 = 0int32
    v2: int32 = 1int32
    v3: int32 = 2int32
    v4: tensor<int32, (2, 1)> = tensor.slice v0, offsets(v1, v2), sizes(v3, v2), strides(v2, v2)
    v5: int64 = 1int64
    v6: int64 = 0int64
    v7: int32 = tensor.extract v4, [v5, v6]
    return v7
}"#;
    run_tensor_expect(
        mir,
        "tensorSlice",
        |interp| vec![tensor_from_values(interp, "tensorSlice", 0, &[1, 2, 3, 4])],
        Value::int32(4),
    );
}

/// Tensor pad inserts the requested padding values.
#[test]
fn test_tensor_pad() {
    let mir = r#"
function tensorPad(v0: tensor<int32, (1, 1)>): int32 {
b0(v0: tensor<int32, (1, 1)>):
    v1: int32 = 0int32
    v2: int32 = 1int32
    v3: tensor<int32, (2, 2)> = tensor.pad v0, value(v1), low(v1, v1), high(v2, v2), interior(v1, v1)
    v4: int64 = 0int64
    v5: int32 = tensor.extract v3, [v4, v4]
    return v5
}"#;
    run_tensor_expect(
        mir,
        "tensorPad",
        |interp| vec![tensor_from_values(interp, "tensorPad", 0, &[9])],
        Value::int32(9),
    );
}

/// Tensor concat appends tensors along the specified axis.
#[test]
fn test_tensor_concat() {
    let mir = r#"
function tensorConcat(v0: tensor<int32, (1, 2)>, v1: tensor<int32, (1, 2)>): int32 {
b0(v0: tensor<int32, (1, 2)>, v1: tensor<int32, (1, 2)>):
    v2: tensor<int32, (2, 2)> = tensor.concat tensors(v0, v1), axis(0)
    v3: int64 = 1int64
    v4: int32 = tensor.extract v2, [v3, v3]
    return v4
}"#;
    run_tensor_expect(
        mir,
        "tensorConcat",
        |interp| {
            vec![
                tensor_from_values(interp, "tensorConcat", 0, &[1, 2]),
                tensor_from_values(interp, "tensorConcat", 1, &[3, 4]),
            ]
        },
        Value::int32(4),
    );
}

/// Tensor reduce collapses the requested axes.
#[test]
fn test_tensor_reduce() {
    let mir = r#"
function tensorReduce(v0: tensor<int32, (2, 2)>): int32 {
b0(v0: tensor<int32, (2, 2)>):
    v1: int32 = 0int32
    v2: tensor<int32, (2)> = tensor.reduce add, v0, v1, axes(1)
    v3: int64 = 1int64
    v4: int32 = tensor.extract v2, [v3]
    return v4
}"#;
    run_tensor_expect(
        mir,
        "tensorReduce",
        |interp| vec![tensor_from_values(interp, "tensorReduce", 0, &[1, 2, 3, 4])],
        Value::int32(7),
    );
}

/// Tensor dot multiplies matrices with the given contraction dimensions.
#[test]
fn test_tensor_dot() {
    let mir = r#"
function tensorDot(v0: tensor<int32, (2, 2)>, v1: tensor<int32, (2, 2)>): int32 {
b0(v0: tensor<int32, (2, 2)>, v1: tensor<int32, (2, 2)>):
    v2: tensor<int32, (2, 2)> = tensor.dot v0, v1, dims(lhsBatch(), rhsBatch(), lhsContract(1), rhsContract(0))
    v3: int64 = 1int64
    v4: int64 = 0int64
    v5: int32 = tensor.extract v2, [v3, v4]
    return v5
}"#;
    run_tensor_expect(
        mir,
        "tensorDot",
        |interp| {
            vec![
                tensor_from_values(interp, "tensorDot", 0, &[1, 2, 3, 4]),
                tensor_from_values(interp, "tensorDot", 1, &[5, 6, 7, 8]),
            ]
        },
        Value::int32(43),
    );
}

/// Tensor convolution applies a windowed dot product across spatial dims.
#[test]
fn test_tensor_convolution() {
    let mir = r#"
function tensorConvolution(v0: tensor<int32, (1, 1, 1, 1)>, v1: tensor<int32, (1, 1, 1, 1)>): int32 {
b0(v0: tensor<int32, (1, 1, 1, 1)>, v1: tensor<int32, (1, 1, 1, 1)>):
    v2: tensor<int32, (1, 1, 1, 1)> = tensor.convolution v0, v1, dims(inputBatch(0), inputFeature(1), inputSpatial(2, 3), kernelInputFeature(0), kernelOutputFeature(1), kernelSpatial(2, 3), outputBatch(0), outputFeature(1), outputSpatial(2, 3)), window(strides(1, 1), paddingLow(0, 0), paddingHigh(0, 0), lhsDilation(1, 1), rhsDilation(1, 1), windowReversal(false, false)), groups(feature(1), batch(1))
    v3: int64 = 0int64
    v4: int32 = tensor.extract v2, [v3, v3, v3, v3]
    return v4
}"#;
    run_tensor_expect(
        mir,
        "tensorConvolution",
        |interp| {
            vec![
                tensor_from_values(interp, "tensorConvolution", 0, &[2]),
                tensor_from_values(interp, "tensorConvolution", 1, &[3]),
            ]
        },
        Value::int32(6),
    );
}

/// Tensor gather selects elements from operand based on indices.
#[test]
fn test_tensor_gather() {
    let mir = r#"
function tensorGather(v0: tensor<int32, (1, 1)>, v1: tensor<int32, (1, 1)>): int32 {
b0(v0: tensor<int32, (1, 1)>, v1: tensor<int32, (1, 1)>):
    v2: tensor<int32, (1, 1)> = tensor.gather v0, v1, dims(offsetDims(0), collapsedSliceDims(1), startIndexMap(0), indexVectorDim(1)), sliceSizes(1, 1)
    v3: int64 = 0int64
    v4: int32 = tensor.extract v2, [v3, v3]
    return v4
}"#;
    run_tensor_expect(
        mir,
        "tensorGather",
        |interp| {
            vec![
                tensor_from_values(interp, "tensorGather", 0, &[7]),
                tensor_from_values(interp, "tensorGather", 1, &[0]),
            ]
        },
        Value::int32(7),
    );
}

/// Tensor scatter writes updates into the operand tensor.
#[test]
fn test_tensor_scatter() {
    let mir = r#"
function tensorScatter(v0: tensor<int32, (1, 1)>, v1: tensor<int32, (1, 1)>, v2: tensor<int32, (1, 1)>): int32 {
b0(v0: tensor<int32, (1, 1)>, v1: tensor<int32, (1, 1)>, v2: tensor<int32, (1, 1)>):
    v3: tensor<int32, (1, 1)> = tensor.scatter v0, v1, v2, dims(updateWindowDims(0), insertedWindowDims(1), scatterDimsToOperandDims(0), indexVectorDim(1)), mode(replace)
    v4: int64 = 0int64
    v5: int32 = tensor.extract v3, [v4, v4]
    return v5
}"#;
    run_tensor_expect(
        mir,
        "tensorScatter",
        |interp| {
            vec![
                tensor_from_values(interp, "tensorScatter", 0, &[0]),
                tensor_from_values(interp, "tensorScatter", 1, &[0]),
                tensor_from_values(interp, "tensorScatter", 2, &[9]),
            ]
        },
        Value::int32(9),
    );
}

/// Tensor convert preserves element values when converting.
#[test]
fn test_tensor_convert() {
    let mir = r#"
function tensorConvert(v0: tensor<int32, (2, 2)>): int32 {
b0(v0: tensor<int32, (2, 2)>):
    v1: tensor<int32, (2, 2)> = tensor.convert exact, v0
    v2: int64 = 1int64
    v3: int64 = 0int64
    v4: int32 = tensor.extract v1, [v2, v3]
    return v4
}"#;
    run_tensor_expect(
        mir,
        "tensorConvert",
        |interp| {
            vec![tensor_from_values(
                interp,
                "tensorConvert",
                0,
                &[1, 2, 3, 4],
            )]
        },
        Value::int32(3),
    );
}

/// Tensor compare returns boolean tensor elements.
#[test]
fn test_tensor_compare() {
    let mir = r#"
function tensorCompare(v0: tensor<int32, (2, 2)>, v1: tensor<int32, (2, 2)>): boolean {
b0(v0: tensor<int32, (2, 2)>, v1: tensor<int32, (2, 2)>):
    v2: tensor<boolean, (2, 2)> = tensor.compare int.eq, v0, v1
    v3: int64 = 0int64
    v4: int64 = 1int64
    v5: boolean = tensor.extract v2, [v3, v4]
    return v5
}"#;
    run_tensor_expect(
        mir,
        "tensorCompare",
        |interp| {
            vec![
                tensor_from_values(interp, "tensorCompare", 0, &[1, 2, 3, 4]),
                tensor_from_values(interp, "tensorCompare", 1, &[1, 9, 3, 4]),
            ]
        },
        Value::bool(false),
    );
}

/// Tensor convert applies element-wise rounding.
#[test]
fn test_tensor_convert_rounding() {
    let mir = r#"
function tensorConvertRounding(v0: tensor<float64, (2, 2)>): int32 {
b0(v0: tensor<float64, (2, 2)>):
    v1: tensor<int32, (2, 2)> = tensor.convert roundTowardZero, v0
    v2: int64 = 0int64
    v3: int64 = 1int64
    v4: int32 = tensor.extract v1, [v2, v3]
    return v4
}"#;
    run_tensor_expect(
        mir,
        "tensorConvertRounding",
        |interp| {
            vec![tensor_from_f64_values(
                interp,
                "tensorConvertRounding",
                0,
                &[1.2, 2.9, 3.1, 4.0],
            )]
        },
        Value::int32(2),
    );
}

/// Tensor cast forwards the tensor value.
#[test]
fn test_tensor_cast() {
    let mir = r#"
function tensorCast(v0: tensor<int32, (2, 2)>): int32 {
b0(v0: tensor<int32, (2, 2)>):
    v1: tensor<int32, (2, 2)> = tensor.cast v0
    v2: int64 = 1int64
    v3: int32 = tensor.extract v1, [v2, v2]
    return v3
}"#;
    run_tensor_expect(
        mir,
        "tensorCast",
        |interp| vec![tensor_from_values(interp, "tensorCast", 0, &[1, 2, 3, 4])],
        Value::int32(4),
    );
}

/// Tensor view offsets the underlying reference.
#[test]
fn test_tensor_view() {
    let mir = r#"
function tensorViewValue(): int32 {
b0:
    v0: ref<int32[4], raw, space(stack)> = stack.alloc int32[4]
    v1: tensorView<int32, raw, space(stack), (2, 2)> = cast.bit v0 -> tensorView<int32, raw, space(stack), (2, 2)>
    v2: int32 = 0int32
    v3: int32 = 1int32
    v4: int32 = 2int32
    v5: int32 = 3int32
    tensor.store v1, [v2, v2], v3
    tensor.store v1, [v2, v3], v4
    tensor.store v1, [v3, v2], v5
    tensor.store v1, [v3, v3], v4
    v6: tensorView<int32, raw, space(stack), (2, 1)> = tensor.view v1, offsets(v2, v3), sizes(v4, v3), strides(v3, v3)
    v7: int32 = tensor.load v6, [v2, v2]
    return v7
}"#;
    // verify the view offset result
    run_mir_expect(mir, "tensorViewValue", &[], Value::int32(2));
}

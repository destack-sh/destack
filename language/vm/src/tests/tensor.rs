use crate::Isolate;
use crate::memory::Value;
use crate::tests::{create_aggregate, run_mir_expect, run_mir_with_ok};

/// Create a tensor aggregate value from the provided elements.
fn tensor_from_values(isolate: &mut Isolate, values: &[i32]) -> Value {
    let elements = values.iter().copied().map(Value::int32).collect();
    create_aggregate(isolate, elements)
}

/// Create a tensor aggregate value from the provided float elements.
fn tensor_from_f64_values(isolate: &mut Isolate, values: &[f64]) -> Value {
    let elements = values.iter().copied().map(Value::float64).collect();
    create_aggregate(isolate, elements)
}

/// Tensor load and store operate on tensor references.
#[test]
fn test_tensor_load_store() {
    let mir = r#"
function @tensor_load_store() -> i32 {
block0:
    v0: ref<raw addrspace(stack) mut [i32; 4]> = stack.alloc [i32; 4]
    v1: tensor_ref<raw addrspace(stack) mut i32, [2, 2]> = bitcast v0 -> tensor_ref<raw addrspace(stack) mut i32, [2, 2]>
    v2: i32 = iconst 42i32
    v3: i32 = iconst 1i32
    v4: i32 = iconst 0i32
    tensor.store v1, [v3, v4], v2
    v5: i32 = tensor.load v1, [v3, v4]
    return v5
}"#;
    run_mir_expect(mir, "tensor_load_store", &[], Value::int32(42));
}

/// Tensor fill and copy write through tensor references.
#[test]
fn test_tensor_fill_copy() {
    let mir = r#"
function @tensor_fill_copy() -> i32 {
block0:
    v0: ref<raw addrspace(stack) mut [i32; 4]> = stack.alloc [i32; 4]
    v1: ref<raw addrspace(stack) mut [i32; 4]> = stack.alloc [i32; 4]
    v2: tensor_ref<raw addrspace(stack) mut i32, [2, 2]> = bitcast v0 -> tensor_ref<raw addrspace(stack) mut i32, [2, 2]>
    v3: tensor_ref<raw addrspace(stack) mut i32, [2, 2]> = bitcast v1 -> tensor_ref<raw addrspace(stack) mut i32, [2, 2]>
    v4: i32 = iconst 5i32
    tensor.fill v2, v4
    tensor.copy v3, v2
    v5: i32 = iconst 1i32
    v6: i32 = iconst 1i32
    v7: i32 = tensor.load v3, [v5, v6]
    return v7
}"#;
    run_mir_expect(mir, "tensor_fill_copy", &[], Value::int32(5));
}

/// Tensor reshape preserves element order.
#[test]
fn test_tensor_reshape() {
    let mir = r#"
function @tensor_reshape(v0: tensor<i32, [2, 2]>) -> i32 {
block0(v0: tensor<i32, [2, 2]>):
    v1: i32 = iconst 4i32
    v2: i32 = iconst 1i32
    v3: tensor<i32, [4, 1]> = tensor.reshape v0, [v1, v2]
    v4: i64 = iconst 2i64
    v5: i32 = element.get v3, v4
    return v5
}"#;
    let output = run_mir_with_ok(mir, "tensor_reshape", |interp| {
        vec![tensor_from_values(interp, &[1, 2, 3, 4])]
    });
    assert_eq!(output.value, Value::int32(3));
}

/// Tensor broadcast duplicates the input across expanded dimensions.
#[test]
fn test_tensor_broadcast() {
    let mir = r#"
function @tensor_broadcast(v0: tensor<i32, [2, 2]>) -> i32 {
block0(v0: tensor<i32, [2, 2]>):
    v1: tensor<i32, [2, 2]> = tensor.broadcast v0, [0, 1]
    v2: i64 = iconst 3i64
    v3: i32 = element.get v1, v2
    return v3
}"#;
    let output = run_mir_with_ok(mir, "tensor_broadcast", |interp| {
        vec![tensor_from_values(interp, &[1, 2, 3, 4])]
    });
    assert_eq!(output.value, Value::int32(4));
}

/// Tensor transpose swaps axes as requested.
#[test]
fn test_tensor_transpose() {
    let mir = r#"
function @tensor_transpose(v0: tensor<i32, [2, 2]>) -> i32 {
block0(v0: tensor<i32, [2, 2]>):
    v1: tensor<i32, [2, 2]> = tensor.transpose v0, [1, 0]
    v2: i64 = iconst 1i64
    v3: i32 = element.get v1, v2
    return v3
}"#;
    let output = run_mir_with_ok(mir, "tensor_transpose", |interp| {
        vec![tensor_from_values(interp, &[1, 2, 3, 4])]
    });
    assert_eq!(output.value, Value::int32(3));
}

/// Tensor slice extracts the requested window.
#[test]
fn test_tensor_slice() {
    let mir = r#"
function @tensor_slice(v0: tensor<i32, [2, 2]>) -> i32 {
block0(v0: tensor<i32, [2, 2]>):
    v1: i32 = iconst 0i32
    v2: i32 = iconst 1i32
    v3: i32 = iconst 2i32
    v4: tensor<i32, [2, 1]> = tensor.slice v0, offsets=[v1, v2], sizes=[v3, v2], strides=[v2, v2]
    v5: i64 = iconst 1i64
    v6: i32 = element.get v4, v5
    return v6
}"#;
    let output = run_mir_with_ok(mir, "tensor_slice", |interp| {
        vec![tensor_from_values(interp, &[1, 2, 3, 4])]
    });
    assert_eq!(output.value, Value::int32(4));
}

/// Tensor pad inserts the requested padding values.
#[test]
fn test_tensor_pad() {
    let mir = r#"
function @tensor_pad(v0: tensor<i32, [1, 1]>) -> i32 {
block0(v0: tensor<i32, [1, 1]>):
    v1: i32 = iconst 0i32
    v2: i32 = iconst 1i32
    v3: tensor<i32, [2, 2]> = tensor.pad v0, value=v1, low=[v1, v1], high=[v2, v2], interior=[v1, v1]
    v4: i64 = iconst 3i64
    v5: i32 = element.get v3, v4
    return v5
}"#;
    let output = run_mir_with_ok(mir, "tensor_pad", |interp| {
        vec![tensor_from_values(interp, &[9])]
    });
    assert_eq!(output.value, Value::int32(0));
}

/// Tensor concat appends tensors along the specified axis.
#[test]
fn test_tensor_concat() {
    let mir = r#"
function @tensor_concat(v0: tensor<i32, [1, 2]>, v1: tensor<i32, [1, 2]>) -> i32 {
block0(v0: tensor<i32, [1, 2]>, v1: tensor<i32, [1, 2]>):
    v2: tensor<i32, [2, 2]> = tensor.concat [v0, v1], axis=0
    v3: i64 = iconst 2i64
    v4: i32 = element.get v2, v3
    return v4
}"#;
    let output = run_mir_with_ok(mir, "tensor_concat", |interp| {
        vec![
            tensor_from_values(interp, &[1, 2]),
            tensor_from_values(interp, &[3, 4]),
        ]
    });
    assert_eq!(output.value, Value::int32(3));
}

/// Tensor reduce collapses the requested axes.
#[test]
fn test_tensor_reduce() {
    let mir = r#"
function @tensor_reduce(v0: tensor<i32, [2, 2]>) -> i32 {
block0(v0: tensor<i32, [2, 2]>):
    v1: i32 = iconst 0i32
    v2: tensor<i32, [2]> = tensor.reduce add, v0, v1, axes=[1]
    v3: i64 = iconst 1i64
    v4: i32 = element.get v2, v3
    return v4
}"#;
    let output = run_mir_with_ok(mir, "tensor_reduce", |interp| {
        vec![tensor_from_values(interp, &[1, 2, 3, 4])]
    });
    assert_eq!(output.value, Value::int32(7));
}

/// Tensor dot multiplies matrices with the given contraction dimensions.
#[test]
fn test_tensor_dot() {
    let mir = r#"
function @tensor_dot(v0: tensor<i32, [2, 2]>, v1: tensor<i32, [2, 2]>) -> i32 {
block0(v0: tensor<i32, [2, 2]>, v1: tensor<i32, [2, 2]>):
    v2: tensor<i32, [2, 2]> = tensor.dot v0, v1, dims(lhs_batch=[], rhs_batch=[], lhs_contract=[1], rhs_contract=[0])
    v3: i64 = iconst 2i64
    v4: i32 = element.get v2, v3
    return v4
}"#;
    let output = run_mir_with_ok(mir, "tensor_dot", |interp| {
        vec![
            tensor_from_values(interp, &[1, 2, 3, 4]),
            tensor_from_values(interp, &[5, 6, 7, 8]),
        ]
    });
    assert_eq!(output.value, Value::int32(43));
}

/// Tensor convolution applies a windowed dot product across spatial dims.
#[test]
fn test_tensor_convolution() {
    let mir = r#"
function @tensor_convolution(v0: tensor<i32, [1, 1, 1, 1]>, v1: tensor<i32, [1, 1, 1, 1]>) -> i32 {
block0(v0: tensor<i32, [1, 1, 1, 1]>, v1: tensor<i32, [1, 1, 1, 1]>):
    v2: tensor<i32, [1, 1, 1, 1]> = tensor.convolution v0, v1, dims(input_batch=0, input_feature=1, input_spatial=[2, 3], kernel_input_feature=0, kernel_output_feature=1, kernel_spatial=[2, 3], output_batch=0, output_feature=1, output_spatial=[2, 3]), strides=[1, 1], padding_low=[0, 0], padding_high=[0, 0], lhs_dilation=[1, 1], rhs_dilation=[1, 1], window_reversal=[false, false], feature_group=1, batch_group=1
    v3: i64 = iconst 0i64
    v4: i32 = element.get v2, v3
    return v4
}"#;
    let output = run_mir_with_ok(mir, "tensor_convolution", |interp| {
        vec![
            tensor_from_values(interp, &[2]),
            tensor_from_values(interp, &[3]),
        ]
    });
    assert_eq!(output.value, Value::int32(6));
}

/// Tensor gather selects elements from operand based on indices.
#[test]
fn test_tensor_gather() {
    let mir = r#"
function @tensor_gather(v0: tensor<i32, [1, 1]>, v1: tensor<i32, [1, 1]>) -> i32 {
block0(v0: tensor<i32, [1, 1]>, v1: tensor<i32, [1, 1]>):
    v2: tensor<i32, [1, 1]> = tensor.gather v0, v1, dims(offset_dims=[0], collapsed_slice_dims=[1], start_index_map=[0], index_vector_dim=1), slice_sizes=[1, 1]
    v3: i64 = iconst 0i64
    v4: i32 = element.get v2, v3
    return v4
}"#;
    let output = run_mir_with_ok(mir, "tensor_gather", |interp| {
        vec![
            tensor_from_values(interp, &[7]),
            tensor_from_values(interp, &[0]),
        ]
    });
    assert_eq!(output.value, Value::int32(7));
}

/// Tensor scatter writes updates into the operand tensor.
#[test]
fn test_tensor_scatter() {
    let mir = r#"
function @tensor_scatter(v0: tensor<i32, [1, 1]>, v1: tensor<i32, [1, 1]>, v2: tensor<i32, [1, 1]>) -> i32 {
block0(v0: tensor<i32, [1, 1]>, v1: tensor<i32, [1, 1]>, v2: tensor<i32, [1, 1]>):
    v3: tensor<i32, [1, 1]> = tensor.scatter v0, v1, v2, dims(update_window_dims=[0], inserted_window_dims=[1], scatter_dims_to_operand_dims=[0], index_vector_dim=1), mode=replace
    v4: i64 = iconst 0i64
    v5: i32 = element.get v3, v4
    return v5
}"#;
    let output = run_mir_with_ok(mir, "tensor_scatter", |interp| {
        vec![
            tensor_from_values(interp, &[0]),
            tensor_from_values(interp, &[0]),
            tensor_from_values(interp, &[9]),
        ]
    });
    assert_eq!(output.value, Value::int32(9));
}

/// Tensor convert preserves element values when converting.
#[test]
fn test_tensor_convert() {
    let mir = r#"
function @tensor_convert(v0: tensor<i32, [2, 2]>) -> i32 {
block0(v0: tensor<i32, [2, 2]>):
    v1: tensor<i32, [2, 2]> = tensor.convert exact, v0
    v2: i64 = iconst 0i64
    v3: i32 = element.get v1, v2
    return v3
}"#;
    let output = run_mir_with_ok(mir, "tensor_convert", |interp| {
        vec![tensor_from_values(interp, &[1, 2, 3, 4])]
    });
    assert_eq!(output.value, Value::int32(1));
}

/// Tensor compare returns boolean tensor elements.
#[test]
fn test_tensor_compare() {
    let mir = r#"
function @tensor_compare(v0: tensor<i32, [2, 2]>, v1: tensor<i32, [2, 2]>) -> bool {
block0(v0: tensor<i32, [2, 2]>, v1: tensor<i32, [2, 2]>):
    v2: tensor<bool, [2, 2]> = tensor.compare icmp_eq, v0, v1
    v3: i64 = iconst 1i64
    v4: bool = element.get v2, v3
    return v4
}"#;
    let output = run_mir_with_ok(mir, "tensor_compare", |interp| {
        vec![
            tensor_from_values(interp, &[1, 2, 3, 4]),
            tensor_from_values(interp, &[1, 9, 3, 4]),
        ]
    });
    // verify the comparison result
    assert_eq!(output.value, Value::bool(false));
}

/// Tensor convert applies element-wise rounding.
#[test]
fn test_tensor_convert_rounding() {
    let mir = r#"
function @tensor_convert_rounding(v0: tensor<f64, [2, 2]>) -> i32 {
block0(v0: tensor<f64, [2, 2]>):
    v1: tensor<i32, [2, 2]> = tensor.convert round_toward_zero, v0
    v2: i64 = iconst 2i64
    v3: i32 = element.get v1, v2
    return v3
}"#;
    let output = run_mir_with_ok(mir, "tensor_convert_rounding", |interp| {
        vec![tensor_from_f64_values(interp, &[1.2, 2.9, 3.1, 4.0])]
    });
    // verify the converted element
    assert_eq!(output.value, Value::int32(3));
}

/// Tensor cast forwards the tensor value.
#[test]
fn test_tensor_cast() {
    let mir = r#"
function @tensor_cast(v0: tensor<i32, [2, 2]>) -> i32 {
block0(v0: tensor<i32, [2, 2]>):
    v1: tensor<i32, [2, 2]> = tensor.cast v0
    v2: i64 = iconst 3i64
    v3: i32 = element.get v1, v2
    return v3
}"#;
    let output = run_mir_with_ok(mir, "tensor_cast", |interp| {
        vec![tensor_from_values(interp, &[1, 2, 3, 4])]
    });
    // verify the casted tensor element
    assert_eq!(output.value, Value::int32(4));
}

/// Tensor view offsets the underlying reference.
#[test]
fn test_tensor_view() {
    let mir = r#"
function @tensor_view() -> i32 {
block0:
    v0: ref<raw addrspace(stack) mut [i32; 4]> = stack.alloc [i32; 4]
    v1: tensor_ref<raw addrspace(stack) mut i32, [2, 2]> = bitcast v0 -> tensor_ref<raw addrspace(stack) mut i32, [2, 2]>
    v2: i32 = iconst 0i32
    v3: i32 = iconst 1i32
    v4: i32 = iconst 2i32
    v5: i32 = iconst 3i32
    tensor.store v1, [v2, v2], v3
    tensor.store v1, [v2, v3], v4
    tensor.store v1, [v3, v2], v5
    tensor.store v1, [v3, v3], v4
    v6: tensor_ref<raw addrspace(stack) mut i32, [2, 1]> = tensor.view v1, offsets=[v2, v3], sizes=[v4, v3], strides=[v3, v3]
    v7: i32 = tensor.load v6, [v2, v2]
    return v7
}"#;
    // verify the view offset result
    run_mir_expect(mir, "tensor_view", &[], Value::int32(2));
}

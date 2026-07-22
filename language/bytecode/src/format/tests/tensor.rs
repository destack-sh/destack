use super::assert_format_eq;

/// Format tensor operations with explicit runtime tensor identities.
#[test]
fn test_format_tensor_operations() {
    assert_format_eq(
        r#"
type Matrix
export function add(r0:tensor<int32,Matrix,space(local)>,r1:tensor<int32,Matrix,space(local)>):tensor<int32,Matrix,space(local)>{
r2:tensor<int32,Matrix,space(local)>=int.add r0,r1
return r2
}
"#,
        r#"
type Matrix

export function add(
    r0: tensor<int32, Matrix, space(local)>,
    r1: tensor<int32, Matrix, space(local)>,
): tensor<int32, Matrix, space(local)> {
    r2: tensor<int32, Matrix, space(local)> = int.add r0, r1
    return r2
}
"#,
    );
}

/// Format tensor shape transforms and their structured operands canonically.
#[test]
fn test_format_tensor_transforms() {
    assert_format_eq(
        r#"
type IntMatrix
type FloatMatrix
export function transform(r0:tensor<int32,IntMatrix,space(local)>,r1:uint64,r2:uint64,r3:int32):tensor<int32,IntMatrix,space(local)>{
r4:tensor<int32,IntMatrix,space(local)>=tensor.transpose r0,permutation(1,0)
r4:tensor<int32,IntMatrix,space(local)>=tensor.reshape r4,shape(r1,r2)
r4:tensor<int32,IntMatrix,space(local)>=tensor.broadcast r4,axes(0,1)
r4:tensor<int32,IntMatrix,space(local)>=tensor.slice r4,offsets(r1,r1),sizes(r2,r2),strides(r2,r2)
r4:tensor<int32,IntMatrix,space(local)>=tensor.pad r4,value(r3),low(r1,r1),high(r2,r2),interior(r1,r1)
r4:tensor<int32,IntMatrix,space(local)>=tensor.concat tensors(r0,r4),axis(0)
r4:tensor<int32,IntMatrix,space(local)>=tensor.splat r3
r5:tensor<float32,FloatMatrix,space(local)>=tensor.convert exact,r4
r4:tensor<int32,IntMatrix,space(local)>=tensor.bitcast r5
return r4
}
"#,
        r#"
type IntMatrix

type FloatMatrix

export function transform(
    r0: tensor<int32, IntMatrix, space(local)>,
    r1: uint64,
    r2: uint64,
    r3: int32,
): tensor<int32, IntMatrix, space(local)> {
    r4: tensor<int32, IntMatrix, space(local)> = tensor.transpose r0, permutation(1, 0)
    r4: tensor<int32, IntMatrix, space(local)> = tensor.reshape r4, shape(r1, r2)
    r4: tensor<int32, IntMatrix, space(local)> = tensor.broadcast r4, axes(0, 1)
    r4: tensor<int32, IntMatrix, space(local)> = tensor.slice r4,
        offsets(r1, r1),
        sizes(r2, r2),
        strides(r2, r2)
    r4: tensor<int32, IntMatrix, space(local)> = tensor.pad r4,
        value(r3),
        low(r1, r1),
        high(r2, r2),
        interior(r1, r1)
    r4: tensor<int32, IntMatrix, space(local)> = tensor.concat tensors(r0, r4), axis(0)
    r4: tensor<int32, IntMatrix, space(local)> = tensor.splat r3
    r5: tensor<float32, FloatMatrix, space(local)> = tensor.convert exact, r4
    r4: tensor<int32, IntMatrix, space(local)> = tensor.bitcast r5
    return r4
}
"#,
    );
}

/// Format tensor reductions, indexed updates, and view memory operations canonically.
#[test]
fn test_format_tensor_reductions_and_memory() {
    assert_format_eq(
        r#"
type Matrix
type Index
type View
export function process(r0:tensor<int32,Matrix,space(local)>,r1:int32,r2:tensor<uint64,Index,space(local)>,r3:tensorView<int32,View,borrowed,space(local),6>,r9:uint64):int32{
r10:tensor<int32,Matrix,space(local)>=tensor.reduce add,r0,r1,axes(0)
r2:tensor<uint64,Index,space(local)>=tensor.indexReduce min,r0,axis(0),tieBreak(first)
r10:tensor<int32,Matrix,space(local)>=tensor.contract r0,r0,axes(leftBatch(),rightBatch(),leftContract(1),rightContract(0))
r10:tensor<int32,Matrix,space(local)>=tensor.gather r0,r2,axes(outputOffset(0),collapsedInput(1),indexToInput(0),indexVector(1)),sliceSizes(1,1)
r10:tensor<int32,Matrix,space(local)>=tensor.scatter r0,r2,r10,axes(updateWindow(0),insertedInput(1),indexToInput(0),indexVector(1)),mode(replace)
r11:int32=tensor.load r3,[r9,r9]
tensor.store r3,[r9,r9],r11
tensor.fill r3,r11
r12:tensorView<int32,View,borrowed,space(local),6>=tensor.view r3,offsets(r9,r9),sizes(r9,r9),strides(r9,r9)
tensor.copy r12,r3
r18:int32=tensor.extract r10,[r9,r9]
return r18
}
"#,
        r#"
type Matrix

type Index

type View

export function process(
    r0: tensor<int32, Matrix, space(local)>,
    r1: int32,
    r2: tensor<uint64, Index, space(local)>,
    r3: tensorView<int32, View, borrowed, space(local), 6>,
    r9: uint64,
): int32 {
    r10: tensor<int32, Matrix, space(local)> = tensor.reduce add, r0, r1, axes(0)
    r2: tensor<uint64, Index, space(local)> = tensor.indexReduce min, r0, axis(0), tieBreak(first)
    r10: tensor<int32, Matrix, space(local)> = tensor.contract r0, r0,
        axes(leftBatch(), rightBatch(), leftContract(1), rightContract(0))
    r10: tensor<int32, Matrix, space(local)> = tensor.gather r0, r2,
        axes(outputOffset(0), collapsedInput(1), indexToInput(0), indexVector(1)),
        sliceSizes(1, 1)
    r10: tensor<int32, Matrix, space(local)> = tensor.scatter r0, r2, r10,
        axes(updateWindow(0), insertedInput(1), indexToInput(0), indexVector(1)),
        mode(replace)
    r11: int32 = tensor.load r3, [r9, r9]
    tensor.store r3, [r9, r9], r11
    tensor.fill r3, r11
    r12: tensorView<int32, View, borrowed, space(local), 6> = tensor.view r3,
        offsets(r9, r9),
        sizes(r9, r9),
        strides(r9, r9)
    tensor.copy r12, r3
    r18: int32 = tensor.extract r10, [r9, r9]
    return r18
}
"#,
    );
}

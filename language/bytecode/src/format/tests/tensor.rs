use super::assert_format_eq;

/// Format tensor operations with explicit runtime tensor identities.
#[test]
fn test_format_tensor_operations() {
    assert_format_eq(
        r#"
type Matrix
export function add(r0:tensor<int32,Matrix>,r1:tensor<int32,Matrix>):tensor<int32,Matrix>{
r2:tensor<int32,Matrix>=int.add r0,r1
return r2
}
"#,
        r#"
type Matrix

export function add(r0: tensor<int32, Matrix>, r1: tensor<int32, Matrix>): tensor<int32, Matrix> {
    r2: tensor<int32, Matrix> = int.add r0, r1
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
export function transform(r0:tensor<int32,IntMatrix>,r1:uint64,r2:uint64,r3:int32):tensor<int32,IntMatrix>{
r4:tensor<int32,IntMatrix>=tensor.transpose r0,permutation(1,0)
r4:tensor<int32,IntMatrix>=tensor.reshape r4,shape(r1,r2)
r4:tensor<int32,IntMatrix>=tensor.broadcast r4,axes(0,1)
r4:tensor<int32,IntMatrix>=tensor.slice r4,offsets(r1,r1),sizes(r2,r2),strides(r2,r2)
r4:tensor<int32,IntMatrix>=tensor.pad r4,value(r3),low(r1,r1),high(r2,r2),interior(r1,r1)
r4:tensor<int32,IntMatrix>=tensor.concat tensors(r0,r4),axis(0)
r4:tensor<int32,IntMatrix>=tensor.splat r3
r5:tensor<float32,FloatMatrix>=tensor.convert exact,r4
r4:tensor<int32,IntMatrix>=tensor.bitcast r5
return r4
}
"#,
        r#"
type IntMatrix

type FloatMatrix

export function transform(
    r0: tensor<int32, IntMatrix>,
    r1: uint64,
    r2: uint64,
    r3: int32,
): tensor<int32, IntMatrix> {
    r4: tensor<int32, IntMatrix> = tensor.transpose r0, permutation(1, 0)
    r4: tensor<int32, IntMatrix> = tensor.reshape r4, shape(r1, r2)
    r4: tensor<int32, IntMatrix> = tensor.broadcast r4, axes(0, 1)
    r4: tensor<int32, IntMatrix> = tensor.slice r4, offsets(r1, r1), sizes(r2, r2), strides(r2, r2)
    r4: tensor<int32, IntMatrix> = tensor.pad r4,
        value(r3),
        low(r1, r1),
        high(r2, r2),
        interior(r1, r1)
    r4: tensor<int32, IntMatrix> = tensor.concat tensors(r0, r4), axis(0)
    r4: tensor<int32, IntMatrix> = tensor.splat r3
    r5: tensor<float32, FloatMatrix> = tensor.convert exact, r4
    r4: tensor<int32, IntMatrix> = tensor.bitcast r5
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
export function process(r0:tensor<int32,Matrix>,r1:int32,r2:tensor<uint64,Index>,r3:tensorView<int32,View>,r8:uint64):int32{
r9:tensor<int32,Matrix>=tensor.reduce add,r0,r1,axes(0)
r2:tensor<uint64,Index>=tensor.indexReduce min,r0,axis(0),tieBreak(first)
r9:tensor<int32,Matrix>=tensor.contract r0,r0,axes(leftBatch(),rightBatch(),leftContract(1),rightContract(0))
r9:tensor<int32,Matrix>=tensor.gather r0,r2,axes(outputOffset(0),collapsedInput(1),indexToInput(0),indexVector(1)),sliceSizes(1,1)
r9:tensor<int32,Matrix>=tensor.scatter r0,r2,r9,axes(updateWindow(0),insertedInput(1),indexToInput(0),indexVector(1)),mode(replace)
r10:int32=tensor.load r3,[r8,r8]
tensor.store r3,[r8,r8],r10
tensor.fill r3,r10
r11:tensorView<int32,View>=tensor.view r3,offsets(r8,r8),sizes(r8,r8),strides(r8,r8)
tensor.copy r11,r3
r16:int32=tensor.extract r9,[r8,r8]
return r16
}
"#,
        r#"
type Matrix

type Index

type View

export function process(
    r0: tensor<int32, Matrix>,
    r1: int32,
    r2: tensor<uint64, Index>,
    r3: tensorView<int32, View>,
    r8: uint64,
): int32 {
    r9: tensor<int32, Matrix> = tensor.reduce add, r0, r1, axes(0)
    r2: tensor<uint64, Index> = tensor.indexReduce min, r0, axis(0), tieBreak(first)
    r9: tensor<int32, Matrix> = tensor.contract r0, r0,
        axes(leftBatch(), rightBatch(), leftContract(1), rightContract(0))
    r9: tensor<int32, Matrix> = tensor.gather r0, r2,
        axes(outputOffset(0), collapsedInput(1), indexToInput(0), indexVector(1)),
        sliceSizes(1, 1)
    r9: tensor<int32, Matrix> = tensor.scatter r0, r2, r9,
        axes(updateWindow(0), insertedInput(1), indexToInput(0), indexVector(1)),
        mode(replace)
    r10: int32 = tensor.load r3, [r8, r8]
    tensor.store r3, [r8, r8], r10
    tensor.fill r3, r10
    r11: tensorView<int32, View> = tensor.view r3, offsets(r8, r8), sizes(r8, r8), strides(r8, r8)
    tensor.copy r11, r3
    r16: int32 = tensor.extract r9, [r8, r8]
    return r16
}
"#,
    );
}

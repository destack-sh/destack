use crate::tests::TestProgram;

/// Emit the complete bytecode tensor operation families.
#[test]
fn test_emit_bytecode_tensor_operations() {
    let program = TestProgram::mir(
        r#"
export function tensors(
    v0: tensor<int32, managed, mutable, (2, 2)>,
    v1: tensorView<int32, borrowed, mutable, (2, 2)>,
    v2: int32,
): tensor<int32, managed, mutable, (2, 2)> {
entry(v0: tensor<int32, managed, mutable, (2, 2)>, v1: tensorView<int32, borrowed, mutable, (2, 2)>, v2: int32):
    v3: int32 = 0
    v4: int32 = 1
    v5: int32 = tensor.load v1, [v3, v4]
    v6: int32 = tensor.extract v0, [v4, v3]
    tensor.store v1, [v3, v4], v5
    tensor.fill v1, v6
    tensor.copy v1, v1
    v7: tensor<int32, managed, mutable, (2, 2)> = tensor.reshape v0, shape(v4, v4)
    v8: tensor<int32, managed, mutable, (2, 2)> = tensor.broadcast v0, dimensions(0, 1)
    v9: tensor<int32, managed, mutable, (2, 2)> = tensor.transpose v0, permutation(1, 0)
    v10: tensor<int32, managed, mutable, (2, 2)> = tensor.cast v0
    v11: tensorView<int32, borrowed, mutable, (2, 2)> = tensor.view v1, offsets(v3, v3), sizes(v4, v4), strides(v4, v4)
    v12: tensor<int32, managed, mutable, (2, 2)> = tensor.slice v0, offsets(v3, v3), sizes(v4, v4), strides(v4, v4)
    v13: tensor<int32, managed, mutable, (2, 2)> = tensor.pad v0, value(v3), low(v3, v3), high(v3, v3), interior(v3, v3)
    v14: tensor<int32, managed, mutable, (2, 2)> = tensor.concat tensors(v0, v0), axis(0)
    v15: tensor<boolean, managed, mutable, (2, 2)> = tensor.compare eq, v0, v0
    v16: tensor<int32, managed, mutable, (2, 2)> = tensor.select v15, v0, v0
    v17: tensor<int32, managed, mutable, (2, 2)> = tensor.reduce add, v0, v3, axes(0)
    v18: tensor<uint64, managed, mutable, (2, 2)> = tensor.indexReduce min, v0, axis(0), tieBreak(first)
    v19: tensor<int32, managed, mutable, (2, 2)> = tensor.dot v0, v0, dims(lhsBatch(), rhsBatch(), lhsContract(1), rhsContract(0))
    v20: tensor<int32, managed, mutable, (2, 2)> = tensor.convolution v0, v0, dims(inputBatch(0), inputFeature(1), inputSpatial(2, 3), kernelInputFeature(0), kernelOutputFeature(1), kernelSpatial(2, 3), outputBatch(0), outputFeature(1), outputSpatial(2, 3)), window(strides(1, 1), paddingLow(0, 0), paddingHigh(0, 0), lhsDilation(1, 1), rhsDilation(1, 1), windowReversal(false, false)), groups(feature(1), batch(1))
    v21: tensor<int32, managed, mutable, (2, 2)> = tensor.gather v0, v0, dims(offsetDims(0), collapsedSliceDims(1), startIndexMap(0), indexVectorDim(1)), sliceSizes(1, 1)
    v22: tensor<int32, managed, mutable, (2, 2)> = tensor.scatter v0, v0, v0, dims(updateWindowDims(0), insertedWindowDims(1), scatterDimsToOperandDims(0), indexVectorDim(1)), mode(replace)
    v23: tensor<float32, managed, mutable, (2, 2)> = tensor.convert exact, v0
    v24: tensor<int32, managed, mutable, (2, 2)> = tensor.splat v2
    return v7
}
"#,
    );

    program.assert_bytecode(
        r#"
function tensors {
    constant.int32 r8, 0
    constant.int32 r9, 1
    tensor.load r10, r1:r6 @ l3, [r8, r9]
    tensor.extract r11, r0 @ l2, [r9, r8]
    tensor.store r1:r6 @ l3, [r8, r9], r10
    tensor.fill r1:r6 @ l3, r11
    tensor.copy r1:r6 @ l3, r1:r6 @ l3
    tensor.reshape r10, r0 @ l2, [r9, r9], a0
    tensor.broadcast r11, r0 @ l2, [0, 1], a1
    tensor.transpose r11, r0 @ l2, [1, 0], a2
    tensor.bitcast r11, r0 @ l2, a3
    tensor.view r12:r17, r1:r6 @ l3, [r8, r8], [r9, r9], [r9, r9], l3
    tensor.slice r1, r0 @ l2, [r8, r8], [r9, r9], [r9, r9], a4
    tensor.pad r1, r0 @ l2, r8, [r8, r8], [r8, r8], [r8, r8], a5
    tensor.concat r1, [r0 @ l2, r0 @ l2], 0, a6
    tensor.compare r1, r0 @ l2, r0 @ l2, eq.int, a7
    tensor.select r2, r1 @ l5, r0 @ l2, r0 @ l2, a8
    tensor.reduce r1, r0 @ l2, r8, add, [0], a9
    tensor.indexReduce r1, r0 @ l2, min, 0, 0, a10
    tensor.contract r1, r0 @ l2, r0 @ l2, axes([], [], [1], [0]), a11
    tensor.convolution r1,
        r0 @ l2,
        r0 @ l2,
        axes((0, 1, [2, 3]), (0, 1, [2, 3]), (0, 1, [2, 3])),
        window([1, 1], [0, 0], [0, 0], [1, 1], [1, 1], [0, 0]),
        groups(1, 1),
        a12
    tensor.gather r1, r0 @ l2, r0 @ l2, axes([0], [1], [0], 1), [1, 1], a13
    tensor.scatter r1, r0 @ l2, r0 @ l2, r0 @ l2, axes([0], [1], [0], 1), replace, a14
    tensor.convert r1, r0 @ l2, exact, a15
    tensor.splat r0, r7, a16
    return r10
}
"#,
    );
}

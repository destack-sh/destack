use crate::tests::TestProgram;

/// Emit every fixed-width vector operation family.
#[test]
fn test_emit_vector_operations() {
    let program = TestProgram::mir(
        r#"
export function vectors(
    v0: vector<int32, 4>,
    v1: vector<int32, 4>,
    v2: vector<boolean, 4>,
    v3: int32,
): int32 {
entry(v0: vector<int32, 4>, v1: vector<int32, 4>, v2: vector<boolean, 4>, v3: int32):
    v4: vector<int32, 4> = vector.splat v3
    v5: int32 = vector.extract v4, v3
    v6: vector<int32, 4> = vector.insert v4, v3, v5
    v7: vector<int32, 4> = vector.shuffle v6, v1, [0, 5, 2, 7]
    v8: vector<int32, 4> = vector.select v2, v7, v0
    v9: vector<int32, 4> = add v0, v1
    v10: int32 = vector.reduce add, v9
    v11: vector<boolean, 4> = vector.compare lt, v8, v1
    v12: vector<int32, 4> = vector.convert exact, v8
    return v10
}
"#,
    );

    program.assert_bytecode(
        r#"
function vectors {
    vector.splat r6:r7, r5: vector<int32, 4>
    vector.extract r8, r6:r7, r5: vector<int32, 4>
    vector.insert r9:r10, r6:r7, r5, r8: vector<int32, 4>
    vector.shuffle r5:r6, r9:r10, r2:r3, [0, 5, 2, 7]: vector<int32, 4>
    vector.select r7:r8, r4, r5:r6, r0:r1: vector<int32, 4>
    vector.add r4:r5, r0:r1, r2:r3: vector<int32, 4>
    vector.reduce.add r0, r4:r5: vector<int32, 4>
    vector.compare.lt r1, r7:r8, r2:r3: vector<int32, 4>
    vector.convert.exact r1:r2, r7:r8: vector<int32, 4> -> vector<int32, 4>
    return r0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i32x4, i32x4, i8x4, i32) -> i32 native {
    ss0 = explicit_slot 16, align = 16
    ss1 = explicit_slot 16, align = 16
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i32x4, v2: i32x4, v3: i8x4, v4: i32):
    v5 = splat.i32x4 v4
    v6 = iconst.i32 4
    v7 = icmp uge v4, v6  ; v6 = 4
    trapnz v7, user6
    v8 = stack_addr.i64 ss0
    store notrap aligned region1 v5, v8
    v9 = iconst.i64 4
    v10 = uextend.i64 v4
    v11 = imul v10, v9  ; v9 = 4
    v12 = iadd v8, v11
    v13 = load.i32 notrap aligned region1 v12
    v14 = iconst.i32 4
    v15 = icmp uge v4, v14  ; v14 = 4
    trapnz v15, user6
    v16 = stack_addr.i64 ss1
    store notrap aligned region1 v5, v16
    v17 = iconst.i64 4
    v18 = uextend.i64 v4
    v19 = imul v18, v17  ; v17 = 4
    v20 = iadd v16, v19
    store notrap aligned region1 v13, v20
    v21 = load.i32x4 notrap aligned region1 v16
    v22 = bitcast.i8x16 little v21
    v23 = bitcast.i8x16 little v2
    v24 = shuffle v22, v23, 0x1f1e1d1c0b0a09081716151403020100
    v25 = bitcast.i32x4 little v24
    v26 = extractlane v3, 0
    v27 = uextend.i32 v26
    v28 = ineg v27
    v29 = splat.i32x4 v28
    v30 = extractlane v3, 1
    v31 = uextend.i32 v30
    v32 = ineg v31
    v33 = insertlane v29, v32, 1
    v34 = extractlane v3, 2
    v35 = uextend.i32 v34
    v36 = ineg v35
    v37 = insertlane v33, v36, 2
    v38 = extractlane v3, 3
    v39 = uextend.i32 v38
    v40 = ineg v39
    v41 = insertlane v37, v40, 3
    v42 = bitselect v41, v25, v1
    v43 = iadd v1, v2
    v44 = extractlane v43, 0
    v45 = extractlane v43, 1
    v46 = iadd v44, v45
    v47 = extractlane v43, 2
    v48 = iadd v46, v47
    v49 = extractlane v43, 3
    v50 = iadd v48, v49
    v51 = icmp slt v42, v2
    v52 = iconst.i32 1
    v53 = splat.i32x4 v52  ; v52 = 1
    v54 = band v51, v53
    v55 = extractlane v54, 0
    v56 = ireduce.i8 v55
    v57 = splat.i8x4 v56
    v58 = extractlane v54, 1
    v59 = ireduce.i8 v58
    v60 = insertlane v57, v59, 1
    v61 = extractlane v54, 2
    v62 = ireduce.i8 v61
    v63 = insertlane v60, v62, 2
    v64 = extractlane v54, 3
    v65 = ireduce.i8 v64
    v66 = insertlane v63, v65, 3
    return v50
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32x4, i32x4, i8x4, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32x4 notrap aligned v1
    v4 = load.i32x4 notrap aligned v1+16
    v5 = load.i8x4 notrap aligned v1+32
    v6 = load.i32 notrap aligned v1+40
    v7 = call fn0(v0, v3, v4, v5, v6)
    store notrap aligned v7, v2
    return
}
"#,
    );
}

/// Select floating-point vector operations from the MIR element type.
#[test]
fn test_emit_float_vector_operations() {
    let program = TestProgram::mir(
        r#"
export function float(
    v0: vector<float32, 2>,
    v1: vector<float32, 2>,
): vector<float32, 2> {
entry(v0: vector<float32, 2>, v1: vector<float32, 2>):
    v2: vector<float32, 2> = add v0, v1
    v3: vector<float32, 2> = rem v2, v1
    return v3
}
"#,
    );

    program.assert_bytecode(
        r#"
function float {
    vector.add r2, r0, r1: vector<float32, 2>
    vector.rem r0, r2, r1: vector<float32, 2>
    return r0
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, f32x2, f32x2) -> f32x2 native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    sig0 = (f32, f32) -> f32 native
    fn0 = colocated u0:2 sig0
    stack_limit = gv1

block0(v0: i64, v1: f32x2, v2: f32x2):
    v3 = fadd v1, v2
    v4 = extractlane v3, 0
    v5 = extractlane v2, 0
    v6 = call fn0(v4, v5)
    v7 = splat.f32x2 v6
    v8 = extractlane v3, 1
    v9 = extractlane v2, 1
    v10 = call fn0(v8, v9)
    v11 = insertlane v7, v10, 1
    return v11
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, f32x2, f32x2) -> f32x2 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.f32x2 notrap aligned v1
    v4 = load.f32x2 notrap aligned v1+8
    v5 = call fn0(v0, v3, v4)
    store notrap aligned v5, v2
    return
}
"#,
    );
}

/// Lower every numeric vector conversion domain with its explicit policy.
#[test]
fn test_emit_vector_conversions() {
    let program = TestProgram::mir(
        r#"
export function convert(
    v0: vector<int32, 2>,
    v1: vector<float32, 2>,
    v2: vector<float64, 2>,
): void {
entry(v0: vector<int32, 2>, v1: vector<float32, 2>, v2: vector<float64, 2>):
    v3: vector<int8, 2> = vector.convert exact, v0
    v4: vector<float32, 2> = vector.convert exact, v0
    v5: vector<int32, 2> = vector.convert roundFloor, v1
    v6: vector<int32, 2> = vector.convert saturate, v1
    v7: vector<float32, 2> = vector.convert exact, v2
    return
}
"#,
    );

    program.assert_bytecode(
        r#"
function convert {
    vector.convert.exact r4, r0: vector<int32, 2> -> vector<int8, 2>
    vector.convert.exact r4, r0: vector<int32, 2> -> vector<float32, 2>
    vector.convert.roundFloor r0, r1: vector<float32, 2> -> vector<int32, 2>
    vector.convert.saturate r0, r1: vector<float32, 2> -> vector<int32, 2>
    vector.convert.exact r0, r2:r3: vector<float64, 2> -> vector<float32, 2>
    return
}
"#,
    );

    program.assert_native(
        r#"
function u0:0(i64 vmctx, i32x2, f32x2, f64x2) native {
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    stack_limit = gv1

block0(v0: i64, v1: i32x2, v2: f32x2, v3: f64x2):
    v4 = extractlane v1, 0
    v5 = iconst.i32 -128
    v6 = icmp slt v4, v5  ; v5 = -128
    v7 = iconst.i32 127
    v8 = icmp sgt v4, v7  ; v7 = 127
    v9 = bor v6, v8
    trapnz v9, user2
    v10 = ireduce.i8 v4
    v11 = splat.i8x2 v10
    v12 = extractlane v1, 1
    v13 = iconst.i32 -128
    v14 = icmp slt v12, v13  ; v13 = -128
    v15 = iconst.i32 127
    v16 = icmp sgt v12, v15  ; v15 = 127
    v17 = bor v14, v16
    trapnz v17, user2
    v18 = ireduce.i8 v12
    v19 = insertlane v11, v18, 1
    v20 = extractlane v1, 0
    v21 = fcvt_from_sint.f32 v20
    v22 = fcvt_to_sint_sat.i32 v21
    v23 = icmp ne v20, v22
    trapnz v23, user2
    v24 = splat.f32x2 v21
    v25 = extractlane v1, 1
    v26 = fcvt_from_sint.f32 v25
    v27 = fcvt_to_sint_sat.i32 v26
    v28 = icmp ne v25, v27
    trapnz v28, user2
    v29 = insertlane v24, v26, 1
    v30 = extractlane v2, 0
    v31 = floor v30
    v32 = f32const -0x1.000000p31
    v33 = f32const 0x1.000000p31
    v34 = fcmp lt v31, v32  ; v32 = -0x1.000000p31
    v35 = fcmp ge v31, v33  ; v33 = 0x1.000000p31
    v36 = fcmp uno v31, v31
    v37 = bor v34, v35
    v38 = bor v37, v36
    trapnz v38, user2
    v39 = fcvt_to_sint.i32 v31
    v40 = splat.i32x2 v39
    v41 = extractlane v2, 1
    v42 = floor v41
    v43 = f32const -0x1.000000p31
    v44 = f32const 0x1.000000p31
    v45 = fcmp lt v42, v43  ; v43 = -0x1.000000p31
    v46 = fcmp ge v42, v44  ; v44 = 0x1.000000p31
    v47 = fcmp uno v42, v42
    v48 = bor v45, v46
    v49 = bor v48, v47
    trapnz v49, user2
    v50 = fcvt_to_sint.i32 v42
    v51 = insertlane v40, v50, 1
    v52 = extractlane v2, 0
    v53 = trunc v52
    v54 = fcvt_to_sint_sat.i32 v53
    v55 = splat.i32x2 v54
    v56 = extractlane v2, 1
    v57 = trunc v56
    v58 = fcvt_to_sint_sat.i32 v57
    v59 = insertlane v55, v58, 1
    v60 = extractlane v3, 0
    v61 = fdemote.f32 v60
    v62 = fpromote.f64 v61
    v63 = fcmp ne v60, v62
    trapnz v63, user3
    v64 = splat.f32x2 v61
    v65 = extractlane v3, 1
    v66 = fdemote.f32 v65
    v67 = fpromote.f64 v66
    v68 = fcmp ne v65, v67
    trapnz v68, user3
    v69 = insertlane v64, v66, 1
    return
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32x2, f32x2, f64x2) native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32x2 notrap aligned v1
    v4 = load.f32x2 notrap aligned v1+8
    v5 = load.f64x2 notrap aligned v1+16
    call fn0(v0, v3, v4, v5)
    return
}
"#,
    );
}

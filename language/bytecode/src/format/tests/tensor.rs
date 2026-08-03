use super::assert_format_eq;

/// Format tensor operations with direct layouts and allocation sites.
#[test]
fn test_format_tensor_operations() {
    assert_format_eq(
        r#"
function f0 {
    tensor.element r2,[r0@l0,r1@l0],int.add,a0
return r2
}
"#,
        r#"
function f0 {
    tensor.element r2, [r0 @ l0, r1 @ l0], int.add, a0
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
function f0 {
    tensor.transpose r4,r0@l0,[1,0],a0
tensor.reshape r4,r4@l0,[r1,r2],a1
tensor.broadcast r4,r4@l0,[0,1],a2
tensor.slice r4,r4@l0,[r1,r1],[r2,r2],[r2,r2],a3
tensor.pad r4,r4@l0,r3,[r1,r1],[r2,r2],[r1,r1],a4
tensor.concat r4,[r0@l0,r4@l0],0,a5
tensor.splat r4,r3,a6
tensor.convert r5,r4@l0,exact,a7
tensor.bitcast r4,r5@l0,a8
return r4
}
"#,
        r#"
function f0 {
    tensor.transpose r4, r0 @ l0, [1, 0], a0
    tensor.reshape r4, r4 @ l0, [r1, r2], a1
    tensor.broadcast r4, r4 @ l0, [0, 1], a2
    tensor.slice r4, r4 @ l0, [r1, r1], [r2, r2], [r2, r2], a3
    tensor.pad r4, r4 @ l0, r3, [r1, r1], [r2, r2], [r1, r1], a4
    tensor.concat r4, [r0 @ l0, r4 @ l0], 0, a5
    tensor.splat r4, r3, a6
    tensor.convert r5, r4 @ l0, exact, a7
    tensor.bitcast r4, r5 @ l0, a8
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
function f0 {
    tensor.reduce r10,r0@l0,r1,add,[0],a0
tensor.indexReduce r2,r0@l0,min,0,0,a1
tensor.contract r10,[r0@l0,r0@l0],axes([],[],[1],[0]),a2
tensor.gather r10,[r0@l0,r2@l1],axes([0],[1],[0],1),[1,1],a3
tensor.scatter r10,[r0@l0,r2@l1,r10@l0],axes([0],[1],[0],1),replace,a4
tensor.load r11,r3@l2,[r9,r9]
tensor.store r3@l2,[r9,r9],r11
tensor.fill r3@l2,r11
tensor.view r12:r17,r3@l2,[r9,r9],[r9,r9],[r9,r9],l2
tensor.copy [r12@l2,r3@l2]
tensor.extract r18,r10@l0,[r9,r9]
return r18
}
"#,
        r#"
function f0 {
    tensor.reduce r10, r0 @ l0, r1, add, [0], a0
    tensor.indexReduce r2, r0 @ l0, min, 0, 0, a1
    tensor.contract r10, [r0 @ l0, r0 @ l0], axes([], [], [1], [0]), a2
    tensor.gather r10, [r0 @ l0, r2 @ l1], axes([0], [1], [0], 1), [1, 1], a3
    tensor.scatter r10, [r0 @ l0, r2 @ l1, r10 @ l0], axes([0], [1], [0], 1), replace, a4
    tensor.load r11, r3 @ l2, [r9, r9]
    tensor.store r3 @ l2, [r9, r9], r11
    tensor.fill r3 @ l2, r11
    tensor.view r12:r17, r3 @ l2, [r9, r9], [r9, r9], [r9, r9], l2
    tensor.copy [r12 @ l2, r3 @ l2]
    tensor.extract r18, r10 @ l0, [r9, r9]
    return r18
}
"#,
    );
}

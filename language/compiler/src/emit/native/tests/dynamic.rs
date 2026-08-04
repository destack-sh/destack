use crate::tests::TestProgram;

/// Bind and project one dynamic fat pointer through a linked dispatch-table id.
#[test]
fn test_emit_native_dynamic_value() {
    let mut program = TestProgram::mir(
        r#"
type Writer { }
type FileWriter { }

export function erase(v0: ref<FileWriter, managed, readonly>): ref<void, managed, readonly> {
entry(v0: ref<FileWriter, managed, readonly>):
    v1: dynamic<Writer, managed, readonly> = dynamic.bind v0, FileWriter
    v2: ref<void, managed, readonly> = dynamic.payload v1
    return v2
}
"#,
    );
    program.mark_dynamic("FileWriter", "Writer");

    program.assert_native(
        r#"
function u0:0(i64, i64) -> i64 native {
    gv0 = symbol colocated userextname0

block0(v0: i64, v1: i64):
    v2 = symbol_value.i64 gv0
    v3 = load.i32 notrap aligned v2
    return v1
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i64) -> i64 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i64 notrap aligned v1
    v4 = call fn0(v0, v3)
    store notrap aligned v4, v2
    return
}
"#,
    );
}

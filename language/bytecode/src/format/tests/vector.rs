use super::assert_format_eq;

/// Format vector operations with explicit lane types and counts.
#[test]
fn test_format_vector_operations() {
    assert_format_eq(
        r#"
export function splat(r0:int32):vector<int32,4>{
r1:vector<int32,4>=vector.splat r0
return r1
}
"#,
        r#"
export function splat(r0: int32): vector<int32, 4> {
    r1: vector<int32, 4> = vector.splat r0
    return r1
}
"#,
    );
}

/// Format vector memory operations with their logical vector type.
#[test]
fn test_format_vector_memory() {
    assert_format_eq(
        r#"
export function copy(r0:address):void{
r1:vector<int32,4>=load r0
store r0,r1
return
}
"#,
        r#"
export function copy(r0: address): void {
    r1: vector<int32, 4> = load r0
    store r0, r1
    return
}
"#,
    );
}

use super::assert_format_eq;

/// Format object declarations in canonical table order.
#[test]
fn test_format_declarations() {
    assert_format_eq(
        r#"
external function consume(int32):void
readonly global current:State=constant initial
local global cache:State=zero
constant initial,align(8)=bytes(1,2,3,4)
type State
"#,
        r#"
type State

constant initial, align(8) = bytes(1, 2, 3, 4)

readonly global current: State = constant initial

local global cache: State = zero

external function consume(int32): void
"#,
    );
}

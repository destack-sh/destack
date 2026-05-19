use super::super::snapshot::assert_check_snapshot;

#[test]
fn test_check_records_literal_types() {
    assert_check_snapshot(
        r#"
const literal = 42;
const widened: int32 = 42;
"#,
        r#"
const literal = 42;
/// @type.node source=42 value=42
/// @type.symbol key=literal value=42

const widened: int32 = 42;
/// @type.node source=42 value=int32
/// @type.symbol key=widened value=int32

/// @type.summary types=2 nodes=2 symbols=2
/// @generic.summary parameters=0 lists=0
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=0 labels=0 members=0 calls=0
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0
"#,
    );
}

use super::super::snapshot::assert_check_snapshot_with_diagnostics;

#[test]
fn test_check_reports_assignment_mismatch() {
    assert_check_snapshot_with_diagnostics(
        r#"
const value: int32 = "text";
"#,
        r#"
=== dir ===
const value: int32 = "text";
/// @type.node source="\"text\"" value=string
/// @type.symbol key=value value=int32

/// @type.summary types=2 nodes=1 symbols=1
/// @generic.summary parameters=0 lists=0
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=0 labels=0 members=0 calls=0
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0

=== diagnostics ===
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=22 source="const value: int32 = \"text\";"
"#,
    );
}

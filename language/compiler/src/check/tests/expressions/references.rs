use super::super::snapshot::assert_check_snapshot;

#[test]
fn test_check_records_local_name_resolution() {
    assert_check_snapshot(
        r#"
const value = 1;
const copy = value;
"#,
        r#"
const value = 1;
/// @type.symbol key=value value=1

const copy = value;
/// @resolution.name source=value target=value
/// @type.symbol key=copy value=1

/// @type.summary types=1 nodes=1 symbols=2
/// @generic.summary parameters=0 lists=0
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=1 labels=0 members=0 calls=0
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0
"#,
    );
}

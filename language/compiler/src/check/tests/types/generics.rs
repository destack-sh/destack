use super::super::snapshot::assert_check_snapshot;

#[test]
fn test_check_records_generic_call_instances() {
    assert_check_snapshot(
        r#"
function identity<T>(value: T): T {
    return value;
}

const number = identity(1);
const text = identity("x");
"#,
        r#"
function identity<T>(value: T): T {
/// @generic.parameters key=identity parameters=[identity.T]
/// @generic.parameter key=identity.T space=type
/// @type.symbol key=identity value=<T>(T) => T

    return value;
}

const number = identity(1);
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity(1)" parameters=[int32] return=int32 kind=direct target=identity instance=instance0
/// @instance.node source="identity(1)" instance=instance0
/// @type.symbol key=number value=int32

const text = identity("x");
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity(\"x\")" parameters=[string] return=string kind=direct target=identity instance=instance1
/// @instance.node source="identity(\"x\")" instance=instance1
/// @type.symbol key=text value=string

/// @instance.entry instance=instance0 key=identity arguments=[int32]
/// @instance.entry instance=instance1 key=identity arguments=[string]
/// @type.summary types=5 nodes=2 symbols=4
/// @generic.summary parameters=1 lists=1
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=2 labels=0 members=0 calls=2
/// @instance.summary instances=2 nodes=2
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0
"#,
    );
}

#[test]
fn test_check_reuses_identical_generic_instances() {
    assert_check_snapshot(
        r#"
function identity<T>(value: T): T {
    return value;
}

const first = identity(1);
const second = identity(2);
"#,
        r#"
function identity<T>(value: T): T {
/// @generic.parameters key=identity parameters=[identity.T]
/// @generic.parameter key=identity.T space=type
/// @type.symbol key=identity value=<T>(T) => T

    return value;
}

const first = identity(1);
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity(1)" parameters=[int32] return=int32 kind=direct target=identity instance=instance0
/// @instance.node source="identity(1)" instance=instance0
/// @type.symbol key=first value=int32

const second = identity(2);
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity(2)" parameters=[int32] return=int32 kind=direct target=identity instance=instance0
/// @instance.node source="identity(2)" instance=instance0
/// @type.symbol key=second value=int32

/// @instance.entry instance=instance0 key=identity arguments=[int32]
/// @type.summary types=4 nodes=2 symbols=4
/// @generic.summary parameters=1 lists=1
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=2 labels=0 members=0 calls=2
/// @instance.summary instances=1 nodes=2
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0
"#,
    );
}

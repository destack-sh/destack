use crate::tests::TestSession;

/// Lower an associated const read at an open receiver to a witness constant, and at a closed
/// receiver to a load of the implementer's global.
#[test]
fn test_lower_an_associated_const_read_through_the_witness() {
    let session = TestSession::single(
        r#"
interface Tagged {
    const Tag: int32;
}

struct Point implements Tagged {
    x: int32;
    const Tag: int32 = 7;
}

function tagOf<T: Tagged>(): int32 {
    return T.Tag;
}

function main(): int32 {
    return tagOf<Point>() + Point.Tag;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type test.main.Point {
    x: int32;
}

@nocopy
type test.main.Tagged { }

constant test.main.Point.Tag: int32 = 7

function test.main.main(): int32 {
entry:
    v0: int32 = call test.main.tagOf<test.main.Point>(): () => int32
    v1: int32 = load @test.main.Point.Tag
    v2: int32 = add v0, v1
    return v2
}

function test.main.tagOf<T: test.main.Tagged>(): int32 {
entry:
    v0: int32 = witness T, test.main.Tagged, Tag
    return v0
}

shared function test.main.tagOf<test.main.Point>(): int32;

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
/// @layout.struct name=type@2 size=4 align=4
/// @layout.field owner=type@2 index=0 name=x offset=0 size=4 align=4

/// @dispatch.shape constraint=type@3
"#,
    );
    session.assert_mir_function(
        "main.ds",
        "test.main.tagOf",
        r#"
@nocopy
type test.main.Tagged { }

function test.main.tagOf<T: test.main.Tagged>(): int32 {
entry:
    v0: int32 = witness T, test.main.Tagged, Tag
    return v0
}
"#,
    );
    session.assert_mir_function(
        "main.ds",
        "test.main.main",
        r#"
function test.main.main(): int32 {
entry:
    v0: int32 = call test.main.tagOf<test.main.Point>(): () => int32
    v1: int32 = load @test.main.Point.Tag
    v2: int32 = add v0, v1
    return v2
}
"#,
    );
}

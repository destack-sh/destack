use crate::tests::TestSession;

#[test]
fn test_lower_newtype_values_through_the_transparent_representation() {
    let session = TestSession::single(
        r#"
newtype Meters = int32;

function span(distance: Meters): Meters {
    return distance;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Meters = newtype<int32>;

function test.main.span(v0: Meters): Meters {
entry(v0: Meters):
    return v0
}
"#,
    );
}

#[test]
fn test_lower_newtype_construction_and_value_read() {
    let session = TestSession::single(
        r#"
newtype Meters = int32;

function total(base: int32): Meters {
    return Meters(base);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Meters = newtype<int32>;

function test.main.total(v0: int32): Meters {
entry(v0: int32):
    v1: Meters = aggregate (v0)
    return v1
}
"#,
    );
}

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

    session.assert_mir_function(
        "main.ds",
        "test.main.span",
        r#"
@copy
type test.main.Meters = newtype<int32>;

function test.main.span(v0: test.main.Meters): test.main.Meters {
    local l0: test.main.Meters

entry(v0: test.main.Meters):
    local.set l0, v0
    v1: test.main.Meters = local.get l0
    return v1
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

    session.assert_mir_function(
        "main.ds",
        "test.main.total",
        r#"
@copy
type test.main.Meters = newtype<int32>;

function test.main.total(v0: int32): test.main.Meters {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    v2: test.main.Meters = aggregate (v1)
    return v2
}
"#,
    );
}

/// A cast unwraps one newtype layer to its backing and wraps one backing value, each a
/// representation-preserving relation.
#[test]
fn test_lower_a_cast_unwrapping_and_wrapping_a_newtype() {
    let session = TestSession::single(
        r#"
newtype UserId = int32;
newtype OrganisationId = int32;

function organisation(user: UserId): OrganisationId {
    return user as int32 as OrganisationId;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.organisation",
        r#"
@copy
type test.main.UserId = newtype<int32>;

@copy
type test.main.OrganisationId = newtype<int32>;

function test.main.organisation(v0: test.main.UserId): test.main.OrganisationId {
    local l0: test.main.UserId

entry(v0: test.main.UserId):
    local.set l0, v0
    v1: test.main.UserId = local.get l0
    v2: int32 = field.get v1, 0
    v3: test.main.OrganisationId = aggregate (v2)
    return v3
}
"#,
    );
}

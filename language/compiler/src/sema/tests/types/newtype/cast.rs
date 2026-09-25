use crate::tests::{DirRows, TestSession};

/// A cast unwraps one newtype layer to its backing, and wraps one backing value into a newtype.
#[test]
fn test_cast_unwraps_and_wraps_one_newtype_layer() {
    let session = TestSession::single(
        r#"
newtype UserId = int32;
newtype OrganisationId = int32;

function organisation(user: UserId): OrganisationId {
    return user as int32 as OrganisationId;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
newtype UserId = int32;
newtype OrganisationId = int32;

function organisation(user: UserId): OrganisationId {
    return user as int32 as OrganisationId;
}

=== dir ===
newtype UserId = int32;
/// @type.symbol symbol=UserId source="newtype UserId = int32" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int32" backing=int32 constructors=[(int32) => UserId]

newtype OrganisationId = int32;
/// @type.symbol symbol=OrganisationId source="newtype OrganisationId = int32" type=OrganisationId
/// @definition.newtype symbol=OrganisationId source="newtype OrganisationId = int32" backing=int32 constructors=[(int32) => OrganisationId]

function organisation(user: UserId): OrganisationId {
/// @type.symbol symbol=organisation type=(UserId) => OrganisationId
/// @type.symbol symbol=organisation.user source="user: UserId" type=UserId
/// @resolution.name source=UserId target=UserId
/// @resolution.name source=OrganisationId target=OrganisationId

    return user as int32 as OrganisationId;
    /// @resolution.name source=user target=organisation.user
    /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user root=organisation.user
    /// @coercion.node source="user as int32" from=int32 adjustments=[{ kind: newtype, target: OrganisationId }] origin=explicit
    /// @coercion.node source=user from=UserId adjustments=[{ kind: newtype, target: int32 }] origin=explicit
    /// @resolution.name source=OrganisationId target=OrganisationId

}
"#,
    );
}

/// A cast names one relation, so two newtypes need their shared backing spelled between them.
#[test]
fn test_reject_a_cast_between_two_newtypes() {
    let session = TestSession::single(
        r#"
newtype UserId = int32;
newtype OrganisationId = int32;

function organisation(user: UserId): OrganisationId {
    return user as OrganisationId;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype UserId = int32;
newtype OrganisationId = int32;

function organisation(user: UserId): OrganisationId {
    return user as OrganisationId;
}

=== dir ===
newtype UserId = int32;
/// @type.symbol symbol=UserId source="newtype UserId = int32" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int32" backing=int32 constructors=[(int32) => UserId]

newtype OrganisationId = int32;
/// @type.symbol symbol=OrganisationId source="newtype OrganisationId = int32" type=OrganisationId
/// @definition.newtype symbol=OrganisationId source="newtype OrganisationId = int32" backing=int32 constructors=[(int32) => OrganisationId]

function organisation(user: UserId): OrganisationId {
/// @type.symbol symbol=organisation type=(UserId) => OrganisationId
/// @type.symbol symbol=organisation.user source="user: UserId" type=UserId
/// @resolution.name source=UserId target=UserId
/// @resolution.name source=OrganisationId target=OrganisationId

    return user as OrganisationId;
    /// @resolution.name source=user target=organisation.user
    /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user root=organisation.user
    /// @resolution.name source=OrganisationId target=OrganisationId

}
"#,
        r#"
/// @diagnostic.error id=invalid-cast message="type 'UserId' cannot be cast to 'OrganisationId'"
/// @diagnostic.label line=6 column=12 span="user" line_source="return user as OrganisationId;"
"#,
    );
}

/// A cast of a newtype value to its own type is the identity.
#[test]
fn test_accept_an_identity_cast_on_a_newtype() {
    let session = TestSession::single(
        r#"
newtype UserId = int32;

function same(user: UserId): UserId {
    return user as UserId;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
newtype UserId = int32;

function same(user: UserId): UserId {
    return user as UserId;
}

=== dir ===
newtype UserId = int32;
/// @type.symbol symbol=UserId source="newtype UserId = int32" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int32" backing=int32 constructors=[(int32) => UserId]

function same(user: UserId): UserId {
/// @type.symbol symbol=same type=(UserId) => UserId
/// @type.symbol symbol=same.user source="user: UserId" type=UserId
/// @resolution.name source=UserId target=UserId
/// @resolution.name source=UserId target=UserId

    return user as UserId;
    /// @resolution.name source=user target=same.user
    /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user root=same.user
    /// @resolution.name source=UserId target=UserId

}
"#,
        r#"
/// @diagnostic.warning id=redundant-cast message="cast to 'UserId' has no effect"
/// @diagnostic.label line=5 column=17 span="as" line_source="return user as UserId;"
/// @diagnostic.suggestion message="remove the cast" applicability=automatic patched="return user;"
"#,
    );
}

/// A raw pointer casts through one newtype layer of its pointee in either direction.
#[test]
fn test_cast_a_pointer_through_one_newtype_layer() {
    let session = TestSession::single(
        r#"
newtype UserId = int32;

function unwrap(id: *UserId): *int32 {
    return id as *int32;
}

function wrap(raw: *int32): *UserId {
    return raw as *UserId;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
newtype UserId = int32;

function unwrap(id: *UserId): *int32 {
    return id as *int32;
}

function wrap(raw: *int32): *UserId {
    return raw as *UserId;
}

=== dir ===
newtype UserId = int32;
/// @type.symbol symbol=UserId source="newtype UserId = int32" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int32" backing=int32 constructors=[(int32) => UserId]

function unwrap(id: *UserId): *int32 {
/// @type.symbol symbol=unwrap type=(*UserId) => *int32
/// @type.symbol symbol=unwrap.id source="id: *UserId" type=*UserId
/// @resolution.name source=UserId target=UserId

    return id as *int32;
    /// @resolution.name source=id target=unwrap.id
    /// @resolution.place source=id placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=id root=unwrap.id
    /// @coercion.node source=id from=*UserId adjustments=[{ kind: newtype, target: *int32 }] origin=explicit

}

function wrap(raw: *int32): *UserId {
/// @type.symbol symbol=wrap type=(*int32) => *UserId
/// @type.symbol symbol=wrap.raw source="raw: *int32" type=*int32
/// @resolution.name source=UserId target=UserId

    return raw as *UserId;
    /// @resolution.name source=raw target=wrap.raw
    /// @resolution.place source=raw placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=raw root=wrap.raw
    /// @coercion.node source=raw from=*int32 adjustments=[{ kind: newtype, target: *UserId }] origin=explicit
    /// @resolution.name source=UserId target=UserId

}
"#,
    );
}

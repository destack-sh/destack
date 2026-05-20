use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_canonicalizes_borrow_surface_forms() {
    let session = TestSession::single(
        r#"
struct Node {
    id: int32;
}

function access(read: &readonly Node, write: &Node, exclusive: &exclusive Node): void {
    read.id;
    write.id;
    exclusive.id;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
struct Node {
/// @type.symbol symbol=Node type=Node

    id: int32;
    /// @type.symbol symbol=Node.id type=int32
}

function access(read: &readonly Node, write: &Node, exclusive: &exclusive Node): void {
/// @generic.slot symbol=access.L0 index=0 kind=static constraint=Lifetime
/// @generic.slot symbol=access.L1 index=1 kind=static constraint=Lifetime
/// @generic.slot symbol=access.L2 index=2 kind=static constraint=Lifetime
/// @type.symbol symbol=access type=<access.L0: Lifetime, access.L1: Lifetime, access.L2: Lifetime>(Borrowed<Node, access.L0, readonly>, Borrowed<Node, access.L1, mutable>, Borrowed<Node, access.L2, exclusive>) => void

    read.id;
    /// @resolution.name source=read target=read
    /// @resolution.member source=read.id receiver=Borrowed<Node, access.L0, readonly> kind=direct target=Node.id
    /// @type.node source=read.id type=int32

    write.id;
    /// @resolution.name source=write target=write
    /// @resolution.member source=write.id receiver=Borrowed<Node, access.L1, mutable> kind=direct target=Node.id
    /// @type.node source=write.id type=int32

    exclusive.id;
    /// @resolution.name source=exclusive target=exclusive
    /// @resolution.member source=exclusive.id receiver=Borrowed<Node, access.L2, exclusive> kind=direct target=Node.id
    /// @type.node source=exclusive.id type=int32
}
"#);
}

#[test]
fn test_check_evaluates_lifetime_and_access_type_functions() {
    let session = TestSession::single(
        r#"
function project<A: Lifetime, B: Lifetime>(value: Borrowed<int32, A>): void {
    type Later = WithLifetime<typeof value, B>;
    type Exclusive = WithAccess<typeof value, "exclusive">;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
function project<A: Lifetime, B: Lifetime>(value: Borrowed<int32, A>): void {
/// @generic.slot symbol=project.A index=0 kind=static constraint=Lifetime
/// @generic.slot symbol=project.B index=1 kind=static constraint=Lifetime
/// @type.symbol symbol=project type=<A: Lifetime, B: Lifetime>(Borrowed<int32, project.A, mutable>) => void

    type Later = WithLifetime<typeof value, B>;
    /// @type.symbol symbol=project.Later type=Borrowed<int32, project.B, mutable>

    type Exclusive = WithAccess<typeof value, "exclusive">;
    /// @type.symbol symbol=project.Exclusive type=Borrowed<int32, project.A, exclusive>
}
"#);
}

use crate::tests::{DirRows, TestSession};

#[test]
fn test_borrow_surface_forms_canonicalize_to_same_type() {
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
        DirRows::checked().with_reference_types(),
        r#"
struct Node {
/// @type.symbol symbol=Node type=Node

    id: int32;
    /// @type.symbol symbol=Node.id type=int32

}

function access(read: &readonly Node, write: &Node, exclusive: &exclusive Node): void {
/// @type.symbol symbol=access type=(Borrowed<Node, access.L0, "readonly">, Borrowed<Node, access.L1, "mutable">, Borrowed<Node, access.L2, "exclusive">) => void
/// @generic.slot key=access.L0 index=0 kind=static constraint=memory.lifetime.Lifetime
/// @generic.slot key=access.L1 index=1 kind=static constraint=memory.lifetime.Lifetime
/// @generic.slot key=access.L2 index=2 kind=static constraint=memory.lifetime.Lifetime
/// @type.symbol symbol=read type=Borrowed<Node, access.L0, "readonly">
/// @type.symbol symbol=write type=Borrowed<Node, access.L1, "mutable">
/// @type.symbol symbol=exclusive type=Borrowed<Node, access.L2, "exclusive">

    read.id;
    /// @type.node source=read type=Borrowed<Node, access.L0, "readonly">
    /// @type.node source=read.id type=int32
    /// @resolution.name source=read target=read
    /// @resolution.member source=read.id receiver=Borrowed<Node, access.L0, "readonly"> kind=symbol target=Node.id

    write.id;
    /// @type.node source=write type=Borrowed<Node, access.L1, "mutable">
    /// @type.node source=write.id type=int32
    /// @resolution.name source=write target=write
    /// @resolution.member source=write.id receiver=Borrowed<Node, access.L1, "mutable"> kind=symbol target=Node.id

    exclusive.id;
    /// @type.node source=exclusive type=Borrowed<Node, access.L2, "exclusive">
    /// @type.node source=exclusive.id type=int32
    /// @resolution.name source=exclusive target=exclusive
    /// @resolution.member source=exclusive.id receiver=Borrowed<Node, access.L2, "exclusive"> kind=symbol target=Node.id

}
"#);
}

#[test]
fn test_lifetime_and_access_type_functions_evaluate() {
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
        DirRows::checked().with_reference_types(),
        r#"
function project<A: Lifetime, B: Lifetime>(value: Borrowed<int32, A>): void {
/// @type.symbol symbol=project type=(Borrowed<int32, project.A, "mutable">) => void
/// @generic.slot key=project.A index=0 kind=static constraint=memory.lifetime.Lifetime
/// @generic.slot key=project.B index=1 kind=static constraint=memory.lifetime.Lifetime
/// @type.symbol symbol=value type=Borrowed<int32, project.A, "mutable">

    type Later = WithLifetime<typeof value, B>;
    /// @type.symbol symbol=Later type=Borrowed<int32, project.B, "mutable">
    /// @type.node source=value type=Borrowed<int32, project.A, "mutable">
    /// @resolution.name source=value target=value

    type Exclusive = WithAccess<typeof value, "exclusive">;
    /// @type.symbol symbol=Exclusive type=Borrowed<int32, project.A, "exclusive">
    /// @type.node source=value type=Borrowed<int32, project.A, "mutable">
    /// @resolution.name source=value target=value

}
"#,
    );
}

use crate::tests::{DirRows, TestSession};

#[test]
fn test_borrow_written_forms_canonicalize_to_same_type() {
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
=== annotated ===
struct Node {
    id: int32;
}

function access<comptime L0: Lifetime, comptime L1: Lifetime, comptime L2: Lifetime>(
    read: Borrowed<Node, L0, "readonly">,
    write: Borrowed<Node, L1, "mutable">,
    exclusive: Borrowed<Node, L2, "exclusive">,
): void {
    read.id;
    write.id;
    exclusive.id;
}

=== checked ===
struct Node {
/// @type.symbol symbol=Node type=Node

    id: int32;
    /// @type.symbol symbol=Node.id type=int32

}

function access(read: &readonly Node, write: &Node, exclusive: &exclusive Node): void {
/// @type.symbol symbol=access type=(Borrowed<Node, access.L0, "readonly">, Borrowed<Node, access.L1, "mutable">, Borrowed<Node, access.L2, "exclusive">) => void
/// @generic.template symbol=access parameters=[comptime L0: memory.lifetime.Lifetime, comptime L1: memory.lifetime.Lifetime, comptime L2: memory.lifetime.Lifetime]
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

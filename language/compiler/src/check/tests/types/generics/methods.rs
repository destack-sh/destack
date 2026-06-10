use crate::tests::{DirRows, TestSession};

#[test]
fn test_infer_method_receiver_lifetime_from_borrowed_argument() {
    let session = TestSession::single(
        r#"
class Box<T> {
    get(&readonly this): T;

    readFrom(&exclusive this, source: &readonly Box<T>): T {
        return source.get();
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"class Box<T> {
/// @generic.template source=declaration parameters=[T]
/// @type.symbol symbol=Box type=Box<T>
/// @definition.class symbol=Box template=LocalGenericTemplateId(0)
/// @definition.method symbol=Box.get source="get(&readonly this): T" slot=get type=<T>(this: Borrowed<Box<T>, member.L0, "readonly">) => T
/// @definition.method symbol=Box.readFrom slot=readFrom type=<T>(this: Borrowed<Box<T>, member.L0, "exclusive">, Borrowed<Box<T>, member.L1, "readonly">) => T
/// @type.symbol symbol=Box.T source=T type=T

    get(&readonly this): T;
    /// @generic.template source=member parent=template#0 parameters=[comptime L0: memory.lifetime.Lifetime origin=induced.form]
    /// @type.symbol symbol=Box.get source="get(&readonly this): T" type=<T>(this: Borrowed<Box<T>, member.L0, "readonly">) => T
    /// @type.symbol symbol=this#1 source="&readonly this" type=Borrowed<Box<T>, member.L0, "readonly">
    /// @resolution.name source=T target=Box.T

    readFrom(&exclusive this, source: &readonly Box<T>): T {
    /// @generic.template source=member parent=template#0 parameters=[comptime L0: memory.lifetime.Lifetime origin=induced.form, comptime L1: memory.lifetime.Lifetime origin=induced.form]
    /// @type.symbol symbol=Box.readFrom type=<T>(this: Borrowed<Box<T>, member.L0, "exclusive">, Borrowed<Box<T>, member.L1, "readonly">) => T
    /// @type.symbol symbol=this#2 source="&exclusive this" type=Borrowed<Box<T>, member.L0, "exclusive">
    /// @type.symbol symbol=source source="source: &readonly Box<T>" type=Borrowed<Box<T>, member.L1, "readonly">
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=T target=Box.T
    /// @resolution.name source=T target=Box.T

        return source.get();
        /// @generic.instance source=source.get id=member<member.L1>
        /// @generic.instance source=source.get() id=member<member.L1>
        /// @type.node source=source type=Borrowed<Box<T>, member.L1, "readonly">
        /// @type.node source=source.get() type=T
        /// @resolution.name source=source target=source
        /// @resolution.member source=source.get receiver=Borrowed<Box<T>, member.L1, "readonly"> kind=symbol target=Box.get instance=member<member.L1>
        /// @resolution.call source=source.get() parameters=() return=T kind=symbol target=Box.get receiver=Borrowed<Box<T>, member.L1, "readonly"> instance=member<member.L1>

    }
}

/// @generic.instance id=member<member.L1> template=member arguments=[member.L1]
"#,
    );
}

use crate::tests::{DirRows, TestSession};

#[test]
fn test_newtype_member_access_dereferences_to_the_backing() {
    let session = TestSession::single(
        r#"
class Wrapper<T> {
    open(): T {
        throw "unreachable";
    }
}

newtype Sealed<T> = Wrapper<T>;

extension<T> of Sealed<T> {
    reveal(): T {
        this.open()
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Wrapper<T> {
    open(): T {
        throw "unreachable";
    }
}

newtype Sealed<T> = Wrapper<T>;

extension<T> of Sealed<T> {
    reveal(): T {
        this.open<T>()
    }
}

=== checked ===
class Wrapper<T> {
/// @generic.template symbol=Wrapper parameters=(T#1)
/// @type.symbol symbol=Wrapper type=Wrapper
/// @definition.class symbol=Wrapper template=(T#1)
/// @definition.method symbol=Wrapper.open slot=open type=(this: Wrapper<T#1>) => T#1
/// @type.symbol symbol=Wrapper.T source=T type=T#1

    open(): T {
    /// @type.symbol symbol=Wrapper.open type=(this: Wrapper<T#1>) => T#1
    /// @resolution.name source=T target=Wrapper.T

        throw "unreachable";
    }
}

newtype Sealed<T> = Wrapper<T>;
/// @generic.template symbol=Sealed parameters=(T#2)
/// @type.symbol symbol=Sealed source="newtype Sealed<T> = Wrapper<T>" type=Sealed
/// @definition.newtype symbol=Sealed source="newtype Sealed<T> = Wrapper<T>" template=(T#2) value=Wrapper<T#2>
/// @type.symbol symbol=Sealed.T source=T type=T#2
/// @resolution.name source=Wrapper target=Wrapper
/// @resolution.name source=T target=Sealed.T

extension<T> of Sealed<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @definition.extension symbol=<module>#2 form=local target=Sealed<T#3>
/// @definition.method symbol=reveal slot=reveal type=(this: Sealed<T#3>) => T#3
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Sealed target=Sealed
/// @resolution.name source=T target=T

    reveal(): T {
    /// @type.symbol symbol=reveal type=(this: Sealed<T#3>) => T#3
    /// @resolution.name source=T target=T

        this.open()
        /// @resolution.member source=this.open receiver=Sealed<T#3> kind=symbol target=Wrapper.open adjustments=(backing)
        /// @resolution.call source=this.open() parameters=() return=T#3 kind=symbol target=Wrapper.open receiver=Wrapper<T#3> adjustments=(backing) instance=Wrapper<T#3>.open
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Sealed<T#3>
        /// @generic.instance source=this.open() id=Wrapper<T#3>.open

    }
}

/// @generic.instance id=Sealed<T#3> template=Sealed arguments=(T#3)
/// @generic.instance id=Wrapper<T#1> template=Wrapper arguments=(T#1)
/// @generic.instance id=Wrapper<T#3>.open template=Wrapper.open arguments=(T#3)
"#,
        r#""#,
    );
}

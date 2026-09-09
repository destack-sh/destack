use crate::tests::{DirRows, TestSession};

/// A member access inside a newtype extension resolves on the backing type.
#[test]
fn test_newtype_member_access_dereferences_to_the_backing() {
    let session = TestSession::single(
        r#"
class Wrapper<T> {
    open(): T {
        return unreachable();
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Wrapper<out T> {
    open(): T {
        return unreachable();
    }
}

newtype Sealed<out T> = Wrapper<T>;

extension<T> of Sealed<T> {
    reveal(): T {
        this.open<T, P0>()
    }
}

=== dir ===
class Wrapper<T> {
/// @generic.template symbol=Wrapper parameters=(out T#1)
/// @type.symbol symbol=Wrapper type=Wrapper
/// @definition.class symbol=Wrapper template=(out T#1)
/// @definition.method symbol=Wrapper.open slot=open type=<Wrapper.open.P0: Place>(this: Managed<Wrapper<T#1>, Wrapper.open.P0>) => T#1
/// @type.symbol symbol=Wrapper.T source=T type=T#1

    open(): T {
    /// @generic.template symbol=Wrapper.open parent=template#0 parameters=(P0: Place)
    /// @type.symbol symbol=Wrapper.open type=<Wrapper.open.P0: Place>(this: Managed<Wrapper<T#1>, Wrapper.open.P0>) => T#1
    /// @type.symbol symbol=Wrapper.open.this type=Managed<Wrapper<T#1>, Wrapper.open.P0>
    /// @resolution.name source=T target=Wrapper.T

        return unreachable();
        /// @resolution.name source=unreachable target=unreachable
        /// @resolution.call source=unreachable() parameters=() return=never kind=symbol target=unreachable

    }
}

newtype Sealed<T> = Wrapper<T>;
/// @generic.template symbol=Sealed parameters=(out T#2)
/// @type.symbol symbol=Sealed source="newtype Sealed<T> = Wrapper<T>" type=Sealed
/// @generic.instance id=Wrapper<T#2> template=Wrapper arguments=(T#2)
/// @definition.newtype symbol=Sealed source="newtype Sealed<T> = Wrapper<T>" template=(out T#2) backing=Wrapper<T#2> constructors=[<T#2>(Wrapper<T#2>) => Sealed<T#2>]
/// @type.symbol symbol=Sealed.T source=T type=T#2
/// @resolution.name source=Wrapper target=Wrapper
/// @resolution.name source=T target=Sealed.T

extension<T> of Sealed<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @generic.instance id=Sealed<T#3> template=Sealed arguments=(T#3)
/// @definition.extension symbol=<module>#2 form=local target=Sealed<T#3>
/// @definition.method symbol=reveal slot=reveal type=<reveal.P0: Place>(this: Managed<Sealed<T#3>, reveal.P0>) => T#3
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Sealed target=Sealed
/// @resolution.name source=T target=T

    reveal(): T {
    /// @generic.template symbol=reveal parent=template#2 parameters=(P0: Place)
    /// @type.symbol symbol=reveal type=<reveal.P0: Place>(this: Managed<Sealed<T#3>, reveal.P0>) => T#3
    /// @type.symbol symbol=reveal.this type=Managed<Sealed<T#3>, reveal.P0>
    /// @resolution.name source=T target=T

        this.open()
        /// @resolution.member source=this.open receiver=Managed<Sealed<T#3>, reveal.P0> type=<Wrapper.open.P0: Place>(this: Managed<Wrapper<T#3>, Wrapper.open.P0>) => T#3 kind=symbol target_receiver=Managed<Sealed<T#3>, reveal.P0> adjustments=(newtype.payload(Sealed, Managed<Wrapper<T#3>, reveal.P0>)) target=Wrapper.open
        /// @resolution.call source=this.open() parameters=() return=T#3 kind=symbol target=Wrapper.open receiver=Managed<Sealed<T#3>, reveal.P0> adjustments=(newtype.payload(Sealed, Managed<Wrapper<T#3>, reveal.P0>)) instance=Wrapper<T#3>.open<reveal.P0>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Managed<Sealed<T#3>, reveal.P0>
        /// @resolution.place source=this placement=reveal.P0 lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id="Wrapper.open<T#3, reveal.P0>" template=Wrapper.open arguments=(T#3, reveal.P0) owner=reveal
        /// @generic.instantiation id=Sealed<T#3> template=Sealed arguments=(T#3) owner=reveal
        /// @generic.instantiation id=Wrapper.open<T#3> template=Wrapper.open arguments=(T#3) owner=reveal
        /// @generic.instance id="Wrapper.open<T#3, \"local\">" template=Wrapper.open arguments=(T#3, "local")
        /// @generic.instance id=Wrapper<T#3> template=Wrapper arguments=(T#3)

    }
}
"#,
    );
}

/// A member access on an imported newtype projects its generic backing.
#[test]
fn test_imported_newtype_member_access_projects_generic_backing() {
    let session = TestSession::builder()
        .module(
            "value.ds",
            r#"
export class Wrapper<T> {
    open(): T {
        return unreachable();
    }
}

export newtype Sealed<T> = Wrapper<T>;

export function value(): Sealed<int32> {
    return unreachable();
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { value } from "./value.ds";

const number = value().open();
number satisfies int32;
"#,
        )
        .build();

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { value } from "./value.ds";

const number: int32 = value().open<int32>();
number satisfies int32;

=== dir ===
import { value } from "./value.ds";

const number = value().open();
/// @type.symbol symbol=number source=number type=int32
/// @resolution.pattern source=number kind=binding target=number
/// @resolution.name source=value target=value.value
/// @resolution.member source=value().open receiver=value.Sealed<int32> type=<value.Wrapper.open.P0: Place>(this: Managed<value.Wrapper<int32>, value.Wrapper.open.P0>) => int32 kind=symbol target_receiver=value.Sealed<int32> adjustments=(newtype.payload(value.Sealed, value.Wrapper<int32>)) target=value.Wrapper.open
/// @resolution.call source=value() parameters=() return=value.Sealed<int32> kind=symbol target=value.value
/// @resolution.call source=value().open() parameters=() return=int32 kind=symbol target=value.Wrapper.open receiver=value.Sealed<int32> adjustments=(newtype.payload(value.Sealed, value.Wrapper<int32>)) instance="value.Wrapper<int32>.open<\"local\">"
/// @generic.instantiation id="value.Wrapper.open<int32, \"local\">" template=value.Wrapper.open arguments=(int32, "local")
/// @generic.instantiation id=value.Sealed<int32> template=value.Sealed arguments=(int32)
/// @generic.instantiation id=value.Wrapper.open<int32> template=value.Wrapper.open arguments=(int32)
/// @generic.instance id="value.Wrapper.open<int32, \"local\">" template=value.Wrapper.open arguments=(int32, "local")
/// @generic.instance id=value.Sealed<int32> template=value.Sealed arguments=(int32)
/// @generic.instance id=value.Wrapper<int32> template=value.Wrapper arguments=(int32)

number satisfies int32;
/// @resolution.name source=number target=number
/// @resolution.place source=number placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=number root=number
"#,
    );
}

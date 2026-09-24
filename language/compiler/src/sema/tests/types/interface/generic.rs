use crate::tests::{DirRows, TestSession};

/// A generic requirement is implemented by a method with matching generics, the requirement's
/// bounds read under the implemented interface's arguments.
#[test]
fn test_implement_a_generic_requirement_with_matching_generics() {
    let session = TestSession::single(
        r#"
import { Iterator } from "destack:iter";

interface Collect<T> {
    static gather<I: Iterator<T>>(values: I): this;
}

struct Bag<T> {
    first: T | undefined;
}

export extension<T> of Bag<T> implements Collect<T> {
    static gather<I: Iterator<T>>(values: I): Bag<T> {
        Bag { first: undefined }
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Iterator } from "destack:iter";

interface Collect<T> {
    static gather<I: Iterator<T>>(values: I): this;
}

struct Bag<out T> {
    first: T | undefined;
}

export extension<T> of Bag<T> implements Collect<T> {
    static gather<I: Iterator<T>>(values: I): Bag<T> {
        Bag<T> { first: undefined as T | undefined }
    }
}

=== dir ===
import { Iterator } from "destack:iter";

interface Collect<T> {
/// @generic.template symbol=Collect parameters=(T#1, this: Collect<T#1>)
/// @type.symbol symbol=Collect type=Collect
/// @definition.interface symbol=Collect template=(T#1, this: Collect<T#1>)
/// @definition.where symbol=Collect relation=satisfies left=this right=Collect<T#1>
/// @definition.method symbol=Collect.gather source="static gather<I: Iterator<T>>(values: I): this" slot=gather static=true type=<I#1: Iterator<T#1>>(I#1) => this
/// @type.symbol symbol=Collect.T source=T type=T#1

    static gather<I: Iterator<T>>(values: I): this;
    /// @generic.template symbol=Collect.gather parent=template#0 parameters=(I#1: Iterator<T#1>)
    /// @type.symbol symbol=Collect.gather source="static gather<I: Iterator<T>>(values: I): this" type=<I#1: Iterator<T#1>>(I#1) => this
    /// @type.symbol symbol=Collect.gather.I source="I: Iterator<T>" type=I#1
    /// @resolution.name source=Iterator target=Iterator
    /// @resolution.name source=T target=Collect.T
    /// @type.symbol symbol=Collect.gather.values source="values: I" type=I#1
    /// @resolution.name source=I target=Collect.gather.I

}

struct Bag<T> {
/// @generic.template symbol=Bag parameters=(out T#2)
/// @type.symbol symbol=Bag type=Bag
/// @definition.struct symbol=Bag template=(out T#2)
/// @definition.field symbol=Bag.first source="first: T | undefined" key=first type=T#2 | undefined
/// @type.symbol symbol=Bag.T source=T type=T#2

    first: T | undefined;
    /// @type.symbol symbol=Bag.first source="first: T | undefined" type=T#2 | undefined
    /// @resolution.name source=T target=Bag.T

}

export extension<T> of Bag<T> implements Collect<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @definition.extension symbol=<module>#2 form=exported target=Bag<T#3>
/// @definition.implements symbol=<module>#2 source=Collect<T> target=Collect<T#3>
/// @definition.method symbol=gather slot=gather static=true type=<I#2: Iterator<T#3>>(I#2) => Bag<T#3>
/// @definition.conformance symbol=<module>#2 member=gather requirement=Collect.gather
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Bag target=Bag
/// @resolution.name source=T target=T
/// @resolution.name source=Collect target=Collect
/// @resolution.name source=T target=T

    static gather<I: Iterator<T>>(values: I): Bag<T> {
    /// @generic.template symbol=gather parent=template#2 parameters=(I#2: Iterator<T#3>)
    /// @type.symbol symbol=gather type=<I#2: Iterator<T#3>>(I#2) => Bag<T#3>
    /// @type.symbol symbol=gather.I source="I: Iterator<T>" type=I#2
    /// @resolution.name source=Iterator target=Iterator
    /// @resolution.name source=T target=T
    /// @type.symbol symbol=gather.values source="values: I" type=I#2
    /// @resolution.name source=I target=gather.I
    /// @resolution.name source=Bag target=Bag
    /// @resolution.name source=T target=T

        Bag { first: undefined }
        /// @resolution.name source=Bag target=Bag

    }
}
"#,
        r#"
"#,
    );
}

/// An iterable's defaulted iterator yields an object newtype placed with the iterator.
#[test]
fn test_yield_an_object_newtype_through_a_defaulted_associated_iterator() {
    let session = TestSession::single(
        r#"
import { Iterable } from "destack:iter";

newtype Key = {
    index: uint32;
};

class Table<T> {
    static new(): Table<T> {
        Table.new()
    }
}

export extension<T> of Table<T> implements Iterable<(Key, T)> {
    iterator(this): this.Iterator {
        this.iterator()
    }
}

export extension<T, 'a, const A: "immutable" | "exclusive"> of Borrowed<Table<T>, 'a, A>
    implements Iterable<(Key, Borrowed<T, 'a, A>)>
{
    iterator(this): this.Iterator {
        this.iterator()
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Iterable } from "destack:iter";

newtype Key = {
    index: uint32;
};

class Table<T> {
    static new(): Table<T> {
        Table.new<T>()
    }
}

export extension<T> of Table<T> implements Iterable<(Key, T)> {
    iterator(this): Iterator<(Key, T)> {
        this.iterator<T>()
    }
}

export extension<T, 'a, const A: "immutable" | "exclusive"> of Borrowed<Table<T>, 'a, A>
    implements Iterable<(Key, Borrowed<T, 'a, A>)>
{
    iterator(this): Iterator<(Key, Borrowed<T, 'a, A>)> {
        this.iterator<T, 'a, A>()
    }
}

=== dir ===
import { Iterable } from "destack:iter";

newtype Key = {
/// @type.symbol symbol=Key type=Key
/// @definition.newtype symbol=Key backing={ index: uint32 } constructors=[({ index: uint32 }) => Key]

    index: uint32;
    /// @type.symbol symbol=Key.index source="index: uint32" type=uint32

};

class Table<T> {
/// @generic.template symbol=Table parameters=(T#1)
/// @type.symbol symbol=Table type=typeof Table
/// @definition.class symbol=Table template=(T#1)
/// @definition.method symbol=Table.new slot=new static=true type=() => Table<T#1>
/// @type.symbol symbol=Table.T source=T type=T#1

    static new(): Table<T> {
    /// @type.symbol symbol=Table.new type=() => Table<T#1>
    /// @resolution.name source=Table target=Table
    /// @resolution.name source=T target=Table.T

        Table.new()
        /// @resolution.name source=Table target=Table
        /// @resolution.member source=Table.new receiver=typeof Table type=() => Table<T#1> kind=symbol target_receiver=typeof Table target=Table.new
        /// @resolution.call source=Table.new() parameters=() return=Table<T#1> kind=symbol target=Table.new instance=Table<T#1>.new
        /// @generic.instantiation id=Table.new<T#1> template=Table.new arguments=(T#1) owner=Table.new

    }
}

export extension<T> of Table<T> implements Iterable<(Key, T)> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=exported target=Table<T#2>
/// @definition.implements symbol=<module>#2 source="Iterable<(Key, T)>" target="Iterable<(Key, T#2)>"
/// @definition.method symbol=iterator#1 slot=iterator type=(this: Table<T#2>) => Iterator<(Key, T#2)>
/// @definition.conformance symbol=<module>#2 member=Iterable.Iterator requirement=Iterable.Iterator
/// @definition.conformance symbol=<module>#2 member=iterator#1 requirement=Iterable.iterator
/// @type.symbol symbol=T#1 source=T type=T#2
/// @resolution.name source=Table target=Table
/// @resolution.name source=T target=T#1
/// @resolution.name source=Iterable target=Iterable
/// @resolution.name source=Key target=Key
/// @resolution.name source=T target=T#1

    iterator(this): this.Iterator {
    /// @type.symbol symbol=iterator#1 type=(this: Table<T#2>) => Iterator<(Key, T#2)>
    /// @type.symbol symbol=iterator.this#1 source=this type=Table<T#2>
    /// @resolution.name source=this.Iterator target=Iterable.Iterator

        this.iterator()
        /// @resolution.member source=this.iterator receiver=Table<T#2> type=(this: Table<T#2>) => Iterator<(Key, T#2)> kind=symbol target_receiver=Table<T#2> target=iterator#1
        /// @resolution.call source=this.iterator() parameters=() return=Iterator<(Key, T#2)> kind=symbol target=iterator#1 receiver=Table<T#2> instance=Table<T#2>.<extension#1>.iterator#1
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Table<T#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=iterator#1<T#2> template=iterator#1 arguments=(T#2) owner=iterator#1

    }
}

export extension<T, 'a, const A: "immutable" | "exclusive"> of Borrowed<Table<T>, 'a, A>
/// @generic.template symbol=<module>#3 parameters=(T#3, 'a, const A: "immutable" | "exclusive")
/// @definition.extension symbol=<module>#3 form=exported target=WithAccess<&'a Table<T#3>, A>
/// @definition.implements symbol=<module>#3 source="Iterable<(Key, Borrowed<T, 'a, A>)>" target="Iterable<(Key, WithAccess<&'a T#3, A>)>"
/// @definition.method symbol=iterator#2 slot=iterator type=(this: WithAccess<&'a Table<T#3>, A>) => Iterator<(Key, WithAccess<&'a T#3, A>)>
/// @definition.conformance symbol=<module>#3 member=Iterable.Iterator requirement=Iterable.Iterator
/// @definition.conformance symbol=<module>#3 member=iterator#2 requirement=Iterable.iterator
/// @type.symbol symbol=T#2 source=T type=T#3
/// @type.symbol symbol='a source='a type='a
/// @type.symbol symbol=A source="const A: \"immutable\" | \"exclusive\"" type=A
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Table target=Table
/// @resolution.name source=T target=T#2
/// @resolution.name source='a target='a
/// @resolution.name source=A target=A

    implements Iterable<(Key, Borrowed<T, 'a, A>)>
    /// @resolution.name source=Iterable target=Iterable
    /// @resolution.name source=Key target=Key
    /// @resolution.name source=Borrowed target=Borrowed
    /// @resolution.name source=T target=T#2
    /// @resolution.name source='a target='a
    /// @resolution.name source=A target=A

{
    iterator(this): this.Iterator {
    /// @type.symbol symbol=iterator#2 type=(this: WithAccess<&'a Table<T#3>, A>) => Iterator<(Key, WithAccess<&'a T#3, A>)>
    /// @type.symbol symbol=iterator.this#2 source=this type=WithAccess<&'a Table<T#3>, A>
    /// @resolution.name source=this.Iterator target=Iterable.Iterator

        this.iterator()
        /// @resolution.member source=this.iterator receiver=WithAccess<&'a Table<T#3>, A> type=(this: WithAccess<&'a Table<T#3>, A>) => Iterator<(Key, WithAccess<&'a T#3, A>)> kind=symbol target_receiver=WithAccess<&'a Table<T#3>, A> target=iterator#2
        /// @resolution.call source=this.iterator() parameters=() return=Iterator<(Key, WithAccess<&'a T#3, A>)> regions=('a) kind=symbol target=iterator#2 receiver=WithAccess<&'a Table<T#3>, A> instance="WithAccess<&'a Table<T#3>, A>.<extension#2>.iterator#2"
        /// @resolution.receiver source=this kind=this declaration=<module>#3 type=WithAccess<&'a Table<T#3>, A>
        /// @resolution.place source=this placement='a lifetime='a access=A
        /// @resolution.access source=this root=this
        /// @generic.instantiation id="iterator#2<T#3, 'a, A>" template=iterator#2 arguments=(T#3, 'a, A) owner=iterator#2

    }
}
"#,
        r#"
"#,
    );
}

use crate::tests::{DirRows, TestSession};

/// An extension keeps its sibling conformances beside a borrowed interface argument.
#[test]
fn test_keep_sibling_conformances_beside_a_borrowed_interface_argument() {
    let session = TestSession::single(
        r#"
import { Add, Hash, Hasher } from "tspp:ops";

class Foo {}

extension of Foo implements Add<&readonly Foo>, Hash {
    type Output = Foo;

    add(&readonly this, other: &readonly Foo): Foo {
        return new Foo();
    }

    hash(state: &Hasher): void {}
}

newtype Name = string;

extension of Name implements Add<&readonly Name>, Hash {
    type Output = Name;

    add(&readonly this, other: &readonly Name): Name {
        return this;
    }

    hash(state: &Hasher): void {}
}

struct Key {
    foo: Foo;
    name: Name;
}

declare function requireHash<T: Hash>(value: T): T;
declare const name: Name;

const direct = requireHash(new Foo());
const named = requireHash(name);
const derived = requireHash(Key { foo: new Foo(), name });
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Add, Hash, Hasher } from "tspp:ops";

class Foo {}

extension<'a> of Foo implements Add<&readonly Foo>, Hash {
    type Output = Foo;

    add(&readonly this, other: &'c readonly Foo): Foo {
        return new Foo();
    }

    hash(state: &'b Hasher): void {}
}

newtype Name = string;

extension<'a> of Name implements Add<&readonly Name>, Hash {
    type Output = Name;

    add(&readonly this, other: &'c readonly Name): Name {
        return this;
    }

    hash(state: &'b Hasher): void {}
}

struct Key {
    foo: Foo;
    name: Name;
}

declare function requireHash<T: Hash>(value: T): T;
declare const name: Name;

const direct: Foo = requireHash<Foo>(new Foo());
const named: Name = requireHash<Name>(name);
const derived: Key = requireHash<Key>(Key { foo: new Foo(), name });

=== dir ===
import { Add, Hash, Hasher } from "tspp:ops";

class Foo {}
/// @type.symbol symbol=Foo source="class Foo {}" type=typeof Foo
/// @definition.class symbol=Foo source="class Foo {}"

extension of Foo implements Add<&readonly Foo>, Hash {
/// @generic.template symbol=<module>#2 parameters=('a)
/// @definition.extension symbol=<module>#2 form=local target=Foo
/// @definition.implements symbol=<module>#2 source="Add<&readonly Foo>" target="Add<&<module>#2.'a readonly Foo>"
/// @definition.implements symbol=<module>#2 source=Hash target=Hash
/// @definition.associated.type symbol=Output#1 source="type Output = Foo" key=Output value=Foo
/// @definition.method symbol=add#1 slot=add type=<add#1.'a, add#1.'b>(this: &add#1.'a readonly Foo, &add#1.'b readonly Foo) => Foo
/// @definition.method symbol=hash#1 source="hash(state: &Hasher): void {}" slot=hash type=<hash#1.'a>(this: Foo, &hash#1.'a Hasher) => void
/// @definition.conformance symbol=<module>#2 member=Output#1 requirement=Add.Output
/// @definition.conformance symbol=<module>#2 member=add#1 requirement=Add.add
/// @definition.conformance symbol=<module>#2 member=hash#1 requirement=Hash.hash
/// @resolution.name source=Foo target=Foo
/// @resolution.name source=Add target=Add
/// @resolution.name source=Foo target=Foo
/// @resolution.name source=Hash target=Hash

    type Output = Foo;
    /// @type.symbol symbol=Output#1 source="type Output = Foo" type=Foo
    /// @resolution.name source=Foo target=Foo

    add(&readonly this, other: &readonly Foo): Foo {
    /// @generic.template symbol=add#1 parent=template#0 parameters=('a, 'b)
    /// @type.symbol symbol=add#1 type=<add#1.'a, add#1.'b>(this: &add#1.'a readonly Foo, &add#1.'b readonly Foo) => Foo
    /// @type.symbol symbol=add.this#1 source="&readonly this" type=&add#1.'a readonly Foo
    /// @type.symbol symbol=add.other#1 source="other: &readonly Foo" type=&add#1.'b readonly Foo
    /// @resolution.name source=Foo target=Foo
    /// @resolution.name source=Foo target=Foo

        return new Foo();
        /// @resolution.construct source="new Foo()" parameters=() return=Foo kind=class target=Foo constructor=default
        /// @resolution.name source=Foo target=Foo

    }

    hash(state: &Hasher): void {}
    /// @generic.template symbol=hash#1 parent=template#0 parameters=('a)
    /// @type.symbol symbol=hash#1 source="hash(state: &Hasher): void {}" type=<hash#1.'a>(this: Foo, &hash#1.'a Hasher) => void
    /// @type.symbol symbol=hash.this#1 type=Foo
    /// @type.symbol symbol=hash.state#1 source="state: &Hasher" type=&hash#1.'a Hasher
    /// @resolution.name source=Hasher target=Hasher

}

newtype Name = string;
/// @type.symbol symbol=Name source="newtype Name = string" type=Name
/// @definition.newtype symbol=Name source="newtype Name = string" backing=string constructors=[(string) => Name]

extension of Name implements Add<&readonly Name>, Hash {
/// @generic.template symbol=<module>#3 parameters=('a)
/// @definition.extension symbol=<module>#3 form=local target=Name
/// @definition.implements symbol=<module>#3 source="Add<&readonly Name>" target="Add<&<module>#3.'a readonly Name>"
/// @definition.implements symbol=<module>#3 source=Hash target=Hash
/// @definition.associated.type symbol=Output#2 source="type Output = Name" key=Output value=Name
/// @definition.method symbol=add#2 slot=add type=<add#2.'a, add#2.'b>(this: &add#2.'a readonly Name, &add#2.'b readonly Name) => Name
/// @definition.method symbol=hash#2 source="hash(state: &Hasher): void {}" slot=hash type=<hash#2.'a>(this: Name, &hash#2.'a Hasher) => void
/// @definition.conformance symbol=<module>#3 member=Output#2 requirement=Add.Output
/// @definition.conformance symbol=<module>#3 member=add#2 requirement=Add.add
/// @definition.conformance symbol=<module>#3 member=hash#2 requirement=Hash.hash
/// @resolution.name source=Name target=Name
/// @resolution.name source=Add target=Add
/// @resolution.name source=Name target=Name
/// @resolution.name source=Hash target=Hash

    type Output = Name;
    /// @type.symbol symbol=Output#2 source="type Output = Name" type=Name
    /// @resolution.name source=Name target=Name

    add(&readonly this, other: &readonly Name): Name {
    /// @generic.template symbol=add#2 parent=template#1 parameters=('a, 'b)
    /// @type.symbol symbol=add#2 type=<add#2.'a, add#2.'b>(this: &add#2.'a readonly Name, &add#2.'b readonly Name) => Name
    /// @type.symbol symbol=add.this#2 source="&readonly this" type=&add#2.'a readonly Name
    /// @type.symbol symbol=add.other#2 source="other: &readonly Name" type=&add#2.'b readonly Name
    /// @resolution.name source=Name target=Name
    /// @resolution.name source=Name target=Name

        return this;
        /// @resolution.receiver source=this kind=this declaration=<module>#3 type=&add#2.'a readonly Name
        /// @resolution.place source=this placement=add#2.'a lifetime=add#2.'a access="readonly"
        /// @resolution.access source=this root=this

    }

    hash(state: &Hasher): void {}
    /// @generic.template symbol=hash#2 parent=template#1 parameters=('a)
    /// @type.symbol symbol=hash#2 source="hash(state: &Hasher): void {}" type=<hash#2.'a>(this: Name, &hash#2.'a Hasher) => void
    /// @type.symbol symbol=hash.this#2 type=Name
    /// @type.symbol symbol=hash.state#2 source="state: &Hasher" type=&hash#2.'a Hasher
    /// @resolution.name source=Hasher target=Hasher

}

struct Key {
/// @type.symbol symbol=Key type=Key
/// @definition.struct symbol=Key
/// @definition.field symbol=Key.foo source="foo: Foo" key=foo type=Foo
/// @definition.field symbol=Key.name source="name: Name" key=name type=Name

    foo: Foo;
    /// @type.symbol symbol=Key.foo source="foo: Foo" type=Foo
    /// @resolution.name source=Foo target=Foo

    name: Name;
    /// @type.symbol symbol=Key.name source="name: Name" type=Name
    /// @resolution.name source=Name target=Name

}

declare function requireHash<T: Hash>(value: T): T;
/// @generic.template symbol=requireHash parameters=(T: Hash)
/// @type.symbol symbol=requireHash source="declare function requireHash<T: Hash>(value: T): T" type=<T: Hash>(T) => T
/// @type.symbol symbol=requireHash.T source="T: Hash" type=T
/// @resolution.name source=Hash target=Hash
/// @resolution.name source=T target=requireHash.T
/// @resolution.name source=T target=requireHash.T

declare const name: Name;
/// @type.symbol symbol=name source=name type=Name
/// @resolution.pattern source=name kind=binding target=name
/// @resolution.name source=Name target=Name

const direct = requireHash(new Foo());
/// @type.symbol symbol=direct source=direct type=Foo
/// @resolution.pattern source=direct kind=binding target=direct
/// @resolution.name source=requireHash target=requireHash
/// @resolution.call source="requireHash(new Foo())" parameters=(Foo) arguments=(provided(new Foo()) as Foo) return=Foo kind=symbol target=requireHash instance=requireHash<Foo>
/// @generic.instantiation id=requireHash<Foo> template=requireHash arguments=(Foo)
/// @resolution.construct source="new Foo()" parameters=() return=Foo kind=class target=Foo constructor=default
/// @resolution.name source=Foo target=Foo

const named = requireHash(name);
/// @type.symbol symbol=named source=named type=Name
/// @resolution.pattern source=named kind=binding target=named
/// @resolution.name source=requireHash target=requireHash
/// @resolution.call source=requireHash(name) parameters=(Name) arguments=(provided(name) as Name) return=Name kind=symbol target=requireHash instance=requireHash<Name>
/// @generic.instantiation id=requireHash<Name> template=requireHash arguments=(Name)
/// @resolution.name source=name target=name
/// @resolution.place source=name placement="local" lifetime="static" access="immutable"
/// @resolution.access source=name root=name

const derived = requireHash(Key { foo: new Foo(), name });
/// @type.symbol symbol=derived source=derived type=Key
/// @resolution.pattern source=derived kind=binding target=derived
/// @resolution.name source=requireHash target=requireHash
/// @resolution.call source="requireHash(Key { foo: new Foo(), name })" parameters=(Key) arguments=(provided(Key { foo: new Foo(), name }) as Key) return=Key kind=symbol target=requireHash instance=requireHash<Key>
/// @generic.instantiation id=requireHash<Key> template=requireHash arguments=(Key)
/// @resolution.name source=Key target=Key
/// @resolution.construct source="new Foo()" parameters=() return=Foo kind=class target=Foo constructor=default
/// @resolution.name source=Foo target=Foo
/// @resolution.name source=name target=name
/// @resolution.place source=name placement="local" lifetime="static" access="immutable"
/// @resolution.access source=name root=name
"#,
        r#"
/// @diagnostic.error id=return-not-assignable message="type '&'b readonly Name' is not assignable to the declared result type 'Name'"
/// @diagnostic.label line=22 column=16 span="this" line_source="return this;"
"#,
    );
}

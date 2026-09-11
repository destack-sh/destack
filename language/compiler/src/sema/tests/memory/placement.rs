use crate::tests::{DirRows, TestSession};

#[test]
fn test_accept_placement_modifiers_in_either_written_order() {
    let session = TestSession::single(
        r#"
class User {}

type LocalReadonly = local readonly User;
type ReadonlyLocal = readonly local User;
type LocalBorrowed<'a> = local &'a readonly User;
type BorrowedLocal<'a> = &'a readonly local User;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

type LocalReadonly = local readonly User;
type ReadonlyLocal = readonly local User;
type LocalBorrowed<'a> = local &'a readonly User;
type BorrowedLocal<'a> = &'a readonly local User;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

type LocalReadonly = local readonly User;
/// @type.symbol symbol=LocalReadonly source="type LocalReadonly = local readonly User" type=Readonly<local User>
/// @definition.type symbol=LocalReadonly source="type LocalReadonly = local readonly User" value=Readonly<local User>
/// @resolution.name source=User target=User

type ReadonlyLocal = readonly local User;
/// @type.symbol symbol=ReadonlyLocal source="type ReadonlyLocal = readonly local User" type=Readonly<local User>
/// @definition.type symbol=ReadonlyLocal source="type ReadonlyLocal = readonly local User" value=Readonly<local User>
/// @resolution.name source=User target=User

type LocalBorrowed<'a> = local &'a readonly User;
/// @generic.template symbol=LocalBorrowed parameters=('a#1)
/// @type.symbol symbol=LocalBorrowed source="type LocalBorrowed<'a> = local &'a readonly User" type=&'a#1 readonly User
/// @definition.type symbol=LocalBorrowed source="type LocalBorrowed<'a> = local &'a readonly User" template=('a#1) value=&'a#1 readonly User
/// @type.symbol symbol=LocalBorrowed.'a source='a type='a#1
/// @resolution.name source='a target=LocalBorrowed.'a
/// @resolution.name source=User target=User

type BorrowedLocal<'a> = &'a readonly local User;
/// @generic.template symbol=BorrowedLocal parameters=('a#2)
/// @type.symbol symbol=BorrowedLocal source="type BorrowedLocal<'a> = &'a readonly local User" type=&'a#2 readonly User
/// @definition.type symbol=BorrowedLocal source="type BorrowedLocal<'a> = &'a readonly local User" template=('a#2) value=&'a#2 readonly local User
/// @type.symbol symbol=BorrowedLocal.'a source='a type='a#2
/// @resolution.name source='a target=BorrowedLocal.'a
/// @resolution.name source=User target=User
"#,
        r#"

"#,
    );
}

#[test]
fn test_reject_relabeling_managed_values_across_spaces() {
    let session = TestSession::single(
        r#"
class User {}

declare const localUser: local User;
declare const sharedUser: shared User;

const localFromShared: local User = sharedUser;
const sharedFromLocal: shared User = localUser;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare const localUser: local User;
declare const sharedUser: shared User;

const localFromShared: local User = sharedUser;
const sharedFromLocal: shared User = localUser;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare const localUser: local User;
/// @type.symbol symbol=localUser source=localUser type=local User
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: shared User;
/// @type.symbol symbol=sharedUser source=sharedUser type=shared User
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

const localFromShared: local User = sharedUser;
/// @type.symbol symbol=localFromShared source=localFromShared type=local User
/// @resolution.pattern source=localFromShared kind=binding target=localFromShared
/// @resolution.name source=User target=User
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.place source=sharedUser placement="shared" lifetime="managed" access="mutable"
/// @resolution.access source=sharedUser root=sharedUser

const sharedFromLocal: shared User = localUser;
/// @type.symbol symbol=sharedFromLocal source=sharedFromLocal type=shared User
/// @resolution.pattern source=sharedFromLocal kind=binding target=sharedFromLocal
/// @resolution.name source=User target=User
/// @resolution.name source=localUser target=localUser
/// @resolution.place source=localUser placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=localUser root=localUser
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'shared User' is not assignable to type 'local User'"
/// @diagnostic.label line=7 column=37 span="sharedUser" line_source="const localFromShared: local User = sharedUser;"
/// @diagnostic.related line=7 column=24 span="local" line_source="const localFromShared: local User = sharedUser;" message="expected due to this annotation"
/// @diagnostic.note message="a value never changes its space"
/// @diagnostic.help message="use a value in the destination placement or create a new value there"
/// @diagnostic.error id=not-assignable message="type 'local User' is not assignable to type 'shared User'"
/// @diagnostic.label line=8 column=38 span="localUser" line_source="const sharedFromLocal: shared User = localUser;"
/// @diagnostic.related line=8 column=24 span="shared" line_source="const sharedFromLocal: shared User = localUser;" message="expected due to this annotation"
/// @diagnostic.note message="a value never changes its space"
/// @diagnostic.help message="use a value in the destination placement or create a new value there"
"#,
    );
}

#[test]
fn test_project_the_place_of_each_union_element() {
    let session = TestSession::single(
        r#"
local class User {}
shared class Team {}

type ChoicePlace = PlaceOf<User | Team>;

declare const choicePlace: ChoicePlace;

choicePlace satisfies "local" | "shared";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
local class User {}
shared class Team {}

type ChoicePlace = PlaceOf<User | Team>;

declare const choicePlace: ChoicePlace;

choicePlace satisfies "local" | "shared";

=== dir ===
local class User {}
/// @type.symbol symbol=User source="local class User {}" type=typeof User
/// @definition.class symbol=User source="local class User {}"

shared class Team {}
/// @type.symbol symbol=Team source="shared class Team {}" type=typeof Team
/// @definition.class symbol=Team source="shared class Team {}"

type ChoicePlace = PlaceOf<User | Team>;
/// @type.symbol symbol=ChoicePlace source="type ChoicePlace = PlaceOf<User | Team>" type="local" | "shared"
/// @definition.type symbol=ChoicePlace source="type ChoicePlace = PlaceOf<User | Team>" value=PlaceOf<User | Team>
/// @resolution.name source=PlaceOf target=PlaceOf
/// @resolution.name source=User target=User
/// @resolution.name source=Team target=Team

declare const choicePlace: ChoicePlace;
/// @type.symbol symbol=choicePlace source=choicePlace type=ChoicePlace
/// @resolution.pattern source=choicePlace kind=binding target=choicePlace
/// @resolution.name source=ChoicePlace target=ChoicePlace

choicePlace satisfies "local" | "shared";
/// @resolution.name source=choicePlace target=choicePlace
/// @resolution.place source=choicePlace placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=choicePlace root=choicePlace
"#,
        r#"
"#,
    );
}

#[test]
fn test_project_a_written_place_through_the_representation() {
    let session = TestSession::single(
        r#"
class User {}

type LocalPlace = PlaceOf<local User>;
type SharedPlace = PlaceOf<shared User>;

declare const localPlace: LocalPlace;
declare const sharedPlace: SharedPlace;

localPlace satisfies "local";
sharedPlace satisfies "shared";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

type LocalPlace = PlaceOf<local User>;
type SharedPlace = PlaceOf<shared User>;

declare const localPlace: LocalPlace;
declare const sharedPlace: SharedPlace;

localPlace satisfies "local";
sharedPlace satisfies "shared";

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

type LocalPlace = PlaceOf<local User>;
/// @type.symbol symbol=LocalPlace source="type LocalPlace = PlaceOf<local User>" type="local"
/// @definition.type symbol=LocalPlace source="type LocalPlace = PlaceOf<local User>" value=PlaceOf<local User>
/// @resolution.name source=PlaceOf target=PlaceOf
/// @resolution.name source=User target=User

type SharedPlace = PlaceOf<shared User>;
/// @type.symbol symbol=SharedPlace source="type SharedPlace = PlaceOf<shared User>" type="shared"
/// @definition.type symbol=SharedPlace source="type SharedPlace = PlaceOf<shared User>" value=PlaceOf<shared User>
/// @resolution.name source=PlaceOf target=PlaceOf
/// @resolution.name source=User target=User

declare const localPlace: LocalPlace;
/// @type.symbol symbol=localPlace source=localPlace type=LocalPlace
/// @resolution.pattern source=localPlace kind=binding target=localPlace
/// @resolution.name source=LocalPlace target=LocalPlace

declare const sharedPlace: SharedPlace;
/// @type.symbol symbol=sharedPlace source=sharedPlace type=SharedPlace
/// @resolution.pattern source=sharedPlace kind=binding target=sharedPlace
/// @resolution.name source=SharedPlace target=SharedPlace

localPlace satisfies "local";
/// @resolution.name source=localPlace target=localPlace
/// @resolution.place source=localPlace placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=localPlace root=localPlace

sharedPlace satisfies "shared";
/// @resolution.name source=sharedPlace target=sharedPlace
/// @resolution.place source=sharedPlace placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=sharedPlace root=sharedPlace
"#,
        r#"
"#,
    );
}

#[test]
fn test_select_static_members_through_a_place_generic_extension() {
    let session = TestSession::single(
        r#"
class Buffer {}

extension of Buffer implements Default {
    static default(): ^this {
        return new Buffer();
    }
}

extension of Buffer {
    static make(): Buffer {
        return new Buffer();
    }

    clear(&this): void {}
}

const object = Buffer;
const made = new Buffer();
const buffer = Buffer.make();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Buffer {}

extension of Buffer implements Default {
    static default(): ^this {
        return new Buffer();
    }
}

extension of Buffer {
    static make(): Buffer {
        return new Buffer();
    }

    clear(&this): void {}
}

const object: typeof Buffer = Buffer;
const made: Buffer = new Buffer();
const buffer: Buffer = Buffer.make();

=== dir ===
class Buffer {}
/// @type.symbol symbol=Buffer source="class Buffer {}" type=typeof Buffer
/// @definition.class symbol=Buffer source="class Buffer {}"

extension of Buffer implements Default {
/// @definition.extension symbol=<module>#2 form=local target=Buffer
/// @definition.implements symbol=<module>#2 source=Default target=Default
/// @definition.method symbol=default slot=default static=true type=() => ^this
/// @definition.conformance symbol=<module>#2 member=default requirement=Default.default
/// @resolution.name source=Buffer target=Buffer
/// @resolution.name source=Default target=Default

    static default(): ^this {
    /// @type.symbol symbol=default type=() => ^this

        return new Buffer();
        /// @resolution.construct source="new Buffer()" parameters=() return=^Buffer kind=class target=Buffer constructor=default
        /// @resolution.name source=Buffer target=Buffer

    }
}

extension of Buffer {
/// @definition.extension symbol=<module>#3 form=local target=Buffer
/// @definition.method symbol=clear source="clear(&this): void {}" slot=clear type=<clear.'a>(this: &clear.'a this) => void
/// @definition.method symbol=make slot=make static=true type=() => Buffer
/// @resolution.name source=Buffer target=Buffer

    static make(): Buffer {
    /// @type.symbol symbol=make type=() => Buffer
    /// @resolution.name source=Buffer target=Buffer

        return new Buffer();
        /// @resolution.construct source="new Buffer()" parameters=() return=Buffer kind=class target=Buffer constructor=default
        /// @resolution.name source=Buffer target=Buffer

    }

    clear(&this): void {}
    /// @generic.template symbol=clear parameters=('a)
    /// @type.symbol symbol=clear source="clear(&this): void {}" type=<clear.'a>(this: &clear.'a this) => void
    /// @type.symbol symbol=clear.this source=&this type=&clear.'a this

}

const object = Buffer;
/// @type.symbol symbol=object source=object type=typeof Buffer
/// @resolution.pattern source=object kind=binding target=object
/// @resolution.name source=Buffer target=Buffer

const made = new Buffer();
/// @type.symbol symbol=made source=made type=Buffer
/// @resolution.pattern source=made kind=binding target=made
/// @resolution.construct source="new Buffer()" parameters=() return=Buffer kind=class target=Buffer constructor=default
/// @resolution.name source=Buffer target=Buffer

const buffer = Buffer.make();
/// @type.symbol symbol=buffer source=buffer type=Buffer
/// @resolution.pattern source=buffer kind=binding target=buffer
/// @resolution.name source=Buffer target=Buffer
/// @resolution.member source=Buffer.make receiver=typeof Buffer type=() => Buffer kind=symbol target_receiver=typeof Buffer target=make
/// @resolution.call source=Buffer.make() parameters=() return=Buffer kind=symbol target=make
"#,
        r#"
/// @diagnostic.error id=return-not-assignable message="type '^Buffer' is not assignable to the declared result type '^this'"
/// @diagnostic.label line=6 column=16 span="new Buffer()" line_source="return new Buffer();"
/// @diagnostic.note message="expected 'this', found 'Buffer'"
"#,
    );
}

#[test]
fn test_select_static_members_through_a_foreign_place_generic_extension() {
    let session = TestSession::builder()
        .module(
            "buffer.ds",
            r#"
export class Buffer {}

export extension of Buffer {
    static make(): Buffer {
        return new Buffer();
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Buffer } from "./buffer.ds";

const buffer = Buffer.make();
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Buffer } from "./buffer.ds";

const buffer: Buffer = Buffer.make();

=== dir ===
import { Buffer } from "./buffer.ds";

const buffer = Buffer.make();
/// @type.symbol symbol=buffer source=buffer type=buffer.Buffer
/// @resolution.pattern source=buffer kind=binding target=buffer
/// @resolution.name source=Buffer target=buffer.Buffer
/// @resolution.member source=Buffer.make receiver=typeof buffer.Buffer type=() => buffer.Buffer kind=symbol target_receiver=typeof buffer.Buffer target=buffer.make
/// @resolution.call source=Buffer.make() parameters=() return=buffer.Buffer kind=symbol target=buffer.make
"#,
        r#"
"#,
    );
}

#[test]
fn test_call_extension_members_through_a_place_parametric_field() {
    let session = TestSession::single(
        r#"
class Item<T> {}

class Holder<T> {
    item: Item<T>;
}

extension<T> of Item<T> {
    size(&readonly this): int32 {
        return 1;
    }
}

extension<T> of Holder<T> {
    peek(&readonly this): int32 {
        return this.item.size();
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Item<T> {}

class Holder<T> {
    item: Item<T>;
}

extension<T> of Item<T> {
    size(&readonly this): int32 {
        return 1;
    }
}

extension<T> of Holder<T> {
    peek(&readonly this): int32 {
        return this.item.size<T>();
    }
}

=== dir ===
class Item<T> {}
/// @generic.template symbol=Item parameters=(T#1)
/// @type.symbol symbol=Item source="class Item<T> {}" type=typeof Item
/// @definition.class symbol=Item source="class Item<T> {}" template=(T#1)
/// @type.symbol symbol=Item.T source=T type=T#1

class Holder<T> {
/// @generic.template symbol=Holder parameters=(T#2)
/// @type.symbol symbol=Holder type=typeof Holder
/// @definition.class symbol=Holder template=(T#2)
/// @definition.field symbol=Holder.item source="item: Item<T>" key=item type=Item<T#2>
/// @type.symbol symbol=Holder.T source=T type=T#2

    item: Item<T>;
    /// @type.symbol symbol=Holder.item source="item: Item<T>" type=Item<T#2>
    /// @resolution.name source=Item target=Item
    /// @resolution.name source=T target=Holder.T

}

extension<T> of Item<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @definition.extension symbol=<module>#2 form=local target=Item<T#3>
/// @definition.method symbol=size slot=size type=<size.'a>(this: &size.'a readonly this) => int32
/// @type.symbol symbol=T#1 source=T type=T#3
/// @resolution.name source=Item target=Item
/// @resolution.name source=T target=T#1

    size(&readonly this): int32 {
    /// @generic.template symbol=size parent=template#2 parameters=('a)
    /// @type.symbol symbol=size type=<size.'a>(this: &size.'a readonly this) => int32
    /// @type.symbol symbol=size.this source="&readonly this" type=&size.'a readonly this

        return 1;
    }
}

extension<T> of Holder<T> {
/// @generic.template symbol=<module>#3 parameters=(T#4)
/// @definition.extension symbol=<module>#3 form=local target=Holder<T#4>
/// @definition.method symbol=peek slot=peek type=<peek.'a>(this: &peek.'a readonly this) => int32
/// @type.symbol symbol=T#2 source=T type=T#4
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=T target=T#2

    peek(&readonly this): int32 {
    /// @generic.template symbol=peek parent=template#3 parameters=('a)
    /// @type.symbol symbol=peek type=<peek.'a>(this: &peek.'a readonly this) => int32
    /// @type.symbol symbol=peek.this source="&readonly this" type=&peek.'a readonly this

        return this.item.size();
        /// @resolution.member source=this.item receiver=&peek.'a readonly Holder<T#4> type=Readonly<Item<T#4>> kind=field target_receiver=&peek.'a readonly Holder<T#4> key=item target=Holder.item target_type=Readonly<Item<T#4>>
        /// @resolution.member source=this.item.size receiver=Readonly<Managed<Item<T#4>, peek.'a>> type=<size.'a>(this: &size.'a readonly Item<T#4>) => int32 kind=symbol target_receiver=Readonly<Managed<Item<T#4>, peek.'a>> target=size
        /// @resolution.call source=this.item.size() parameters=() return=int32 regions=(peek.'a) kind=symbol target=size receiver=Readonly<Managed<Item<T#4>, peek.'a>> adjustments=(Readonly<Managed<Item<T#4>, peek.'a>> => direct -> Managed<Item<T#4>, peek.'a>, borrow(&peek.'a readonly Managed<Item<T#4>, peek.'a>)) instance=Item<T#4>.<extension#1>.size
        /// @resolution.receiver source=this kind=this declaration=<module>#3 type=&peek.'a readonly Holder<T#4>
        /// @resolution.place source=this placement=peek.'a lifetime=peek.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.item placement=peek.'a lifetime="managed" access="readonly"
        /// @resolution.access source=this.item root=this keys=[item]
        /// @generic.instantiation id=size<T#4> template=size arguments=(T#4) owner=peek

    }
}
"#,
        r#"
/// @diagnostic.error id=field-not-definitely-initialized message="field 'item' is not initialized on every constructor path"
/// @diagnostic.label line=5 column=5 span="item" line_source="item: Item<T>;"
/// @diagnostic.error id=unused-generic-parameter message="generic parameter 'T' is never used"
/// @diagnostic.label line=2 column=12 span="T" line_source="class Item<T> {}"
/// @diagnostic.help message="declare explicit variance like 'out T' to keep a marker parameter"
"#,
    );
}

#[test]
fn test_call_foreign_extension_members_through_a_place_parametric_field() {
    let session = TestSession::builder()
        .module(
            "item.ds",
            r#"
export class Item<T> {}

export extension<T> of Item<T> {
    size(&readonly this): int32 {
        return 1;
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Item } from "./item.ds";

class Holder<T> {
    item: Item<T>;
}

extension<T> of Holder<T> {
    peek(&readonly this): int32 {
        return this.item.size();
    }
}
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Item } from "./item.ds";

class Holder<T> {
    item: Item<T>;
}

extension<T> of Holder<T> {
    peek(&readonly this): int32 {
        return this.item.size<T>();
    }
}

=== dir ===
import { Item } from "./item.ds";

class Holder<T> {
/// @generic.template symbol=Holder parameters=(T#1)
/// @type.symbol symbol=Holder type=typeof Holder
/// @definition.class symbol=Holder template=(T#1)
/// @definition.field symbol=Holder.item source="item: Item<T>" key=item type=item.Item<T#1>
/// @type.symbol symbol=Holder.T source=T type=T#1

    item: Item<T>;
    /// @type.symbol symbol=Holder.item source="item: Item<T>" type=item.Item<T#1>
    /// @resolution.name source=Item target=item.Item
    /// @resolution.name source=T target=Holder.T

}

extension<T> of Holder<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Holder<T#2>
/// @definition.method symbol=peek slot=peek type=<peek.'a>(this: &peek.'a readonly this) => int32
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=T target=T

    peek(&readonly this): int32 {
    /// @generic.template symbol=peek parent=template#1 parameters=('a)
    /// @type.symbol symbol=peek type=<peek.'a>(this: &peek.'a readonly this) => int32
    /// @type.symbol symbol=peek.this source="&readonly this" type=&peek.'a readonly this

        return this.item.size();
        /// @resolution.member source=this.item receiver=&peek.'a readonly Holder<T#2> type=Readonly<item.Item<T#2>> kind=field target_receiver=&peek.'a readonly Holder<T#2> key=item target=Holder.item target_type=Readonly<item.Item<T#2>>
        /// @resolution.member source=this.item.size receiver=Readonly<Managed<item.Item<T#2>, peek.'a>> type=<item.size.'a>(this: &item.size.'a readonly item.Item<T#2>) => int32 kind=symbol target_receiver=Readonly<Managed<item.Item<T#2>, peek.'a>> target=item.size
        /// @resolution.call source=this.item.size() parameters=() return=int32 regions=(peek.'a) kind=symbol target=item.size receiver=Readonly<Managed<item.Item<T#2>, peek.'a>> adjustments=(Readonly<Managed<item.Item<T#2>, peek.'a>> => direct -> Managed<item.Item<T#2>, peek.'a>, borrow(&peek.'a readonly Managed<item.Item<T#2>, peek.'a>)) instance=item.Item<T#2>.<extension#1>.size
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&peek.'a readonly Holder<T#2>
        /// @resolution.place source=this placement=peek.'a lifetime=peek.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.item placement=peek.'a lifetime="managed" access="readonly"
        /// @resolution.access source=this.item root=this keys=[item]
        /// @generic.instantiation id=item.size<T#2> template=item.size arguments=(T#2) owner=peek

    }
}
"#,
        r#"
/// @diagnostic.error id=field-not-definitely-initialized message="field 'item' is not initialized on every constructor path"
/// @diagnostic.label line=5 column=5 span="item" line_source="item: Item<T>;"
/// @diagnostic.error id=unused-generic-parameter message="generic parameter 'T' is never used"
/// @diagnostic.label line=2 column=19 span="T" line_source="export class Item<T> {}"
/// @diagnostic.help message="declare explicit variance like 'out T' to keep a marker parameter"
"#,
    );
}

#[test]
fn test_read_array_getters_through_a_readonly_receiver() {
    let session = TestSession::single(
        r#"
class Pile<T> {
    items: T[];

    count(&readonly this): isize {
        const item = this.items.at(0);
        return this.items.size;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Pile<in out T> {
    items: T[];

    count(&readonly this): isize {
        const item: &'frame readonly T | undefined = this.items.at<T, "readonly">(0);
        return this.items.size;
    }
}

=== dir ===
class Pile<T> {
/// @generic.template symbol=Pile parameters=(in out T)
/// @type.symbol symbol=Pile type=typeof Pile
/// @definition.class symbol=Pile template=(in out T)
/// @definition.field symbol=Pile.items source="items: T[]" key=items type=T[]
/// @definition.method symbol=Pile.count slot=count type=<Pile.count.'a>(this: &Pile.count.'a readonly this) => isize
/// @type.symbol symbol=Pile.T source=T type=T

    items: T[];
    /// @type.symbol symbol=Pile.items source="items: T[]" type=T[]
    /// @resolution.name source=T target=Pile.T

    count(&readonly this): isize {
    /// @generic.template symbol=Pile.count parent=template#0 parameters=('a)
    /// @type.symbol symbol=Pile.count type=<Pile.count.'a>(this: &Pile.count.'a readonly this) => isize
    /// @type.symbol symbol=Pile.count.this source="&readonly this" type=&Pile.count.'a readonly this

        const item = this.items.at(0);
        /// @type.symbol symbol=Pile.count.item source=item type=&'frame readonly T | undefined
        /// @resolution.pattern source=item kind=binding target=Pile.count.item
        /// @resolution.member source=this.items receiver=&Pile.count.'a readonly Pile<T> type=readonly T[] kind=field target_receiver=&Pile.count.'a readonly Pile<T> key=items target=Pile.items target_type=readonly T[]
        /// @resolution.member source=this.items.at receiver=Readonly<Managed<T[], Pile.count.'a>> type=(this: &'frame readonly T[], isize) => &'frame readonly T | undefined kind=symbol target_receiver=Readonly<Managed<T[], Pile.count.'a>> adjustments=(Readonly<Managed<T[], Pile.count.'a>> => direct -> Managed<T[], Pile.count.'a>, Managed<T[], Pile.count.'a> => direct -> T[]) target=at#2
        /// @resolution.call source=this.items.at(0) parameters=(isize) arguments=(provided(0) as isize) return=&'frame readonly T | undefined regions=("frame" & "local") kind=symbol target=at#2 receiver=Readonly<Managed<T[], Pile.count.'a>> adjustments=(Readonly<Managed<T[], Pile.count.'a>> => direct -> Managed<T[], Pile.count.'a>, Managed<T[], Pile.count.'a> => direct -> T[]) instance="Borrowed<T#5[], 'a#1, A#1>.<extension#5>.at#2"
        /// @resolution.receiver source=this kind=this declaration=Pile type=&Pile.count.'a readonly Pile<T>
        /// @resolution.place source=this placement=Pile.count.'a lifetime=Pile.count.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.items placement=Pile.count.'a lifetime="managed" access="readonly"
        /// @resolution.access source=this.items root=this keys=[items]
        /// @generic.instantiation id="at#2<T, \"readonly\">" template=at#2 arguments=(T, "readonly") owner=Pile.count

        return this.items.size;
        /// @resolution.member source=this.items receiver=&Pile.count.'a readonly Pile<T> type=readonly T[] kind=field target_receiver=&Pile.count.'a readonly Pile<T> key=items target=Pile.items target_type=readonly T[]
        /// @resolution.member source=this.items.size receiver=Readonly<Managed<T[], Pile.count.'a>> type=isize kind=call target="size(parameters=(), arguments=(), return=isize, regions=(Pile.count.'a))"
        /// @resolution.receiver source=this kind=this declaration=Pile type=&Pile.count.'a readonly Pile<T>
        /// @resolution.place source=this placement=Pile.count.'a lifetime=Pile.count.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.items placement=Pile.count.'a lifetime="managed" access="readonly"
        /// @resolution.access source=this.items root=this keys=[items]
        /// @generic.instantiation id=size<T> template=size arguments=(T) owner=Pile.count

    }
}
"#,
        r#"
/// @diagnostic.error id=field-not-definitely-initialized message="field 'items' is not initialized on every constructor path"
/// @diagnostic.label line=3 column=5 span="items" line_source="items: T[];"
/// @diagnostic.error id=receiver-not-assignable message="receiver type 'T[]' is not assignable to the method's 'this' type '&readonly local T[]'"
/// @diagnostic.label line=6 column=22 span="this.items.at(0)" line_source="const item = this.items.at(0);"
"#,
    );
}

#[test]
fn test_pack_an_owned_slice_rest_parameter() {
    let session = TestSession::single(
        r#"
function total(...values: ^[int32]): int32 {
    return values.length.truncate<int32>();
}

function main(): int32 {
    return total(1, 2, 3);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function total(...values: ^[int32]): int32 {
    return values.length.truncate<int32>();
}

function main(): int32 {
    return total(1, 2, 3);
}

=== dir ===
function total(...values: ^[int32]): int32 {
/// @type.symbol symbol=total type=(...^Slice<int32>) => int32
/// @type.symbol symbol=total.values source="...values: ^[int32]" type=^Slice<int32>

    return values.length.truncate<int32>();
    /// @resolution.name source=values target=total.values
    /// @resolution.member source=values.length receiver=^Slice<int32> type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"frame\" & \"local\"))"
    /// @resolution.member source=values.length.truncate receiver=isize type=<Cast.truncate.U: Integer>(this: isize) => Cast.truncate.U kind=symbol target_receiver=isize target=Cast.truncate
    /// @resolution.call source=values.length.truncate<int32>() parameters=() return=int32 kind=symbol target=Cast.truncate receiver=isize instance=Cast<isize>.truncate<int32>
    /// @resolution.place source=values placement="local" lifetime="frame" access="mutable"
    /// @resolution.access source=values root=total.values
    /// @generic.instantiation id="Cast.truncate<isize, int32>" template=Cast.truncate arguments=(isize, int32)
    /// @generic.instantiation id=Cast.truncate<isize> template=Cast.truncate arguments=(isize)
    /// @generic.instantiation id=length<int32> template=length arguments=(int32)

}

function main(): int32 {
/// @type.symbol symbol=main type=() => int32

    return total(1, 2, 3);
    /// @resolution.name source=total target=total
    /// @resolution.call source="total(1, 2, 3)" parameters=(^Slice<int32>) arguments=(rest(provided(1) as int32, provided(2) as int32, provided(3) as int32) as int32) return=int32 kind=symbol target=total

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_pack_a_borrowed_slice_rest_parameter() {
    let session = TestSession::single(
        r#"
function total(...values: &readonly [int32]): int32 {
    return values.length.truncate<int32>();
}

function main(): int32 {
    return total(1, 2, 3);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function total<'a>(...values: &'a readonly [int32]): int32 {
    return values.length.truncate<int32>();
}

function main(): int32 {
    return total(1, 2, 3);
}

=== dir ===
function total(...values: &readonly [int32]): int32 {
/// @generic.template symbol=total parameters=('a)
/// @type.symbol symbol=total type=<total.'a>(...&total.'a readonly Slice<int32>) => int32
/// @type.symbol symbol=total.values source="...values: &readonly [int32]" type=&total.'a readonly Slice<int32>

    return values.length.truncate<int32>();
    /// @resolution.name source=values target=total.values
    /// @resolution.member source=values.length receiver=&total.'a readonly Slice<int32> type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(total.'a))"
    /// @resolution.member source=values.length.truncate receiver=isize type=<Cast.truncate.U: Integer>(this: isize) => Cast.truncate.U kind=symbol target_receiver=isize target=Cast.truncate
    /// @resolution.call source=values.length.truncate<int32>() parameters=() return=int32 kind=symbol target=Cast.truncate receiver=isize instance=Cast<isize>.truncate<int32>
    /// @resolution.place source=values placement=total.'a lifetime=total.'a access="readonly"
    /// @resolution.access source=values root=total.values
    /// @generic.instantiation id="Cast.truncate<isize, int32>" template=Cast.truncate arguments=(isize, int32)
    /// @generic.instantiation id=Cast.truncate<isize> template=Cast.truncate arguments=(isize)
    /// @generic.instantiation id=length<int32> template=length arguments=(int32)

}

function main(): int32 {
/// @type.symbol symbol=main type=() => int32

    return total(1, 2, 3);
    /// @resolution.name source=total target=total
    /// @resolution.call source="total(1, 2, 3)" parameters=(&'frame readonly Slice<int32>) arguments=(rest(provided(1) as int32, provided(2) as int32, provided(3) as int32) as int32) return=int32 regions=("frame") kind=symbol target=total

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_read_length_on_a_rest_parameter() {
    let session = TestSession::single(
        r#"
function total(...values: int32[]): int32 {
    return values.length.truncate<int32>();
}

function main(): int32 {
    return total(1, 2, 3);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function total(...values: int32[]): int32 {
    return values.length.truncate<int32>();
}

function main(): int32 {
    return total(1, 2, 3);
}

=== dir ===
function total(...values: int32[]): int32 {
/// @type.symbol symbol=total type=(...int32[]) => int32
/// @type.symbol symbol=total.values source="...values: int32[]" type=int32[]

    return values.length.truncate<int32>();
    /// @resolution.name source=values target=total.values
    /// @resolution.member source=values.length receiver=int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"managed\" & \"local\"))"
    /// @resolution.member source=values.length.truncate receiver=isize type=<Cast.truncate.U: Integer>(this: isize) => Cast.truncate.U kind=symbol target_receiver=isize target=Cast.truncate
    /// @resolution.call source=values.length.truncate<int32>() parameters=() return=int32 kind=symbol target=Cast.truncate receiver=isize instance=Cast<isize>.truncate<int32>
    /// @resolution.place source=values placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=values root=total.values
    /// @generic.instantiation id="Cast.truncate<isize, int32>" template=Cast.truncate arguments=(isize, int32)
    /// @generic.instantiation id=Cast.truncate<isize> template=Cast.truncate arguments=(isize)
    /// @generic.instantiation id=length<int32> template=length arguments=(int32)

}

function main(): int32 {
/// @type.symbol symbol=main type=() => int32

    return total(1, 2, 3);
    /// @resolution.name source=total target=total
    /// @resolution.call source="total(1, 2, 3)" parameters=(int32[]) arguments=(rest(provided(1) as int32, provided(2) as int32, provided(3) as int32) pack=arrayFromOwnedSlice as int32) return=int32 kind=symbol target=total
    /// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_call_slice_extension_members() {
    let session = TestSession::single(
        r#"
import { Slice } from "destack:collections";

function first<T>(values: Slice<T>): Slice<T> {
    return values.subslice(0, 1);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Slice } from "destack:collections";

function first<T>(values: Slice<T>): Slice<T> {
    return values.subslice<T, "mutable">(0, 1);
}

=== dir ===
import { Slice } from "destack:collections";

function first<T>(values: Slice<T>): Slice<T> {
/// @generic.template symbol=first parameters=(T)
/// @type.symbol symbol=first type=<T>(Slice<T>) => Slice<T>
/// @type.symbol symbol=first.T source=T type=T
/// @type.symbol symbol=first.values source="values: Slice<T>" type=Slice<T>
/// @resolution.name source=Slice target=Slice
/// @resolution.name source=T target=first.T
/// @resolution.name source=Slice target=Slice
/// @resolution.name source=T target=first.T

    return values.subslice(0, 1);
    /// @resolution.name source=values target=first.values
    /// @resolution.member source=values.subslice receiver=Slice<T> type=<const subslice.A: Access = "readonly", subslice.'a>(this: WithAccess<&subslice.'a Slice<T>, subslice.A>, usize, usize) => WithAccess<&subslice.'a Slice<T>, subslice.A> kind=symbol target_receiver=Slice<T> target=subslice
    /// @resolution.call source="values.subslice(0, 1)" parameters=(usize, usize) arguments=(provided(0) as usize, provided(1) as usize) return=WithAccess<Borrowed<Slice<T>, "managed" & "local", "mutable">, "mutable"> regions=("managed" & "local") kind=symbol target=subslice receiver=Slice<T> adjustments=(borrow(Borrowed<Slice<T>, "managed" & "local", "mutable">)) instance="Slice<T>.<extension#1>.subslice<\"mutable\">"
    /// @resolution.place source=values placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=values root=first.values
    /// @generic.instantiation id="subslice<T, \"mutable\">" template=subslice arguments=(T, "mutable") owner=first
    /// @generic.instantiation id=subslice<T> template=subslice arguments=(T) owner=first

}
"#,
        r#"
/// @diagnostic.error id=return-not-assignable message="type 'WithAccess<&local Slice<T>, \"mutable\">' is not assignable to the declared result type 'Slice<T>'"
/// @diagnostic.label line=5 column=12 span="values.subslice(0, 1)" line_source="return values.subslice(0, 1);"
/// @diagnostic.note message="'WithAccess<&local Slice<T>, \"mutable\">' reduces to '&local Slice<T>'"
"#,
    );
}

#[test]
fn test_keep_an_unplaced_place_projection_opaque() {
    let session = TestSession::single(
        r#"
class Free {}

type FreePlace = PlaceOf<Free>;

declare const freePlace: FreePlace;

freePlace satisfies "local";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Free {}

type FreePlace = PlaceOf<Free>;

declare const freePlace: FreePlace;

freePlace satisfies "local";

=== dir ===
class Free {}
/// @type.symbol symbol=Free source="class Free {}" type=typeof Free
/// @definition.class symbol=Free source="class Free {}"

type FreePlace = PlaceOf<Free>;
/// @type.symbol symbol=FreePlace source="type FreePlace = PlaceOf<Free>" type=PlaceOf<Free>
/// @definition.type symbol=FreePlace source="type FreePlace = PlaceOf<Free>" value=PlaceOf<Free>
/// @resolution.name source=PlaceOf target=PlaceOf
/// @resolution.name source=Free target=Free

declare const freePlace: FreePlace;
/// @type.symbol symbol=freePlace source=freePlace type=FreePlace
/// @resolution.pattern source=freePlace kind=binding target=freePlace
/// @resolution.name source=FreePlace target=FreePlace

freePlace satisfies "local";
/// @resolution.name source=freePlace target=freePlace
/// @resolution.place source=freePlace placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=freePlace root=freePlace
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'FreePlace' does not satisfy '\"local\"'"
/// @diagnostic.label line=8 column=1 span="freePlace" line_source="freePlace satisfies \"local\";"
/// @diagnostic.note message="'FreePlace' reduces to 'PlaceOf<Free>'"
"#,
    );
}

#[test]
fn test_reduce_conditionals_on_projected_places_per_instantiation() {
    let session = TestSession::single(
        r#"
shared class Channel {}
local class Queue {}

type Transport<T> = PlaceOf<T> extends "shared" ? Channel : Queue;

declare const sharedTransport: Transport<Channel>;
declare const localTransport: Transport<Queue>;

sharedTransport satisfies Channel;
localTransport satisfies Queue;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
shared class Channel {}
local class Queue {}

type Transport<T> = PlaceOf<T> extends "shared" ? Channel : Queue;

declare const sharedTransport: Transport<Channel>;
declare const localTransport: Transport<Queue>;

sharedTransport satisfies Channel;
localTransport satisfies Queue;

=== dir ===
shared class Channel {}
/// @type.symbol symbol=Channel source="shared class Channel {}" type=typeof Channel
/// @definition.class symbol=Channel source="shared class Channel {}"

local class Queue {}
/// @type.symbol symbol=Queue source="local class Queue {}" type=typeof Queue
/// @definition.class symbol=Queue source="local class Queue {}"

type Transport<T> = PlaceOf<T> extends "shared" ? Channel : Queue;
/// @generic.template symbol=Transport parameters=(T)
/// @type.symbol symbol=Transport source="type Transport<T> = PlaceOf<T> extends \"shared\" ? Channel : Queue" type=PlaceOf<T> extends "shared" ? Channel : Queue
/// @definition.type symbol=Transport source="type Transport<T> = PlaceOf<T> extends \"shared\" ? Channel : Queue" template=(T) value=PlaceOf<T> extends "shared" ? Channel : Queue
/// @type.symbol symbol=Transport.T source=T type=T
/// @resolution.name source=PlaceOf target=PlaceOf
/// @resolution.name source=T target=Transport.T
/// @resolution.name source=Channel target=Channel
/// @resolution.name source=Queue target=Queue

declare const sharedTransport: Transport<Channel>;
/// @type.symbol symbol=sharedTransport source=sharedTransport type=Transport<Channel>
/// @resolution.pattern source=sharedTransport kind=binding target=sharedTransport
/// @resolution.name source=Transport target=Transport
/// @resolution.name source=Channel target=Channel

declare const localTransport: Transport<Queue>;
/// @type.symbol symbol=localTransport source=localTransport type=Transport<Queue>
/// @resolution.pattern source=localTransport kind=binding target=localTransport
/// @resolution.name source=Transport target=Transport
/// @resolution.name source=Queue target=Queue

sharedTransport satisfies Channel;
/// @resolution.name source=sharedTransport target=sharedTransport
/// @resolution.place source=sharedTransport placement="shared" lifetime="managed" access="mutable"
/// @resolution.access source=sharedTransport root=sharedTransport
/// @resolution.name source=Channel target=Channel

localTransport satisfies Queue;
/// @resolution.name source=localTransport target=localTransport
/// @resolution.place source=localTransport placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=localTransport root=localTransport
/// @resolution.name source=Queue target=Queue
"#,
        r#"
"#,
    );
}

#[test]
fn test_reject_placement_on_owned_values() {
    let session = TestSession::single(
        r#"
class User {}

declare const localUser: local ^User;
declare const sharedUser: shared ^User;
type SharedOwned = shared ^User;
type OwnedShared = ^shared User;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare const localUser: ^User;
declare const sharedUser: ^User;
type SharedOwned = shared ^User;
type OwnedShared = ^shared User;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare const localUser: local ^User;
/// @type.symbol symbol=localUser source=localUser type=^User
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: shared ^User;
/// @type.symbol symbol=sharedUser source=sharedUser type=^User
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

type SharedOwned = shared ^User;
/// @type.symbol symbol=SharedOwned source="type SharedOwned = shared ^User" type=^User
/// @definition.type symbol=SharedOwned source="type SharedOwned = shared ^User" value=^User
/// @resolution.name source=User target=User

type OwnedShared = ^shared User;
/// @type.symbol symbol=OwnedShared source="type OwnedShared = ^shared User" type=^User
/// @definition.type symbol=OwnedShared source="type OwnedShared = ^shared User" value=^User
/// @resolution.name source=User target=User
"#,
        r#"
/// @diagnostic.error id=placement-on-owned message="an owned value lives in its container's space and takes no placement"
/// @diagnostic.label line=4 column=26 span="local" line_source="declare const localUser: local ^User;"
/// @diagnostic.error id=placement-on-owned message="an owned value lives in its container's space and takes no placement"
/// @diagnostic.label line=5 column=27 span="shared" line_source="declare const sharedUser: shared ^User;"
/// @diagnostic.error id=placement-on-owned message="an owned value lives in its container's space and takes no placement"
/// @diagnostic.label line=6 column=20 span="shared" line_source="type SharedOwned = shared ^User;"
/// @diagnostic.error id=placement-on-owned message="an owned value lives in its container's space and takes no placement"
/// @diagnostic.label line=7 column=20 span="^" line_source="type OwnedShared = ^shared User;"
"#,
    );
}

#[test]
fn test_compose_a_written_place_with_an_opaque_region_borrow() {
    let session = TestSession::single(
        r#"
struct Cell { value: int32; }

function read<R: Region>(borrow: local Borrowed<Cell, R, "readonly">): int32 {
    return borrow.value;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cell {
    value: int32;
}

function read<R: Region>(borrow: Borrowed<Cell, R & "local", "readonly">): int32 {
    return borrow.value;
}

=== dir ===
struct Cell { value: int32; }
/// @type.symbol symbol=Cell source="struct Cell { value: int32; }" type=Cell
/// @definition.struct symbol=Cell source="struct Cell { value: int32; }"
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32
/// @type.symbol symbol=Cell.value source="value: int32" type=int32

function read<R: Region>(borrow: local Borrowed<Cell, R, "readonly">): int32 {
/// @generic.template symbol=read parameters=(R: Region)
/// @type.symbol symbol=read type=<R>(Borrowed<Cell, R & "local", "readonly">) => int32
/// @type.symbol symbol=read.R source="R: Region" type=R
/// @resolution.name source=Region target=Region
/// @type.symbol symbol=read.borrow source="borrow: local Borrowed<Cell, R, \"readonly\">" type=Borrowed<Cell, R & "local", "readonly">
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Cell target=Cell
/// @resolution.name source=R target=read.R

    return borrow.value;
    /// @resolution.name source=borrow target=read.borrow
    /// @resolution.member source=borrow.value receiver=Borrowed<Cell, R & "local", "readonly"> type=int32 kind=field target_receiver=Borrowed<Cell, R & "local", "readonly"> key=value target=Cell.value target_type=int32
    /// @resolution.place source=borrow placement="local" lifetime=R access="readonly"
    /// @resolution.access source=borrow root=read.borrow
    /// @resolution.place source=borrow.value placement="local" lifetime=R access="readonly"
    /// @resolution.access source=borrow.value root=read.borrow keys=[value]

}
"#,
    );
}

/// Call a struct method through its elided receiver region on local and shared values.
#[test]
fn test_call_a_struct_method_on_local_and_shared_receivers() {
    let session = TestSession::single(
        r#"
struct DrawnPoint {
    x: int32;
    draw(): int32 { return this.x; }
}
declare shared const remote: ^DrawnPoint;
const near = DrawnPoint { x: 1 };
const a = near.draw();
const b = remote.draw();
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct DrawnPoint {
    x: int32;
    draw(): int32 {
        return this.x;
    }
}
declare shared const remote: DrawnPoint;
const near: DrawnPoint = DrawnPoint { x: 1 };
const a: int32 = near.draw();
const b: int32 = remote.draw();

=== dir ===
struct DrawnPoint {
/// @type.symbol symbol=DrawnPoint type=DrawnPoint
/// @definition.struct symbol=DrawnPoint
/// @definition.field symbol=DrawnPoint.x source="x: int32" key=x type=int32
/// @definition.method symbol=DrawnPoint.draw source="draw(): int32 { return this.x; }" slot=draw type=<DrawnPoint.draw.'a>(this: &DrawnPoint.draw.'a readonly DrawnPoint) => int32

    x: int32;
    /// @type.symbol symbol=DrawnPoint.x source="x: int32" type=int32

    draw(): int32 { return this.x; }
    /// @generic.template symbol=DrawnPoint.draw parameters=('a)
    /// @type.symbol symbol=DrawnPoint.draw source="draw(): int32 { return this.x; }" type=<DrawnPoint.draw.'a>(this: &DrawnPoint.draw.'a readonly DrawnPoint) => int32
    /// @type.symbol symbol=DrawnPoint.draw.this type=&DrawnPoint.draw.'a readonly DrawnPoint
    /// @type.node source=this type=&DrawnPoint.draw.'a readonly DrawnPoint
    /// @type.node source=this.x type=int32
    /// @resolution.member source=this.x receiver=&DrawnPoint.draw.'a readonly DrawnPoint type=int32 kind=field target_receiver=&DrawnPoint.draw.'a readonly DrawnPoint key=x target=DrawnPoint.x target_type=int32
    /// @resolution.receiver source=this kind=this declaration=DrawnPoint type=&DrawnPoint.draw.'a readonly DrawnPoint
    /// @resolution.place source=this placement=DrawnPoint.draw.'a lifetime=DrawnPoint.draw.'a access="readonly"
    /// @resolution.access source=this root=this
    /// @resolution.place source=this.x placement=DrawnPoint.draw.'a lifetime=DrawnPoint.draw.'a access="readonly"
    /// @resolution.access source=this.x root=this keys=[x]

}
declare shared const remote: ^DrawnPoint;
/// @type.symbol symbol=remote source=remote type=DrawnPoint
/// @resolution.pattern source=remote kind=binding target=remote
/// @resolution.name source=DrawnPoint target=DrawnPoint

const near = DrawnPoint { x: 1 };
/// @type.symbol symbol=near source=near type=DrawnPoint
/// @resolution.pattern source=near kind=binding target=near
/// @type.node source="DrawnPoint { x: 1 }" type=DrawnPoint
/// @resolution.name source=DrawnPoint target=DrawnPoint
/// @type.node source=1 type=1

const a = near.draw();
/// @type.symbol symbol=a source=a type=int32
/// @resolution.pattern source=a kind=binding target=a
/// @type.node source=near type=DrawnPoint
/// @type.node source=near.draw type=<DrawnPoint.draw.'a>(this: &DrawnPoint.draw.'a readonly DrawnPoint) => int32
/// @type.node source=near.draw() type=int32
/// @resolution.name source=near target=near
/// @resolution.member source=near.draw receiver=DrawnPoint type=<DrawnPoint.draw.'a>(this: &DrawnPoint.draw.'a readonly DrawnPoint) => int32 kind=symbol target_receiver=DrawnPoint target=DrawnPoint.draw
/// @resolution.call source=near.draw() parameters=() return=int32 regions=("static" & "constant") kind=symbol target=DrawnPoint.draw receiver=DrawnPoint adjustments=(borrow(&'static readonly constant DrawnPoint))
/// @resolution.place source=near placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=near root=near

const b = remote.draw();
/// @type.symbol symbol=b source=b type=int32
/// @resolution.pattern source=b kind=binding target=b
/// @type.node source=remote type=DrawnPoint
/// @type.node source=remote.draw type=<DrawnPoint.draw.'a>(this: &DrawnPoint.draw.'a readonly DrawnPoint) => int32
/// @type.node source=remote.draw() type=int32
/// @resolution.name source=remote target=remote
/// @resolution.member source=remote.draw receiver=DrawnPoint type=<DrawnPoint.draw.'a>(this: &DrawnPoint.draw.'a readonly DrawnPoint) => int32 kind=symbol target_receiver=DrawnPoint target=DrawnPoint.draw
/// @resolution.call source=remote.draw() parameters=() return=int32 regions=("static" & "shared") kind=symbol target=DrawnPoint.draw receiver=DrawnPoint adjustments=(borrow(&'static readonly shared DrawnPoint))
/// @resolution.place source=remote placement="shared" lifetime="static" access="readonly"
/// @resolution.access source=remote root=remote
"#,
    );
}

/// Call a callback over borrows from both spaces through its late-bound region.
#[test]
fn test_call_a_callback_over_borrows_from_both_spaces() {
    let session = TestSession::single(
        r#"
struct Item { x: int32; }
declare shared const remote: ^Item;
function apply(f: (item: &readonly Item) => int32): int32 {
    const near = Item { x: 1 };
    return f(&readonly near) + f(&readonly remote);
}
const total = apply((item) => item.x);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Item {
    x: int32;
}
declare shared const remote: Item;
function apply(f: (item: &'a readonly Item) => int32): int32 {
    const near: Item = Item { x: 1 };
    return f(&readonly near) + f(&readonly remote);
}
const total: int32 = apply((item: &'a readonly Item): int32 => item.x);

=== dir ===
struct Item { x: int32; }
/// @type.symbol symbol=Item source="struct Item { x: int32; }" type=Item
/// @definition.struct symbol=Item source="struct Item { x: int32; }"
/// @definition.field symbol=Item.x source="x: int32" key=x type=int32
/// @type.symbol symbol=Item.x source="x: int32" type=int32

declare shared const remote: ^Item;
/// @type.symbol symbol=remote source=remote type=Item
/// @resolution.pattern source=remote kind=binding target=remote
/// @resolution.name source=Item target=Item

function apply(f: (item: &readonly Item) => int32): int32 {
/// @type.symbol symbol=apply type=(Function<(&type_expression.'a readonly Item,), int32>) => int32
/// @type.symbol symbol=apply.f source="f: (item: &readonly Item) => int32" type=Function<(&type_expression.'a readonly Item,), int32>
/// @generic.template source=type_expression parameters=('a)
/// @type.symbol symbol=apply.item source="item: &readonly Item" type=&type_expression.'a readonly Item
/// @resolution.name source=Item target=Item

    const near = Item { x: 1 };
    /// @type.symbol symbol=apply.near source=near type=Item
    /// @resolution.pattern source=near kind=binding target=apply.near
    /// @type.node source="Item { x: 1 }" type=Item
    /// @resolution.name source=Item target=Item
    /// @type.node source=1 type=1

    return f(&readonly near) + f(&readonly remote);
    /// @type.node source="f(&readonly near) + f(&readonly remote)" type=int32
    /// @type.node source="f(&readonly near)" type=int32
    /// @type.node source=f type=Function<(&type_expression.'a readonly Item,), int32>
    /// @resolution.name source=f target=apply.f
    /// @resolution.call source="f(&readonly near)" parameters=(&'frame readonly Item) arguments=(provided(&readonly near) as &'frame readonly Item) return=int32 regions=("frame" & "local") kind=expression target=expression
    /// @resolution.operator source="f(&readonly near) + f(&readonly remote)" type=int32 operator="+" kind=builtin operands=[f(&readonly near) as int32 families=(integer), f(&readonly remote) as int32 families=(integer)]
    /// @resolution.place source=f placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=f root=apply.f
    /// @type.node source="&readonly near" type=&'frame readonly Item
    /// @type.node source=near type=Item
    /// @resolution.name source=near target=apply.near
    /// @resolution.place source=near placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=near root=apply.near
    /// @type.node source="f(&readonly remote)" type=int32
    /// @type.node source=f type=Function<(&type_expression.'a readonly Item,), int32>
    /// @resolution.name source=f target=apply.f
    /// @resolution.call source="f(&readonly remote)" parameters=(&'static readonly shared Item) arguments=(provided(&readonly remote) as &'static readonly shared Item) return=int32 regions=("static" & "shared") kind=expression target=expression
    /// @resolution.place source=f placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=f root=apply.f
    /// @type.node source="&readonly remote" type=&'static readonly shared Item
    /// @type.node source=remote type=Item
    /// @resolution.name source=remote target=remote
    /// @resolution.place source=remote placement="shared" lifetime="static" access="readonly"
    /// @resolution.access source=remote root=remote

}
const total = apply((item) => item.x);
/// @type.symbol symbol=total source=total type=int32
/// @resolution.pattern source=total kind=binding target=total
/// @type.node source="apply((item) => item.x)" type=int32
/// @type.node source=apply type=(Function<(&type_expression.'a readonly Item,), int32>) => int32
/// @resolution.name source=apply target=apply
/// @resolution.call source="apply((item) => item.x)" parameters=(Function<(&type_expression.'a readonly Item,), int32>) arguments=(provided((item) => item.x) as Function<(&type_expression.'a readonly Item,), int32>) return=int32 kind=symbol target=apply
/// @type.symbol symbol=symbol9 source="(item) => item.x" type=Function<(&type_expression.'a readonly Item,), int32, "readonly">
/// @type.node source="(item) => item.x" type=Function<(&type_expression.'a readonly Item,), int32, "readonly">
/// @type.symbol symbol=symbol9.item source=item type=&type_expression.'a readonly Item
/// @type.node source=item type=&type_expression.'a readonly Item
/// @type.node source=item.x type=int32
/// @resolution.name source=item target=symbol9.item
/// @resolution.member source=item.x receiver=&type_expression.'a readonly Item type=int32 kind=field target_receiver=&type_expression.'a readonly Item key=x target=Item.x target_type=int32
/// @resolution.place source=item placement=type_expression.'a lifetime=type_expression.'a access="readonly"
/// @resolution.access source=item root=symbol9.item
/// @resolution.place source=item.x placement=type_expression.'a lifetime=type_expression.'a access="readonly"
/// @resolution.access source=item.x root=symbol9.item keys=[x]
"#,
    );
}

/// Reject a shared class handle where a bare class parameter assumes a local one.
#[test]
fn test_reject_a_shared_handle_for_a_bare_class_parameter() {
    let session = TestSession::single(
        r#"
class Player { score: int32 = 0; }
declare shared const remote: Player;
const near = new Player();
function tick(player: Player): int32 { return player.score; }
const a = tick(near);
const b = tick(remote);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Player {
    score: int32 = 0;
}
declare shared const remote: shared Player;
const near: Player = new Player();
function tick(player: Player): int32 {
    return player.score;
}
const a: int32 = tick(near);
const b: int32 = tick(remote);

=== dir ===
class Player { score: int32 = 0; }
/// @type.symbol symbol=Player source="class Player { score: int32 = 0; }" type=typeof Player
/// @definition.class symbol=Player source="class Player { score: int32 = 0; }"
/// @definition.field symbol=Player.score source="score: int32 = 0" key=score type=int32
/// @type.symbol symbol=Player.score source="score: int32 = 0" type=int32
/// @type.node source=0 type=0

declare shared const remote: Player;
/// @type.symbol symbol=remote source=remote type=shared Player
/// @resolution.pattern source=remote kind=binding target=remote
/// @resolution.name source=Player target=Player

const near = new Player();
/// @type.symbol symbol=near source=near type=Player
/// @resolution.pattern source=near kind=binding target=near
/// @type.node source="new Player()" type=Player
/// @resolution.construct source="new Player()" parameters=() return=Player kind=class target=Player constructor=default
/// @type.node source=Player type=typeof Player
/// @resolution.name source=Player target=Player

function tick(player: Player): int32 { return player.score; }
/// @type.symbol symbol=tick source="function tick(player: Player): int32 { return player.score; }" type=(Player) => int32
/// @type.symbol symbol=tick.player source="player: Player" type=Player
/// @resolution.name source=Player target=Player
/// @type.node source=player type=Player
/// @type.node source=player.score type=int32
/// @resolution.name source=player target=tick.player
/// @resolution.member source=player.score receiver=Player type=int32 kind=field target_receiver=Player key=score target=Player.score target_type=int32
/// @resolution.place source=player placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=player root=tick.player
/// @resolution.place source=player.score placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=player.score root=tick.player keys=[score]

const a = tick(near);
/// @type.symbol symbol=a source=a type=int32
/// @resolution.pattern source=a kind=binding target=a
/// @type.node source=tick type=(Player) => int32
/// @type.node source=tick(near) type=int32
/// @resolution.name source=tick target=tick
/// @resolution.call source=tick(near) parameters=(Player) arguments=(provided(near) as Player) return=int32 kind=symbol target=tick
/// @type.node source=near type=Player
/// @resolution.name source=near target=near
/// @resolution.place source=near placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=near root=near

const b = tick(remote);
/// @type.symbol symbol=b source=b type=int32
/// @resolution.pattern source=b kind=binding target=b
/// @type.node source=tick type=(Player) => int32
/// @type.node source=tick(remote) type=int32
/// @resolution.name source=tick target=tick
/// @resolution.call source=tick(remote) parameters=(Player) arguments=(provided(remote) as Player) return=int32 kind=symbol target=tick
/// @type.node source=remote type=shared Player
/// @resolution.name source=remote target=remote
/// @resolution.place source=remote placement="shared" lifetime="managed" access="mutable"
/// @resolution.access source=remote root=remote
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'shared Player' is not assignable to parameter of type 'Player'"
/// @diagnostic.label line=7 column=16 span="remote" line_source="const b = tick(remote);"
/// @diagnostic.related line=7 column=11 span="tick(remote)" line_source="const b = tick(remote);" message="in this call"
"#,
    );
}

/// Call a borrowed class parameter with local and shared handles through its region.
#[test]
fn test_call_a_borrowed_class_parameter_from_both_spaces() {
    let session = TestSession::single(
        r#"
class Player { score: int32 = 0; }
declare shared const remote: Player;
const near = new Player();
function tick(player: &readonly Player): int32 { return player.score; }
const a = tick(&readonly near);
const b = tick(&readonly remote);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Player {
    score: int32 = 0;
}
declare shared const remote: shared Player;
const near: Player = new Player();
function tick<'a>(player: &'a readonly Player): int32 {
    return player.score;
}
const a: int32 = tick(&readonly near);
const b: int32 = tick(&readonly remote);

=== dir ===
class Player { score: int32 = 0; }
/// @type.symbol symbol=Player source="class Player { score: int32 = 0; }" type=typeof Player
/// @definition.class symbol=Player source="class Player { score: int32 = 0; }"
/// @definition.field symbol=Player.score source="score: int32 = 0" key=score type=int32
/// @type.symbol symbol=Player.score source="score: int32 = 0" type=int32
/// @type.node source=0 type=0

declare shared const remote: Player;
/// @type.symbol symbol=remote source=remote type=shared Player
/// @resolution.pattern source=remote kind=binding target=remote
/// @resolution.name source=Player target=Player

const near = new Player();
/// @type.symbol symbol=near source=near type=Player
/// @resolution.pattern source=near kind=binding target=near
/// @type.node source="new Player()" type=Player
/// @resolution.construct source="new Player()" parameters=() return=Player kind=class target=Player constructor=default
/// @type.node source=Player type=typeof Player
/// @resolution.name source=Player target=Player

function tick(player: &readonly Player): int32 { return player.score; }
/// @generic.template symbol=tick parameters=('a)
/// @type.symbol symbol=tick source="function tick(player: &readonly Player): int32 { return player.score; }" type=<tick.'a>(&tick.'a readonly Player) => int32
/// @type.symbol symbol=tick.player source="player: &readonly Player" type=&tick.'a readonly Player
/// @resolution.name source=Player target=Player
/// @type.node source=player type=&tick.'a readonly Player
/// @type.node source=player.score type=int32
/// @resolution.name source=player target=tick.player
/// @resolution.member source=player.score receiver=&tick.'a readonly Player type=int32 kind=field target_receiver=&tick.'a readonly Player key=score target=Player.score target_type=int32
/// @resolution.place source=player placement=tick.'a lifetime=tick.'a access="readonly"
/// @resolution.access source=player root=tick.player
/// @resolution.place source=player.score placement=tick.'a lifetime=tick.'a access="readonly"
/// @resolution.access source=player.score root=tick.player keys=[score]

const a = tick(&readonly near);
/// @type.symbol symbol=a source=a type=int32
/// @resolution.pattern source=a kind=binding target=a
/// @type.node source="tick(&readonly near)" type=int32
/// @type.node source=tick type=(Borrowed<Player, "managed" & "local", "readonly">) => int32
/// @resolution.name source=tick target=tick
/// @resolution.call source="tick(&readonly near)" parameters=(Borrowed<Player, "managed" & "local", "readonly">) arguments=(provided(&readonly near) as Borrowed<Player, "managed" & "local", "readonly">) return=int32 regions=("managed" & "local") kind=symbol target=tick
/// @type.node source="&readonly near" type=Borrowed<Player, "managed" & "local", "readonly">
/// @type.node source=near type=Player
/// @resolution.name source=near target=near
/// @resolution.place source=near placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=near root=near

const b = tick(&readonly remote);
/// @type.symbol symbol=b source=b type=int32
/// @resolution.pattern source=b kind=binding target=b
/// @type.node source="tick(&readonly remote)" type=int32
/// @type.node source=tick type=(Borrowed<Player, "managed" & "shared", "readonly">) => int32
/// @resolution.name source=tick target=tick
/// @resolution.call source="tick(&readonly remote)" parameters=(Borrowed<Player, "managed" & "shared", "readonly">) arguments=(provided(&readonly remote) as Borrowed<Player, "managed" & "shared", "readonly">) return=int32 regions=("managed" & "shared") kind=symbol target=tick
/// @type.node source="&readonly remote" type=Borrowed<Player, "managed" & "shared", "readonly">
/// @type.node source=remote type=shared Player
/// @resolution.name source=remote target=remote
/// @resolution.place source=remote placement="shared" lifetime="managed" access="mutable"
/// @resolution.access source=remote root=remote
"#,
    );
}

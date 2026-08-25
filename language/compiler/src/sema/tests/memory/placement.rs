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
/// @type.symbol symbol=User source="class User {}" type=User
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
/// @type.symbol symbol=BorrowedLocal source="type BorrowedLocal<'a> = &'a readonly local User" type=&'a#2 readonly local User
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
/// @type.symbol symbol=User source="class User {}" type=User
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
/// @resolution.place source=sharedUser placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=sharedUser root=sharedUser

const sharedFromLocal: shared User = localUser;
/// @type.symbol symbol=sharedFromLocal source=sharedFromLocal type=shared User
/// @resolution.pattern source=sharedFromLocal kind=binding target=sharedFromLocal
/// @resolution.name source=User target=User
/// @resolution.name source=localUser target=localUser
/// @resolution.place source=localUser placement="local" lifetime="static" access="exclusive"
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

declare const choicePlace: "local" | "shared";

choicePlace satisfies "local" | "shared";

=== dir ===
local class User {}
/// @type.symbol symbol=User source="local class User {}" type=User
/// @definition.class symbol=User source="local class User {}"

shared class Team {}
/// @type.symbol symbol=Team source="shared class Team {}" type=Team
/// @definition.class symbol=Team source="shared class Team {}"

type ChoicePlace = PlaceOf<User | Team>;
/// @type.symbol symbol=ChoicePlace source="type ChoicePlace = PlaceOf<User | Team>" type="local" | "shared"
/// @definition.type symbol=ChoicePlace source="type ChoicePlace = PlaceOf<User | Team>" value="local" | "shared"
/// @resolution.name source=PlaceOf target=PlaceOf
/// @resolution.name source=User target=User
/// @resolution.name source=Team target=Team

declare const choicePlace: ChoicePlace;
/// @type.symbol symbol=choicePlace source=choicePlace type="local" | "shared"
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

declare const localPlace: "local";
declare const sharedPlace: "shared";

localPlace satisfies "local";
sharedPlace satisfies "shared";

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

type LocalPlace = PlaceOf<local User>;
/// @type.symbol symbol=LocalPlace source="type LocalPlace = PlaceOf<local User>" type="local"
/// @definition.type symbol=LocalPlace source="type LocalPlace = PlaceOf<local User>" value="local"
/// @resolution.name source=PlaceOf target=PlaceOf
/// @resolution.name source=User target=User

type SharedPlace = PlaceOf<shared User>;
/// @type.symbol symbol=SharedPlace source="type SharedPlace = PlaceOf<shared User>" type="shared"
/// @definition.type symbol=SharedPlace source="type SharedPlace = PlaceOf<shared User>" value="shared"
/// @resolution.name source=PlaceOf target=PlaceOf
/// @resolution.name source=User target=User

declare const localPlace: LocalPlace;
/// @type.symbol symbol=localPlace source=localPlace type="local"
/// @resolution.pattern source=localPlace kind=binding target=localPlace
/// @resolution.name source=LocalPlace target=LocalPlace

declare const sharedPlace: SharedPlace;
/// @type.symbol symbol=sharedPlace source=sharedPlace type="shared"
/// @resolution.pattern source=sharedPlace kind=binding target=sharedPlace
/// @resolution.name source=SharedPlace target=SharedPlace

localPlace satisfies "local";
/// @resolution.name source=localPlace target=localPlace
/// @resolution.place source=localPlace placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=localPlace root=localPlace

sharedPlace satisfies "shared";
/// @resolution.name source=sharedPlace target=sharedPlace
/// @resolution.place source=sharedPlace placement="local" lifetime="static" access="exclusive"
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

    clear(&exclusive this): void {}
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

    clear(&exclusive this): void {}
}

const object: Buffer = Buffer;
const made: Buffer = new Buffer();
const buffer: Buffer = Buffer.make();

=== dir ===
class Buffer {}
/// @type.symbol symbol=Buffer source="class Buffer {}" type=Buffer
/// @definition.class symbol=Buffer source="class Buffer {}"

extension of Buffer implements Default {
/// @definition.extension symbol=<module>#2 form=local target=Buffer
/// @definition.implements symbol=<module>#2 source=Default target=Default
/// @definition.method symbol=default slot=default static=true type=() => Owned<this>
/// @definition.conformance symbol=<module>#2 member=default requirement=Default.default
/// @resolution.name source=Buffer target=Buffer
/// @resolution.name source=Default target=Default

    static default(): ^this {
    /// @type.symbol symbol=default type=() => Owned<this>

        return new Buffer();
        /// @resolution.construct source="new Buffer()" parameters=() return=Owned<Buffer> kind=class target=Buffer constructor=default
        /// @resolution.name source=Buffer target=Buffer

    }
}

extension of Buffer {
/// @definition.extension symbol=<module>#3 form=local target=Buffer
/// @definition.method symbol=clear source="clear(&exclusive this): void {}" slot=clear type=<clear.'a, clear.P1: Place>(this: &clear.'a exclusive this) => void
/// @definition.method symbol=make slot=make static=true type=() => Buffer
/// @resolution.name source=Buffer target=Buffer

    static make(): Buffer {
    /// @type.symbol symbol=make type=() => Buffer
    /// @resolution.name source=Buffer target=Buffer

        return new Buffer();
        /// @resolution.construct source="new Buffer()" parameters=() return=Buffer kind=class target=Buffer constructor=default
        /// @resolution.name source=Buffer target=Buffer

    }

    clear(&exclusive this): void {}
    /// @generic.template symbol=clear parameters=('a, P1: Place)
    /// @type.symbol symbol=clear source="clear(&exclusive this): void {}" type=<clear.'a, clear.P1: Place>(this: &clear.'a exclusive this) => void
    /// @type.symbol symbol=clear.this source="&exclusive this" type=&clear.'a exclusive this

}

const object = Buffer;
/// @type.symbol symbol=object source=object type=Buffer
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
/// @resolution.member source=Buffer.make receiver=Buffer type=() => Buffer kind=symbol target_receiver=Buffer target=make
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
/// @resolution.member source=Buffer.make receiver=buffer.Buffer type=() => buffer.Buffer kind=symbol target_receiver=buffer.Buffer target=buffer.make
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
        return this.item.size<T, P1>();
    }
}

=== dir ===
class Item<T> {}
/// @generic.template symbol=Item parameters=(T#1)
/// @type.symbol symbol=Item source="class Item<T> {}" type=Item
/// @definition.class symbol=Item source="class Item<T> {}" template=(T#1)
/// @type.symbol symbol=Item.T source=T type=T#1

class Holder<T> {
/// @generic.template symbol=Holder parameters=(T#2)
/// @type.symbol symbol=Holder type=Holder
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
/// @definition.method symbol=size slot=size type=<size.'a, size.P1: Place>(this: &size.'a readonly this) => int32
/// @type.symbol symbol=T#1 source=T type=T#3
/// @resolution.name source=Item target=Item
/// @resolution.name source=T target=T#1

    size(&readonly this): int32 {
    /// @generic.template symbol=size parent=template#2 parameters=('a, P1: Place)
    /// @type.symbol symbol=size type=<size.'a, size.P1: Place>(this: &size.'a readonly this) => int32
    /// @type.symbol symbol=size.this source="&readonly this" type=&size.'a readonly this

        return 1;
    }
}

extension<T> of Holder<T> {
/// @generic.template symbol=<module>#3 parameters=(T#4)
/// @definition.extension symbol=<module>#3 form=local target=Holder<T#4>
/// @definition.method symbol=peek slot=peek type=<peek.'a, peek.P1: Place>(this: &peek.'a readonly this) => int32
/// @type.symbol symbol=T#2 source=T type=T#4
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=T target=T#2

    peek(&readonly this): int32 {
    /// @generic.template symbol=peek parent=template#3 parameters=('a, P1: Place)
    /// @type.symbol symbol=peek type=<peek.'a, peek.P1: Place>(this: &peek.'a readonly this) => int32
    /// @type.symbol symbol=peek.this source="&readonly this" type=&peek.'a readonly this

        return this.item.size();
        /// @resolution.member source=this.item receiver=&peek.'a readonly Holder<T#4> type=Readonly<Item<T#4>> kind=field target_receiver=&peek.'a readonly Holder<T#4> key=item target=Holder.item target_type=Readonly<Item<T#4>>
        /// @resolution.member source=this.item.size receiver=Readonly<Managed<Item<T#4>, peek.P1>> type=<size.'a, size.P1: Place>(this: &size.'a readonly Item<T#4>) => int32 kind=symbol target_receiver=Readonly<Managed<Item<T#4>, peek.P1>> target=size
        /// @resolution.call source=this.item.size() parameters=() return=int32 kind=symbol target=size receiver=Readonly<Managed<Item<T#4>, peek.P1>> adjustments=(Readonly<Managed<Item<T#4>, peek.P1>> => direct -> Managed<Item<T#4>, peek.P1>, borrow(&peek.'a readonly Managed<Item<T#4>, peek.P1>)) instance=Item<T#4>.<extension#1>.size<peek.P1>
        /// @resolution.receiver source=this kind=this declaration=<module>#3 type=&peek.'a readonly Holder<T#4>
        /// @resolution.place source=this placement=peek.P1 lifetime=peek.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.item placement=peek.P1 lifetime=peek.'a access="readonly"
        /// @resolution.access source=this.item root=this keys=[item]
        /// @generic.instantiation id="size<T#4, peek.P1>" template=size arguments=(T#4, peek.P1) owner=peek
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
        return this.item.size<T, P1>();
    }
}

=== dir ===
import { Item } from "./item.ds";

class Holder<T> {
/// @generic.template symbol=Holder parameters=(T#1)
/// @type.symbol symbol=Holder type=Holder
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
/// @definition.method symbol=peek slot=peek type=<peek.'a, peek.P1: Place>(this: &peek.'a readonly this) => int32
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=T target=T

    peek(&readonly this): int32 {
    /// @generic.template symbol=peek parent=template#1 parameters=('a, P1: Place)
    /// @type.symbol symbol=peek type=<peek.'a, peek.P1: Place>(this: &peek.'a readonly this) => int32
    /// @type.symbol symbol=peek.this source="&readonly this" type=&peek.'a readonly this

        return this.item.size();
        /// @resolution.member source=this.item receiver=&peek.'a readonly Holder<T#2> type=Readonly<item.Item<T#2>> kind=field target_receiver=&peek.'a readonly Holder<T#2> key=item target=Holder.item target_type=Readonly<item.Item<T#2>>
        /// @resolution.member source=this.item.size receiver=Readonly<Managed<item.Item<T#2>, peek.P1>> type=<item.size.'a, item.size.P1: Place>(this: Borrowed<item.Item<T#2>, item.size.'a & item.size.P1, "readonly">) => int32 kind=symbol target_receiver=Readonly<Managed<item.Item<T#2>, peek.P1>> target=item.size
        /// @resolution.call source=this.item.size() parameters=() return=int32 kind=symbol target=item.size receiver=Readonly<Managed<item.Item<T#2>, peek.P1>> adjustments=(Readonly<Managed<item.Item<T#2>, peek.P1>> => direct -> Managed<item.Item<T#2>, peek.P1>, borrow(&peek.'a readonly Managed<item.Item<T#2>, peek.P1>)) instance=item.Item<T#2>.<extension#1>.size<peek.P1>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&peek.'a readonly Holder<T#2>
        /// @resolution.place source=this placement=peek.P1 lifetime=peek.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.item placement=peek.P1 lifetime=peek.'a access="readonly"
        /// @resolution.access source=this.item root=this keys=[item]
        /// @generic.instantiation id="item.size<T#2, peek.P1>" template=item.size arguments=(T#2, peek.P1) owner=peek
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
/// @type.symbol symbol=Pile type=Pile
/// @definition.class symbol=Pile template=(in out T)
/// @definition.field symbol=Pile.items source="items: T[]" key=items type=T[]
/// @definition.method symbol=Pile.count slot=count type=<Pile.count.'a, Pile.count.P1: Place>(this: &Pile.count.'a readonly this) => isize
/// @type.symbol symbol=Pile.T source=T type=T

    items: T[];
    /// @type.symbol symbol=Pile.items source="items: T[]" type=T[]
    /// @resolution.name source=T target=Pile.T

    count(&readonly this): isize {
    /// @generic.template symbol=Pile.count parent=template#0 parameters=('a, P1: Place)
    /// @type.symbol symbol=Pile.count type=<Pile.count.'a, Pile.count.P1: Place>(this: &Pile.count.'a readonly this) => isize
    /// @type.symbol symbol=Pile.count.this source="&readonly this" type=&Pile.count.'a readonly this

        const item = this.items.at(0);
        /// @type.symbol symbol=Pile.count.item source=item type=&'frame readonly T | undefined
        /// @resolution.pattern source=item kind=binding target=Pile.count.item
        /// @resolution.member source=this.items receiver=&Pile.count.'a readonly Pile<T> type=readonly T[] kind=field target_receiver=&Pile.count.'a readonly Pile<T> key=items target=Pile.items target_type=readonly T[]
        /// @resolution.member source=this.items.at receiver=Readonly<Managed<T[], Pile.count.P1>> type=(this: &'frame readonly T[], isize) => &'frame readonly T | undefined kind=symbol target_receiver=Readonly<Managed<T[], Pile.count.P1>> adjustments=(Readonly<Managed<T[], Pile.count.P1>> => direct -> Managed<T[], Pile.count.P1>, Managed<T[], Pile.count.P1> => direct -> T[]) target=at#2
        /// @resolution.call source=this.items.at(0) parameters=(isize) arguments=(provided(0) as isize) return=&'frame readonly T | undefined kind=symbol target=at#2 receiver=Readonly<Managed<T[], Pile.count.P1>> adjustments=(Readonly<Managed<T[], Pile.count.P1>> => direct -> Managed<T[], Pile.count.P1>, Managed<T[], Pile.count.P1> => direct -> T[], borrow(&'frame readonly T[])) instance="Borrowed<T#4[], 'a, A#1>.<extension#4>.at#2"
        /// @resolution.receiver source=this kind=this declaration=Pile type=&Pile.count.'a readonly Pile<T>
        /// @resolution.place source=this placement=Pile.count.P1 lifetime=Pile.count.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.items placement=Pile.count.P1 lifetime=Pile.count.'a access="readonly"
        /// @resolution.access source=this.items root=this keys=[items]
        /// @generic.instantiation id="at#2<T, \"readonly\">" template=at#2 arguments=(T, "readonly") owner=Pile.count

        return this.items.size;
        /// @resolution.member source=this.items receiver=&Pile.count.'a readonly Pile<T> type=readonly T[] kind=field target_receiver=&Pile.count.'a readonly Pile<T> key=items target=Pile.items target_type=readonly T[]
        /// @resolution.member source=this.items.size receiver=Readonly<Managed<T[], Pile.count.P1>> type=isize kind=call target="size(parameters=(), arguments=(), return=isize)"
        /// @resolution.receiver source=this kind=this declaration=Pile type=&Pile.count.'a readonly Pile<T>
        /// @resolution.place source=this placement=Pile.count.P1 lifetime=Pile.count.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.items placement=Pile.count.P1 lifetime=Pile.count.'a access="readonly"
        /// @resolution.access source=this.items root=this keys=[items]
        /// @generic.instantiation id="size<T, Pile.count.P1>" template=size arguments=(T, Pile.count.P1) owner=Pile.count

    }
}
"#,
        r#"
/// @diagnostic.error id=field-not-definitely-initialized message="field 'items' is not initialized on every constructor path"
/// @diagnostic.label line=3 column=5 span="items" line_source="items: T[];"
"#,
    );
}

#[test]
fn test_read_length_on_a_rest_parameter() {
    let session = TestSession::single(
        r#"
function total(...values: int32[]): int32 {
    return values.length as int32;
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
    return values.length as int32;
}

function main(): int32 {
    return total(1, 2, 3);
}

=== dir ===
function total(...values: int32[]): int32 {
/// @type.symbol symbol=total type=(...int32[]) => int32
/// @type.symbol symbol=total.values source="...values: int32[]" type=int32[]

    return values.length as int32;
    /// @resolution.name source=values target=total.values
    /// @resolution.member source=values.length receiver=int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize)"
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=total.values
    /// @generic.instantiation id="length<int32, \"local\">" template=length arguments=(int32, "local")

}

function main(): int32 {
/// @type.symbol symbol=main type=() => int32

    return total(1, 2, 3);
    /// @resolution.name source=total target=total
    /// @resolution.call source="total(1, 2, 3)" parameters=(int32[]) arguments=(rest(1, 2, 3) pack=arrayFromSlice as int32) return=int32 kind=symbol target=total
    /// @generic.instantiation id=arrayFromSlice<int32> template=arrayFromSlice arguments=(int32)

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
    return values.subslice(0, 1);
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
    /// @resolution.member source=values.subslice receiver=Slice<T> type=<const subslice.A: Access = "readonly", subslice.'a, subslice.P2: Place>(this: WithAccess<Borrowed<Slice<T>, subslice.'a & subslice.P2, "mutable">, subslice.A>, usize, usize) => WithAccess<Borrowed<Slice<T>, subslice.'a & subslice.P2, "mutable">, subslice.A> kind=symbol target_receiver=Slice<T> target=subslice
    /// @resolution.call source="values.subslice(0, 1)" parameters=(usize, usize) arguments=(provided(0) as usize, provided(1) as usize) return=WithAccess<Borrowed<Slice<T>, <error> & <error>, "mutable">, <error>> kind=symbol target=subslice receiver=Slice<T> adjustments=(borrow(Borrowed<Slice<T>, <error> & <error>, <error>>)) instance="Slice<T>.<extension#1>.subslice<<error>, \"local\">"
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=first.values
    /// @generic.instantiation id="subslice<T, <error>, \"local\">" template=subslice arguments=(T, <error>, "local") owner=first
    /// @generic.instantiation id=subslice<T> template=subslice arguments=(T) owner=first

}
"#,
        r#"
/// @diagnostic.error id=return-not-assignable message="type 'WithAccess<Borrowed<Slice<T>, <error> & <error>, \"mutable\">, <error>>' is not assignable to the declared result type 'Slice<T>'"
/// @diagnostic.label line=5 column=12 span="values.subslice(0, 1)" line_source="return values.subslice(0, 1);"
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

declare const freePlace: PlaceOf<Free>;

freePlace satisfies "local";

=== dir ===
class Free {}
/// @type.symbol symbol=Free source="class Free {}" type=Free
/// @definition.class symbol=Free source="class Free {}"

type FreePlace = PlaceOf<Free>;
/// @type.symbol symbol=FreePlace source="type FreePlace = PlaceOf<Free>" type=PlaceOf<Free>
/// @definition.type symbol=FreePlace source="type FreePlace = PlaceOf<Free>" value=PlaceOf<Free>
/// @resolution.name source=PlaceOf target=PlaceOf
/// @resolution.name source=Free target=Free

declare const freePlace: FreePlace;
/// @type.symbol symbol=freePlace source=freePlace type=PlaceOf<Free>
/// @resolution.pattern source=freePlace kind=binding target=freePlace
/// @resolution.name source=FreePlace target=FreePlace

freePlace satisfies "local";
/// @resolution.name source=freePlace target=freePlace
/// @resolution.place source=freePlace placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=freePlace root=freePlace
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'PlaceOf<Free>' does not satisfy '\"local\"'"
/// @diagnostic.label line=8 column=11 span="satisfies" line_source="freePlace satisfies \"local\";"
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

declare const sharedTransport: Channel;
declare const localTransport: Queue;

sharedTransport satisfies Channel;
localTransport satisfies Queue;

=== dir ===
shared class Channel {}
/// @type.symbol symbol=Channel source="shared class Channel {}" type=Channel
/// @definition.class symbol=Channel source="shared class Channel {}"

local class Queue {}
/// @type.symbol symbol=Queue source="local class Queue {}" type=Queue
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
/// @type.symbol symbol=sharedTransport source=sharedTransport type=Channel
/// @resolution.pattern source=sharedTransport kind=binding target=sharedTransport
/// @resolution.name source=Transport target=Transport
/// @resolution.name source=Channel target=Channel

declare const localTransport: Transport<Queue>;
/// @type.symbol symbol=localTransport source=localTransport type=Queue
/// @resolution.pattern source=localTransport kind=binding target=localTransport
/// @resolution.name source=Transport target=Transport
/// @resolution.name source=Queue target=Queue

sharedTransport satisfies Channel;
/// @resolution.name source=sharedTransport target=sharedTransport
/// @resolution.place source=sharedTransport placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=sharedTransport root=sharedTransport
/// @resolution.name source=Channel target=Channel

localTransport satisfies Queue;
/// @resolution.name source=localTransport target=localTransport
/// @resolution.place source=localTransport placement="local" lifetime="static" access="exclusive"
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
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const localUser: local ^User;
/// @type.symbol symbol=localUser source=localUser type=Owned<User>
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: shared ^User;
/// @type.symbol symbol=sharedUser source=sharedUser type=Owned<User>
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

type SharedOwned = shared ^User;
/// @type.symbol symbol=SharedOwned source="type SharedOwned = shared ^User" type=Owned<User>
/// @definition.type symbol=SharedOwned source="type SharedOwned = shared ^User" value=Owned<User>
/// @resolution.name source=User target=User

type OwnedShared = ^shared User;
/// @type.symbol symbol=OwnedShared source="type OwnedShared = ^shared User" type=Owned<User>
/// @definition.type symbol=OwnedShared source="type OwnedShared = ^shared User" value=Owned<User>
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

use crate::tests::{DirRows, TestSession};

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
    static default(): ^Buffer {
        return new Buffer();
    }
}

extension of Buffer {
    static make(): Buffer {
        return new Buffer();
    }

    clear(&this): void {}
}

const object: new () => Buffer = Buffer;
const made: Buffer = new Buffer();
const buffer: Buffer = Buffer.make();

=== dir ===
class Buffer {}
/// @type.symbol symbol=Buffer source="class Buffer {}" type=typeof Buffer
/// @definition.class symbol=Buffer source="class Buffer {}"

extension of Buffer implements Default {
/// @definition.extension symbol=<module>#2 form=local target=Buffer
/// @definition.implements symbol=<module>#2 source=Default target=Default
/// @definition.method symbol=default slot=default static=true type=() => ^Buffer
/// @definition.conformance symbol=<module>#2 member=default requirement=Default.default
/// @resolution.name source=Buffer target=Buffer
/// @resolution.name source=Default target=Default

    static default(): ^this {
    /// @type.symbol symbol=default type=() => ^Buffer

        return new Buffer();
        /// @resolution.construct source="new Buffer()" parameters=() return=^Buffer kind=class target=Buffer constructor=default
        /// @resolution.name source=Buffer target=Buffer

    }
}

extension of Buffer {
/// @definition.extension symbol=<module>#3 form=local target=Buffer
/// @definition.method symbol=clear source="clear(&this): void {}" slot=clear type=<clear.'a>(this: &clear.'a Buffer) => void
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
    /// @type.symbol symbol=clear source="clear(&this): void {}" type=<clear.'a>(this: &clear.'a Buffer) => void
    /// @type.symbol symbol=clear.this source=&this type=&clear.'a Buffer

}

const object = Buffer;
/// @type.symbol symbol=object source=object type=Function<(), Buffer, "readonly">
/// @resolution.pattern source=object kind=binding target=object
/// @resolution.name source=Buffer target=Buffer
/// @resolution.function source=Buffer type=Function<(), Buffer, "readonly"> target=Buffer

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
        return this.item.size<T, "managed">();
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
/// @definition.method symbol=size slot=size type=<size.'a>(this: &size.'a readonly Item<T#3>) => int32
/// @type.symbol symbol=T#1 source=T type=T#3
/// @resolution.name source=Item target=Item
/// @resolution.name source=T target=T#1

    size(&readonly this): int32 {
    /// @generic.template symbol=size parent=template#2 parameters=('a)
    /// @type.symbol symbol=size type=<size.'a>(this: &size.'a readonly Item<T#3>) => int32
    /// @type.symbol symbol=size.this source="&readonly this" type=&size.'a readonly Item<T#3>

        return 1;
    }
}

extension<T> of Holder<T> {
/// @generic.template symbol=<module>#3 parameters=(T#4)
/// @definition.extension symbol=<module>#3 form=local target=Holder<T#4>
/// @definition.method symbol=peek slot=peek type=<peek.'a>(this: &peek.'a readonly Holder<T#4>) => int32
/// @type.symbol symbol=T#2 source=T type=T#4
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=T target=T#2

    peek(&readonly this): int32 {
    /// @generic.template symbol=peek parent=template#3 parameters=('a)
    /// @type.symbol symbol=peek type=<peek.'a>(this: &peek.'a readonly Holder<T#4>) => int32
    /// @type.symbol symbol=peek.this source="&readonly this" type=&peek.'a readonly Holder<T#4>

        return this.item.size();
        /// @resolution.member source=this.item receiver=&peek.'a readonly Holder<T#4> type=Item<T#4> kind=field target_receiver=&peek.'a readonly Holder<T#4> key=item target=Holder.item target_type=Item<T#4>
        /// @resolution.member source=this.item.size receiver=Item<T#4> type=<size.'a>(this: &size.'a readonly Item<T#4>) => int32 kind=symbol target_receiver=Item<T#4> target=size
        /// @resolution.call source=this.item.size() parameters=() return=int32 regions=("managed" & "local") kind=symbol target=size receiver=Item<T#4> adjustments=(borrow(&'managed readonly Item<T#4>)) instance="Item<T#4>.<extension#1>.size<\"managed\" & \"local\">"
        /// @resolution.receiver source=this kind=this declaration=<module>#3 type=&peek.'a readonly Holder<T#4>
        /// @resolution.place source=this placement=peek.'a lifetime=peek.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.item placement="local" lifetime=peek.'a access="readonly"
        /// @resolution.access source=this.item root=this keys=[item]
        /// @generic.instantiation id="size<T#4, \"managed\" & \"local\">" template=size arguments=(T#4, "managed" & "local") owner=peek
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
        return this.item.size<T, "managed">();
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
/// @definition.method symbol=peek slot=peek type=<peek.'a>(this: &peek.'a readonly Holder<T#2>) => int32
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=T target=T

    peek(&readonly this): int32 {
    /// @generic.template symbol=peek parent=template#1 parameters=('a)
    /// @type.symbol symbol=peek type=<peek.'a>(this: &peek.'a readonly Holder<T#2>) => int32
    /// @type.symbol symbol=peek.this source="&readonly this" type=&peek.'a readonly Holder<T#2>

        return this.item.size();
        /// @resolution.member source=this.item receiver=&peek.'a readonly Holder<T#2> type=item.Item<T#2> kind=field target_receiver=&peek.'a readonly Holder<T#2> key=item target=Holder.item target_type=item.Item<T#2>
        /// @resolution.member source=this.item.size receiver=item.Item<T#2> type=<item.size.'a>(this: &item.size.'a readonly item.Item<T#2>) => int32 kind=symbol target_receiver=item.Item<T#2> target=item.size
        /// @resolution.call source=this.item.size() parameters=() return=int32 regions=("managed" & "local") kind=symbol target=item.size receiver=item.Item<T#2> adjustments=(borrow(&'managed readonly item.Item<T#2>)) instance="item.Item<T#2>.<extension#1>.size<\"managed\" & \"local\">"
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&peek.'a readonly Holder<T#2>
        /// @resolution.place source=this placement=peek.'a lifetime=peek.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.item placement="local" lifetime=peek.'a access="readonly"
        /// @resolution.access source=this.item root=this keys=[item]
        /// @generic.instantiation id="item.size<T#2, \"managed\" & \"local\">" template=item.size arguments=(T#2, "managed" & "local") owner=peek
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
        const item = this.items.at(0);
        return this.items.size;
    }
}

=== dir ===
class Pile<T> {
/// @generic.template symbol=Pile parameters=(in out T)
/// @type.symbol symbol=Pile type=typeof Pile
/// @definition.class symbol=Pile template=(in out T)
/// @definition.field symbol=Pile.items source="items: T[]" key=items type=T[]
/// @definition.method symbol=Pile.count slot=count type=<Pile.count.'a>(this: &Pile.count.'a readonly Pile<T>) => isize
/// @type.symbol symbol=Pile.T source=T type=T

    items: T[];
    /// @type.symbol symbol=Pile.items source="items: T[]" type=T[]
    /// @resolution.name source=T target=Pile.T

    count(&readonly this): isize {
    /// @generic.template symbol=Pile.count parent=template#0 parameters=('a)
    /// @type.symbol symbol=Pile.count type=<Pile.count.'a>(this: &Pile.count.'a readonly Pile<T>) => isize
    /// @type.symbol symbol=Pile.count.this source="&readonly this" type=&Pile.count.'a readonly Pile<T>

        const item = this.items.at(0);
        /// @type.symbol symbol=Pile.count.item source=item type=<error>
        /// @resolution.pattern source=item kind=binding target=Pile.count.item
        /// @resolution.member source=this.items receiver=&Pile.count.'a readonly Pile<T> type=T[] kind=field target_receiver=&Pile.count.'a readonly Pile<T> key=items target=Pile.items target_type=T[]
        /// @resolution.receiver source=this kind=this declaration=Pile type=&Pile.count.'a readonly Pile<T>
        /// @resolution.place source=this placement=Pile.count.'a lifetime=Pile.count.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.items placement="local" lifetime=Pile.count.'a access="readonly"
        /// @resolution.access source=this.items root=this keys=[items]
        /// @resolution.rejected source=this.items.at
        /// @resolution.rejected source=this.items.at(0)

        return this.items.size;
        /// @resolution.member source=this.items receiver=&Pile.count.'a readonly Pile<T> type=T[] kind=field target_receiver=&Pile.count.'a readonly Pile<T> key=items target=Pile.items target_type=T[]
        /// @resolution.member source=this.items.size receiver=T[] type=isize kind=call target="size(parameters=(), arguments=(), return=isize, regions=(\"managed\" & \"local\"))"
        /// @resolution.receiver source=this kind=this declaration=Pile type=&Pile.count.'a readonly Pile<T>
        /// @resolution.place source=this placement=Pile.count.'a lifetime=Pile.count.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.items placement="local" lifetime=Pile.count.'a access="readonly"
        /// @resolution.access source=this.items root=this keys=[items]
        /// @generic.instantiation id="size<T, \"managed\" & \"local\">" template=size arguments=(T, "managed" & "local") owner=Pile.count

    }
}
"#,
        r#"
/// @diagnostic.error id=field-not-definitely-initialized message="field 'items' is not initialized on every constructor path"
/// @diagnostic.label line=3 column=5 span="items" line_source="items: T[];"
/// @diagnostic.error id=missing-member message="member 'at' does not exist on type 'T[]'; did you mean 'as'?"
/// @diagnostic.label line=6 column=33 span="at" line_source="const item = this.items.at(0);"
/// @diagnostic.suggestion message="rename to 'as'" applicability=dangerous patched="const item = this.items.as(0);"
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
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=total.values
    /// @generic.instantiation id="Cast.truncate<isize, int32>" template=Cast.truncate arguments=(isize, int32)
    /// @generic.instantiation id="length<int32, \"frame\" & \"local\">" template=length arguments=(int32, "frame" & "local")
    /// @generic.instantiation id=Cast.truncate<isize> template=Cast.truncate arguments=(isize)

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
    return total<"frame">(1, 2, 3);
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
    /// @generic.instantiation id="length<int32, total.'a>" template=length arguments=(int32, total.'a)
    /// @generic.instantiation id=Cast.truncate<isize> template=Cast.truncate arguments=(isize)

}

function main(): int32 {
/// @type.symbol symbol=main type=() => int32

    return total(1, 2, 3);
    /// @resolution.name source=total target=total
    /// @resolution.call source="total(1, 2, 3)" parameters=(&'frame readonly Slice<int32>) arguments=(rest(provided(1) as int32, provided(2) as int32, provided(3) as int32) as int32) return=int32 regions=("frame") kind=symbol target=total instance="total<\"frame\">"
    /// @generic.instantiation id="total<\"frame\">" template=total arguments=("frame")

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
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=total.values
    /// @generic.instantiation id="Cast.truncate<isize, int32>" template=Cast.truncate arguments=(isize, int32)
    /// @generic.instantiation id="length<int32, \"managed\" & \"local\">" template=length arguments=(int32, "managed" & "local")
    /// @generic.instantiation id=Cast.truncate<isize> template=Cast.truncate arguments=(isize)

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
    return values.subslice<T, "managed", "mutable">(0, 1);
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
    /// @resolution.member source=values.subslice receiver=Slice<T> type=<const subslice.R, const subslice.A: Access>(this: Borrowed<Slice<T>, subslice.R, subslice.A>, usize, usize) => Borrowed<Slice<T>, subslice.R, subslice.A> kind=symbol target_receiver=Slice<T> target=subslice
    /// @resolution.call source="values.subslice(0, 1)" parameters=(usize, usize) arguments=(provided(0) as usize, provided(1) as usize) return=&'managed Slice<T> regions=("managed" & "local") kind=symbol target=subslice receiver=Slice<T> adjustments=(borrow(&'managed Slice<T>)) instance="Slice<T>.<extension#1>.subslice<\"managed\" & \"local\", \"mutable\">"
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=first.values
    /// @generic.instantiation id="subslice<T, \"managed\" & \"local\", \"mutable\">" template=subslice arguments=(T, "managed" & "local", "mutable") owner=first
    /// @generic.instantiation id=subslice<T> template=subslice arguments=(T) owner=first

}
"#,
        r#"
/// @diagnostic.error id=return-not-assignable message="type '&'managed Slice<T>' is not assignable to the declared result type 'Slice<T>'"
/// @diagnostic.label line=5 column=12 span="values.subslice(0, 1)" line_source="return values.subslice(0, 1);"
"#,
    );
}

#[test]
fn test_compose_a_written_place_with_an_opaque_region_borrow() {
    let session = TestSession::single(
        r#"
struct Cell { value: int32; }

function read<R: Region>(borrow: Borrowed<Cell, R, "readonly">): int32 {
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

function read<R: Region>(borrow: Borrowed<Cell, R, "readonly">): int32 {
    return borrow.value;
}

=== dir ===
struct Cell { value: int32; }
/// @type.symbol symbol=Cell source="struct Cell { value: int32; }" type=Cell
/// @definition.struct symbol=Cell source="struct Cell { value: int32; }"
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32
/// @type.symbol symbol=Cell.value source="value: int32" type=int32

function read<R: Region>(borrow: Borrowed<Cell, R, "readonly">): int32 {
/// @generic.template symbol=read parameters=(R: Region)
/// @type.symbol symbol=read type=<R>(Borrowed<Cell, R, "readonly">) => int32
/// @type.symbol symbol=read.R source="R: Region" type=R
/// @resolution.name source=Region target=Region
/// @type.symbol symbol=read.borrow source="borrow: Borrowed<Cell, R, \"readonly\">" type=Borrowed<Cell, R, "readonly">
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Cell target=Cell
/// @resolution.name source=R target=read.R

    return borrow.value;
    /// @resolution.name source=borrow target=read.borrow
    /// @resolution.member source=borrow.value receiver=Borrowed<Cell, R, "readonly"> type=int32 kind=field target_receiver=Borrowed<Cell, R, "readonly"> key=value target=Cell.value target_type=int32
    /// @resolution.place source=borrow placement=R lifetime=R access="readonly"
    /// @resolution.access source=borrow root=read.borrow
    /// @resolution.place source=borrow.value placement=R lifetime=R access="readonly"
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
const a: int32 = near.draw<"static">();
const b: int32 = remote.draw<"static">();

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
/// @resolution.call source=near.draw() parameters=() return=int32 regions=("static" & "local") kind=symbol target=DrawnPoint.draw receiver=DrawnPoint adjustments=(borrow(&'static readonly DrawnPoint)) instance="DrawnPoint.draw<\"static\" & \"local\">"
/// @resolution.place source=near placement="local" lifetime="static" access="immutable"
/// @resolution.access source=near root=near
/// @generic.instantiation id="DrawnPoint.draw<\"static\" & \"local\">" template=DrawnPoint.draw arguments=("static" & "local")
/// @generic.instance id="DrawnPoint.draw<\"bound0\" & \"local\">" template=DrawnPoint.draw arguments=("bound0" & "local")

const b = remote.draw();
/// @type.symbol symbol=b source=b type=int32
/// @resolution.pattern source=b kind=binding target=b
/// @type.node source=remote type=DrawnPoint
/// @type.node source=remote.draw type=<DrawnPoint.draw.'a>(this: &DrawnPoint.draw.'a readonly DrawnPoint) => int32
/// @type.node source=remote.draw() type=int32
/// @resolution.name source=remote target=remote
/// @resolution.member source=remote.draw receiver=DrawnPoint type=<DrawnPoint.draw.'a>(this: &DrawnPoint.draw.'a readonly DrawnPoint) => int32 kind=symbol target_receiver=DrawnPoint target=DrawnPoint.draw
/// @resolution.call source=remote.draw() parameters=() return=int32 regions=("static" & "shared") kind=symbol target=DrawnPoint.draw receiver=DrawnPoint adjustments=(borrow(&'static readonly DrawnPoint)) instance="DrawnPoint.draw<\"static\" & \"shared\">"
/// @resolution.place source=remote placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=remote root=remote
/// @generic.instantiation id="DrawnPoint.draw<\"static\" & \"shared\">" template=DrawnPoint.draw arguments=("static" & "shared")
/// @generic.instance id="DrawnPoint.draw<\"bound0\" & \"shared\">" template=DrawnPoint.draw arguments=("bound0" & "shared")
"#,
    );
}

/// Reject calling one callback over borrows from both spaces, one lifetime binding one place.
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Item {
    x: int32;
}
declare shared const remote: Item;
function apply(f: (item: &readonly Item) => int32): int32 {
    const near: Item = Item { x: 1 };
    return f<"frame">(&readonly near) + f<"static">(&readonly remote);
}
const total: int32 = apply((item: &readonly Item): int32 => item.x);

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
/// @type.symbol symbol=apply type=(<type_expression.'a>(&type_expression.'a readonly Item) => int32) => int32
/// @type.symbol symbol=apply.f source="f: (item: &readonly Item) => int32" type=<type_expression.'a>(&type_expression.'a readonly Item) => int32
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
    /// @type.node source=f type=<type_expression.'a>(&type_expression.'a readonly Item) => int32
    /// @resolution.name source=f target=apply.f
    /// @resolution.call source="f(&readonly near)" parameters=(&'frame readonly Item) arguments=(provided(&readonly near) as &'frame readonly Item) return=int32 regions=("frame" & "local") kind=expression target=expression generic_arguments=("frame" & "local")
    /// @resolution.operator source="f(&readonly near) + f(&readonly remote)" type=int32 operator="+" kind=builtin operands=[f(&readonly near) as int32 families=(integer), f(&readonly remote) as int32 families=(integer)]
    /// @resolution.place source=f placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=f root=apply.f
    /// @type.node source="&readonly near" type=&'frame readonly Item
    /// @type.node source=near type=Item
    /// @resolution.name source=near target=apply.near
    /// @resolution.place source=near placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=near root=apply.near
    /// @type.node source="f(&readonly remote)" type=int32
    /// @type.node source=f type=<type_expression.'a>(&type_expression.'a readonly Item) => int32
    /// @resolution.name source=f target=apply.f
    /// @resolution.call source="f(&readonly remote)" parameters=(&'static readonly Item) arguments=(provided(&readonly remote) as &'static readonly Item) return=int32 regions=("static" & "shared") kind=expression target=expression generic_arguments=("static" & "shared")
    /// @resolution.place source=f placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=f root=apply.f
    /// @type.node source="&readonly remote" type=&'static readonly Item
    /// @type.node source=remote type=Item
    /// @resolution.name source=remote target=remote
    /// @resolution.place source=remote placement="shared" lifetime="static" access="immutable"
    /// @resolution.access source=remote root=remote

}
const total = apply((item) => item.x);
/// @type.symbol symbol=total source=total type=int32
/// @resolution.pattern source=total kind=binding target=total
/// @type.node source="apply((item) => item.x)" type=int32
/// @type.node source=apply type=(<type_expression.'a>(&type_expression.'a readonly Item) => int32) => int32
/// @resolution.name source=apply target=apply
/// @resolution.call source="apply((item) => item.x)" parameters=(<type_expression.'a>(&type_expression.'a readonly Item) => int32) arguments=(provided((item) => item.x) as <type_expression.'a>(&type_expression.'a readonly Item) => int32) return=int32 kind=symbol target=apply
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
        r#"

"#,
    );
}

/// A receiver borrowed at a generic place binds the callee's induced place to that place.
#[test]
fn test_bind_a_callee_place_from_a_generic_receiver_place() {
    let session = TestSession::single(
        r#"
import { Clone } from "destack:memory";

function duplicate<T: Clone>(value: &immutable T): T {
    return value.clone();
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Clone } from "destack:memory";

function duplicate<T: Clone, 'a>(value: &'a immutable T): T {
    return value.clone<'a>() as T;
}

=== dir ===
import { Clone } from "destack:memory";

function duplicate<T: Clone>(value: &immutable T): T {
/// @generic.template symbol=duplicate parameters=(T: Clone, 'a)
/// @type.symbol symbol=duplicate type=<T: Clone, duplicate.'a>(&duplicate.'a immutable T) => T
/// @type.symbol symbol=duplicate.T source="T: Clone" type=T
/// @resolution.name source=Clone target=Clone
/// @type.symbol symbol=duplicate.value source="value: &immutable T" type=&duplicate.'a immutable T
/// @resolution.name source=T target=duplicate.T
/// @resolution.name source=T target=duplicate.T

    return value.clone();
    /// @resolution.name source=value target=duplicate.value
    /// @resolution.member source=value.clone receiver=&duplicate.'a immutable T type=<Clone.clone.'a>(this: &Clone.clone.'a immutable T) => ^T kind=symbol target_receiver=&duplicate.'a immutable T target=Clone.clone
    /// @resolution.call source=value.clone() parameters=() return=^T regions=(duplicate.'a) kind=symbol target=Clone.clone receiver=&duplicate.'a immutable T instance=Clone.clone<duplicate.'a>
    /// @resolution.place source=value placement=duplicate.'a lifetime=duplicate.'a access="immutable"
    /// @resolution.access source=value root=duplicate.value
    /// @generic.instantiation id="Clone.clone<T, duplicate.'a>" template=Clone.clone arguments=(duplicate.'a) owner=duplicate
    /// @generic.instance id="Clone.clone<T, duplicate.'a>" template=Clone.clone arguments=(duplicate.'a)

}
"#,
    );
}

/// A handle receiver in a generic place binds the callee's induced place to that place.
#[test]
fn test_bind_a_callee_place_from_a_handle_receiver_place() {
    let session = TestSession::single(
        r#"
function grow(items: int32[]): void {
    items.push(1);
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function grow(items: int32[]): void {
    items.push<int32, "managed">(1);
}

=== dir ===
function grow(items: int32[]): void {
/// @type.symbol symbol=grow type=(int32[]) => void
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @type.symbol symbol=grow.items source="items: int32[]" type=int32[]

    items.push(1);
    /// @resolution.name source=items target=grow.items
    /// @resolution.member source=items.push receiver=int32[] type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
    /// @resolution.call source=items.push(1) parameters=(int32[]) arguments=(rest(provided(1) as int32) pack=arrayFromOwnedSlice as int32) return=isize regions=("managed" & "local") kind=symbol target=push receiver=int32[] adjustments=(borrow(&'managed int32[])) instance="Array<int32>.<extension#6>.push<\"managed\" & \"local\">"
    /// @resolution.place source=items placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=items root=grow.items
    /// @generic.instantiation id="push<int32, \"managed\" & \"local\">" template=push arguments=(int32, "managed" & "local")
    /// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
    /// @generic.instantiation id=push<int32> template=push arguments=(int32)
    /// @generic.instance id="push<int32, \"bound0\" & \"local\">" template=push arguments=(int32, "bound0" & "local")
    /// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)

}
"#,
    );
}

/// A handle recovered from a borrowed receiver binds the callee's place to the borrow's.
#[test]
fn test_bind_a_callee_place_from_a_borrowed_handle_receiver() {
    let session = TestSession::single(
        r#"
function grow(items: &int64[]): void {
    items.push(1);
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function grow<'a>(items: &'a int64[]): void {
    items.push<int64, 'a>(1);
}

=== dir ===
function grow(items: &int64[]): void {
/// @generic.template symbol=grow parameters=('a)
/// @type.symbol symbol=grow type=<grow.'a>(&grow.'a int64[]) => void
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int64>> template=sliceAssumeInit arguments=(MaybeUninit<int64>)
/// @generic.instance id=sliceUninit<MaybeUninit<int64>> template=sliceUninit arguments=(MaybeUninit<int64>)
/// @type.symbol symbol=grow.items source="items: &int64[]" type=&grow.'a int64[]

    items.push(1);
    /// @resolution.name source=items target=grow.items
    /// @resolution.member source=items.push receiver=&grow.'a int64[] type=<push.'a>(this: &push.'a int64[], ...int64[]) => isize kind=symbol target_receiver=&grow.'a int64[] target=push
    /// @resolution.call source=items.push(1) parameters=(int64[]) arguments=(rest(provided(1) as int64) pack=arrayFromOwnedSlice as int64) return=isize regions=(grow.'a) kind=symbol target=push receiver=&grow.'a int64[] instance=Array<int64>.<extension#6>.push<grow.'a>
    /// @resolution.place source=items placement=grow.'a lifetime=grow.'a access="mutable"
    /// @resolution.access source=items root=grow.items
    /// @generic.instantiation id="push<int64, grow.'a>" template=push arguments=(int64, grow.'a)
    /// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
    /// @generic.instantiation id=push<int64> template=push arguments=(int64)
    /// @generic.instance id="push<int64, grow.'a>" template=push arguments=(int64, grow.'a)
    /// @generic.instance id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)

}
"#,
    );
}

/// A parameter typed by an imported handle alias places at the receiver like a local one.
#[test]
fn test_place_an_imported_alias_parameter_at_the_receiver_place() {
    let session = TestSession::builder()
        .module("bytes.ds", "export type Bytes = readonly [uint8];\n")
        .module(
            "main.ds",
            r#"
import { Bytes } from "./bytes.ds";

class Reader {
    bytes: Bytes;

    constructor(source: Bytes) {
        this.bytes = source;
    }
}
"#,
        )
        .build();

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Bytes } from "./bytes.ds";

class Reader {
    bytes: Bytes;

    constructor(source: Bytes) {
        this.bytes = source;
    }
}

=== dir ===
import { Bytes } from "./bytes.ds";

class Reader {
/// @type.symbol symbol=Reader type=typeof Reader
/// @definition.class symbol=Reader
/// @definition.field symbol=Reader.bytes source="bytes: Bytes" key=bytes type=bytes.Bytes
/// @definition.method symbol=Reader.constructor slot=constructor role=constructor type=(this: &'managed Reader, bytes.Bytes) => Reader

    bytes: Bytes;
    /// @type.symbol symbol=Reader.bytes source="bytes: Bytes" type=bytes.Bytes
    /// @resolution.name source=Bytes target=bytes.Bytes

    constructor(source: Bytes) {
    /// @type.symbol symbol=Reader.constructor type=(this: &'managed Reader, bytes.Bytes) => Reader
    /// @type.symbol symbol=Reader.constructor.this type=&'managed Reader
    /// @type.symbol symbol=Reader.constructor.source source="source: Bytes" type=bytes.Bytes
    /// @resolution.name source=Bytes target=bytes.Bytes

        this.bytes = source;
        /// @resolution.receiver source=this kind=this declaration=Reader type=&'managed Reader
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.bytes kind=place
        /// @resolution.place source=this.bytes placement="local" lifetime="managed" access="readonly"
        /// @resolution.access source=this.bytes root=this keys=[bytes]
        /// @resolution.assignment source=this.bytes write="receiver=&'managed Reader, target=field(receiver=&'managed Reader, target=Reader.bytes, type=readonly Slice<uint8>), type=readonly Slice<uint8>" type=readonly Slice<uint8>
        /// @resolution.name source=source target=Reader.constructor.source
        /// @resolution.place source=source placement="local" lifetime="frame" access="readonly"
        /// @resolution.access source=source root=Reader.constructor.source

    }
}
"#);
}

/// A field read on a frame-owned class object projects the field without borrowing the object.
#[test]
fn test_project_a_field_of_a_frame_owned_object_at_its_place() {
    let session = TestSession::single(
        r#"
struct Payload {
    value: int32;
}

class Holder {
    slot: ^Payload | undefined;

    constructor(&exclusive this, slot: ^Payload | undefined) {
        this.slot = slot;
    }
}

export function inspect(slot: ^Payload | undefined): int32 {
    const holder: ^Holder = new Holder(slot);
    if (holder.slot !== undefined) {
        const held = &readonly holder.slot;
        return held.value;
    }
    return 0;
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Payload {
    value: int32;
}

class Holder {
    slot: Payload | undefined;

    constructor(&exclusive this, slot: Payload | undefined) {
        this.slot = slot;
    }
}

export function inspect(slot: Payload | undefined): int32 {
    const holder: ^Holder = new Holder(slot);
    if (holder.slot !== (undefined as Payload | undefined)) {
        const held: &'frame readonly Payload = &readonly holder.slot;
        return held.value;
    }
    return 0;
}

=== dir ===
struct Payload {
/// @type.symbol symbol=Payload type=Payload
/// @definition.struct symbol=Payload
/// @definition.field symbol=Payload.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Payload.value source="value: int32" type=int32

}

class Holder {
/// @type.symbol symbol=Holder type=typeof Holder
/// @definition.class symbol=Holder
/// @definition.field symbol=Holder.slot source="slot: ^Payload | undefined" key=slot type=Payload | undefined
/// @definition.method symbol=Holder.constructor slot=constructor role=constructor type=<Holder.constructor.'a>(this: &Holder.constructor.'a exclusive Holder, Payload | undefined) => Holder

    slot: ^Payload | undefined;
    /// @type.symbol symbol=Holder.slot source="slot: ^Payload | undefined" type=Payload | undefined
    /// @resolution.name source=Payload target=Payload

    constructor(&exclusive this, slot: ^Payload | undefined) {
    /// @generic.template symbol=Holder.constructor parameters=('a)
    /// @type.symbol symbol=Holder.constructor type=<Holder.constructor.'a>(this: &Holder.constructor.'a exclusive Holder, Payload | undefined) => Holder
    /// @type.symbol symbol=Holder.constructor.this source="&exclusive this" type=&Holder.constructor.'a exclusive Holder
    /// @type.symbol symbol=Holder.constructor.slot source="slot: ^Payload | undefined" type=Payload | undefined
    /// @resolution.name source=Payload target=Payload

        this.slot = slot;
        /// @resolution.receiver source=this kind=this declaration=Holder type=&Holder.constructor.'a exclusive Holder
        /// @resolution.place source=this placement=Holder.constructor.'a lifetime=Holder.constructor.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.slot kind=place
        /// @resolution.place source=this.slot placement=Holder.constructor.'a lifetime=Holder.constructor.'a access="exclusive"
        /// @resolution.access source=this.slot root=this keys=[slot]
        /// @resolution.assignment source=this.slot write="receiver=&Holder.constructor.'a exclusive Holder, target=field(receiver=&Holder.constructor.'a exclusive Holder, target=Holder.slot, type=Payload | undefined), type=Payload | undefined" type=Payload | undefined
        /// @resolution.name source=slot target=Holder.constructor.slot
        /// @resolution.place source=slot placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=slot root=Holder.constructor.slot

    }
}

export function inspect(slot: ^Payload | undefined): int32 {
/// @type.symbol symbol=inspect type=(Payload | undefined) => int32
/// @type.symbol symbol=inspect.slot source="slot: ^Payload | undefined" type=Payload | undefined
/// @resolution.name source=Payload target=Payload

    const holder: ^Holder = new Holder(slot);
    /// @type.symbol symbol=inspect.holder source=holder type=^Holder
    /// @resolution.pattern source=holder kind=binding target=inspect.holder
    /// @resolution.name source=Holder target=Holder
    /// @resolution.construct source="new Holder(slot)" parameters=(Payload | undefined) arguments=(provided(slot) as Payload | undefined) return=^Holder regions=("managed" & "local") kind=class target=Holder constructor=Holder.constructor call="Holder.constructor<\"managed\" & \"local\">"
    /// @generic.instantiation id="Holder.constructor<\"managed\" & \"local\">" template=Holder.constructor arguments=("managed" & "local")
    /// @generic.instance id="Holder.constructor<\"bound0\" & \"local\">" template=Holder.constructor arguments=("bound0" & "local")
    /// @resolution.name source=Holder target=Holder
    /// @resolution.name source=slot target=inspect.slot
    /// @resolution.place source=slot placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=slot root=inspect.slot

    if (holder.slot !== undefined) {
    /// @resolution.name source=holder target=inspect.holder
    /// @resolution.member source=holder.slot receiver=^Holder type=Payload | undefined kind=field target_receiver=^Holder key=slot target=Holder.slot target_type=Payload | undefined
    /// @resolution.operator source="holder.slot !== undefined" type=boolean operator="!==" kind=builtin operands=[holder.slot as Payload | undefined, undefined as Payload | undefined]
    /// @resolution.place source=holder placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=holder root=inspect.holder
    /// @resolution.place source=holder.slot placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=holder.slot root=inspect.holder keys=[slot]

        const held = &readonly holder.slot;
        /// @type.symbol symbol=inspect.held source=held type=&'frame readonly Payload
        /// @resolution.pattern source=held kind=binding target=inspect.held
        /// @resolution.name source=holder target=inspect.holder
        /// @resolution.member source=holder.slot receiver=^Holder type=Payload | undefined kind=field target_receiver=^Holder key=slot target=Holder.slot target_type=Payload | undefined
        /// @resolution.place source=holder placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=holder root=inspect.holder
        /// @resolution.place source=holder.slot placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=holder.slot root=inspect.holder keys=[slot]
        /// @resolution.narrowing source=holder.slot union=Payload | undefined arms=Payload

        return held.value;
        /// @resolution.name source=held target=inspect.held
        /// @resolution.member source=held.value receiver=&'frame readonly Payload type=int32 kind=field target_receiver=&'frame readonly Payload key=value target=Payload.value target_type=int32
        /// @resolution.place source=held placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=held root=inspect.held
        /// @resolution.place source=held.value placement="local" lifetime="frame" access="readonly"
        /// @resolution.access source=held.value root=inspect.held keys=[value]

    }
    return 0;
}
"#);
}

/// An elided return borrow through a value receiver takes the receiver's region.
#[test]
fn test_return_an_elided_field_borrow_through_a_value_receiver() {
    let session = TestSession::single(
        r#"
struct Pair<T> {
    start: T;
    end: T;
}

extension<T: Copy> of Pair<T> {
    first(&immutable this): &immutable T {
        &immutable this.start
    }
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Pair<out T> {
    start: T;
    end: T;
}

extension<T: Copy> of Pair<T> {
    first(&immutable this): &'a immutable T {
        &immutable this.start
    }
}

=== dir ===
struct Pair<T> {
/// @generic.template symbol=Pair parameters=(out T#1)
/// @type.symbol symbol=Pair type=Pair
/// @definition.struct symbol=Pair template=(out T#1)
/// @definition.field symbol=Pair.end source="end: T" key=end type=T#1
/// @definition.field symbol=Pair.start source="start: T" key=start type=T#1
/// @type.symbol symbol=Pair.T source=T type=T#1

    start: T;
    /// @type.symbol symbol=Pair.start source="start: T" type=T#1
    /// @resolution.name source=T target=Pair.T

    end: T;
    /// @type.symbol symbol=Pair.end source="end: T" type=T#1
    /// @resolution.name source=T target=Pair.T

}

extension<T: Copy> of Pair<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2: Copy)
/// @generic.instance id=Pair<T#2> template=Pair arguments=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Pair<T#2>
/// @definition.method symbol=first slot=first type=<first.'a>(this: &first.'a immutable Pair<T#2>) => &first.'a immutable T#2
/// @type.symbol symbol=T source="T: Copy" type=T#2
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=Pair target=Pair
/// @resolution.name source=T target=T

    first(&immutable this): &immutable T {
    /// @generic.template symbol=first parent=template#1 parameters=('a)
    /// @type.symbol symbol=first type=<first.'a>(this: &first.'a immutable Pair<T#2>) => &first.'a immutable T#2
    /// @type.symbol symbol=first.this source="&immutable this" type=&first.'a immutable Pair<T#2>
    /// @resolution.name source=T target=T

        &immutable this.start
        /// @resolution.member source=this.start receiver=&first.'a immutable Pair<T#2> type=T#2 kind=field target_receiver=&first.'a immutable Pair<T#2> key=start target=Pair.start target_type=T#2
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&first.'a immutable Pair<T#2>
        /// @resolution.place source=this placement=first.'a lifetime=first.'a access="immutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.start placement=first.'a lifetime=first.'a access="immutable"
        /// @resolution.access source=this.start root=this keys=[start]

    }
}
"#);
}

/// Passing `this` to a sibling method of a class stores it at the shared place.
#[test]
fn test_pass_this_to_a_sibling_method_of_a_local_class() {
    let session = TestSession::single(
        r#"
class Box<T: Copy> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }

    take(other: Box<T>): void {}

    pass(): void {
        this.take(this);
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Box<in out T: Copy> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }

    take(other: Box<T>): void {}

    pass(): void {
        this.take<T>(this);
    }
}

=== dir ===
class Box<T: Copy> {
/// @generic.template symbol=Box parameters=(in out T: Copy)
/// @type.symbol symbol=Box type=typeof Box
/// @definition.class symbol=Box template=(in out T: Copy)
/// @definition.field symbol=Box.value source="value: T" key=value type=T
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(this: &'managed Box<T>, T) => Box<T>
/// @definition.method symbol=Box.pass slot=pass type=(this: Box<T>) => void
/// @definition.method symbol=Box.take source="take(other: Box<T>): void {}" slot=take type=(this: Box<T>, Box<T>) => void
/// @type.symbol symbol=Box.T source="T: Copy" type=T
/// @resolution.name source=Copy target=Copy

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

    constructor(value: T) {
    /// @type.symbol symbol=Box.constructor type=(this: &'managed Box<T>, T) => Box<T>
    /// @type.symbol symbol=Box.constructor.this type=&'managed Box<T>
    /// @type.symbol symbol=Box.constructor.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Box type=&'managed Box<T>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Box<T>, target=field(receiver=&'managed Box<T>, target=Box.value, type=T), type=T" type=T
        /// @resolution.name source=value target=Box.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Box.constructor.value

    }

    take(other: Box<T>): void {}
    /// @type.symbol symbol=Box.take source="take(other: Box<T>): void {}" type=(this: Box<T>, Box<T>) => void
    /// @type.symbol symbol=Box.take.this type=Box<T>
    /// @type.symbol symbol=Box.take.other source="other: Box<T>" type=Box<T>
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=T target=Box.T

    pass(): void {
    /// @type.symbol symbol=Box.pass type=(this: Box<T>) => void
    /// @type.symbol symbol=Box.pass.this type=Box<T>

        this.take(this);
        /// @resolution.member source=this.take receiver=Box<T> type=(this: Box<T>, Box<T>) => void kind=symbol target_receiver=Box<T> target=Box.take
        /// @resolution.call source=this.take(this) parameters=(Box<T>) arguments=(provided(this) as Box<T>) return=void kind=symbol target=Box.take receiver=Box<T> instance=Box<T>.take
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<T>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=Box.take<T> template=Box.take arguments=(T) owner=Box.pass
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<T>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this

    }
}
"#,
        r#"
"#,
    );
}

/// Calling a bounded free function with a bounded class parameter satisfies the bound.
#[test]
fn test_satisfy_a_free_function_bound_with_a_class_parameter() {
    let session = TestSession::single(
        r#"
function keep<T: Copy>(value: T): T {
    value
}

class Holder<T: Copy> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }

    read(): T {
        keep(this.value)
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function keep<T: Copy>(value: T): T {
    value
}

class Holder<in out T: Copy> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }

    read(): T {
        keep<T>(this.value)
    }
}

=== dir ===
function keep<T: Copy>(value: T): T {
/// @generic.template symbol=keep parameters=(T#1: Copy)
/// @type.symbol symbol=keep type=<T#1: Copy>(T#1) => T#1
/// @type.symbol symbol=keep.T source="T: Copy" type=T#1
/// @resolution.name source=Copy target=Copy
/// @type.symbol symbol=keep.value source="value: T" type=T#1
/// @resolution.name source=T target=keep.T
/// @resolution.name source=T target=keep.T

    value
    /// @resolution.name source=value target=keep.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=keep.value

}

class Holder<T: Copy> {
/// @generic.template symbol=Holder parameters=(in out T#2: Copy)
/// @type.symbol symbol=Holder type=typeof Holder
/// @definition.class symbol=Holder template=(in out T#2: Copy)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T#2
/// @definition.method symbol=Holder.constructor slot=constructor role=constructor type=(this: &'managed Holder<T#2>, T#2) => Holder<T#2>
/// @definition.method symbol=Holder.read slot=read type=(this: Holder<T#2>) => T#2
/// @type.symbol symbol=Holder.T source="T: Copy" type=T#2
/// @resolution.name source=Copy target=Copy

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T#2
    /// @resolution.name source=T target=Holder.T

    constructor(value: T) {
    /// @type.symbol symbol=Holder.constructor type=(this: &'managed Holder<T#2>, T#2) => Holder<T#2>
    /// @type.symbol symbol=Holder.constructor.this type=&'managed Holder<T#2>
    /// @type.symbol symbol=Holder.constructor.value source="value: T" type=T#2
    /// @resolution.name source=T target=Holder.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Holder type=&'managed Holder<T#2>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Holder<T#2>, target=field(receiver=&'managed Holder<T#2>, target=Holder.value, type=T#2), type=T#2" type=T#2
        /// @resolution.name source=value target=Holder.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Holder.constructor.value

    }

    read(): T {
    /// @type.symbol symbol=Holder.read type=(this: Holder<T#2>) => T#2
    /// @type.symbol symbol=Holder.read.this type=Holder<T#2>
    /// @resolution.name source=T target=Holder.T

        keep(this.value)
        /// @resolution.name source=keep target=keep
        /// @resolution.call source=keep(this.value) parameters=(T#2) arguments=(provided(this.value) as T#2) return=T#2 kind=symbol target=keep instance=keep<T#2>
        /// @generic.instantiation id=keep<T#2> template=keep arguments=(T#2) owner=Holder.read
        /// @resolution.member source=this.value receiver=Holder<T#2> type=T#2 kind=field target_receiver=Holder<T#2> key=value target=Holder.value target_type=T#2
        /// @resolution.receiver source=this kind=this declaration=Holder type=Holder<T#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}
"#,
        r#"
"#,
    );
}

/// A template string flows into a readonly string borrow.
#[test]
fn test_borrow_a_template_string_readonly() {
    let session = TestSession::single(
        r#"
function show(message: &readonly string): void {}

function run(name: string): void {
    show(`hello ${name}`);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function show<'a>(message: &'a readonly string): void {}

function run(name: string): void {
    show<"managed">(`hello ${name}` as &'managed readonly string);
}

=== dir ===
function show(message: &readonly string): void {}
/// @generic.template symbol=show parameters=('a)
/// @type.symbol symbol=show source="function show(message: &readonly string): void {}" type=<show.'a>(&show.'a readonly string) => void
/// @type.symbol symbol=show.message source="message: &readonly string" type=&show.'a readonly string

function run(name: string): void {
/// @type.symbol symbol=run type=(string) => void
/// @type.symbol symbol=run.name source="name: string" type=string

    show(`hello ${name}`);
    /// @resolution.name source=show target=show
    /// @resolution.call source="show(`hello ${name}`)" parameters=(&'managed readonly string) arguments=(provided(`hello ${name}`) as &'managed readonly string) return=void regions=("managed" & "local") kind=symbol target=show instance="show<\"managed\" & \"local\">"
    /// @generic.instantiation id="show<\"managed\" & \"local\">" template=show arguments=("managed" & "local")
    /// @resolution.template source="`hello ${name}`" spans=[Display.display(parameters=(), arguments=(), return=^string, regions=("managed" & "local"))] build="stringFromTemplate(parameters=(&'frame readonly Slice<string>, &'frame readonly Slice<string>), arguments=(supplied(0) as &'frame readonly Slice<string>, supplied(1) as &'frame readonly Slice<string>), return=string, regions=(\"frame\", \"frame\"))"
    /// @generic.instantiation id="Display.display<string, \"managed\" & \"local\">" template=Display.display arguments=("managed" & "local")
    /// @generic.instantiation id="stringFromTemplate<\"frame\", \"frame\">" template=stringFromTemplate arguments=("frame", "frame")
    /// @resolution.name source=name target=run.name
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=run.name

}
"#,
        r#"
"#,
    );
}

/// A closure flows into a callback parameter taking a handle.
#[test]
fn test_pass_a_closure_taking_a_handle_to_a_callback_parameter() {
    let session = TestSession::single(
        r#"
class Item {}

class Source {
    map<R>(body: (value: Item) => R): R {
        body(new Item())
    }
}

function run(source: Source): Item {
    source.map((value) => value)
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Item {}

class Source {
    map<R>(body: (value: Item) => R): R {
        body(new Item())
    }
}

function run(source: Source): Item {
    source.map<Item>((value: Item): Item => value)
}

=== dir ===
class Item {}
/// @type.symbol symbol=Item source="class Item {}" type=typeof Item
/// @definition.class symbol=Item source="class Item {}"

class Source {
/// @type.symbol symbol=Source type=typeof Source
/// @definition.class symbol=Source
/// @definition.method symbol=Source.map slot=map type=<R>(this: Source, (Item) => R) => R

    map<R>(body: (value: Item) => R): R {
    /// @generic.template symbol=Source.map parameters=(R)
    /// @type.symbol symbol=Source.map type=<R>(this: Source, (Item) => R) => R
    /// @type.symbol symbol=Source.map.this type=Source
    /// @type.symbol symbol=Source.map.R source=R type=R
    /// @type.symbol symbol=Source.map.body source="body: (value: Item) => R" type=(Item) => R
    /// @type.symbol symbol=Source.map.value source="value: Item" type=Item
    /// @resolution.name source=Item target=Item
    /// @resolution.name source=R target=Source.map.R
    /// @resolution.name source=R target=Source.map.R

        body(new Item())
        /// @resolution.name source=body target=Source.map.body
        /// @resolution.call source="body(new Item())" parameters=(Item) arguments=(provided(new Item()) as Item) return=R kind=expression target=expression
        /// @resolution.place source=body placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=body root=Source.map.body
        /// @resolution.construct source="new Item()" parameters=() return=Item kind=class target=Item constructor=default
        /// @resolution.name source=Item target=Item

    }
}

function run(source: Source): Item {
/// @type.symbol symbol=run type=(Source) => Item
/// @type.symbol symbol=run.source source="source: Source" type=Source
/// @resolution.name source=Source target=Source
/// @resolution.name source=Item target=Item

    source.map((value) => value)
    /// @resolution.name source=source target=run.source
    /// @resolution.member source=source.map receiver=Source type=<R>(this: Source, (Item) => R) => R kind=symbol target_receiver=Source target=Source.map
    /// @resolution.call source="source.map((value) => value)" parameters=((Item) => Item) arguments=(provided((value) => value) as (Item) => Item) return=Item kind=symbol target=Source.map receiver=Source instance=Source.map<Item>
    /// @resolution.place source=source placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=source root=run.source
    /// @generic.instantiation id=Source.map<Item> template=Source.map arguments=(Item)
    /// @type.symbol symbol=run.symbol10 source="(value) => value" type=Function<(Item,), Item, "readonly">
    /// @type.symbol symbol=run.symbol10.value source=value type=Item
    /// @resolution.name source=value target=run.symbol10.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=run.symbol10.value

}
"#,
        r#"
"#,
    );
}

/// An interface default method calls a requirement on a borrowed receiver.
#[test]
fn test_call_a_requirement_on_a_borrowed_receiver_in_a_default_method() {
    let session = TestSession::single(
        r#"
newtype interface Cloneable {
    clone(this: &immutable this): ^this;

    cloneFrom(&exclusive this, source: &immutable this): void {
        *this = source.clone();
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Cloneable {
    clone(this: &immutable this): ^this;

    cloneFrom(&exclusive this, source: &'b immutable this): void {
        *this = source.clone<'b>();
    }
}

=== dir ===
newtype interface Cloneable {
/// @generic.template symbol=Cloneable parameters=(this: Cloneable)
/// @type.symbol symbol=Cloneable type=Cloneable
/// @definition.interface symbol=Cloneable template=(this: Cloneable) nominal=true
/// @definition.where symbol=Cloneable relation=satisfies left=this right=Cloneable
/// @definition.method symbol=Cloneable.clone source="clone(this: &immutable this): ^this" slot=clone type=<Cloneable.clone.'a>(this: &Cloneable.clone.'a immutable this) => ^this
/// @definition.method symbol=Cloneable.cloneFrom slot=cloneFrom type=<Cloneable.cloneFrom.'a, Cloneable.cloneFrom.'b>(this: &Cloneable.cloneFrom.'a exclusive this, &Cloneable.cloneFrom.'b immutable this) => void

    clone(this: &immutable this): ^this;
    /// @generic.template symbol=Cloneable.clone parent=template#0 parameters=('a)
    /// @type.symbol symbol=Cloneable.clone source="clone(this: &immutable this): ^this" type=<Cloneable.clone.'a>(this: &Cloneable.clone.'a immutable this) => ^this
    /// @type.symbol symbol=Cloneable.clone.this source="this: &immutable this" type=&Cloneable.clone.'a immutable this

    cloneFrom(&exclusive this, source: &immutable this): void {
    /// @generic.template symbol=Cloneable.cloneFrom parent=template#0 parameters=('a, 'b)
    /// @type.symbol symbol=Cloneable.cloneFrom type=<Cloneable.cloneFrom.'a, Cloneable.cloneFrom.'b>(this: &Cloneable.cloneFrom.'a exclusive this, &Cloneable.cloneFrom.'b immutable this) => void
    /// @type.symbol symbol=Cloneable.cloneFrom.this source="&exclusive this" type=&Cloneable.cloneFrom.'a exclusive this
    /// @type.symbol symbol=Cloneable.cloneFrom.source source="source: &immutable this" type=&Cloneable.cloneFrom.'b immutable this

        *this = source.clone();
        /// @resolution.pattern.assign source=*this kind=place
        /// @resolution.place source=*this placement=Cloneable.cloneFrom.'a lifetime=Cloneable.cloneFrom.'a access="exclusive"
        /// @resolution.assignment source=*this write="&Cloneable.cloneFrom.'a exclusive this => builtin -> ^this" type=^this
        /// @resolution.name source=this target=Cloneable.cloneFrom.this
        /// @resolution.place source=this placement=Cloneable.cloneFrom.'a lifetime=Cloneable.cloneFrom.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.name source=source target=Cloneable.cloneFrom.source
        /// @resolution.member source=source.clone receiver=&Cloneable.cloneFrom.'b immutable this type=<Cloneable.clone.'a>(this: &Cloneable.clone.'a immutable this) => ^this kind=symbol target_receiver=&Cloneable.cloneFrom.'b immutable this target=Cloneable.clone
        /// @resolution.call source=source.clone() parameters=() return=^this regions=(Cloneable.cloneFrom.'b) kind=symbol target=Cloneable.clone receiver=&Cloneable.cloneFrom.'b immutable this instance=Cloneable.clone<Cloneable.cloneFrom.'b>
        /// @resolution.place source=source placement=Cloneable.cloneFrom.'b lifetime=Cloneable.cloneFrom.'b access="immutable"
        /// @resolution.access source=source root=Cloneable.cloneFrom.source
        /// @generic.instantiation id="Cloneable.clone<this, Cloneable.cloneFrom.'b>" template=Cloneable.clone arguments=(Cloneable.cloneFrom.'b) owner=Cloneable

    }
}
"#,
        r#"
"#,
    );
}

/// A class handle flows into a bounded free function's parameter through its own bound.
#[test]
fn test_pass_a_bounded_class_handle_to_a_bounded_free_function() {
    let session = TestSession::single(
        r#"
class Holder<T: Copy> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

function read<T: Copy>(holder: Holder<T>): T {
    holder.value
}

class Reader<T: Copy> {
    holder: Holder<T>;

    constructor(holder: Holder<T>) {
        this.holder = holder;
    }

    read(): T {
        read(this.holder)
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Holder<in out T: Copy> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

function read<T: Copy>(holder: Holder<T>): T {
    holder.value
}

class Reader<in out T: Copy> {
    holder: Holder<T>;

    constructor(holder: Holder<T>) {
        this.holder = holder;
    }

    read(): T {
        read<T>(this.holder)
    }
}

=== dir ===
class Holder<T: Copy> {
/// @generic.template symbol=Holder parameters=(in out T#1: Copy)
/// @type.symbol symbol=Holder type=typeof Holder
/// @definition.class symbol=Holder template=(in out T#1: Copy)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T#1
/// @definition.method symbol=Holder.constructor slot=constructor role=constructor type=(this: &'managed Holder<T#1>, T#1) => Holder<T#1>
/// @type.symbol symbol=Holder.T source="T: Copy" type=T#1
/// @resolution.name source=Copy target=Copy

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T#1
    /// @resolution.name source=T target=Holder.T

    constructor(value: T) {
    /// @type.symbol symbol=Holder.constructor type=(this: &'managed Holder<T#1>, T#1) => Holder<T#1>
    /// @type.symbol symbol=Holder.constructor.this type=&'managed Holder<T#1>
    /// @type.symbol symbol=Holder.constructor.value source="value: T" type=T#1
    /// @resolution.name source=T target=Holder.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Holder type=&'managed Holder<T#1>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Holder<T#1>, target=field(receiver=&'managed Holder<T#1>, target=Holder.value, type=T#1), type=T#1" type=T#1
        /// @resolution.name source=value target=Holder.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Holder.constructor.value

    }
}

function read<T: Copy>(holder: Holder<T>): T {
/// @generic.template symbol=read parameters=(T#2: Copy)
/// @type.symbol symbol=read type=<T#2: Copy>(Holder<T#2>) => T#2
/// @type.symbol symbol=read.T source="T: Copy" type=T#2
/// @resolution.name source=Copy target=Copy
/// @type.symbol symbol=read.holder source="holder: Holder<T>" type=Holder<T#2>
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=T target=read.T
/// @resolution.name source=T target=read.T

    holder.value
    /// @resolution.name source=holder target=read.holder
    /// @resolution.member source=holder.value receiver=Holder<T#2> type=T#2 kind=field target_receiver=Holder<T#2> key=value target=Holder.value target_type=T#2
    /// @resolution.place source=holder placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=holder root=read.holder
    /// @resolution.place source=holder.value placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=holder.value root=read.holder keys=[value]

}

class Reader<T: Copy> {
/// @generic.template symbol=Reader parameters=(in out T#3: Copy)
/// @type.symbol symbol=Reader type=typeof Reader
/// @definition.class symbol=Reader template=(in out T#3: Copy)
/// @definition.field symbol=Reader.holder source="holder: Holder<T>" key=holder type=Holder<T#3>
/// @definition.method symbol=Reader.constructor slot=constructor role=constructor type=(this: &'managed Reader<T#3>, Holder<T#3>) => Reader<T#3>
/// @definition.method symbol=Reader.read slot=read type=(this: Reader<T#3>) => T#3
/// @type.symbol symbol=Reader.T source="T: Copy" type=T#3
/// @resolution.name source=Copy target=Copy

    holder: Holder<T>;
    /// @type.symbol symbol=Reader.holder source="holder: Holder<T>" type=Holder<T#3>
    /// @resolution.name source=Holder target=Holder
    /// @resolution.name source=T target=Reader.T

    constructor(holder: Holder<T>) {
    /// @type.symbol symbol=Reader.constructor type=(this: &'managed Reader<T#3>, Holder<T#3>) => Reader<T#3>
    /// @type.symbol symbol=Reader.constructor.this type=&'managed Reader<T#3>
    /// @type.symbol symbol=Reader.constructor.holder source="holder: Holder<T>" type=Holder<T#3>
    /// @resolution.name source=Holder target=Holder
    /// @resolution.name source=T target=Reader.T

        this.holder = holder;
        /// @resolution.receiver source=this kind=this declaration=Reader type=&'managed Reader<T#3>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.holder kind=place
        /// @resolution.place source=this.holder placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.holder root=this keys=[holder]
        /// @resolution.assignment source=this.holder write="receiver=&'managed Reader<T#3>, target=field(receiver=&'managed Reader<T#3>, target=Reader.holder, type=Holder<T#3>), type=Holder<T#3>" type=Holder<T#3>
        /// @resolution.name source=holder target=Reader.constructor.holder
        /// @resolution.place source=holder placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=holder root=Reader.constructor.holder

    }

    read(): T {
    /// @type.symbol symbol=Reader.read type=(this: Reader<T#3>) => T#3
    /// @type.symbol symbol=Reader.read.this type=Reader<T#3>
    /// @resolution.name source=T target=Reader.T

        read(this.holder)
        /// @resolution.name source=read target=read
        /// @resolution.call source=read(this.holder) parameters=(Holder<T#3>) arguments=(provided(this.holder) as Holder<T#3>) return=T#3 kind=symbol target=read instance=read<T#3>
        /// @generic.instantiation id=read<T#3> template=read arguments=(T#3) owner=Reader.read
        /// @resolution.member source=this.holder receiver=Reader<T#3> type=Holder<T#3> kind=field target_receiver=Reader<T#3> key=holder target=Reader.holder target_type=Holder<T#3>
        /// @resolution.receiver source=this kind=this declaration=Reader type=Reader<T#3>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.holder placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.holder root=this keys=[holder]

    }
}
"#,
        r#"
"#,
    );
}

/// A handle stores into an optional field of the same handle type.
#[test]
fn test_store_a_handle_into_an_optional_field() {
    let session = TestSession::single(
        r#"
class Node {
    next?: Node;

    link(node: Node): void {
        this.next = node;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Node {
    next?: Node;

    link(node: Node): void {
        this.next = node as Node | undefined;
    }
}

=== dir ===
class Node {
/// @type.symbol symbol=Node type=typeof Node
/// @definition.class symbol=Node
/// @definition.field symbol=Node.next source="next?: Node" key=next type=Node
/// @definition.method symbol=Node.link slot=link type=(this: Node, Node) => void

    next?: Node;
    /// @type.symbol symbol=Node.next source="next?: Node" type=Node
    /// @resolution.name source=Node target=Node

    link(node: Node): void {
    /// @type.symbol symbol=Node.link type=(this: Node, Node) => void
    /// @type.symbol symbol=Node.link.this type=Node
    /// @type.symbol symbol=Node.link.node source="node: Node" type=Node
    /// @resolution.name source=Node target=Node

        this.next = node;
        /// @resolution.receiver source=this kind=this declaration=Node type=Node
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.next kind=place
        /// @resolution.place source=this.next placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.next root=this keys=[next]
        /// @resolution.assignment source=this.next write="receiver=Node, target=field(receiver=Node, target=Node.next, type=Node), type=Node" type=Node
        /// @resolution.name source=node target=Node.link.node
        /// @resolution.place source=node placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=node root=Node.link.node

    }
}
"#,
        r#"
"#,
    );
}

/// An array literal initializes a field of handles at the holder's place.
#[test]
fn test_initialize_a_handle_array_field_with_an_empty_literal() {
    let session = TestSession::single(
        r#"
class Worker {}

class Pool {
    workers: Worker[] = [];
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Worker {}

class Pool {
    workers: Worker[] = [];
}

=== dir ===
class Worker {}
/// @type.symbol symbol=Worker source="class Worker {}" type=typeof Worker
/// @definition.class symbol=Worker source="class Worker {}"

class Pool {
/// @type.symbol symbol=Pool type=typeof Pool
/// @definition.class symbol=Pool
/// @definition.field symbol=Pool.workers source="workers: Worker[] = []" key=workers type=Worker[]

    workers: Worker[] = [];
    /// @type.symbol symbol=Pool.workers source="workers: Worker[] = []" type=Worker[]
    /// @resolution.name source=Worker target=Worker
    /// @resolution.call source=[] parameters=(^Slice<Worker>) arguments=(rest() as Worker) return=Worker[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<Worker>
    /// @generic.instantiation id=arrayFromOwnedSlice<Worker> template=arrayFromOwnedSlice arguments=(Worker)

}
"#,
        r#"
"#,
    );
}

/// A handle argument flows into a generic member whose parameter names the class argument.
#[test]
fn test_pass_a_handle_to_a_generic_member_taking_the_class_argument() {
    let session = TestSession::single(
        r#"
class Cell<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }

    set(value: T): void {
        this.value = value;
    }
}

class Item {}

class Owner {
    cell: Cell<Item>;

    replace(item: Item): void {
        this.cell.set(item);
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Cell<in out T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }

    set(value: T): void {
        this.value = value;
    }
}

class Item {}

class Owner {
    cell: Cell<Item>;

    replace(item: Item): void {
        this.cell.set<Item>(item);
    }
}

=== dir ===
class Cell<T> {
/// @generic.template symbol=Cell parameters=(in out T)
/// @type.symbol symbol=Cell type=typeof Cell
/// @definition.class symbol=Cell template=(in out T)
/// @definition.field symbol=Cell.value source="value: T" key=value type=T
/// @definition.method symbol=Cell.constructor slot=constructor role=constructor type=(this: &'managed Cell<T>, T) => Cell<T>
/// @definition.method symbol=Cell.set slot=set type=(this: Cell<T>, T) => void
/// @type.symbol symbol=Cell.T source=T type=T

    value: T;
    /// @type.symbol symbol=Cell.value source="value: T" type=T
    /// @resolution.name source=T target=Cell.T

    constructor(value: T) {
    /// @type.symbol symbol=Cell.constructor type=(this: &'managed Cell<T>, T) => Cell<T>
    /// @type.symbol symbol=Cell.constructor.this type=&'managed Cell<T>
    /// @type.symbol symbol=Cell.constructor.value source="value: T" type=T
    /// @resolution.name source=T target=Cell.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Cell type=&'managed Cell<T>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Cell<T>, target=field(receiver=&'managed Cell<T>, target=Cell.value, type=T), type=T" type=T
        /// @resolution.name source=value target=Cell.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Cell.constructor.value

    }

    set(value: T): void {
    /// @type.symbol symbol=Cell.set type=(this: Cell<T>, T) => void
    /// @type.symbol symbol=Cell.set.this type=Cell<T>
    /// @type.symbol symbol=Cell.set.value source="value: T" type=T
    /// @resolution.name source=T target=Cell.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=Cell<T>, target=field(receiver=Cell<T>, target=Cell.value, type=T), type=T" type=T
        /// @resolution.name source=value target=Cell.set.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Cell.set.value

    }
}

class Item {}
/// @type.symbol symbol=Item source="class Item {}" type=typeof Item
/// @definition.class symbol=Item source="class Item {}"

class Owner {
/// @type.symbol symbol=Owner type=typeof Owner
/// @definition.class symbol=Owner
/// @definition.field symbol=Owner.cell source="cell: Cell<Item>" key=cell type=Cell<Item>
/// @definition.method symbol=Owner.replace slot=replace type=(this: Owner, Item) => void

    cell: Cell<Item>;
    /// @type.symbol symbol=Owner.cell source="cell: Cell<Item>" type=Cell<Item>
    /// @resolution.name source=Cell target=Cell
    /// @resolution.name source=Item target=Item

    replace(item: Item): void {
    /// @type.symbol symbol=Owner.replace type=(this: Owner, Item) => void
    /// @type.symbol symbol=Owner.replace.this type=Owner
    /// @type.symbol symbol=Owner.replace.item source="item: Item" type=Item
    /// @resolution.name source=Item target=Item

        this.cell.set(item);
        /// @resolution.member source=this.cell receiver=Owner type=Cell<Item> kind=field target_receiver=Owner key=cell target=Owner.cell target_type=Cell<Item>
        /// @resolution.member source=this.cell.set receiver=Cell<Item> type=(this: Cell<Item>, Item) => void kind=symbol target_receiver=Cell<Item> target=Cell.set
        /// @resolution.call source=this.cell.set(item) parameters=(Item) arguments=(provided(item) as Item) return=void kind=symbol target=Cell.set receiver=Cell<Item> instance=Cell<Item>.set
        /// @resolution.receiver source=this kind=this declaration=Owner type=Owner
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.cell placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.cell root=this keys=[cell]
        /// @generic.instantiation id=Cell.set<Item> template=Cell.set arguments=(Item)
        /// @resolution.name source=item target=Owner.replace.item
        /// @resolution.place source=item placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=item root=Owner.replace.item

    }
}
"#,
        r#"
/// @diagnostic.error id=field-not-definitely-initialized message="field 'cell' is not initialized on every constructor path"
/// @diagnostic.label line=17 column=5 span="cell" line_source="cell: Cell<Item>;"
"#,
    );
}

/// An object literal flows into an optional options parameter.
#[test]
fn test_pass_an_object_literal_to_an_optional_options_parameter() {
    let session = TestSession::single(
        r#"
class Fields {}

type Options = {
    fields?: Fields;
    message?: string;
};

function log(options?: Options): void {}

function run(fields: Fields): void {
    log({ fields, message: "hello" });
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Fields {}

type Options = {
    fields?: Fields;
    message?: string;
};

function log(options?: { fields?: Fields; message?: string }): void {}

function run(fields: Fields): void {
    log({ fields, message: "hello" as string | undefined } as | {
          fields?: Fields;
          message?: string;
      }
    | undefined);
}

=== dir ===
class Fields {}
/// @type.symbol symbol=Fields source="class Fields {}" type=typeof Fields
/// @definition.class symbol=Fields source="class Fields {}"

type Options = {
/// @type.symbol symbol=Options type={ fields?: Fields; message?: string }
/// @definition.type symbol=Options value={ fields?: Fields; message?: string }

    fields?: Fields;
    /// @type.symbol symbol=Options.fields source="fields?: Fields" type=Fields
    /// @resolution.name source=Fields target=Fields

    message?: string;
    /// @type.symbol symbol=Options.message source="message?: string" type=string

};

function log(options?: Options): void {}
/// @type.symbol symbol=log source="function log(options?: Options): void {}" type=({ fields?: Fields; message?: string } | undefined?) => void
/// @type.symbol symbol=log.options source="options?: Options" type={ fields?: Fields; message?: string } | undefined
/// @resolution.name source=Options target=Options

function run(fields: Fields): void {
/// @type.symbol symbol=run type=(Fields) => void
/// @type.symbol symbol=run.fields source="fields: Fields" type=Fields
/// @resolution.name source=Fields target=Fields

    log({ fields, message: "hello" });
    /// @resolution.name source=log target=log
    /// @resolution.call source="log({ fields, message: \"hello\" })" parameters=({ fields?: Fields; message?: string } | undefined) arguments=(provided({ fields, message: "hello" }) as { fields?: Fields; message?: string } | undefined) return=void kind=symbol target=log
    /// @resolution.name source=fields target=run.fields
    /// @resolution.place source=fields placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=fields root=run.fields

}
"#,
        r#"
"#,
    );
}

/// A class passes itself to a bounded free function beside a generic argument.
#[test]
fn test_pass_a_local_receiver_to_a_bounded_free_function() {
    let session = TestSession::single(
        r#"
class Var<T: Copy> {
    fallback: T;

    constructor(fallback: T) {
        this.fallback = fallback;
    }
}

class Context {
    get<T: Copy>(variable: Var<T>): T {
        return read(this, variable, variable.fallback) ?? never();
    }
}

function read<T: Copy>(context: Context, variable: Var<T>, fallback: T): T | undefined {
    fallback
}

function never(): never {
    never()
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Var<in out T: Copy> {
    fallback: T;

    constructor(fallback: T) {
        this.fallback = fallback;
    }
}

class Context {
    get<T: Copy>(variable: Var<T>): T {
        return read<T>(this, variable, variable.fallback) ?? never();
    }
}

function read<T: Copy>(context: Context, variable: Var<T>, fallback: T): T | undefined {
    fallback as T | undefined
}

function never(): never {
    never()
}

=== dir ===
class Var<T: Copy> {
/// @generic.template symbol=Var parameters=(in out T#1: Copy)
/// @type.symbol symbol=Var type=typeof Var
/// @definition.class symbol=Var template=(in out T#1: Copy)
/// @definition.field symbol=Var.fallback source="fallback: T" key=fallback type=T#1
/// @definition.method symbol=Var.constructor slot=constructor role=constructor type=(this: &'managed Var<T#1>, T#1) => Var<T#1>
/// @type.symbol symbol=Var.T source="T: Copy" type=T#1
/// @resolution.name source=Copy target=Copy

    fallback: T;
    /// @type.symbol symbol=Var.fallback source="fallback: T" type=T#1
    /// @resolution.name source=T target=Var.T

    constructor(fallback: T) {
    /// @type.symbol symbol=Var.constructor type=(this: &'managed Var<T#1>, T#1) => Var<T#1>
    /// @type.symbol symbol=Var.constructor.this type=&'managed Var<T#1>
    /// @type.symbol symbol=Var.constructor.fallback source="fallback: T" type=T#1
    /// @resolution.name source=T target=Var.T

        this.fallback = fallback;
        /// @resolution.receiver source=this kind=this declaration=Var type=&'managed Var<T#1>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.fallback kind=place
        /// @resolution.place source=this.fallback placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.fallback root=this keys=[fallback]
        /// @resolution.assignment source=this.fallback write="receiver=&'managed Var<T#1>, target=field(receiver=&'managed Var<T#1>, target=Var.fallback, type=T#1), type=T#1" type=T#1
        /// @resolution.name source=fallback target=Var.constructor.fallback
        /// @resolution.place source=fallback placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=fallback root=Var.constructor.fallback

    }
}

class Context {
/// @type.symbol symbol=Context type=typeof Context
/// @definition.class symbol=Context
/// @definition.method symbol=Context.get slot=get type=<T#2: Copy>(this: Context, Var<T#2>) => T#2

    get<T: Copy>(variable: Var<T>): T {
    /// @generic.template symbol=Context.get parameters=(T#2: Copy)
    /// @type.symbol symbol=Context.get type=<T#2: Copy>(this: Context, Var<T#2>) => T#2
    /// @type.symbol symbol=Context.get.this type=Context
    /// @type.symbol symbol=Context.get.T source="T: Copy" type=T#2
    /// @resolution.name source=Copy target=Copy
    /// @type.symbol symbol=Context.get.variable source="variable: Var<T>" type=Var<T#2>
    /// @resolution.name source=Var target=Var
    /// @resolution.name source=T target=Context.get.T
    /// @resolution.name source=T target=Context.get.T

        return read(this, variable, variable.fallback) ?? never();
        /// @resolution.name source=read target=read
        /// @resolution.call source="read(this, variable, variable.fallback)" parameters=(Context, Var<T#2>, T#2) arguments=(provided(this) as Context, provided(variable) as Var<T#2>, provided(variable.fallback) as T#2) return=T#2 | undefined kind=symbol target=read instance=read<T#2>
        /// @resolution.operator source="read(this, variable, variable.fallback) ?? never()" type=T#2 operator="??" kind=builtin operands=[read(this, variable, variable.fallback) as T#2 | undefined, never() as never]
        /// @generic.instantiation id=read<T#2> template=read arguments=(T#2) owner=Context.get
        /// @resolution.receiver source=this kind=this declaration=Context type=Context
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.name source=variable target=Context.get.variable
        /// @resolution.place source=variable placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=variable root=Context.get.variable
        /// @resolution.name source=variable target=Context.get.variable
        /// @resolution.member source=variable.fallback receiver=Var<T#2> type=T#2 kind=field target_receiver=Var<T#2> key=fallback target=Var.fallback target_type=T#2
        /// @resolution.place source=variable placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=variable root=Context.get.variable
        /// @resolution.place source=variable.fallback placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=variable.fallback root=Context.get.variable keys=[fallback]
        /// @resolution.name source=never target=never
        /// @resolution.call source=never() parameters=() return=never kind=symbol target=never

    }
}

function read<T: Copy>(context: Context, variable: Var<T>, fallback: T): T | undefined {
/// @generic.template symbol=read parameters=(T#3: Copy)
/// @type.symbol symbol=read type=<T#3: Copy>(Context, Var<T#3>, T#3) => T#3 | undefined
/// @type.symbol symbol=read.T source="T: Copy" type=T#3
/// @resolution.name source=Copy target=Copy
/// @type.symbol symbol=read.context source="context: Context" type=Context
/// @resolution.name source=Context target=Context
/// @type.symbol symbol=read.variable source="variable: Var<T>" type=Var<T#3>
/// @resolution.name source=Var target=Var
/// @resolution.name source=T target=read.T
/// @type.symbol symbol=read.fallback source="fallback: T" type=T#3
/// @resolution.name source=T target=read.T
/// @resolution.name source=T target=read.T

    fallback
    /// @resolution.name source=fallback target=read.fallback
    /// @resolution.place source=fallback placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=fallback root=read.fallback

}

function never(): never {
/// @type.symbol symbol=never type=() => never

    never()
    /// @resolution.name source=never target=never
    /// @resolution.call source=never() parameters=() return=never kind=symbol target=never

}
"#,
        r#"
"#,
    );
}

/// A local generic class implements an interface returning its argument.
#[test]
fn test_implement_an_interface_on_a_local_generic_class() {
    let session = TestSession::single(
        r#"
interface Await<T> {
    wait(): T;
}

class Ready<T: Copy> implements Await<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }

    wait(): T {
        this.value
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Await<out T> {
    wait(): T;
}

class Ready<in out T: Copy> implements Await<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }

    wait(): T {
        this.value
    }
}

=== dir ===
interface Await<T> {
/// @generic.template symbol=Await parameters=(out T#1, this: Await<T#1>)
/// @type.symbol symbol=Await type=Await
/// @definition.interface symbol=Await template=(out T#1, this: Await<T#1>)
/// @definition.where symbol=Await relation=satisfies left=this right=Await<T#1>
/// @definition.method symbol=Await.wait source="wait(): T" slot=wait type=() => T#1
/// @type.symbol symbol=Await.T source=T type=T#1

    wait(): T;
    /// @type.symbol symbol=Await.wait source="wait(): T" type=() => T#1
    /// @resolution.name source=T target=Await.T

}

class Ready<T: Copy> implements Await<T> {
/// @generic.template symbol=Ready parameters=(in out T#2: Copy)
/// @type.symbol symbol=Ready type=typeof Ready
/// @definition.class symbol=Ready template=(in out T#2: Copy)
/// @definition.where symbol=Ready source=Await<T> relation=satisfies left=this right=Await<T#2>
/// @definition.implements symbol=Ready source=Await<T> target=Await<T#2>
/// @definition.field symbol=Ready.value source="value: T" key=value type=T#2
/// @definition.method symbol=Ready.constructor slot=constructor role=constructor type=(this: &'managed Ready<T#2>, T#2) => Ready<T#2>
/// @definition.method symbol=Ready.wait slot=wait type=(this: Ready<T#2>) => T#2
/// @definition.conformance symbol=Ready member=Ready.wait requirement=Await.wait
/// @type.symbol symbol=Ready.T source="T: Copy" type=T#2
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=Await target=Await
/// @resolution.name source=T target=Ready.T

    value: T;
    /// @type.symbol symbol=Ready.value source="value: T" type=T#2
    /// @resolution.name source=T target=Ready.T

    constructor(value: T) {
    /// @type.symbol symbol=Ready.constructor type=(this: &'managed Ready<T#2>, T#2) => Ready<T#2>
    /// @type.symbol symbol=Ready.constructor.this type=&'managed Ready<T#2>
    /// @type.symbol symbol=Ready.constructor.value source="value: T" type=T#2
    /// @resolution.name source=T target=Ready.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Ready type=&'managed Ready<T#2>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Ready<T#2>, target=field(receiver=&'managed Ready<T#2>, target=Ready.value, type=T#2), type=T#2" type=T#2
        /// @resolution.name source=value target=Ready.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Ready.constructor.value

    }

    wait(): T {
    /// @type.symbol symbol=Ready.wait type=(this: Ready<T#2>) => T#2
    /// @type.symbol symbol=Ready.wait.this type=Ready<T#2>
    /// @resolution.name source=T target=Ready.T

        this.value
        /// @resolution.member source=this.value receiver=Ready<T#2> type=T#2 kind=field target_receiver=Ready<T#2> key=value target=Ready.value target_type=T#2
        /// @resolution.receiver source=this kind=this declaration=Ready type=Ready<T#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}
"#,
        r#"
"#,
    );
}

/// A generic cell returns its argument handle through a readonly receiver.
#[test]
fn test_take_a_handle_from_a_generic_cell_through_a_readonly_receiver() {
    let session = TestSession::single(
        r#"
class Cell<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }

    replace(value: T): T {
        let previous = this.value;
        this.value = value;
        previous
    }
}

class Request {}

class Queue {
    active: Cell<Request | undefined>;

    take(&readonly this): Request {
        let request = this.active.replace(undefined);
        if (request == undefined) {
            return new Request();
        }
        request
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Cell<in out T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }

    replace(value: T): T {
        let previous: T = this.value;
        this.value = value;
        previous
    }
}

class Request {}

class Queue {
    active: Cell<Request | undefined>;

    take(&readonly this): Request {
        let request: Request | undefined = this.active.replace<Request | undefined>(
            undefined as Request | undefined,
        );
        if (request == undefined) {
            return new Request();
        }
        request
    }
}

=== dir ===
class Cell<T> {
/// @generic.template symbol=Cell parameters=(in out T)
/// @type.symbol symbol=Cell type=typeof Cell
/// @definition.class symbol=Cell template=(in out T)
/// @definition.field symbol=Cell.value source="value: T" key=value type=T
/// @definition.method symbol=Cell.constructor slot=constructor role=constructor type=(this: &'managed Cell<T>, T) => Cell<T>
/// @definition.method symbol=Cell.replace slot=replace type=(this: Cell<T>, T) => T
/// @type.symbol symbol=Cell.T source=T type=T

    value: T;
    /// @type.symbol symbol=Cell.value source="value: T" type=T
    /// @resolution.name source=T target=Cell.T

    constructor(value: T) {
    /// @type.symbol symbol=Cell.constructor type=(this: &'managed Cell<T>, T) => Cell<T>
    /// @type.symbol symbol=Cell.constructor.this type=&'managed Cell<T>
    /// @type.symbol symbol=Cell.constructor.value source="value: T" type=T
    /// @resolution.name source=T target=Cell.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Cell type=&'managed Cell<T>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Cell<T>, target=field(receiver=&'managed Cell<T>, target=Cell.value, type=T), type=T" type=T
        /// @resolution.name source=value target=Cell.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Cell.constructor.value

    }

    replace(value: T): T {
    /// @type.symbol symbol=Cell.replace type=(this: Cell<T>, T) => T
    /// @type.symbol symbol=Cell.replace.this type=Cell<T>
    /// @type.symbol symbol=Cell.replace.value source="value: T" type=T
    /// @resolution.name source=T target=Cell.T
    /// @resolution.name source=T target=Cell.T

        let previous = this.value;
        /// @type.symbol symbol=Cell.replace.previous source=previous type=T
        /// @resolution.pattern source=previous kind=binding target=Cell.replace.previous
        /// @resolution.member source=this.value receiver=Cell<T> type=T kind=field target_receiver=Cell<T> key=value target=Cell.value target_type=T
        /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.access source=this.value root=this keys=[value]

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=Cell<T>, target=field(receiver=Cell<T>, target=Cell.value, type=T), type=T" type=T
        /// @resolution.name source=value target=Cell.replace.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Cell.replace.value

        previous
        /// @resolution.name source=previous target=Cell.replace.previous
        /// @resolution.place source=previous placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=previous root=Cell.replace.previous

    }
}

class Request {}
/// @type.symbol symbol=Request source="class Request {}" type=typeof Request
/// @definition.class symbol=Request source="class Request {}"

class Queue {
/// @type.symbol symbol=Queue type=typeof Queue
/// @definition.class symbol=Queue
/// @definition.field symbol=Queue.active source="active: Cell<Request | undefined>" key=active type=Cell<Request | undefined>
/// @definition.method symbol=Queue.take slot=take type=<Queue.take.'a>(this: &Queue.take.'a readonly Queue) => Request

    active: Cell<Request | undefined>;
    /// @type.symbol symbol=Queue.active source="active: Cell<Request | undefined>" type=Cell<Request | undefined>
    /// @resolution.name source=Cell target=Cell
    /// @resolution.name source=Request target=Request

    take(&readonly this): Request {
    /// @generic.template symbol=Queue.take parameters=('a)
    /// @type.symbol symbol=Queue.take type=<Queue.take.'a>(this: &Queue.take.'a readonly Queue) => Request
    /// @type.symbol symbol=Queue.take.this source="&readonly this" type=&Queue.take.'a readonly Queue
    /// @resolution.name source=Request target=Request

        let request = this.active.replace(undefined);
        /// @type.symbol symbol=Queue.take.request source=request type=Request | undefined
        /// @resolution.pattern source=request kind=binding target=Queue.take.request
        /// @resolution.member source=this.active receiver=&Queue.take.'a readonly Queue type=Cell<Request | undefined> kind=field target_receiver=&Queue.take.'a readonly Queue key=active target=Queue.active target_type=Cell<Request | undefined>
        /// @resolution.member source=this.active.replace receiver=Cell<Request | undefined> type=(this: Cell<Request | undefined>, Request | undefined) => Request | undefined kind=symbol target_receiver=Cell<Request | undefined> target=Cell.replace
        /// @resolution.call source=this.active.replace(undefined) parameters=(Request | undefined) arguments=(provided(undefined) as Request | undefined) return=Request | undefined kind=symbol target=Cell.replace receiver=Cell<Request | undefined> instance="Cell<Request | undefined>.replace"
        /// @resolution.receiver source=this kind=this declaration=Queue type=&Queue.take.'a readonly Queue
        /// @resolution.place source=this placement=Queue.take.'a lifetime=Queue.take.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.active placement="local" lifetime=Queue.take.'a access="readonly"
        /// @resolution.access source=this.active root=this keys=[active]
        /// @generic.instantiation id="Cell.replace<Request | undefined>" template=Cell.replace arguments=(Request | undefined)

        if (request == undefined) {
        /// @resolution.name source=request target=Queue.take.request
        /// @resolution.operator source="request == undefined" type=boolean operator="==" kind=builtin operands=[request as Request | undefined, undefined as undefined families=(undefined)]
        /// @resolution.place source=request placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=request root=Queue.take.request

            return new Request();
            /// @resolution.construct source="new Request()" parameters=() return=Request kind=class target=Request constructor=default
            /// @resolution.name source=Request target=Request

        }
        request
        /// @resolution.name source=request target=Queue.take.request
        /// @resolution.place source=request placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=request root=Queue.take.request
        /// @resolution.narrowing source=request union=Request | undefined arms=Request

    }
}
"#,
        r#"
/// @diagnostic.error id=field-not-definitely-initialized message="field 'active' is not initialized on every constructor path"
/// @diagnostic.label line=19 column=5 span="active" line_source="active: Cell<Request | undefined>;"
"#,
    );
}

/// A field handle flows into a method of a narrowed optional field.
#[test]
fn test_pass_a_field_handle_to_a_narrowed_optional_field_method() {
    let session = TestSession::single(
        r#"
class Listener {}

class State {
    remove(listener: Listener): void {}
}

class Registration {
    state?: State;
    listener: Listener;

    constructor(listener: Listener) {
        this.listener = listener;
    }

    dispose(this): void {
        if (this.state != undefined) {
            this.state.remove(this.listener);
        }
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Listener {}

class State {
    remove(listener: Listener): void {}
}

class Registration {
    state?: State;
    listener: Listener;

    constructor(listener: Listener) {
        this.listener = listener;
    }

    dispose(this): void {
        if (this.state != undefined) {
            this.state.remove(this.listener);
        }
    }
}

=== dir ===
class Listener {}
/// @type.symbol symbol=Listener source="class Listener {}" type=typeof Listener
/// @definition.class symbol=Listener source="class Listener {}"

class State {
/// @type.symbol symbol=State type=typeof State
/// @definition.class symbol=State
/// @definition.method symbol=State.remove source="remove(listener: Listener): void {}" slot=remove type=(this: State, Listener) => void

    remove(listener: Listener): void {}
    /// @type.symbol symbol=State.remove source="remove(listener: Listener): void {}" type=(this: State, Listener) => void
    /// @type.symbol symbol=State.remove.this type=State
    /// @type.symbol symbol=State.remove.listener source="listener: Listener" type=Listener
    /// @resolution.name source=Listener target=Listener

}

class Registration {
/// @type.symbol symbol=Registration type=typeof Registration
/// @definition.class symbol=Registration
/// @definition.field symbol=Registration.listener source="listener: Listener" key=listener type=Listener
/// @definition.field symbol=Registration.state source="state?: State" key=state type=State
/// @definition.method symbol=Registration.constructor slot=constructor role=constructor type=(this: &'managed Registration, Listener) => Registration
/// @definition.method symbol=Registration.dispose slot=dispose type=(this: Registration) => void

    state?: State;
    /// @type.symbol symbol=Registration.state source="state?: State" type=State
    /// @resolution.name source=State target=State

    listener: Listener;
    /// @type.symbol symbol=Registration.listener source="listener: Listener" type=Listener
    /// @resolution.name source=Listener target=Listener

    constructor(listener: Listener) {
    /// @type.symbol symbol=Registration.constructor type=(this: &'managed Registration, Listener) => Registration
    /// @type.symbol symbol=Registration.constructor.this type=&'managed Registration
    /// @type.symbol symbol=Registration.constructor.listener source="listener: Listener" type=Listener
    /// @resolution.name source=Listener target=Listener

        this.listener = listener;
        /// @resolution.receiver source=this kind=this declaration=Registration type=&'managed Registration
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.listener kind=place
        /// @resolution.place source=this.listener placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.listener root=this keys=[listener]
        /// @resolution.assignment source=this.listener write="receiver=&'managed Registration, target=field(receiver=&'managed Registration, target=Registration.listener, type=Listener), type=Listener" type=Listener
        /// @resolution.name source=listener target=Registration.constructor.listener
        /// @resolution.place source=listener placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=listener root=Registration.constructor.listener

    }

    dispose(this): void {
    /// @type.symbol symbol=Registration.dispose type=(this: Registration) => void
    /// @type.symbol symbol=Registration.dispose.this source=this type=Registration

        if (this.state != undefined) {
        /// @resolution.member source=this.state receiver=Registration type=State | undefined kind=field target_receiver=Registration key=state target=Registration.state target_type=State | undefined
        /// @resolution.operator source="this.state != undefined" type=boolean operator="!=" kind=builtin operands=[this.state as State | undefined, undefined as undefined families=(undefined)]
        /// @resolution.receiver source=this kind=this declaration=Registration type=Registration
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.state placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.state root=this keys=[state]

            this.state.remove(this.listener);
            /// @resolution.member source=this.state receiver=Registration type=State | undefined kind=field target_receiver=Registration key=state target=Registration.state target_type=State | undefined
            /// @resolution.member source=this.state.remove receiver=State type=(this: State, Listener) => void kind=symbol target_receiver=State target=State.remove
            /// @resolution.call source=this.state.remove(this.listener) parameters=(Listener) arguments=(provided(this.listener) as Listener) return=void kind=symbol target=State.remove receiver=State
            /// @resolution.receiver source=this kind=this declaration=Registration type=Registration
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @resolution.place source=this.state placement="local" lifetime="managed" access="mutable"
            /// @resolution.access source=this.state root=this keys=[state]
            /// @resolution.narrowing source=this.state union=State | undefined arms=State
            /// @resolution.member source=this.listener receiver=Registration type=Listener kind=field target_receiver=Registration key=listener target=Registration.listener target_type=Listener
            /// @resolution.receiver source=this kind=this declaration=Registration type=Registration
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @resolution.place source=this.listener placement="local" lifetime="managed" access="mutable"
            /// @resolution.access source=this.listener root=this keys=[listener]

        }
    }
}
"#,
        r#"
"#,
    );
}

/// A narrowed once-callable parameter returns its result at the declared place.
#[test]
fn test_call_a_narrowed_once_callable_parameter() {
    let session = TestSession::single(
        r#"
function evaluate(message: string | ^Function<(), string, "once">): string {
    if (message is ^Function<(), string, "once">) {
        return message();
    }
    message
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function evaluate(message: string | ^(() => string)): string {
    if (message is ^Function<(), string, "once">) {
        return message();
    }
    message
}

=== dir ===
function evaluate(message: string | ^Function<(), string, "once">): string {
/// @type.symbol symbol=evaluate type=(string | ^Function<(), string, "once">) => string
/// @type.symbol symbol=evaluate.message source="message: string | ^Function<(), string, \"once\">" type=string | ^Function<(), string, "once">
/// @resolution.name source=Function target=Function

    if (message is ^Function<(), string, "once">) {
    /// @resolution.name source=message target=evaluate.message
    /// @resolution.guard source="message is ^Function<(), string, \"once\">" kind=is value=string | ^Function<(), string, "once"> target=^Function<(), string, "once"> predicate="string | ^Function<(), string, \"once\"> is type(^Function<(), string, \"once\">)" narrowed=Narrow<string | ^Function<(), string, "once">, ^Function<(), string, "once">>
    /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=message root=evaluate.message
    /// @resolution.name source=Function target=Function

        return message();
        /// @resolution.name source=message target=evaluate.message
        /// @resolution.call source=message() parameters=() return=string kind=expression target=expression
        /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=message root=evaluate.message
        /// @resolution.narrowing source=message union=string | ^Function<(), string, "once"> arms=^Function<(), string, "once">

    }
    message
    /// @resolution.name source=message target=evaluate.message
    /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=message root=evaluate.message
    /// @resolution.narrowing source=message union=string | ^Function<(), string, "once"> arms=string

}
"#,
        r#"
"#,
    );
}

/// A template with several spans flows into a readonly string borrow.
#[test]
fn test_borrow_a_multi_span_template_string_readonly() {
    let session = TestSession::single(
        r#"
function show(message: &readonly string): never {
    show(message)
}

function run(actual: string, expected: string): void {
    show(`actual: ${actual}\nexpected: ${expected}`);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function show<'a>(message: &'a readonly string): never {
    show<'a>(message)
}

function run(actual: string, expected: string): void {
    show<"managed">(`actual: ${actual}\nexpected: ${expected}` as &'managed readonly string);
}

=== dir ===
function show(message: &readonly string): never {
/// @generic.template symbol=show parameters=('a)
/// @type.symbol symbol=show type=<show.'a>(&show.'a readonly string) => never
/// @type.symbol symbol=show.message source="message: &readonly string" type=&show.'a readonly string

    show(message)
    /// @resolution.name source=show target=show
    /// @resolution.call source=show(message) parameters=(&show.'a readonly string) arguments=(provided(message) as &show.'a readonly string) return=never regions=(show.'a) kind=symbol target=show instance=show<show.'a>
    /// @generic.instantiation id=show<show.'a> template=show arguments=(show.'a)
    /// @resolution.name source=message target=show.message
    /// @resolution.place source=message placement=show.'a lifetime=show.'a access="readonly"
    /// @resolution.access source=message root=show.message

}

function run(actual: string, expected: string): void {
/// @type.symbol symbol=run type=(string, string) => void
/// @type.symbol symbol=run.actual source="actual: string" type=string
/// @type.symbol symbol=run.expected source="expected: string" type=string

    show(`actual: ${actual}\nexpected: ${expected}`);
    /// @resolution.name source=show target=show
    /// @resolution.call source="show(`actual: ${actual}\\nexpected: ${expected}`)" parameters=(&'managed readonly string) arguments=(provided(`actual: ${actual}\nexpected: ${expected}`) as &'managed readonly string) return=never regions=("managed" & "local") kind=symbol target=show instance="show<\"managed\" & \"local\">"
    /// @generic.instantiation id="show<\"managed\" & \"local\">" template=show arguments=("managed" & "local")
    /// @resolution.template source="`actual: ${actual}\\nexpected: ${expected}`" spans=[Display.display(parameters=(), arguments=(), return=^string, regions=("managed" & "local")), Display.display(parameters=(), arguments=(), return=^string, regions=("managed" & "local"))] build="stringFromTemplate(parameters=(&'frame readonly Slice<string>, &'frame readonly Slice<string>), arguments=(supplied(0) as &'frame readonly Slice<string>, supplied(1) as &'frame readonly Slice<string>), return=string, regions=(\"frame\", \"frame\"))"
    /// @generic.instantiation id="Display.display<string, \"managed\" & \"local\">" template=Display.display arguments=("managed" & "local")
    /// @generic.instantiation id="stringFromTemplate<\"frame\", \"frame\">" template=stringFromTemplate arguments=("frame", "frame")
    /// @resolution.name source=actual target=run.actual
    /// @resolution.place source=actual placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=actual root=run.actual
    /// @resolution.name source=expected target=run.expected
    /// @resolution.place source=expected placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=expected root=run.expected

}
"#,
        r#"
"#,
    );
}

/// An optional alias-typed once-callable parameter narrows and returns at the declared place.
#[test]
fn test_call_a_narrowed_optional_alias_typed_once_callable() {
    let session = TestSession::single(
        r#"
type Message = string | ^Function<(), string, "once">;

function evaluate(message: Message | undefined, fallback: string): string {
    if (message is ^Function<(), string, "once">) {
        return message();
    }
    message ?? fallback
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Message = string | ^Function<(), string, "once">;

function evaluate(message: string | ^(() => string) | undefined, fallback: string): string {
    if (message is ^Function<(), string, "once">) {
        return message();
    }
    message ?? fallback
}

=== dir ===
type Message = string | ^Function<(), string, "once">;
/// @type.symbol symbol=Message source="type Message = string | ^Function<(), string, \"once\">" type=string | ^Function<(), string, "once">
/// @definition.type symbol=Message source="type Message = string | ^Function<(), string, \"once\">" value=string | ^Function<(), string, "once">
/// @resolution.name source=Function target=Function

function evaluate(message: Message | undefined, fallback: string): string {
/// @type.symbol symbol=evaluate type=(string | ^Function<(), string, "once"> | undefined, string) => string
/// @type.symbol symbol=evaluate.message source="message: Message | undefined" type=string | ^Function<(), string, "once"> | undefined
/// @resolution.name source=Message target=Message
/// @type.symbol symbol=evaluate.fallback source="fallback: string" type=string

    if (message is ^Function<(), string, "once">) {
    /// @resolution.name source=message target=evaluate.message
    /// @resolution.guard source="message is ^Function<(), string, \"once\">" kind=is value=string | ^Function<(), string, "once"> | undefined target=^Function<(), string, "once"> predicate="string | ^Function<(), string, \"once\"> | undefined is type(^Function<(), string, \"once\">)" narrowed=Narrow<string | ^Function<(), string, "once"> | undefined, ^Function<(), string, "once">>
    /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=message root=evaluate.message
    /// @resolution.name source=Function target=Function

        return message();
        /// @resolution.name source=message target=evaluate.message
        /// @resolution.call source=message() parameters=() return=string kind=expression target=expression
        /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=message root=evaluate.message
        /// @resolution.narrowing source=message union=string | ^Function<(), string, "once"> | undefined arms=^Function<(), string, "once">

    }
    message ?? fallback
    /// @resolution.name source=message target=evaluate.message
    /// @resolution.operator source="message ?? fallback" type=string operator="??" kind=builtin operands=[message as string | undefined families=(string | undefined), fallback as string families=(string)]
    /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=message root=evaluate.message
    /// @resolution.narrowing source=message union=string | ^Function<(), string, "once"> | undefined arms=string | undefined
    /// @resolution.name source=fallback target=evaluate.fallback
    /// @resolution.place source=fallback placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=fallback root=evaluate.fallback

}
"#,
        r#"
"#,
    );
}

/// A template string flows into an ambient function's readonly string borrow.
#[test]
fn test_borrow_a_template_string_for_an_ambient_function() {
    let session = TestSession::single(
        r#"
declare function show(message: &readonly string): never;

function run(actual: string): void {
    show(`actual: ${actual}`);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function show<'a>(message: &readonly string): never;

function run(actual: string): void {
    show<"managed">(`actual: ${actual}` as &'managed readonly string);
}

=== dir ===
declare function show(message: &readonly string): never;
/// @generic.template symbol=show parameters=('a)
/// @type.symbol symbol=show source="declare function show(message: &readonly string): never" type=<show.'a>(&show.'a readonly string) => never

function run(actual: string): void {
/// @type.symbol symbol=run type=(string) => void
/// @type.symbol symbol=run.actual source="actual: string" type=string

    show(`actual: ${actual}`);
    /// @resolution.name source=show target=show
    /// @resolution.call source="show(`actual: ${actual}`)" parameters=(&'managed readonly string) arguments=(provided(`actual: ${actual}`) as &'managed readonly string) return=never regions=("managed" & "local") kind=symbol target=show instance="show<\"managed\" & \"local\">"
    /// @generic.instantiation id="show<\"managed\" & \"local\">" template=show arguments=("managed" & "local")
    /// @resolution.template source="`actual: ${actual}`" spans=[Display.display(parameters=(), arguments=(), return=^string, regions=("managed" & "local"))] build="stringFromTemplate(parameters=(&'frame readonly Slice<string>, &'frame readonly Slice<string>), arguments=(supplied(0) as &'frame readonly Slice<string>, supplied(1) as &'frame readonly Slice<string>), return=string, regions=(\"frame\", \"frame\"))"
    /// @generic.instantiation id="Display.display<string, \"managed\" & \"local\">" template=Display.display arguments=("managed" & "local")
    /// @generic.instantiation id="stringFromTemplate<\"frame\", \"frame\">" template=stringFromTemplate arguments=("frame", "frame")
    /// @resolution.name source=actual target=run.actual
    /// @resolution.place source=actual placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=actual root=run.actual

}
"#,
        r#"
"#,
    );
}

/// A string literal flows into string parameters.
#[test]
fn test_pass_a_string_literal_to_string_parameters() {
    let session = TestSession::single(
        r#"
function plain(name: string): void {}

function optional(name?: string): void {}

function run(): void {
    plain("x");
    optional("y");
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function plain(name: string): void {}

function optional(name?: string): void {}

function run(): void {
    plain("x");
    optional("y" as string | undefined);
}

=== dir ===
function plain(name: string): void {}
/// @type.symbol symbol=plain source="function plain(name: string): void {}" type=(string) => void
/// @type.symbol symbol=plain.name source="name: string" type=string

function optional(name?: string): void {}
/// @type.symbol symbol=optional source="function optional(name?: string): void {}" type=(string | undefined?) => void
/// @type.symbol symbol=optional.name source="name?: string" type=string | undefined

function run(): void {
/// @type.symbol symbol=run type=() => void

    plain("x");
    /// @resolution.name source=plain target=plain
    /// @resolution.call source="plain(\"x\")" parameters=(string) arguments=(provided("x") as string) return=void kind=symbol target=plain

    optional("y");
    /// @resolution.name source=optional target=optional
    /// @resolution.call source="optional(\"y\")" parameters=(string | undefined) arguments=(provided("y") as string | undefined) return=void kind=symbol target=optional

}
"#,
        r#"
"#,
    );
}

/// A string literal flows into an imported ambient function's optional string parameter.
#[test]
fn test_pass_a_string_literal_to_an_imported_ambient_parameter() {
    let session = TestSession::builder()
        .module(
            "lib.ds",
            r#"
export declare function todo(message?: string): never;
"#,
        )
        .module(
            "main.ds",
            r#"
import { todo } from "./lib.ds";

declare function local_todo(message?: string): never;

function run(): void {
    local_todo("x");
    todo("y");
}
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { todo } from "./lib.ds";

declare function local_todo(message?: string): never;

function run(): void {
    local_todo("x" as string | undefined);
    todo("y" as string | undefined);
}

=== dir ===
import { todo } from "./lib.ds";

declare function local_todo(message?: string): never;
/// @type.symbol symbol=local_todo source="declare function local_todo(message?: string): never" type=(string | undefined?) => never

function run(): void {
/// @type.symbol symbol=run type=() => void

    local_todo("x");
    /// @resolution.name source=local_todo target=local_todo
    /// @resolution.call source="local_todo(\"x\")" parameters=(string | undefined) arguments=(provided("x") as string | undefined) return=never kind=symbol target=local_todo

    todo("y");
    /// @resolution.name source=todo target=lib.todo
    /// @resolution.call source="todo(\"y\")" parameters=(string | undefined) arguments=(provided("y") as string | undefined) return=never kind=symbol target=lib.todo

}
"#,
        r#"
"#,
    );
}

/// A string literal flows into the library's ambient todo parameter.
#[test]
fn test_pass_a_string_literal_to_the_library_todo() {
    let session = TestSession::single(
        r#"
import { todo } from "destack:error";

function run(): never {
    todo("x")
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { todo } from "destack:error";

function run(): never {
    todo("x" as string | undefined)
}

=== dir ===
import { todo } from "destack:error";

function run(): never {
/// @type.symbol symbol=run type=() => never

    todo("x")
    /// @resolution.name source=todo target=todo
    /// @resolution.call source="todo(\"x\")" parameters=(string | undefined) arguments=(provided("x") as string | undefined) return=never kind=symbol target=todo

}
"#,
        r#"
"#,
    );
}

/// A string literal flows into the library's ambient todo parameter from a cold session.
#[test]
fn test_pass_a_string_literal_to_the_library_todo_in_a_cold_session() {
    let session = TestSession::builder()
        .cold()
        .module(
            "main.ds",
            r#"
import { todo } from "destack:error";

function run(): never {
    todo("x")
}
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { todo } from "destack:error";

function run(): never {
    todo("x" as string | undefined)
}

=== dir ===
import { todo } from "destack:error";

function run(): never {
/// @type.symbol symbol=run type=() => never

    todo("x")
    /// @resolution.name source=todo target=todo
    /// @resolution.call source="todo(\"x\")" parameters=(string | undefined) arguments=(provided("x") as string | undefined) return=never kind=symbol target=todo

}
"#,
        r#"
"#,
    );
}

/// The library's utf8 module checks verbatim as a user module.
#[test]
fn test_check_the_library_utf8_module_as_a_user_module() {
    let session = TestSession::builder()
        .cold()
        .module(
            "main.ds",
            r####"
import { Bytes } from "destack:bytes";
import { Error, Result, todo } from "destack:error";

/// Byte decoding failure.
@languageItem("string.Utf8DecodeError")
export struct Utf8DecodeError {
    /// The error discriminator.
    kind: "invalidUtf8" = "invalidUtf8";

    /// The byte offset where decoding failed.
    offset: usize;

    /// The error message.
    message: string;
}

export extension of Utf8DecodeError implements Error {
    /// Format this error for users.
    display(&immutable this): ^string {
        this.message
    }
}

/// Encode text as UTF-8 bytes.
export function encodeUtf8(text: string): Bytes {
    todo("string.encodeUtf8")
}

/// Decode UTF-8 bytes as text.
export function decodeUtf8(bytes: &readonly [uint8]): Result<^string, Utf8DecodeError> {
    todo("string.decodeUtf8")
}

"####,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Bytes } from "destack:bytes";
import { Error, Result, todo } from "destack:error";

/// Byte decoding failure.
@languageItem("string.Utf8DecodeError")
export struct Utf8DecodeError {
    /// The error discriminator.
    kind: "invalidUtf8" = "invalidUtf8";

    /// The byte offset where decoding failed.
    offset: usize;

    /// The error message.
    message: string;
}

export extension of Utf8DecodeError implements Error {
    /// Format this error for users.
    display(&immutable this): ^string {
        this.message
    }
}

/// Encode text as UTF-8 bytes.
export function encodeUtf8(text: string): Bytes {
    todo("string.encodeUtf8" as string | undefined)
}

/// Decode UTF-8 bytes as text.
export function decodeUtf8<'a>(bytes: &'a readonly [uint8]): Result<^string, Utf8DecodeError> {
    todo("string.decodeUtf8" as string | undefined)
}

=== dir ===
import { Bytes } from "destack:bytes";
import { Error, Result, todo } from "destack:error";

/// Byte decoding failure.
@languageItem("string.Utf8DecodeError")
/// @resolution.name source=languageItem target=languageItem

export struct Utf8DecodeError {
/// @type.symbol symbol=Utf8DecodeError type=Utf8DecodeError
/// @definition.struct symbol=Utf8DecodeError
/// @definition.field symbol=Utf8DecodeError.kind source="kind: \"invalidUtf8\" = \"invalidUtf8\"" key=kind type="invalidUtf8"
/// @definition.field symbol=Utf8DecodeError.message source="message: string" key=message type=string
/// @definition.field symbol=Utf8DecodeError.offset source="offset: usize" key=offset type=usize

    /// The error discriminator.
    kind: "invalidUtf8" = "invalidUtf8";
    /// @type.symbol symbol=Utf8DecodeError.kind source="kind: \"invalidUtf8\" = \"invalidUtf8\"" type="invalidUtf8"

    /// The byte offset where decoding failed.
    offset: usize;
    /// @type.symbol symbol=Utf8DecodeError.offset source="offset: usize" type=usize

    /// The error message.
    message: string;
    /// @type.symbol symbol=Utf8DecodeError.message source="message: string" type=string

}

export extension of Utf8DecodeError implements Error {
/// @definition.extension symbol=<module>#2 form=exported target=Utf8DecodeError
/// @definition.implements symbol=<module>#2 source=Error target=Error
/// @definition.method symbol=display slot=display type=<display.'a>(this: &display.'a immutable Utf8DecodeError) => ^string
/// @definition.conformance symbol=<module>#2 member=Error.source requirement=Error.source
/// @definition.conformance symbol=<module>#2 member=display requirement=Error.display
/// @resolution.name source=Utf8DecodeError target=Utf8DecodeError
/// @resolution.name source=Error target=Error

    /// Format this error for users.
    display(&immutable this): ^string {
    /// @generic.template symbol=display parent=template#0 parameters=('a)
    /// @type.symbol symbol=display type=<display.'a>(this: &display.'a immutable Utf8DecodeError) => ^string
    /// @type.symbol symbol=display.this source="&immutable this" type=&display.'a immutable Utf8DecodeError

        this.message
        /// @resolution.member source=this.message receiver=&display.'a immutable Utf8DecodeError type=string kind=field target_receiver=&display.'a immutable Utf8DecodeError key=message target=Utf8DecodeError.message target_type=string
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&display.'a immutable Utf8DecodeError
        /// @resolution.place source=this placement=display.'a lifetime=display.'a access="immutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.message placement=display.'a lifetime=display.'a access="immutable"
        /// @resolution.access source=this.message root=this keys=[message]

    }
}

/// Encode text as UTF-8 bytes.
export function encodeUtf8(text: string): Bytes {
/// @type.symbol symbol=encodeUtf8 type=(string) => Bytes
/// @type.symbol symbol=encodeUtf8.text source="text: string" type=string
/// @resolution.name source=Bytes target=Bytes

    todo("string.encodeUtf8")
    /// @resolution.name source=todo target=todo
    /// @resolution.call source="todo(\"string.encodeUtf8\")" parameters=(string | undefined) arguments=(provided("string.encodeUtf8") as string | undefined) return=never kind=symbol target=todo

}

/// Decode UTF-8 bytes as text.
export function decodeUtf8(bytes: &readonly [uint8]): Result<^string, Utf8DecodeError> {
/// @generic.template symbol=decodeUtf8 parameters=('a)
/// @type.symbol symbol=decodeUtf8 type=<decodeUtf8.'a>(&decodeUtf8.'a readonly Slice<uint8>) => Result<^string, Utf8DecodeError>
/// @type.symbol symbol=decodeUtf8.bytes source="bytes: &readonly [uint8]" type=&decodeUtf8.'a readonly Slice<uint8>
/// @resolution.name source=Result target=Result
/// @resolution.name source=Utf8DecodeError target=Utf8DecodeError

    todo("string.decodeUtf8")
    /// @resolution.name source=todo target=todo
    /// @resolution.call source="todo(\"string.decodeUtf8\")" parameters=(string | undefined) arguments=(provided("string.decodeUtf8") as string | undefined) return=never kind=symbol target=todo

}
"#,
        r#"
/// @diagnostic.error id=return-not-assignable message="type 'string' is not assignable to the declared result type '^string'"
/// @diagnostic.label line=20 column=39 span="{\n        this.message\n    }" line_source="display(&immutable this): ^string {"
"#,
    );
}

/// A mutable string binding flows into an optional string parameter.
#[test]
fn test_pass_a_mutable_string_binding_to_an_optional_string_parameter() {
    let session = TestSession::single(
        r#"
function optional(name?: string): void {}

function run(): void {
    let name = "x";
    name = "y";
    optional(name);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function optional(name?: string): void {}

function run(): void {
    let name: string = "x";
    name = "y";
    optional(name as string | undefined);
}

=== dir ===
function optional(name?: string): void {}
/// @type.symbol symbol=optional source="function optional(name?: string): void {}" type=(string | undefined?) => void
/// @type.symbol symbol=optional.name source="name?: string" type=string | undefined

function run(): void {
/// @type.symbol symbol=run type=() => void

    let name = "x";
    /// @type.symbol symbol=run.name source=name type=string
    /// @resolution.pattern source=name kind=binding target=run.name

    name = "y";
    /// @resolution.name source=name target=run.name
    /// @resolution.pattern.assign source=name kind=place
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=run.name
    /// @resolution.assignment source=name write=binding(run.name) type=string

    optional(name);
    /// @resolution.name source=optional target=optional
    /// @resolution.call source=optional(name) parameters=(string | undefined) arguments=(provided(name) as string | undefined) return=void kind=symbol target=optional
    /// @resolution.name source=name target=run.name
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=run.name

}
"#,
        r#"
"#,
    );
}

/// A string literal lends constant storage to a readonly borrow parameter.
#[test]
fn test_borrow_a_string_literal_readonly() {
    let session = TestSession::single(
        r#"
function show(message: &readonly string): void {}

function run(): void {
    show("hello");
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function show<'a>(message: &'a readonly string): void {}

function run(): void {
    show<"managed">("hello" as &'managed readonly string);
}

=== dir ===
function show(message: &readonly string): void {}
/// @generic.template symbol=show parameters=('a)
/// @type.symbol symbol=show source="function show(message: &readonly string): void {}" type=<show.'a>(&show.'a readonly string) => void
/// @type.symbol symbol=show.message source="message: &readonly string" type=&show.'a readonly string

function run(): void {
/// @type.symbol symbol=run type=() => void

    show("hello");
    /// @resolution.name source=show target=show
    /// @resolution.call source="show(\"hello\")" parameters=(&'managed readonly string) arguments=(provided("hello") as &'managed readonly string) return=void regions=("managed" & "local") kind=symbol target=show instance="show<\"managed\" & \"local\">"
    /// @generic.instantiation id="show<\"managed\" & \"local\">" template=show arguments=("managed" & "local")

}
"#,
        r#"
"#,
    );
}

/// A string literal lends constant storage to an imported ambient function's borrow parameter.
#[test]
fn test_borrow_a_string_literal_for_an_imported_ambient_function() {
    let session = TestSession::builder()
        .module(
            "show.ds",
            "export declare function show(message: &readonly string): never;\n",
        )
        .module(
            "main.ds",
            r#"
import { show } from "./show.ds";

function run(): void {
    show("hello");
}
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { show } from "./show.ds";

function run(): void {
    show<"managed">("hello" as &'managed readonly string);
}

=== dir ===
import { show } from "./show.ds";

function run(): void {
/// @type.symbol symbol=run type=() => void

    show("hello");
    /// @resolution.name source=show target=show.show
    /// @resolution.call source="show(\"hello\")" parameters=(&'managed readonly string) arguments=(provided("hello") as &'managed readonly string) return=never regions=("managed" & "local") kind=symbol target=show.show instance="show.show<\"managed\" & \"local\">"
    /// @generic.instantiation id="show.show<\"managed\" & \"local\">" template=show.show arguments=("managed" & "local")

}
"#,
        r#"
"#,
    );
}

/// A string literal lends constant storage to a borrow parameter from a generic body.
#[test]
fn test_borrow_a_string_literal_readonly_from_a_generic_body() {
    let session = TestSession::single(
        r#"
declare function show(message: &readonly string): never;

function first<T>(values: &immutable [T]): &immutable T {
    if (values.size == 0) {
        show("empty");
    }

    &immutable values[0]
}

extension<T> of [T] {
    head(&immutable this): &immutable T {
        if (this.size == 0) {
            show("empty");
        }

        &immutable this[0]
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function show<'a>(message: &readonly string): never;

function first<T, 'a>(values: &'a immutable [T]): &'a immutable T {
    if (values.size == 0) {
        show<"managed">("empty" as &'managed readonly string);
    }

    &immutable values[0]
}

extension<T> of [T] {
    head(&immutable this): &'a immutable T {
        if (this.size == 0) {
            show<"managed">("empty" as &'managed readonly string);
        }

        &immutable this[0]
    }
}

=== dir ===
declare function show(message: &readonly string): never;
/// @generic.template symbol=show parameters=('a)
/// @type.symbol symbol=show source="declare function show(message: &readonly string): never" type=<show.'a>(&show.'a readonly string) => never

function first<T>(values: &immutable [T]): &immutable T {
/// @generic.template symbol=first parameters=(T#1, 'a)
/// @type.symbol symbol=first type=<T#1, first.'a>(&first.'a immutable Slice<T#1>) => &first.'a immutable T#1
/// @type.symbol symbol=first.T source=T type=T#1
/// @type.symbol symbol=first.values source="values: &immutable [T]" type=&first.'a immutable Slice<T#1>
/// @resolution.name source=T target=first.T
/// @resolution.name source=T target=first.T

    if (values.size == 0) {
    /// @resolution.name source=values target=first.values
    /// @resolution.member source=values.size receiver=&first.'a immutable Slice<T#1> type=isize kind=call target="size(parameters=(), arguments=(), return=isize, regions=(first.'a))"
    /// @resolution.operator source="values.size == 0" type=boolean operator="==" kind=builtin operands=[values.size as isize families=(integer), 0 as isize families=(integer)]
    /// @resolution.place source=values placement=first.'a lifetime=first.'a access="immutable"
    /// @resolution.access source=values root=first.values
    /// @generic.instantiation id="size<T#1, first.'a>" template=size arguments=(T#1, first.'a) owner=first

        show("empty");
        /// @resolution.name source=show target=show
        /// @resolution.call source="show(\"empty\")" parameters=(&'managed readonly string) arguments=(provided("empty") as &'managed readonly string) return=never regions=("managed" & "local") kind=symbol target=show instance="show<\"managed\" & \"local\">"
        /// @generic.instantiation id="show<\"managed\" & \"local\">" template=show arguments=("managed" & "local") owner=first

    }

    &immutable values[0]
    /// @resolution.name source=values target=first.values
    /// @resolution.place source=values placement=first.'a lifetime=first.'a access="immutable"
    /// @resolution.access source=values root=first.values
    /// @resolution.place source=values[0] placement=first.'a lifetime=first.'a access="immutable"
    /// @resolution.access source=values[0] root=first.values keys=[0]
    /// @resolution.subscript source=values[0] type=T#1 kind=call target="index#3(parameters=(isize), arguments=(provided(0) as isize), return=&first.'a immutable T#1, regions=(first.'a))"
    /// @generic.instantiation id="index#3<T#1, \"immutable\", first.'a>" template=index#3 arguments=(T#1, "immutable", first.'a) owner=first

}

extension<T> of [T] {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Slice<T#2>
/// @definition.method symbol=head slot=head type=<head.'a>(this: &head.'a immutable Slice<T#2>) => &head.'a immutable T#2
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=T target=T

    head(&immutable this): &immutable T {
    /// @generic.template symbol=head parent=template#1 parameters=('a)
    /// @type.symbol symbol=head type=<head.'a>(this: &head.'a immutable Slice<T#2>) => &head.'a immutable T#2
    /// @type.symbol symbol=head.this source="&immutable this" type=&head.'a immutable Slice<T#2>
    /// @resolution.name source=T target=T

        if (this.size == 0) {
        /// @resolution.member source=this.size receiver=&head.'a immutable Slice<T#2> type=isize kind=call target="size(parameters=(), arguments=(), return=isize, regions=(head.'a))"
        /// @resolution.operator source="this.size == 0" type=boolean operator="==" kind=builtin operands=[this.size as isize families=(integer), 0 as isize families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&head.'a immutable Slice<T#2>
        /// @resolution.place source=this placement=head.'a lifetime=head.'a access="immutable"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id="size<T#2, head.'a>" template=size arguments=(T#2, head.'a) owner=head

            show("empty");
            /// @resolution.name source=show target=show
            /// @resolution.call source="show(\"empty\")" parameters=(&'managed readonly string) arguments=(provided("empty") as &'managed readonly string) return=never regions=("managed" & "local") kind=symbol target=show instance="show<\"managed\" & \"local\">"
            /// @generic.instantiation id="show<\"managed\" & \"local\">" template=show arguments=("managed" & "local") owner=head

        }

        &immutable this[0]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&head.'a immutable Slice<T#2>
        /// @resolution.place source=this placement=head.'a lifetime=head.'a access="immutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this[0] placement=head.'a lifetime=head.'a access="immutable"
        /// @resolution.access source=this[0] root=this keys=[0]
        /// @resolution.subscript source=this[0] type=T#2 kind=call target="index#3(parameters=(isize), arguments=(provided(0) as isize), return=&head.'a immutable T#2, regions=(head.'a))"
        /// @generic.instantiation id="index#3<T#2, \"immutable\", head.'a>" template=index#3 arguments=(T#2, "immutable", head.'a) owner=head

    }
}
"#,
        r#"
"#,
    );
}

/// A narrowed once callable behind an exported alias returns a handle the caller keeps.
#[test]
fn test_call_a_narrowed_once_callable_through_an_exported_alias() {
    let session = TestSession::single(
        r#"
export type Message = string | ^Function<(), string, "once">;

function evaluate(message: Message | undefined, fallback: string): string {
    if (message is ^Function<(), string, "once">) {
        return message();
    }

    message ?? fallback
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
export type Message = string | ^Function<(), string, "once">;

function evaluate(message: string | ^(() => string) | undefined, fallback: string): string {
    if (message is ^Function<(), string, "once">) {
        return message();
    }

    message ?? fallback
}

=== dir ===
export type Message = string | ^Function<(), string, "once">;
/// @type.symbol symbol=Message source="export type Message = string | ^Function<(), string, \"once\">" type=string | ^Function<(), string, "once">
/// @definition.type symbol=Message source="export type Message = string | ^Function<(), string, \"once\">" value=string | ^Function<(), string, "once">
/// @resolution.name source=Function target=Function

function evaluate(message: Message | undefined, fallback: string): string {
/// @type.symbol symbol=evaluate type=(string | ^Function<(), string, "once"> | undefined, string) => string
/// @type.symbol symbol=evaluate.message source="message: Message | undefined" type=string | ^Function<(), string, "once"> | undefined
/// @resolution.name source=Message target=Message
/// @type.symbol symbol=evaluate.fallback source="fallback: string" type=string

    if (message is ^Function<(), string, "once">) {
    /// @resolution.name source=message target=evaluate.message
    /// @resolution.guard source="message is ^Function<(), string, \"once\">" kind=is value=string | ^Function<(), string, "once"> | undefined target=^Function<(), string, "once"> predicate="string | ^Function<(), string, \"once\"> | undefined is type(^Function<(), string, \"once\">)" narrowed=Narrow<string | ^Function<(), string, "once"> | undefined, ^Function<(), string, "once">>
    /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=message root=evaluate.message
    /// @resolution.name source=Function target=Function

        return message();
        /// @resolution.name source=message target=evaluate.message
        /// @resolution.call source=message() parameters=() return=string kind=expression target=expression
        /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=message root=evaluate.message
        /// @resolution.narrowing source=message union=string | ^Function<(), string, "once"> | undefined arms=^Function<(), string, "once">

    }

    message ?? fallback
    /// @resolution.name source=message target=evaluate.message
    /// @resolution.operator source="message ?? fallback" type=string operator="??" kind=builtin operands=[message as string | undefined families=(string | undefined), fallback as string families=(string)]
    /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=message root=evaluate.message
    /// @resolution.narrowing source=message union=string | ^Function<(), string, "once"> | undefined arms=string | undefined
    /// @resolution.name source=fallback target=evaluate.fallback
    /// @resolution.place source=fallback placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=fallback root=evaluate.fallback

}
"#,
        r#"
"#,
    );
}

/// A narrowed once callable behind an imported alias returns a handle the caller keeps.
#[test]
fn test_call_a_narrowed_once_callable_through_an_imported_alias() {
    let session = TestSession::builder()
        .module(
            "message.ds",
            r#"
export type Message = string | ^Function<(), string, "once">;
"#,
        )
        .module(
            "main.ds",
            r#"
import { Message } from "./message";

function evaluate(message: Message | undefined, fallback: string): string {
    if (message is ^Function<(), string, "once">) {
        return message();
    }

    message ?? fallback
}
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Message } from "./message";

function evaluate(message: string | ^(() => string) | undefined, fallback: string): string {
    if (message is ^Function<(), string, "once">) {
        return message();
    }

    message ?? fallback
}

=== dir ===
import { Message } from "./message";

function evaluate(message: Message | undefined, fallback: string): string {
/// @type.symbol symbol=evaluate type=(string | ^Function<(), string, "once"> | undefined, string) => string
/// @type.symbol symbol=evaluate.message source="message: Message | undefined" type=string | ^Function<(), string, "once"> | undefined
/// @resolution.name source=Message target=message.Message
/// @type.symbol symbol=evaluate.fallback source="fallback: string" type=string

    if (message is ^Function<(), string, "once">) {
    /// @resolution.name source=message target=evaluate.message
    /// @resolution.guard source="message is ^Function<(), string, \"once\">" kind=is value=string | ^Function<(), string, "once"> | undefined target=^Function<(), string, "once"> predicate="string | ^Function<(), string, \"once\"> | undefined is type(^Function<(), string, \"once\">)" narrowed=Narrow<string | ^Function<(), string, "once"> | undefined, ^Function<(), string, "once">>
    /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=message root=evaluate.message
    /// @resolution.name source=Function target=Function

        return message();
        /// @resolution.name source=message target=evaluate.message
        /// @resolution.call source=message() parameters=() return=string kind=expression target=expression
        /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=message root=evaluate.message
        /// @resolution.narrowing source=message union=string | ^Function<(), string, "once"> | undefined arms=^Function<(), string, "once">

    }

    message ?? fallback
    /// @resolution.name source=message target=evaluate.message
    /// @resolution.operator source="message ?? fallback" type=string operator="??" kind=builtin operands=[message as string | undefined families=(string | undefined), fallback as string families=(string)]
    /// @resolution.place source=message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=message root=evaluate.message
    /// @resolution.narrowing source=message union=string | ^Function<(), string, "once"> | undefined arms=string | undefined
    /// @resolution.name source=fallback target=evaluate.fallback
    /// @resolution.place source=fallback placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=fallback root=evaluate.fallback

}
"#,
        r#"
"#,
    );
}

/// A shared class implements a requirement returning `this` in the class's own space.
#[test]
fn test_implement_a_this_returning_requirement_on_a_shared_class() {
    let session = TestSession::single(
        r#"
import { Iterable } from "destack:iter";

interface Gather<T> {
    static gather<I: Iterable<T>>(values: I): this;
}

shared class Bucket<T> {
    static gather<I: Iterable<T>>(values: I): Bucket<T> {
        Bucket.gather(values)
    }
}

export extension<T> of Bucket<T> implements Gather<T> {
    static gather<I: Iterable<T>>(values: I): Bucket<T> {
        Bucket.gather(values)
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

interface Gather<T> {
    static gather<I: Iterable<T>>(values: I): this;
}

shared class Bucket<T> {
    static gather<I: Iterable<T>>(values: I): Bucket<T> {
        Bucket.gather<T, I>(values)
    }
}

export extension<T> of Bucket<T> implements Gather<T> {
    static gather<I: Iterable<T>>(values: I): Bucket<T> {
        Bucket.gather<T, I>(values)
    }
}

=== dir ===
import { Iterable } from "destack:iter";

interface Gather<T> {
/// @generic.template symbol=Gather parameters=(T#1, this: Gather<T#1>)
/// @type.symbol symbol=Gather type=Gather
/// @definition.interface symbol=Gather template=(T#1, this: Gather<T#1>)
/// @definition.where symbol=Gather relation=satisfies left=this right=Gather<T#1>
/// @definition.method symbol=Gather.gather source="static gather<I: Iterable<T>>(values: I): this" slot=gather static=true type=<I#1: Iterable<T#1>>(I#1) => this
/// @type.symbol symbol=Gather.T source=T type=T#1

    static gather<I: Iterable<T>>(values: I): this;
    /// @generic.template symbol=Gather.gather parent=template#0 parameters=(I#1: Iterable<T#1>)
    /// @type.symbol symbol=Gather.gather source="static gather<I: Iterable<T>>(values: I): this" type=<I#1: Iterable<T#1>>(I#1) => this
    /// @type.symbol symbol=Gather.gather.I source="I: Iterable<T>" type=I#1
    /// @resolution.name source=Iterable target=Iterable
    /// @resolution.name source=T target=Gather.T
    /// @type.symbol symbol=Gather.gather.values source="values: I" type=I#1
    /// @resolution.name source=I target=Gather.gather.I

}

shared class Bucket<T> {
/// @generic.template symbol=Bucket parameters=(T#2)
/// @type.symbol symbol=Bucket type=typeof Bucket
/// @definition.class symbol=Bucket template=(T#2)
/// @definition.method symbol=Bucket.gather slot=gather static=true type=<I#2: Iterable<T#2>>(I#2) => Bucket<T#2>
/// @type.symbol symbol=Bucket.T source=T type=T#2

    static gather<I: Iterable<T>>(values: I): Bucket<T> {
    /// @generic.template symbol=Bucket.gather parent=template#1 parameters=(I#2: Iterable<T#2>)
    /// @type.symbol symbol=Bucket.gather type=<I#2: Iterable<T#2>>(I#2) => Bucket<T#2>
    /// @type.symbol symbol=Bucket.gather.I source="I: Iterable<T>" type=I#2
    /// @resolution.name source=Iterable target=Iterable
    /// @resolution.name source=T target=Bucket.T
    /// @type.symbol symbol=Bucket.gather.values source="values: I" type=I#2
    /// @resolution.name source=I target=Bucket.gather.I
    /// @resolution.name source=Bucket target=Bucket
    /// @resolution.name source=T target=Bucket.T

        Bucket.gather(values)
        /// @resolution.name source=Bucket target=Bucket
        /// @resolution.member source=Bucket.gather receiver=typeof Bucket type=<I#2: Iterable<T#2>>(I#2) => Bucket<T#2> kind=symbol target_receiver=typeof Bucket target=Bucket.gather
        /// @resolution.call source=Bucket.gather(values) parameters=(I#2) arguments=(provided(values) as I#2) return=Bucket<T#2> kind=symbol target=Bucket.gather instance=Bucket<T#2>.gather<I#2>
        /// @generic.instantiation id="Bucket.gather<T#2, I#2>" template=Bucket.gather arguments=(T#2, I#2) owner=Bucket.gather
        /// @resolution.name source=values target=Bucket.gather.values
        /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=values root=Bucket.gather.values

    }
}

export extension<T> of Bucket<T> implements Gather<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @definition.extension symbol=<module>#2 form=exported target=Bucket<T#3>
/// @definition.implements symbol=<module>#2 source=Gather<T> target=Gather<T#3>
/// @definition.method symbol=gather slot=gather static=true type=<I#3: Iterable<T#3>>(I#3) => Bucket<T#3>
/// @definition.conformance symbol=<module>#2 member=gather requirement=Gather.gather
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Bucket target=Bucket
/// @resolution.name source=T target=T
/// @resolution.name source=Gather target=Gather
/// @resolution.name source=T target=T

    static gather<I: Iterable<T>>(values: I): Bucket<T> {
    /// @generic.template symbol=gather parent=template#2 parameters=(I#3: Iterable<T#3>)
    /// @type.symbol symbol=gather type=<I#3: Iterable<T#3>>(I#3) => Bucket<T#3>
    /// @type.symbol symbol=gather.I source="I: Iterable<T>" type=I#3
    /// @resolution.name source=Iterable target=Iterable
    /// @resolution.name source=T target=T
    /// @type.symbol symbol=gather.values source="values: I" type=I#3
    /// @resolution.name source=I target=gather.I
    /// @resolution.name source=Bucket target=Bucket
    /// @resolution.name source=T target=T

        Bucket.gather(values)
        /// @resolution.name source=Bucket target=Bucket
        /// @resolution.member source=Bucket.gather receiver=typeof Bucket type=<I#2: Iterable<T#2>>(I#2) => Bucket<T#2> kind=symbol target_receiver=typeof Bucket target=Bucket.gather
        /// @resolution.call source=Bucket.gather(values) parameters=(I#3) arguments=(provided(values) as I#3) return=Bucket<T#3> kind=symbol target=Bucket.gather instance=Bucket<T#3>.gather<I#3>
        /// @generic.instantiation id="Bucket.gather<T#3, I#3>" template=Bucket.gather arguments=(T#3, I#3) owner=gather
        /// @resolution.name source=values target=gather.values
        /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=values root=gather.values

    }
}
"#,
        r#"
"#,
    );
}

/// A closure capturing the constructor's receiver flows into a callback parameter.
#[test]
fn test_pass_a_receiver_capturing_closure_to_a_callback_parameter() {
    let session = TestSession::single(
        r#"
class Deferred<T> {
    value: T | undefined;

    constructor(executor: (resolve: (value: T) => void) => void) {
        this.value = undefined;

        let resolve = (value: T) => {
            Deferred.settle(this, value);
        };

        executor(resolve);
    }

    static settle(target: Deferred<T>, value: T): void {
        target.value = value;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Deferred<in out T> {
    value: T | undefined;

    constructor(executor: (resolve: (value: T) => void) => void) {
        this.value = undefined as T | undefined;

        let resolve: (value: T) => void = (value: T): void => {
            Deferred.settle<T>(this as Deferred<T>, value);
        };

        executor(resolve);
    }

    static settle(target: Deferred<T>, value: T): void {
        target.value = value as T | undefined;
    }
}

=== dir ===
class Deferred<T> {
/// @generic.template symbol=Deferred parameters=(in out T)
/// @type.symbol symbol=Deferred type=typeof Deferred
/// @definition.class symbol=Deferred template=(in out T)
/// @definition.field symbol=Deferred.value source="value: T | undefined" key=value type=T | undefined
/// @definition.method symbol=Deferred.constructor slot=constructor role=constructor type=(this: &'managed Deferred<T>, ((T) => void) => void) => Deferred<T>
/// @definition.method symbol=Deferred.settle slot=settle static=true type=(Deferred<T>, T) => void
/// @type.symbol symbol=Deferred.T source=T type=T

    value: T | undefined;
    /// @type.symbol symbol=Deferred.value source="value: T | undefined" type=T | undefined
    /// @resolution.name source=T target=Deferred.T

    constructor(executor: (resolve: (value: T) => void) => void) {
    /// @type.symbol symbol=Deferred.constructor type=(this: &'managed Deferred<T>, ((T) => void) => void) => Deferred<T>
    /// @type.symbol symbol=Deferred.constructor.this type=&'managed Deferred<T>
    /// @type.symbol symbol=Deferred.constructor.executor source="executor: (resolve: (value: T) => void) => void" type=((T) => void) => void
    /// @type.symbol symbol=Deferred.constructor.resolve#1 source="resolve: (value: T) => void" type=(T) => void
    /// @type.symbol symbol=Deferred.constructor.value source="value: T" type=T
    /// @resolution.name source=T target=Deferred.T

        this.value = undefined;
        /// @resolution.receiver source=this kind=this declaration=Deferred type=&'managed Deferred<T>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Deferred<T>, target=field(receiver=&'managed Deferred<T>, target=Deferred.value, type=T | undefined), type=T | undefined" type=T | undefined

        let resolve = (value: T) => {
        /// @type.symbol symbol=Deferred.constructor.resolve#2 source=resolve type=Function<(T,), void, "readonly">
        /// @resolution.pattern source=resolve kind=binding target=Deferred.constructor.resolve#2
        /// @type.symbol symbol=Deferred.constructor.symbol10 type=Function<(T,), void, "readonly">
        /// @type.symbol symbol=Deferred.constructor.symbol10.value source="value: T" type=T
        /// @resolution.name source=T target=Deferred.T

            Deferred.settle(this, value);
            /// @resolution.name source=Deferred target=Deferred
            /// @resolution.member source=Deferred.settle receiver=typeof Deferred type=(Deferred<T>, T) => void kind=symbol target_receiver=typeof Deferred target=Deferred.settle
            /// @resolution.call source="Deferred.settle(this, value)" parameters=(Deferred<T>, T) arguments=(provided(this) as Deferred<T>, provided(value) as T) return=void kind=symbol target=Deferred.settle instance=Deferred<T>.settle
            /// @generic.instantiation id=Deferred.settle<T> template=Deferred.settle arguments=(T) owner=Deferred.constructor
            /// @resolution.name source=this target=Deferred.constructor.this
            /// @resolution.receiver source=this kind=this declaration=Deferred type=&'managed Deferred<T>
            /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
            /// @resolution.access source=this root=this
            /// @resolution.name source=value target=Deferred.constructor.symbol10.value
            /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=value root=Deferred.constructor.symbol10.value

        };

        executor(resolve);
        /// @resolution.name source=executor target=Deferred.constructor.executor
        /// @resolution.call source=executor(resolve) parameters=((T) => void) arguments=(provided(resolve) as (T) => void) return=void kind=expression target=expression
        /// @resolution.place source=executor placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=executor root=Deferred.constructor.executor
        /// @resolution.name source=resolve target=Deferred.constructor.resolve#2
        /// @resolution.place source=resolve placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=resolve root=Deferred.constructor.resolve#2

    }

    static settle(target: Deferred<T>, value: T): void {
    /// @type.symbol symbol=Deferred.settle type=(Deferred<T>, T) => void
    /// @type.symbol symbol=Deferred.settle.target source="target: Deferred<T>" type=Deferred<T>
    /// @resolution.name source=Deferred target=Deferred
    /// @resolution.name source=T target=Deferred.T
    /// @type.symbol symbol=Deferred.settle.value source="value: T" type=T
    /// @resolution.name source=T target=Deferred.T

        target.value = value;
        /// @resolution.name source=target target=Deferred.settle.target
        /// @resolution.place source=target placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=target root=Deferred.settle.target
        /// @resolution.pattern.assign source=target.value kind=place
        /// @resolution.place source=target.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=target.value root=Deferred.settle.target keys=[value]
        /// @resolution.assignment source=target.value write="receiver=Deferred<T>, target=field(receiver=Deferred<T>, target=Deferred.value, type=T | undefined), type=T | undefined" type=T | undefined
        /// @resolution.name source=value target=Deferred.settle.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Deferred.settle.value

    }
}
"#,
        r#"
"#,
    );
}

/// A struct constructor infers its class argument from an owned handle stored in a field.
#[test]
fn test_infer_a_struct_argument_from_an_owned_class_handle() {
    let session = TestSession::single(
        r#"
struct Ready<T> {
    value: T;
}

struct Slot<T> {
    value: T;

    static new(value: T): Slot<T> {
        Slot { value }
    }
}

class Queue<T> {
    static new(): ^Queue<T> {
        Queue.new()
    }
}

class State<T> {
    private readonly values: Slot<Queue<Ready<T>>>;

    constructor() {
        this.values = Slot.new(Queue.new());
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Ready<out T> {
    value: T;
}

struct Slot<out T> {
    value: T;

    static new(value: T): Slot<T> {
        Slot<T> { value }
    }
}

class Queue<T> {
    static new(): ^Queue<T> {
        Queue.new<T>()
    }
}

class State<T> {
    private readonly values: Slot<Queue<Ready<T>>>;

    constructor() {
        this.values = Slot.new<Queue<Ready<T>>>(Queue.new<Ready<T>>() as Queue<Ready<T>>);
    }
}

=== dir ===
struct Ready<T> {
/// @generic.template symbol=Ready parameters=(out T#1)
/// @type.symbol symbol=Ready type=Ready
/// @definition.struct symbol=Ready template=(out T#1)
/// @definition.field symbol=Ready.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Ready.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Ready.value source="value: T" type=T#1
    /// @resolution.name source=T target=Ready.T

}

struct Slot<T> {
/// @generic.template symbol=Slot parameters=(out T#2)
/// @type.symbol symbol=Slot type=Slot
/// @definition.struct symbol=Slot template=(out T#2)
/// @definition.field symbol=Slot.value source="value: T" key=value type=T#2
/// @definition.method symbol=Slot.new slot=new static=true type=(T#2) => Slot<T#2>
/// @type.symbol symbol=Slot.T source=T type=T#2

    value: T;
    /// @type.symbol symbol=Slot.value source="value: T" type=T#2
    /// @resolution.name source=T target=Slot.T

    static new(value: T): Slot<T> {
    /// @type.symbol symbol=Slot.new type=(T#2) => Slot<T#2>
    /// @type.symbol symbol=Slot.new.value source="value: T" type=T#2
    /// @resolution.name source=T target=Slot.T
    /// @resolution.name source=Slot target=Slot
    /// @resolution.name source=T target=Slot.T

        Slot { value }
        /// @resolution.name source=Slot target=Slot
        /// @resolution.name source=value target=Slot.new.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Slot.new.value

    }
}

class Queue<T> {
/// @generic.template symbol=Queue parameters=(T#3)
/// @type.symbol symbol=Queue type=typeof Queue
/// @definition.class symbol=Queue template=(T#3)
/// @definition.method symbol=Queue.new slot=new static=true type=() => ^Queue<T#3>
/// @type.symbol symbol=Queue.T source=T type=T#3

    static new(): ^Queue<T> {
    /// @type.symbol symbol=Queue.new type=() => ^Queue<T#3>
    /// @resolution.name source=Queue target=Queue
    /// @resolution.name source=T target=Queue.T

        Queue.new()
        /// @resolution.name source=Queue target=Queue
        /// @resolution.member source=Queue.new receiver=typeof Queue type=() => ^Queue<T#3> kind=symbol target_receiver=typeof Queue target=Queue.new
        /// @resolution.call source=Queue.new() parameters=() return=^Queue<T#3> kind=symbol target=Queue.new instance=Queue<T#3>.new
        /// @generic.instantiation id=Queue.new<T#3> template=Queue.new arguments=(T#3) owner=Queue.new

    }
}

class State<T> {
/// @generic.template symbol=State parameters=(T#4)
/// @type.symbol symbol=State type=typeof State
/// @definition.class symbol=State template=(T#4)
/// @definition.field symbol=State.values source="private readonly values: Slot<Queue<Ready<T>>>" key=values visibility=private type=Slot<Queue<Ready<T#4>>>
/// @definition.method symbol=State.constructor slot=constructor role=constructor type=(this: &'managed State<T#4>) => State<T#4>
/// @type.symbol symbol=State.T source=T type=T#4

    private readonly values: Slot<Queue<Ready<T>>>;
    /// @type.symbol symbol=State.values source="private readonly values: Slot<Queue<Ready<T>>>" type=Slot<Queue<Ready<T#4>>>
    /// @resolution.name source=Slot target=Slot
    /// @resolution.name source=Queue target=Queue
    /// @resolution.name source=Ready target=Ready
    /// @resolution.name source=T target=State.T

    constructor() {
    /// @type.symbol symbol=State.constructor type=(this: &'managed State<T#4>) => State<T#4>
    /// @type.symbol symbol=State.constructor.this type=&'managed State<T#4>

        this.values = Slot.new(Queue.new());
        /// @resolution.receiver source=this kind=this declaration=State type=&'managed State<T#4>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.values kind=place
        /// @resolution.place source=this.values placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.values root=this keys=[values]
        /// @resolution.assignment source=this.values write="receiver=&'managed State<T#4>, target=field(receiver=&'managed State<T#4>, target=State.values, type=Slot<Queue<Ready<T#4>>>), type=Slot<Queue<Ready<T#4>>>" type=Slot<Queue<Ready<T#4>>>
        /// @resolution.name source=Slot target=Slot
        /// @resolution.member source=Slot.new receiver=Slot type=(T#2) => Slot<T#2> kind=symbol target_receiver=Slot target=Slot.new
        /// @resolution.call source=Slot.new(Queue.new()) parameters=(Queue<Ready<T#4>>) arguments=(provided(Queue.new()) as Queue<Ready<T#4>>) return=Slot<Queue<Ready<T#4>>> kind=symbol target=Slot.new instance=Slot<Queue<Ready<T#4>>>.new
        /// @generic.instantiation id=Slot.new<Queue<Ready<T#4>>> template=Slot.new arguments=(Queue<Ready<T#4>>) owner=State.constructor
        /// @resolution.name source=Queue target=Queue
        /// @resolution.member source=Queue.new receiver=typeof Queue type=() => ^Queue<T#3> kind=symbol target_receiver=typeof Queue target=Queue.new
        /// @resolution.call source=Queue.new() parameters=() return=^Queue<Ready<T#4>> kind=symbol target=Queue.new instance=Queue<Ready<T#4>>.new
        /// @generic.instantiation id=Queue.new<Ready<T#4>> template=Queue.new arguments=(Ready<T#4>) owner=State.constructor

    }
}
"#,
        r#"
"#,
    );
}

/// A constructed local-space object flows into a parameter typed by a union alias of local classes.
#[test]
fn test_pass_a_constructed_local_object_to_a_union_alias_parameter() {
    let session = TestSession::single(
        r#"
class Fiber {
    static current(): Fiber {
        Fiber.current()
    }
}

type Waiter<T: Copy> = Awaiter<T> | Reaction<T>;

class Awaiter<T: Copy> {
    readonly fiber: Fiber;

    next: Waiter<T> | undefined;

    constructor(fiber: Fiber) {
        this.fiber = fiber;
        this.next = undefined;
    }
}

class Reaction<T: Copy> {
    readonly run: (value: T) => void;

    next: Waiter<T> | undefined;

    constructor(run: (value: T) => void) {
        this.run = run;
        this.next = undefined;
    }
}

class Deferred<T: Copy> {
    private head: Waiter<T> | undefined;

    constructor() {
        this.head = undefined;
    }

    static park<T: Copy>(deferred: Deferred<T>): void {
        deferred.addWaiter(new Awaiter(Fiber.current()));
    }

    private addReaction(run: (value: T) => void): void {
        this.addWaiter(new Reaction(run));
    }

    private addWaiter(waiter: Waiter<T>): void {
        this.head = waiter;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Fiber {
    static current(): Fiber {
        Fiber.current()
    }
}

type Waiter<T: Copy> = Awaiter<T> | Reaction<T>;

class Awaiter<T: Copy> {
    readonly fiber: Fiber;

    next: Waiter<T> | undefined;

    constructor(fiber: Fiber) {
        this.fiber = fiber;
        this.next = undefined as Awaiter<T> | Reaction<T> | undefined;
    }
}

class Reaction<in T: Copy> {
    readonly run: (value: T) => void;

    next: Waiter<T> | undefined;

    constructor(run: (value: T) => void) {
        this.run = run;
        this.next = undefined as Awaiter<T> | Reaction<T> | undefined;
    }
}

class Deferred<out T: Copy> {
    private head: Waiter<T> | undefined;

    constructor() {
        this.head = undefined as Awaiter<T> | Reaction<T> | undefined;
    }

    static park<T: Copy>(deferred: Deferred<T>): void {
        deferred.addWaiter<T>(new Awaiter<T>(Fiber.current()) as Waiter<T>);
    }

    private addReaction(run: (value: T) => void): void {
        this.addWaiter<T>(new Reaction<T>(run) as Waiter<T>);
    }

    private addWaiter(waiter: Waiter<T>): void {
        this.head = waiter as Awaiter<T> | Reaction<T> | undefined;
    }
}

=== dir ===
class Fiber {
/// @type.symbol symbol=Fiber type=typeof Fiber
/// @definition.class symbol=Fiber
/// @definition.method symbol=Fiber.current slot=current static=true type=() => Fiber

    static current(): Fiber {
    /// @type.symbol symbol=Fiber.current type=() => Fiber
    /// @resolution.name source=Fiber target=Fiber

        Fiber.current()
        /// @resolution.name source=Fiber target=Fiber
        /// @resolution.member source=Fiber.current receiver=typeof Fiber type=() => Fiber kind=symbol target_receiver=typeof Fiber target=Fiber.current
        /// @resolution.call source=Fiber.current() parameters=() return=Fiber kind=symbol target=Fiber.current

    }
}

type Waiter<T: Copy> = Awaiter<T> | Reaction<T>;
/// @generic.template symbol=Waiter parameters=(T#1: Copy)
/// @type.symbol symbol=Waiter source="type Waiter<T: Copy> = Awaiter<T> | Reaction<T>" type=Awaiter<T#1> | Reaction<T#1>
/// @definition.type symbol=Waiter source="type Waiter<T: Copy> = Awaiter<T> | Reaction<T>" template=(T#1: Copy) value=Awaiter<T#1> | Reaction<T#1>
/// @type.symbol symbol=Waiter.T source="T: Copy" type=T#1
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=Awaiter target=Awaiter
/// @resolution.name source=T target=Waiter.T
/// @resolution.name source=Reaction target=Reaction
/// @resolution.name source=T target=Waiter.T

class Awaiter<T: Copy> {
/// @generic.template symbol=Awaiter parameters=(T#2: Copy)
/// @type.symbol symbol=Awaiter type=typeof Awaiter
/// @definition.class symbol=Awaiter template=(T#2: Copy)
/// @definition.field symbol=Awaiter.fiber source="readonly fiber: Fiber" key=fiber type=Fiber
/// @definition.field symbol=Awaiter.next source="next: Waiter<T> | undefined" key=next type=Waiter<T#2> | undefined
/// @definition.method symbol=Awaiter.constructor slot=constructor role=constructor type=(this: &'managed Awaiter<T#2>, Fiber) => Awaiter<T#2>
/// @type.symbol symbol=Awaiter.T source="T: Copy" type=T#2
/// @resolution.name source=Copy target=Copy

    readonly fiber: Fiber;
    /// @type.symbol symbol=Awaiter.fiber source="readonly fiber: Fiber" type=Fiber
    /// @resolution.name source=Fiber target=Fiber

    next: Waiter<T> | undefined;
    /// @type.symbol symbol=Awaiter.next source="next: Waiter<T> | undefined" type=Waiter<T#2> | undefined
    /// @resolution.name source=Waiter target=Waiter
    /// @resolution.name source=T target=Awaiter.T

    constructor(fiber: Fiber) {
    /// @type.symbol symbol=Awaiter.constructor type=(this: &'managed Awaiter<T#2>, Fiber) => Awaiter<T#2>
    /// @type.symbol symbol=Awaiter.constructor.this type=&'managed Awaiter<T#2>
    /// @type.symbol symbol=Awaiter.constructor.fiber source="fiber: Fiber" type=Fiber
    /// @resolution.name source=Fiber target=Fiber

        this.fiber = fiber;
        /// @resolution.receiver source=this kind=this declaration=Awaiter type=&'managed Awaiter<T#2>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.fiber kind=place
        /// @resolution.place source=this.fiber placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.fiber root=this keys=[fiber]
        /// @resolution.assignment source=this.fiber write="receiver=&'managed Awaiter<T#2>, target=field(receiver=&'managed Awaiter<T#2>, target=Awaiter.fiber, type=Fiber), type=Fiber" type=Fiber
        /// @resolution.name source=fiber target=Awaiter.constructor.fiber
        /// @resolution.place source=fiber placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=fiber root=Awaiter.constructor.fiber

        this.next = undefined;
        /// @resolution.receiver source=this kind=this declaration=Awaiter type=&'managed Awaiter<T#2>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.next kind=place
        /// @resolution.place source=this.next placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.next root=this keys=[next]
        /// @resolution.assignment source=this.next write="receiver=&'managed Awaiter<T#2>, target=field(receiver=&'managed Awaiter<T#2>, target=Awaiter.next, type=Awaiter<T#2> | Reaction<T#2> | undefined), type=Awaiter<T#2> | Reaction<T#2> | undefined" type=Awaiter<T#2> | Reaction<T#2> | undefined

    }
}

class Reaction<T: Copy> {
/// @generic.template symbol=Reaction parameters=(in T#3: Copy)
/// @type.symbol symbol=Reaction type=typeof Reaction
/// @definition.class symbol=Reaction template=(in T#3: Copy)
/// @definition.field symbol=Reaction.next source="next: Waiter<T> | undefined" key=next type=Waiter<T#3> | undefined
/// @definition.field symbol=Reaction.run source="readonly run: (value: T) => void" key=run type=(T#3) => void
/// @definition.method symbol=Reaction.constructor slot=constructor role=constructor type=(this: &'managed Reaction<T#3>, (T#3) => void) => Reaction<T#3>
/// @type.symbol symbol=Reaction.T source="T: Copy" type=T#3
/// @resolution.name source=Copy target=Copy

    readonly run: (value: T) => void;
    /// @type.symbol symbol=Reaction.run source="readonly run: (value: T) => void" type=(T#3) => void
    /// @type.symbol symbol=Reaction.value source="value: T" type=T#3
    /// @resolution.name source=T target=Reaction.T

    next: Waiter<T> | undefined;
    /// @type.symbol symbol=Reaction.next source="next: Waiter<T> | undefined" type=Waiter<T#3> | undefined
    /// @resolution.name source=Waiter target=Waiter
    /// @resolution.name source=T target=Reaction.T

    constructor(run: (value: T) => void) {
    /// @type.symbol symbol=Reaction.constructor type=(this: &'managed Reaction<T#3>, (T#3) => void) => Reaction<T#3>
    /// @type.symbol symbol=Reaction.constructor.this type=&'managed Reaction<T#3>
    /// @type.symbol symbol=Reaction.constructor.run source="run: (value: T) => void" type=(T#3) => void
    /// @type.symbol symbol=Reaction.constructor.value source="value: T" type=T#3
    /// @resolution.name source=T target=Reaction.T

        this.run = run;
        /// @resolution.receiver source=this kind=this declaration=Reaction type=&'managed Reaction<T#3>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.run kind=place
        /// @resolution.place source=this.run placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.run root=this keys=[run]
        /// @resolution.assignment source=this.run write="receiver=&'managed Reaction<T#3>, target=field(receiver=&'managed Reaction<T#3>, target=Reaction.run, type=(T#3) => void), type=(T#3) => void" type=(T#3) => void
        /// @resolution.name source=run target=Reaction.constructor.run
        /// @resolution.place source=run placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=run root=Reaction.constructor.run

        this.next = undefined;
        /// @resolution.receiver source=this kind=this declaration=Reaction type=&'managed Reaction<T#3>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.next kind=place
        /// @resolution.place source=this.next placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.next root=this keys=[next]
        /// @resolution.assignment source=this.next write="receiver=&'managed Reaction<T#3>, target=field(receiver=&'managed Reaction<T#3>, target=Reaction.next, type=Awaiter<T#3> | Reaction<T#3> | undefined), type=Awaiter<T#3> | Reaction<T#3> | undefined" type=Awaiter<T#3> | Reaction<T#3> | undefined

    }
}

class Deferred<T: Copy> {
/// @generic.template symbol=Deferred parameters=(out T#4: Copy)
/// @type.symbol symbol=Deferred type=typeof Deferred
/// @definition.class symbol=Deferred template=(out T#4: Copy)
/// @definition.field symbol=Deferred.head source="private head: Waiter<T> | undefined" key=head visibility=private type=Waiter<T#4> | undefined
/// @definition.method symbol=Deferred.addReaction slot=addReaction visibility=private type=(this: Deferred<T#4>, (T#4) => void) => void
/// @definition.method symbol=Deferred.addWaiter slot=addWaiter visibility=private type=(this: Deferred<T#4>, Waiter<T#4>) => void
/// @definition.method symbol=Deferred.constructor slot=constructor role=constructor type=(this: &'managed Deferred<T#4>) => Deferred<T#4>
/// @definition.method symbol=Deferred.park slot=park static=true type=<T#5: Copy>(Deferred<T#5>) => void
/// @type.symbol symbol=Deferred.T source="T: Copy" type=T#4
/// @resolution.name source=Copy target=Copy

    private head: Waiter<T> | undefined;
    /// @type.symbol symbol=Deferred.head source="private head: Waiter<T> | undefined" type=Waiter<T#4> | undefined
    /// @resolution.name source=Waiter target=Waiter
    /// @resolution.name source=T target=Deferred.T

    constructor() {
    /// @type.symbol symbol=Deferred.constructor type=(this: &'managed Deferred<T#4>) => Deferred<T#4>
    /// @type.symbol symbol=Deferred.constructor.this type=&'managed Deferred<T#4>

        this.head = undefined;
        /// @resolution.receiver source=this kind=this declaration=Deferred type=&'managed Deferred<T#4>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.head kind=place
        /// @resolution.place source=this.head placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.head root=this keys=[head]
        /// @resolution.assignment source=this.head write="receiver=&'managed Deferred<T#4>, target=field(receiver=&'managed Deferred<T#4>, target=Deferred.head, type=Awaiter<T#4> | Reaction<T#4> | undefined), type=Awaiter<T#4> | Reaction<T#4> | undefined" type=Awaiter<T#4> | Reaction<T#4> | undefined

    }

    static park<T: Copy>(deferred: Deferred<T>): void {
    /// @generic.template symbol=Deferred.park parent=template#3 parameters=(T#5: Copy)
    /// @type.symbol symbol=Deferred.park type=<T#5: Copy>(Deferred<T#5>) => void
    /// @type.symbol symbol=Deferred.park.T source="T: Copy" type=T#5
    /// @resolution.name source=Copy target=Copy
    /// @type.symbol symbol=Deferred.park.deferred source="deferred: Deferred<T>" type=Deferred<T#5>
    /// @resolution.name source=Deferred target=Deferred
    /// @resolution.name source=T target=Deferred.park.T

        deferred.addWaiter(new Awaiter(Fiber.current()));
        /// @resolution.name source=deferred target=Deferred.park.deferred
        /// @resolution.member source=deferred.addWaiter receiver=Deferred<T#5> type=(this: Deferred<T#5>, Waiter<T#5>) => void kind=symbol target_receiver=Deferred<T#5> target=Deferred.addWaiter
        /// @resolution.call source="deferred.addWaiter(new Awaiter(Fiber.current()))" parameters=(Waiter<T#5>) arguments=(provided(new Awaiter(Fiber.current())) as Waiter<T#5>) return=void kind=symbol target=Deferred.addWaiter receiver=Deferred<T#5> instance=Deferred<T#5>.addWaiter
        /// @resolution.place source=deferred placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=deferred root=Deferred.park.deferred
        /// @generic.instantiation id=Deferred.addWaiter<T#5> template=Deferred.addWaiter arguments=(T#5) owner=Deferred.park
        /// @resolution.construct source="new Awaiter(Fiber.current())" parameters=(Fiber) arguments=(provided(Fiber.current()) as Fiber) return=Awaiter<T#5> kind=class target=Awaiter constructor=Awaiter.constructor instance=Awaiter<T#5>
        /// @generic.instantiation id=Awaiter.constructor<T#5> template=Awaiter.constructor arguments=(T#5) owner=Deferred.park
        /// @generic.instantiation id=Awaiter<T#5> template=Awaiter arguments=(T#5) owner=Deferred.park
        /// @resolution.name source=Awaiter target=Awaiter
        /// @resolution.name source=Fiber target=Fiber
        /// @resolution.member source=Fiber.current receiver=typeof Fiber type=() => Fiber kind=symbol target_receiver=typeof Fiber target=Fiber.current
        /// @resolution.call source=Fiber.current() parameters=() return=Fiber kind=symbol target=Fiber.current

    }

    private addReaction(run: (value: T) => void): void {
    /// @type.symbol symbol=Deferred.addReaction type=(this: Deferred<T#4>, (T#4) => void) => void
    /// @type.symbol symbol=Deferred.addReaction.this type=Deferred<T#4>
    /// @type.symbol symbol=Deferred.addReaction.run source="run: (value: T) => void" type=(T#4) => void
    /// @type.symbol symbol=Deferred.addReaction.value source="value: T" type=T#4
    /// @resolution.name source=T target=Deferred.T

        this.addWaiter(new Reaction(run));
        /// @resolution.member source=this.addWaiter receiver=Deferred<T#4> type=(this: Deferred<T#4>, Waiter<T#4>) => void kind=symbol target_receiver=Deferred<T#4> target=Deferred.addWaiter
        /// @resolution.call source="this.addWaiter(new Reaction(run))" parameters=(Waiter<T#4>) arguments=(provided(new Reaction(run)) as Waiter<T#4>) return=void kind=symbol target=Deferred.addWaiter receiver=Deferred<T#4> instance=Deferred<T#4>.addWaiter
        /// @resolution.receiver source=this kind=this declaration=Deferred type=Deferred<T#4>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=Deferred.addWaiter<T#4> template=Deferred.addWaiter arguments=(T#4) owner=Deferred.addReaction
        /// @resolution.construct source="new Reaction(run)" parameters=((T#4) => void) arguments=(provided(run) as (T#4) => void) return=Reaction<T#4> kind=class target=Reaction constructor=Reaction.constructor instance=Reaction<T#4>
        /// @generic.instantiation id=Reaction.constructor<T#4> template=Reaction.constructor arguments=(T#4) owner=Deferred.addReaction
        /// @generic.instantiation id=Reaction<T#4> template=Reaction arguments=(T#4) owner=Deferred.addReaction
        /// @resolution.name source=Reaction target=Reaction
        /// @resolution.name source=run target=Deferred.addReaction.run
        /// @resolution.place source=run placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=run root=Deferred.addReaction.run

    }

    private addWaiter(waiter: Waiter<T>): void {
    /// @type.symbol symbol=Deferred.addWaiter type=(this: Deferred<T#4>, Waiter<T#4>) => void
    /// @type.symbol symbol=Deferred.addWaiter.this type=Deferred<T#4>
    /// @type.symbol symbol=Deferred.addWaiter.waiter source="waiter: Waiter<T>" type=Waiter<T#4>
    /// @resolution.name source=Waiter target=Waiter
    /// @resolution.name source=T target=Deferred.T

        this.head = waiter;
        /// @resolution.receiver source=this kind=this declaration=Deferred type=Deferred<T#4>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.head kind=place
        /// @resolution.place source=this.head placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.head root=this keys=[head]
        /// @resolution.assignment source=this.head write="receiver=Deferred<T#4>, target=field(receiver=Deferred<T#4>, target=Deferred.head, type=Awaiter<T#4> | Reaction<T#4> | undefined), type=Awaiter<T#4> | Reaction<T#4> | undefined" type=Awaiter<T#4> | Reaction<T#4> | undefined
        /// @resolution.name source=waiter target=Deferred.addWaiter.waiter
        /// @resolution.place source=waiter placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=waiter root=Deferred.addWaiter.waiter

    }
}
"#,
        r#"
"#,
    );
}

/// A class's constructor infers a struct argument from an owned class handle.
#[test]
fn test_infer_a_struct_argument_from_an_owned_class_handle_in_a_local_class() {
    let session = TestSession::single(
        r#"
struct Slot<T> {
    value: T;

    static new(value: T): Slot<T> {
        Slot { value }
    }
}

class Queue<T> {
    static new(): ^Queue<T> {
        Queue.new()
    }
}

class State<T> {
    private readonly values: Slot<Queue<T>>;

    constructor() {
        this.values = Slot.new(Queue.new());
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Slot<out T> {
    value: T;

    static new(value: T): Slot<T> {
        Slot<T> { value }
    }
}

class Queue<T> {
    static new(): ^Queue<T> {
        Queue.new<T>()
    }
}

class State<T> {
    private readonly values: Slot<Queue<T>>;

    constructor() {
        this.values = Slot.new<Queue<T>>(Queue.new<T>() as Queue<T>);
    }
}

=== dir ===
struct Slot<T> {
/// @generic.template symbol=Slot parameters=(out T#1)
/// @type.symbol symbol=Slot type=Slot
/// @definition.struct symbol=Slot template=(out T#1)
/// @definition.field symbol=Slot.value source="value: T" key=value type=T#1
/// @definition.method symbol=Slot.new slot=new static=true type=(T#1) => Slot<T#1>
/// @type.symbol symbol=Slot.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Slot.value source="value: T" type=T#1
    /// @resolution.name source=T target=Slot.T

    static new(value: T): Slot<T> {
    /// @type.symbol symbol=Slot.new type=(T#1) => Slot<T#1>
    /// @type.symbol symbol=Slot.new.value source="value: T" type=T#1
    /// @resolution.name source=T target=Slot.T
    /// @resolution.name source=Slot target=Slot
    /// @resolution.name source=T target=Slot.T

        Slot { value }
        /// @resolution.name source=Slot target=Slot
        /// @resolution.name source=value target=Slot.new.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Slot.new.value

    }
}

class Queue<T> {
/// @generic.template symbol=Queue parameters=(T#2)
/// @type.symbol symbol=Queue type=typeof Queue
/// @definition.class symbol=Queue template=(T#2)
/// @definition.method symbol=Queue.new slot=new static=true type=() => ^Queue<T#2>
/// @type.symbol symbol=Queue.T source=T type=T#2

    static new(): ^Queue<T> {
    /// @type.symbol symbol=Queue.new type=() => ^Queue<T#2>
    /// @resolution.name source=Queue target=Queue
    /// @resolution.name source=T target=Queue.T

        Queue.new()
        /// @resolution.name source=Queue target=Queue
        /// @resolution.member source=Queue.new receiver=typeof Queue type=() => ^Queue<T#2> kind=symbol target_receiver=typeof Queue target=Queue.new
        /// @resolution.call source=Queue.new() parameters=() return=^Queue<T#2> kind=symbol target=Queue.new instance=Queue<T#2>.new
        /// @generic.instantiation id=Queue.new<T#2> template=Queue.new arguments=(T#2) owner=Queue.new

    }
}

class State<T> {
/// @generic.template symbol=State parameters=(T#3)
/// @type.symbol symbol=State type=typeof State
/// @definition.class symbol=State template=(T#3)
/// @definition.field symbol=State.values source="private readonly values: Slot<Queue<T>>" key=values visibility=private type=Slot<Queue<T#3>>
/// @definition.method symbol=State.constructor slot=constructor role=constructor type=(this: &'managed State<T#3>) => State<T#3>
/// @type.symbol symbol=State.T source=T type=T#3

    private readonly values: Slot<Queue<T>>;
    /// @type.symbol symbol=State.values source="private readonly values: Slot<Queue<T>>" type=Slot<Queue<T#3>>
    /// @resolution.name source=Slot target=Slot
    /// @resolution.name source=Queue target=Queue
    /// @resolution.name source=T target=State.T

    constructor() {
    /// @type.symbol symbol=State.constructor type=(this: &'managed State<T#3>) => State<T#3>
    /// @type.symbol symbol=State.constructor.this type=&'managed State<T#3>

        this.values = Slot.new(Queue.new());
        /// @resolution.receiver source=this kind=this declaration=State type=&'managed State<T#3>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.values kind=place
        /// @resolution.place source=this.values placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.values root=this keys=[values]
        /// @resolution.assignment source=this.values write="receiver=&'managed State<T#3>, target=field(receiver=&'managed State<T#3>, target=State.values, type=Slot<Queue<T#3>>), type=Slot<Queue<T#3>>" type=Slot<Queue<T#3>>
        /// @resolution.name source=Slot target=Slot
        /// @resolution.member source=Slot.new receiver=Slot type=(T#1) => Slot<T#1> kind=symbol target_receiver=Slot target=Slot.new
        /// @resolution.call source=Slot.new(Queue.new()) parameters=(Queue<T#3>) arguments=(provided(Queue.new()) as Queue<T#3>) return=Slot<Queue<T#3>> kind=symbol target=Slot.new instance=Slot<Queue<T#3>>.new
        /// @generic.instantiation id=Slot.new<Queue<T#3>> template=Slot.new arguments=(Queue<T#3>) owner=State.constructor
        /// @resolution.name source=Queue target=Queue
        /// @resolution.member source=Queue.new receiver=typeof Queue type=() => ^Queue<T#2> kind=symbol target_receiver=typeof Queue target=Queue.new
        /// @resolution.call source=Queue.new() parameters=() return=^Queue<T#3> kind=symbol target=Queue.new instance=Queue<T#3>.new
        /// @generic.instantiation id=Queue.new<T#3> template=Queue.new arguments=(T#3) owner=State.constructor

    }
}
"#,
        r#"
"#,
    );
}

/// A class's constructor stores an owned stdlib deque in a stdlib cell field.
#[test]
fn test_store_an_owned_deque_in_a_cell_field_of_a_local_class() {
    let session = TestSession::single(
        r#"
import { Ready } from "destack:async";
import { Deque } from "destack:collections";
import { Cell } from "destack:memory";

class State<T> {
    private readonly values: Cell<Deque<Ready<T>>>;

    constructor() {
        this.values = Cell.new(Deque.new());
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Ready } from "destack:async";
import { Deque } from "destack:collections";
import { Cell } from "destack:memory";

class State<in out T> {
    private readonly values: Cell<Deque<Ready<T>>>;

    constructor() {
        this.values = Cell.new<Deque<Ready<T>>>(Deque.new<Ready<T>>() as Deque<Ready<T>>);
    }
}

=== dir ===
import { Ready } from "destack:async";
import { Deque } from "destack:collections";
import { Cell } from "destack:memory";

class State<T> {
/// @generic.template symbol=State parameters=(in out T)
/// @type.symbol symbol=State type=typeof State
/// @definition.class symbol=State template=(in out T)
/// @definition.field symbol=State.values source="private readonly values: Cell<Deque<Ready<T>>>" key=values visibility=private type=Cell<Deque<Ready<T>>>
/// @definition.method symbol=State.constructor slot=constructor role=constructor type=(this: &'managed State<T>) => State<T>
/// @type.symbol symbol=State.T source=T type=T

    private readonly values: Cell<Deque<Ready<T>>>;
    /// @type.symbol symbol=State.values source="private readonly values: Cell<Deque<Ready<T>>>" type=Cell<Deque<Ready<T>>>
    /// @resolution.name source=Cell target=Cell
    /// @resolution.name source=Deque target=Deque
    /// @resolution.name source=Ready target=Ready
    /// @resolution.name source=T target=State.T

    constructor() {
    /// @type.symbol symbol=State.constructor type=(this: &'managed State<T>) => State<T>
    /// @type.symbol symbol=State.constructor.this type=&'managed State<T>

        this.values = Cell.new(Deque.new());
        /// @resolution.receiver source=this kind=this declaration=State type=&'managed State<T>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.values kind=place
        /// @resolution.place source=this.values placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.values root=this keys=[values]
        /// @resolution.assignment source=this.values write="receiver=&'managed State<T>, target=field(receiver=&'managed State<T>, target=State.values, type=Cell<Deque<Ready<T>>>), type=Cell<Deque<Ready<T>>>" type=Cell<Deque<Ready<T>>>
        /// @resolution.name source=Cell target=Cell
        /// @resolution.member source=Cell.new receiver=Cell type=(T#2) => Cell<T#2> kind=symbol target_receiver=Cell target=new#2
        /// @resolution.call source=Cell.new(Deque.new()) parameters=(Deque<Ready<T>>) arguments=(provided(Deque.new()) as Deque<Ready<T>>) return=Cell<Deque<Ready<T>>> kind=symbol target=new#2 instance=Cell<Deque<Ready<T>>>.<extension#2>.new#2
        /// @generic.instantiation id=new#2<Deque<Ready<T>>> template=new#2 arguments=(Deque<Ready<T>>) owner=State.constructor
        /// @resolution.name source=Deque target=Deque
        /// @resolution.member source=Deque.new receiver=typeof Deque type=() => ^Deque<T#4> kind=symbol target_receiver=typeof Deque target=new
        /// @resolution.call source=Deque.new() parameters=() return=^Deque<Ready<T>> kind=symbol target=new instance=Deque<Ready<T>>.<extension#4>.new
        /// @generic.instantiation id=new<Ready<T>> template=new arguments=(Ready<T>) owner=State.constructor

    }
}
"#,
        r#"
"#,
    );
}

/// A capture-free closure bound by `let` flows into a callback parameter.
#[test]
fn test_pass_a_capture_free_closure_to_a_callback_parameter() {
    let session = TestSession::single(
        r#"
function run<T>(executor: (resolve: (value: T) => void) => void): void {
    let resolve = (value: T) => {
        drop(value);
    };

    executor(resolve);
}

function drop<T>(value: T): void {}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function run<T>(executor: (resolve: (value: T) => void) => void): void {
    let resolve: (value: T) => void = (value: T): void => {
        drop<T>(value);
    };

    executor(resolve);
}

function drop<T>(value: T): void {}

=== dir ===
function run<T>(executor: (resolve: (value: T) => void) => void): void {
/// @generic.template symbol=run parameters=(T#1)
/// @type.symbol symbol=run type=<T#1>(((T#1) => void) => void) => void
/// @type.symbol symbol=run.T source=T type=T#1
/// @type.symbol symbol=run.executor source="executor: (resolve: (value: T) => void) => void" type=((T#1) => void) => void
/// @type.symbol symbol=run.resolve#1 source="resolve: (value: T) => void" type=(T#1) => void
/// @type.symbol symbol=run.value source="value: T" type=T#1
/// @resolution.name source=T target=run.T

    let resolve = (value: T) => {
    /// @type.symbol symbol=run.resolve#2 source=resolve type=Function<(T#1,), void, "readonly">
    /// @resolution.pattern source=resolve kind=binding target=run.resolve#2
    /// @type.symbol symbol=run.symbol6 type=Function<(T#1,), void, "readonly">
    /// @type.symbol symbol=run.symbol6.value source="value: T" type=T#1
    /// @resolution.name source=T target=run.T

        drop(value);
        /// @resolution.name source=drop target=drop
        /// @resolution.call source=drop(value) parameters=(T#1) arguments=(provided(value) as T#1) return=void kind=symbol target=drop instance=drop<T#1>
        /// @generic.instantiation id=drop<T#1> template=drop arguments=(T#1) owner=run
        /// @resolution.name source=value target=run.symbol6.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=run.symbol6.value

    };

    executor(resolve);
    /// @resolution.name source=executor target=run.executor
    /// @resolution.call source=executor(resolve) parameters=((T#1) => void) arguments=(provided(resolve) as (T#1) => void) return=void kind=expression target=expression
    /// @resolution.place source=executor placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=executor root=run.executor
    /// @resolution.name source=resolve target=run.resolve#2
    /// @resolution.place source=resolve placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=resolve root=run.resolve#2

}

function drop<T>(value: T): void {}
/// @generic.template symbol=drop parameters=(T#2)
/// @type.symbol symbol=drop source="function drop<T>(value: T): void {}" type=<T#2>(T#2) => void
/// @type.symbol symbol=drop.T source=T type=T#2
/// @type.symbol symbol=drop.value source="value: T" type=T#2
/// @resolution.name source=T target=drop.T
"#,
        r#"
"#,
    );
}

/// A closure literal at the argument flows into a callback parameter.
#[test]
fn test_pass_a_closure_literal_to_a_callback_parameter() {
    let session = TestSession::single(
        r#"
function run<T>(executor: (resolve: (value: T) => void) => void): void {
    executor((value: T) => {
        drop(value);
    });
}

function drop<T>(value: T): void {}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function run<T>(executor: (resolve: (value: T) => void) => void): void {
    executor((value: T): void => {
        drop<T>(value);
    });
}

function drop<T>(value: T): void {}

=== dir ===
function run<T>(executor: (resolve: (value: T) => void) => void): void {
/// @generic.template symbol=run parameters=(T#1)
/// @type.symbol symbol=run type=<T#1>(((T#1) => void) => void) => void
/// @type.symbol symbol=run.T source=T type=T#1
/// @type.symbol symbol=run.executor source="executor: (resolve: (value: T) => void) => void" type=((T#1) => void) => void
/// @type.symbol symbol=run.resolve source="resolve: (value: T) => void" type=(T#1) => void
/// @type.symbol symbol=run.value source="value: T" type=T#1
/// @resolution.name source=T target=run.T

    executor((value: T) => {
    /// @resolution.name source=executor target=run.executor
    /// @resolution.call parameters=((T#1) => void) arguments=(provided(argument) as (T#1) => void) return=void kind=expression target=expression
    /// @resolution.place source=executor placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=executor root=run.executor
    /// @type.symbol symbol=run.symbol6 type=Function<(T#1,), void, "readonly">
    /// @type.symbol symbol=run.symbol6.value source="value: T" type=T#1
    /// @resolution.name source=T target=run.T

        drop(value);
        /// @resolution.name source=drop target=drop
        /// @resolution.call source=drop(value) parameters=(T#1) arguments=(provided(value) as T#1) return=void kind=symbol target=drop instance=drop<T#1>
        /// @generic.instantiation id=drop<T#1> template=drop arguments=(T#1) owner=run
        /// @resolution.name source=value target=run.symbol6.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=run.symbol6.value

    });
}

function drop<T>(value: T): void {}
/// @generic.template symbol=drop parameters=(T#2)
/// @type.symbol symbol=drop source="function drop<T>(value: T): void {}" type=<T#2>(T#2) => void
/// @type.symbol symbol=drop.T source=T type=T#2
/// @type.symbol symbol=drop.value source="value: T" type=T#2
/// @resolution.name source=T target=drop.T
"#,
        r#"
"#,
    );
}

/// A closure borrowing its parameter selects the borrowing overload of a generic method.
#[test]
fn test_select_a_borrowing_overload_with_a_borrowing_closure() {
    let session = TestSession::single(
        r#"
class Source<T> {
    find(this, predicate: (value: &immutable T, index: isize) => boolean): T | undefined {
        this.find(predicate)
    }
}

export extension<T: Copy> of Source<T> {
    findCopy(this, predicate: (value: T, index: isize) => boolean): T | undefined {
        return this.find((value: &immutable T, index: isize) => predicate(*value, index));
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Source<in out T> {
    find(this, predicate: (value: &immutable T, index: isize) => boolean): T | undefined {
        this.find<T>(predicate)
    }
}

export extension<T: Copy> of Source<T> {
    findCopy(this, predicate: (value: T, index: isize) => boolean): T | undefined {
        return this.find<T>(<'a,>(value: &'a immutable T, index: isize): boolean =>
            predicate(*value as T, index),
        );
    }
}

=== dir ===
class Source<T> {
/// @generic.template symbol=Source parameters=(in out T#1)
/// @type.symbol symbol=Source type=typeof Source
/// @definition.class symbol=Source template=(in out T#1)
/// @definition.method symbol=Source.find slot=find type=(this: Source<T#1>, <type_expression.'a>(&type_expression.'a immutable T#1, isize) => boolean) => T#1 | undefined
/// @type.symbol symbol=Source.T source=T type=T#1

    find(this, predicate: (value: &immutable T, index: isize) => boolean): T | undefined {
    /// @type.symbol symbol=Source.find type=(this: Source<T#1>, <type_expression.'a>(&type_expression.'a immutable T#1, isize) => boolean) => T#1 | undefined
    /// @type.symbol symbol=Source.find.this source=this type=Source<T#1>
    /// @type.symbol symbol=Source.find.predicate source="predicate: (value: &immutable T, index: isize) => boolean" type=<type_expression.'a>(&type_expression.'a immutable T#1, isize) => boolean
    /// @generic.template source=type_expression parent=template#0 parameters=('a)
    /// @type.symbol symbol=Source.find.value source="value: &immutable T" type=&type_expression.'a immutable T#1
    /// @resolution.name source=T target=Source.T
    /// @type.symbol symbol=Source.find.index source="index: isize" type=isize
    /// @resolution.name source=T target=Source.T

        this.find(predicate)
        /// @resolution.member source=this.find receiver=Source<T#1> type=(this: Source<T#1>, <type_expression.'a>(&type_expression.'a immutable T#1, isize) => boolean) => T#1 | undefined kind=symbol target_receiver=Source<T#1> target=Source.find
        /// @resolution.call source=this.find(predicate) parameters=(<type_expression.'a>(&type_expression.'a immutable T#1, isize) => boolean) arguments=(provided(predicate) as <type_expression.'a>(&type_expression.'a immutable T#1, isize) => boolean) return=T#1 | undefined kind=symbol target=Source.find receiver=Source<T#1> instance=Source<T#1>.find
        /// @resolution.receiver source=this kind=this declaration=Source type=Source<T#1>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=Source.find<T#1> template=Source.find arguments=(T#1) owner=Source.find
        /// @resolution.name source=predicate target=Source.find.predicate
        /// @resolution.place source=predicate placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=predicate root=Source.find.predicate

    }
}

export extension<T: Copy> of Source<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2: Copy)
/// @definition.extension symbol=<module>#2 form=exported target=Source<T#2>
/// @definition.method symbol=findCopy slot=findCopy type=(this: Source<T#2>, (T#2, isize) => boolean) => T#2 | undefined
/// @type.symbol symbol=T source="T: Copy" type=T#2
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=Source target=Source
/// @resolution.name source=T target=T

    findCopy(this, predicate: (value: T, index: isize) => boolean): T | undefined {
    /// @type.symbol symbol=findCopy type=(this: Source<T#2>, (T#2, isize) => boolean) => T#2 | undefined
    /// @type.symbol symbol=findCopy.this source=this type=Source<T#2>
    /// @type.symbol symbol=findCopy.predicate source="predicate: (value: T, index: isize) => boolean" type=(T#2, isize) => boolean
    /// @type.symbol symbol=findCopy.value source="value: T" type=T#2
    /// @resolution.name source=T target=T
    /// @type.symbol symbol=findCopy.index source="index: isize" type=isize
    /// @resolution.name source=T target=T

        return this.find((value: &immutable T, index: isize) => predicate(*value, index));
        /// @resolution.member source=this.find receiver=Source<T#2> type=(this: Source<T#2>, <type_expression.'a>(&type_expression.'a immutable T#2, isize) => boolean) => T#2 | undefined kind=symbol target_receiver=Source<T#2> target=Source.find
        /// @resolution.call source="this.find((value: &immutable T, index: isize) => predicate(*value, index))" parameters=(<type_expression.'a>(&type_expression.'a immutable T#2, isize) => boolean) arguments=(provided((value: &immutable T, index: isize) => predicate(*value, index)) as <type_expression.'a>(&type_expression.'a immutable T#2, isize) => boolean) return=T#2 | undefined kind=symbol target=Source.find receiver=Source<T#2> instance=Source<T#2>.find
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Source<T#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=Source.find<T#2> template=Source.find arguments=(T#2) owner=findCopy
        /// @generic.template symbol=findCopy.symbol17 parameters=('a)
        /// @type.symbol symbol=findCopy.symbol17 source=(value: &immutable T, index: isize) => predicate(*value, index) type=Function<(&findCopy.symbol17.'a immutable T#2, isize), boolean, "readonly">
        /// @type.symbol symbol=findCopy.symbol17.value source="value: &immutable T" type=&findCopy.symbol17.'a immutable T#2
        /// @resolution.name source=T target=T
        /// @type.symbol symbol=findCopy.symbol17.index source="index: isize" type=isize
        /// @resolution.name source=predicate target=findCopy.predicate
        /// @resolution.call source="predicate(*value, index)" parameters=(T#2, isize) arguments=(provided(*value) as T#2, provided(index) as isize) return=boolean kind=expression target=expression
        /// @resolution.place source=predicate placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=predicate root=findCopy.predicate
        /// @resolution.place source=*value placement=findCopy.symbol17.'a lifetime=findCopy.symbol17.'a access="immutable"
        /// @resolution.operator source=*value type=^T#2 operator="*" kind=builtin operands=[value as &findCopy.symbol17.'a immutable T#2]
        /// @resolution.name source=value target=findCopy.symbol17.value
        /// @resolution.place source=value placement=findCopy.symbol17.'a lifetime=findCopy.symbol17.'a access="immutable"
        /// @resolution.access source=value root=findCopy.symbol17.value
        /// @resolution.name source=index target=findCopy.symbol17.index
        /// @resolution.place source=index placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=index root=findCopy.symbol17.index

    }
}
"#,
        r#"
"#,
    );
}

/// A string literal flows into a free function's string parameter.
#[test]
fn test_pass_a_string_literal_to_a_free_function() {
    let session = TestSession::single(
        r#"
function show(message?: string): void {}

function run(): void {
    show("ready");
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function show(message?: string): void {}

function run(): void {
    show("ready" as string | undefined);
}

=== dir ===
function show(message?: string): void {}
/// @type.symbol symbol=show source="function show(message?: string): void {}" type=(string | undefined?) => void
/// @type.symbol symbol=show.message source="message?: string" type=string | undefined

function run(): void {
/// @type.symbol symbol=run type=() => void

    show("ready");
    /// @resolution.name source=show target=show
    /// @resolution.call source="show(\"ready\")" parameters=(string | undefined) arguments=(provided("ready") as string | undefined) return=void kind=symbol target=show

}
"#,
        r#"
"#,
    );
}

/// A string literal flows into a method's string parameter.
#[test]
fn test_pass_a_string_literal_to_a_method() {
    let session = TestSession::single(
        r#"
class Logger {
    log(&this, message: string): void {}

    run(&this): void {
        this.log("ready");
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Logger {
    log(&this, message: string): void {}

    run(&this): void {
        this.log<'a>("ready");
    }
}

=== dir ===
class Logger {
/// @type.symbol symbol=Logger type=typeof Logger
/// @definition.class symbol=Logger
/// @definition.method symbol=Logger.log source="log(&this, message: string): void {}" slot=log type=<Logger.log.'a>(this: &Logger.log.'a Logger, string) => void
/// @definition.method symbol=Logger.run slot=run type=<Logger.run.'a>(this: &Logger.run.'a Logger) => void

    log(&this, message: string): void {}
    /// @generic.template symbol=Logger.log parameters=('a)
    /// @type.symbol symbol=Logger.log source="log(&this, message: string): void {}" type=<Logger.log.'a>(this: &Logger.log.'a Logger, string) => void
    /// @type.symbol symbol=Logger.log.this source=&this type=&Logger.log.'a Logger
    /// @type.symbol symbol=Logger.log.message source="message: string" type=string

    run(&this): void {
    /// @generic.template symbol=Logger.run parameters=('a)
    /// @type.symbol symbol=Logger.run type=<Logger.run.'a>(this: &Logger.run.'a Logger) => void
    /// @type.symbol symbol=Logger.run.this source=&this type=&Logger.run.'a Logger

        this.log("ready");
        /// @resolution.member source=this.log receiver=&Logger.run.'a Logger type=<Logger.log.'a>(this: &Logger.log.'a Logger, string) => void kind=symbol target_receiver=&Logger.run.'a Logger target=Logger.log
        /// @resolution.call source="this.log(\"ready\")" parameters=(string) arguments=(provided("ready") as string) return=void regions=(Logger.run.'a) kind=symbol target=Logger.log receiver=&Logger.run.'a Logger instance=Logger.log<Logger.run.'a>
        /// @resolution.receiver source=this kind=this declaration=Logger type=&Logger.run.'a Logger
        /// @resolution.place source=this placement=Logger.run.'a lifetime=Logger.run.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=Logger.log<Logger.run.'a> template=Logger.log arguments=(Logger.run.'a)

    }
}
"#,
        r#"
"#,
    );
}

/// A constructor stores its string argument in a field.
#[test]
fn test_store_a_constructor_argument_in_a_field() {
    let session = TestSession::single(
        r#"
class Named {
    readonly name: string;

    constructor(name: string) {
        this.name = name;
    }
}

function run(): void {
    const named = new Named("ready");
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Named {
    readonly name: string;

    constructor(name: string) {
        this.name = name;
    }
}

function run(): void {
    const named: Named = new Named("ready");
}

=== dir ===
class Named {
/// @type.symbol symbol=Named type=typeof Named
/// @definition.class symbol=Named
/// @definition.field symbol=Named.name source="readonly name: string" key=name type=string
/// @definition.method symbol=Named.constructor slot=constructor role=constructor type=(this: &'managed Named, string) => Named

    readonly name: string;
    /// @type.symbol symbol=Named.name source="readonly name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=Named.constructor type=(this: &'managed Named, string) => Named
    /// @type.symbol symbol=Named.constructor.this type=&'managed Named
    /// @type.symbol symbol=Named.constructor.name source="name: string" type=string

        this.name = name;
        /// @resolution.receiver source=this kind=this declaration=Named type=&'managed Named
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.name kind=place
        /// @resolution.place source=this.name placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.name root=this keys=[name]
        /// @resolution.assignment source=this.name write="receiver=&'managed Named, target=field(receiver=&'managed Named, target=Named.name, type=string), type=string" type=string
        /// @resolution.name source=name target=Named.constructor.name
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=Named.constructor.name

    }
}

function run(): void {
/// @type.symbol symbol=run type=() => void

    const named = new Named("ready");
    /// @type.symbol symbol=run.named source=named type=Named
    /// @resolution.pattern source=named kind=binding target=run.named
    /// @resolution.construct source="new Named(\"ready\")" parameters=(string) arguments=(provided("ready") as string) return=Named kind=class target=Named constructor=Named.constructor
    /// @resolution.name source=Named target=Named

}
"#,
        r#"
"#,
    );
}

/// A local object flows into a free function's handle parameter.
#[test]
fn test_pass_a_local_object_to_a_free_function() {
    let session = TestSession::single(
        r#"
class Item {}

function take(item: Item): void {}

function run(): void {
    take(new Item());
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Item {}

function take(item: Item): void {}

function run(): void {
    take(new Item());
}

=== dir ===
class Item {}
/// @type.symbol symbol=Item source="class Item {}" type=typeof Item
/// @definition.class symbol=Item source="class Item {}"

function take(item: Item): void {}
/// @type.symbol symbol=take source="function take(item: Item): void {}" type=(Item) => void
/// @type.symbol symbol=take.item source="item: Item" type=Item
/// @resolution.name source=Item target=Item

function run(): void {
/// @type.symbol symbol=run type=() => void

    take(new Item());
    /// @resolution.name source=take target=take
    /// @resolution.call source="take(new Item())" parameters=(Item) arguments=(provided(new Item()) as Item) return=void kind=symbol target=take
    /// @resolution.construct source="new Item()" parameters=() return=Item kind=class target=Item constructor=default
    /// @resolution.name source=Item target=Item

}
"#,
        r#"
"#,
    );
}

/// A closure assigns to a callback-typed field of a class.
#[test]
fn test_store_a_closure_in_a_callback_field() {
    let session = TestSession::single(
        r#"
class Reaction<T> {
    readonly run: (value: T) => void;

    constructor(run: (value: T) => void) {
        this.run = run;
    }
}

function run<T>(): void {
    const reaction = new Reaction<T>((value: T) => {});
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Reaction<in T> {
    readonly run: (value: T) => void;

    constructor(run: (value: T) => void) {
        this.run = run;
    }
}

function run<T>(): void {
    const reaction: Reaction<T> = new Reaction<T>((value: T): void => {});
}

=== dir ===
class Reaction<T> {
/// @generic.template symbol=Reaction parameters=(in T#1)
/// @type.symbol symbol=Reaction type=typeof Reaction
/// @definition.class symbol=Reaction template=(in T#1)
/// @definition.field symbol=Reaction.run source="readonly run: (value: T) => void" key=run type=(T#1) => void
/// @definition.method symbol=Reaction.constructor slot=constructor role=constructor type=(this: &'managed Reaction<T#1>, (T#1) => void) => Reaction<T#1>
/// @type.symbol symbol=Reaction.T source=T type=T#1

    readonly run: (value: T) => void;
    /// @type.symbol symbol=Reaction.run source="readonly run: (value: T) => void" type=(T#1) => void
    /// @type.symbol symbol=Reaction.value source="value: T" type=T#1
    /// @resolution.name source=T target=Reaction.T

    constructor(run: (value: T) => void) {
    /// @type.symbol symbol=Reaction.constructor type=(this: &'managed Reaction<T#1>, (T#1) => void) => Reaction<T#1>
    /// @type.symbol symbol=Reaction.constructor.this type=&'managed Reaction<T#1>
    /// @type.symbol symbol=Reaction.constructor.run source="run: (value: T) => void" type=(T#1) => void
    /// @type.symbol symbol=Reaction.constructor.value source="value: T" type=T#1
    /// @resolution.name source=T target=Reaction.T

        this.run = run;
        /// @resolution.receiver source=this kind=this declaration=Reaction type=&'managed Reaction<T#1>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.run kind=place
        /// @resolution.place source=this.run placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.run root=this keys=[run]
        /// @resolution.assignment source=this.run write="receiver=&'managed Reaction<T#1>, target=field(receiver=&'managed Reaction<T#1>, target=Reaction.run, type=(T#1) => void), type=(T#1) => void" type=(T#1) => void
        /// @resolution.name source=run target=Reaction.constructor.run
        /// @resolution.place source=run placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=run root=Reaction.constructor.run

    }
}

function run<T>(): void {
/// @generic.template symbol=run parameters=(T#2)
/// @type.symbol symbol=run type=<T#2>() => void
/// @type.symbol symbol=run.T source=T type=T#2

    const reaction = new Reaction<T>((value: T) => {});
    /// @type.symbol symbol=run.reaction source=reaction type=Reaction<T#2>
    /// @resolution.pattern source=reaction kind=binding target=run.reaction
    /// @resolution.construct source="new Reaction<T>((value: T) => {})" parameters=((T#2) => void) arguments=(provided((value: T) => {}) as (T#2) => void) return=Reaction<T#2> kind=class target=Reaction constructor=Reaction.constructor instance=Reaction<T#2>
    /// @generic.instantiation id=Reaction.constructor<T#2> template=Reaction.constructor arguments=(T#2) owner=run
    /// @generic.instantiation id=Reaction<T#2> template=Reaction arguments=(T#2) owner=run
    /// @resolution.name source=Reaction target=Reaction
    /// @resolution.name source=T target=run.T
    /// @type.symbol symbol=run.symbol12 source="(value: T) => {}" type=Function<(T#2,), void, "readonly">
    /// @type.symbol symbol=run.symbol12.value source="value: T" type=T#2
    /// @resolution.name source=T target=run.T

}
"#,
        r#"
"#,
    );
}

/// A static method calls an instance method on its handle parameter.
#[test]
fn test_call_an_instance_method_on_a_handle_parameter() {
    let session = TestSession::single(
        r#"
class Box<T> {
    value: T | undefined;

    static clear(box: Box<T>): void {
        box.reset();
    }

    reset(&this): void {
        this.value = undefined;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Box<in out T> {
    value: T | undefined;

    static clear(box: Box<T>): void {
        box.reset<T, "managed">();
    }

    reset(&this): void {
        this.value = undefined as T | undefined;
    }
}

=== dir ===
class Box<T> {
/// @generic.template symbol=Box parameters=(in out T)
/// @type.symbol symbol=Box type=typeof Box
/// @definition.class symbol=Box template=(in out T)
/// @definition.field symbol=Box.value source="value: T | undefined" key=value type=T | undefined
/// @definition.method symbol=Box.clear slot=clear static=true type=(Box<T>) => void
/// @definition.method symbol=Box.reset slot=reset type=<Box.reset.'a>(this: &Box.reset.'a Box<T>) => void
/// @type.symbol symbol=Box.T source=T type=T

    value: T | undefined;
    /// @type.symbol symbol=Box.value source="value: T | undefined" type=T | undefined
    /// @resolution.name source=T target=Box.T

    static clear(box: Box<T>): void {
    /// @type.symbol symbol=Box.clear type=(Box<T>) => void
    /// @type.symbol symbol=Box.clear.box source="box: Box<T>" type=Box<T>
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=T target=Box.T

        box.reset();
        /// @resolution.name source=box target=Box.clear.box
        /// @resolution.member source=box.reset receiver=Box<T> type=<Box.reset.'a>(this: &Box.reset.'a Box<T>) => void kind=symbol target_receiver=Box<T> target=Box.reset
        /// @resolution.call source=box.reset() parameters=() return=void regions=("managed" & "local") kind=symbol target=Box.reset receiver=Box<T> adjustments=(borrow(&'managed Box<T>)) instance="Box<T>.reset<\"managed\" & \"local\">"
        /// @resolution.place source=box placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=box root=Box.clear.box
        /// @generic.instantiation id="Box.reset<T, \"managed\" & \"local\">" template=Box.reset arguments=(T, "managed" & "local") owner=Box.clear
        /// @generic.instantiation id=Box.reset<T> template=Box.reset arguments=(T) owner=Box.clear

    }

    reset(&this): void {
    /// @generic.template symbol=Box.reset parent=template#0 parameters=('a)
    /// @type.symbol symbol=Box.reset type=<Box.reset.'a>(this: &Box.reset.'a Box<T>) => void
    /// @type.symbol symbol=Box.reset.this source=&this type=&Box.reset.'a Box<T>

        this.value = undefined;
        /// @resolution.receiver source=this kind=this declaration=Box type=&Box.reset.'a Box<T>
        /// @resolution.place source=this placement=Box.reset.'a lifetime=Box.reset.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement=Box.reset.'a lifetime=Box.reset.'a access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&Box.reset.'a Box<T>, target=field(receiver=&Box.reset.'a Box<T>, target=Box.value, type=T | undefined), type=T | undefined" type=T | undefined

    }
}
"#,
        r#"
"#,
    );
}

/// A callback parameter forwards to another function's callback parameter.
#[test]
fn test_forward_a_callback_parameter() {
    let session = TestSession::single(
        r#"
class Span {}

function inner<T>(body: (span: Span) => T): T {
    body(new Span())
}

function outer<T>(body: (span: Span) => T): T {
    inner(body)
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Span {}

function inner<T>(body: (span: Span) => T): T {
    body(new Span())
}

function outer<T>(body: (span: Span) => T): T {
    inner<T>(body)
}

=== dir ===
class Span {}
/// @type.symbol symbol=Span source="class Span {}" type=typeof Span
/// @definition.class symbol=Span source="class Span {}"

function inner<T>(body: (span: Span) => T): T {
/// @generic.template symbol=inner parameters=(T#1)
/// @type.symbol symbol=inner type=<T#1>((Span) => T#1) => T#1
/// @type.symbol symbol=inner.T source=T type=T#1
/// @type.symbol symbol=inner.body source="body: (span: Span) => T" type=(Span) => T#1
/// @type.symbol symbol=inner.span source="span: Span" type=Span
/// @resolution.name source=Span target=Span
/// @resolution.name source=T target=inner.T
/// @resolution.name source=T target=inner.T

    body(new Span())
    /// @resolution.name source=body target=inner.body
    /// @resolution.call source="body(new Span())" parameters=(Span) arguments=(provided(new Span()) as Span) return=T#1 kind=expression target=expression
    /// @resolution.place source=body placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=body root=inner.body
    /// @resolution.construct source="new Span()" parameters=() return=Span kind=class target=Span constructor=default
    /// @resolution.name source=Span target=Span

}

function outer<T>(body: (span: Span) => T): T {
/// @generic.template symbol=outer parameters=(T#2)
/// @type.symbol symbol=outer type=<T#2>((Span) => T#2) => T#2
/// @type.symbol symbol=outer.T source=T type=T#2
/// @type.symbol symbol=outer.body source="body: (span: Span) => T" type=(Span) => T#2
/// @type.symbol symbol=outer.span source="span: Span" type=Span
/// @resolution.name source=Span target=Span
/// @resolution.name source=T target=outer.T
/// @resolution.name source=T target=outer.T

    inner(body)
    /// @resolution.name source=inner target=inner
    /// @resolution.call source=inner(body) parameters=((Span) => T#2) arguments=(provided(body) as (Span) => T#2) return=T#2 kind=symbol target=inner instance=inner<T#2>
    /// @generic.instantiation id=inner<T#2> template=inner arguments=(T#2) owner=outer
    /// @resolution.name source=body target=outer.body
    /// @resolution.place source=body placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=body root=outer.body

}
"#,
        r#"
"#,
    );
}

/// A module constant annotated with its class holds a constructed object.
#[test]
fn test_hold_a_constructed_object_in_an_annotated_module_constant() {
    let session = TestSession::single(
        r#"
class Tag {
    readonly name: string;

    constructor(name: string) {
        this.name = name;
    }
}

export const tag: Tag = new Tag("fs");
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Tag {
    readonly name: string;

    constructor(name: string) {
        this.name = name;
    }
}

export const tag: Tag = new Tag("fs");

=== dir ===
class Tag {
/// @type.symbol symbol=Tag type=typeof Tag
/// @definition.class symbol=Tag
/// @definition.field symbol=Tag.name source="readonly name: string" key=name type=string
/// @definition.method symbol=Tag.constructor slot=constructor role=constructor type=(this: &'managed Tag, string) => Tag

    readonly name: string;
    /// @type.symbol symbol=Tag.name source="readonly name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=Tag.constructor type=(this: &'managed Tag, string) => Tag
    /// @type.symbol symbol=Tag.constructor.this type=&'managed Tag
    /// @type.symbol symbol=Tag.constructor.name source="name: string" type=string

        this.name = name;
        /// @resolution.receiver source=this kind=this declaration=Tag type=&'managed Tag
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.name kind=place
        /// @resolution.place source=this.name placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.name root=this keys=[name]
        /// @resolution.assignment source=this.name write="receiver=&'managed Tag, target=field(receiver=&'managed Tag, target=Tag.name, type=string), type=string" type=string
        /// @resolution.name source=name target=Tag.constructor.name
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=Tag.constructor.name

    }
}

export const tag: Tag = new Tag("fs");
/// @type.symbol symbol=tag source=tag type=Tag
/// @resolution.pattern source=tag kind=binding target=tag
/// @resolution.name source=Tag target=Tag
/// @resolution.construct source="new Tag(\"fs\")" parameters=(string) arguments=(provided("fs") as string) return=Tag kind=class target=Tag constructor=Tag.constructor
/// @resolution.name source=Tag target=Tag
"#,
        r#"
"#,
    );
}

/// A closure flows into an interface method's callback parameter through an interface receiver.
#[test]
fn test_pass_a_closure_to_an_interface_method_callback() {
    let session = TestSession::single(
        r#"
class Span {}

interface Tracer {
    span<T>(name: string, body: (span: Span) => T): T;
}

function tracer(): Tracer {
    tracer()
}

function span<T>(name: string, body: (span: Span) => T): T {
    tracer().span(name, body)
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Span {}

interface Tracer {
    span<T>(name: string, body: (span: Span) => T): T;
}

function tracer(): Tracer {
    tracer()
}

function span<T>(name: string, body: (span: Span) => T): T {
    tracer().span<T>(name, body)
}

=== dir ===
class Span {}
/// @type.symbol symbol=Span source="class Span {}" type=typeof Span
/// @definition.class symbol=Span source="class Span {}"

interface Tracer {
/// @generic.template symbol=Tracer parameters=(this: Tracer)
/// @type.symbol symbol=Tracer type=Tracer
/// @definition.interface symbol=Tracer template=(this: Tracer)
/// @definition.where symbol=Tracer relation=satisfies left=this right=Tracer
/// @definition.method symbol=Tracer.span source="span<T>(name: string, body: (span: Span) => T): T" slot=span type=<T#1>(string, (Span) => T#1) => T#1

    span<T>(name: string, body: (span: Span) => T): T;
    /// @generic.template symbol=Tracer.span parent=template#0 parameters=(T#1)
    /// @type.symbol symbol=Tracer.span source="span<T>(name: string, body: (span: Span) => T): T" type=<T#1>(string, (Span) => T#1) => T#1
    /// @type.symbol symbol=Tracer.span.T source=T type=T#1
    /// @type.symbol symbol=Tracer.span.name source="name: string" type=string
    /// @type.symbol symbol=Tracer.span.body source="body: (span: Span) => T" type=(Span) => T#1
    /// @type.symbol symbol=Tracer.span.span source="span: Span" type=Span
    /// @resolution.name source=Span target=Span
    /// @resolution.name source=T target=Tracer.span.T
    /// @resolution.name source=T target=Tracer.span.T

}

function tracer(): Tracer {
/// @type.symbol symbol=tracer type=() => Tracer
/// @resolution.name source=Tracer target=Tracer

    tracer()
    /// @resolution.name source=tracer target=tracer
    /// @resolution.call source=tracer() parameters=() return=Tracer kind=symbol target=tracer

}

function span<T>(name: string, body: (span: Span) => T): T {
/// @generic.template symbol=span parameters=(T#2)
/// @type.symbol symbol=span type=<T#2>(string, (Span) => T#2) => T#2
/// @type.symbol symbol=span.T source=T type=T#2
/// @type.symbol symbol=span.name source="name: string" type=string
/// @type.symbol symbol=span.body source="body: (span: Span) => T" type=(Span) => T#2
/// @type.symbol symbol=span.span source="span: Span" type=Span
/// @resolution.name source=Span target=Span
/// @resolution.name source=T target=span.T
/// @resolution.name source=T target=span.T

    tracer().span(name, body)
    /// @resolution.name source=tracer target=tracer
    /// @resolution.member source=tracer().span receiver=Tracer type=<T#1>(string, (Span) => T#1) => T#1 kind=symbol target_receiver=Tracer dispatch=dynamic constraint=Tracer target=Tracer.span
    /// @resolution.call source="tracer().span(name, body)" parameters=(string, (Span) => T#2) arguments=(provided(name) as string, provided(body) as (Span) => T#2) return=T#2 kind=dynamic target=Tracer.span receiver=Tracer constraint=Tracer generic_arguments=(T#2)
    /// @resolution.call source=tracer() parameters=() return=Tracer kind=symbol target=tracer
    /// @resolution.name source=name target=span.name
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=span.name
    /// @resolution.name source=body target=span.body
    /// @resolution.place source=body placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=body root=span.body

}
"#,
        r#"
"#,
    );
}

/// A static method of a bounded generic class calls a private instance method on its parameter.
#[test]
fn test_call_a_private_method_on_a_bounded_class_parameter() {
    let session = TestSession::single(
        r#"
class Deferred<T: Copy> {
    value: T | undefined;

    static settle<T: Copy>(deferred: Deferred<T>, value: T): void {
        deferred.store(value);
    }

    private store(value: T): void {
        this.value = value;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Deferred<in out T: Copy> {
    value: T | undefined;

    static settle<T: Copy>(deferred: Deferred<T>, value: T): void {
        deferred.store<T>(value);
    }

    private store(value: T): void {
        this.value = value as T | undefined;
    }
}

=== dir ===
class Deferred<T: Copy> {
/// @generic.template symbol=Deferred parameters=(in out T#1: Copy)
/// @type.symbol symbol=Deferred type=typeof Deferred
/// @definition.class symbol=Deferred template=(in out T#1: Copy)
/// @definition.field symbol=Deferred.value source="value: T | undefined" key=value type=T#1 | undefined
/// @definition.method symbol=Deferred.settle slot=settle static=true type=<T#2: Copy>(Deferred<T#2>, T#2) => void
/// @definition.method symbol=Deferred.store slot=store visibility=private type=(this: Deferred<T#1>, T#1) => void
/// @type.symbol symbol=Deferred.T source="T: Copy" type=T#1
/// @resolution.name source=Copy target=Copy

    value: T | undefined;
    /// @type.symbol symbol=Deferred.value source="value: T | undefined" type=T#1 | undefined
    /// @resolution.name source=T target=Deferred.T

    static settle<T: Copy>(deferred: Deferred<T>, value: T): void {
    /// @generic.template symbol=Deferred.settle parent=template#0 parameters=(T#2: Copy)
    /// @type.symbol symbol=Deferred.settle type=<T#2: Copy>(Deferred<T#2>, T#2) => void
    /// @type.symbol symbol=Deferred.settle.T source="T: Copy" type=T#2
    /// @resolution.name source=Copy target=Copy
    /// @type.symbol symbol=Deferred.settle.deferred source="deferred: Deferred<T>" type=Deferred<T#2>
    /// @resolution.name source=Deferred target=Deferred
    /// @resolution.name source=T target=Deferred.settle.T
    /// @type.symbol symbol=Deferred.settle.value source="value: T" type=T#2
    /// @resolution.name source=T target=Deferred.settle.T

        deferred.store(value);
        /// @resolution.name source=deferred target=Deferred.settle.deferred
        /// @resolution.member source=deferred.store receiver=Deferred<T#2> type=(this: Deferred<T#2>, T#2) => void kind=symbol target_receiver=Deferred<T#2> target=Deferred.store
        /// @resolution.call source=deferred.store(value) parameters=(T#2) arguments=(provided(value) as T#2) return=void kind=symbol target=Deferred.store receiver=Deferred<T#2> instance=Deferred<T#2>.store
        /// @resolution.place source=deferred placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=deferred root=Deferred.settle.deferred
        /// @generic.instantiation id=Deferred.store<T#2> template=Deferred.store arguments=(T#2) owner=Deferred.settle
        /// @resolution.name source=value target=Deferred.settle.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Deferred.settle.value

    }

    private store(value: T): void {
    /// @type.symbol symbol=Deferred.store type=(this: Deferred<T#1>, T#1) => void
    /// @type.symbol symbol=Deferred.store.this type=Deferred<T#1>
    /// @type.symbol symbol=Deferred.store.value source="value: T" type=T#1
    /// @resolution.name source=T target=Deferred.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Deferred type=Deferred<T#1>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=Deferred<T#1>, target=field(receiver=Deferred<T#1>, target=Deferred.value, type=T#1 | undefined), type=T#1 | undefined" type=T#1 | undefined
        /// @resolution.name source=value target=Deferred.store.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Deferred.store.value

    }
}
"#,
        r#"
"#,
    );
}

/// An exported module constant holds an imported generic class constructed with a local argument.
#[test]
fn test_hold_an_imported_generic_object_in_a_module_constant() {
    let session = TestSession::builder()
        .module(
            "binding.ds",
            r#"
export class Binding<out B> {
    readonly name: string;

    constructor(name: string) {
        this.name = name;
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import * as binding from "./binding";

export interface Binding {
    ready(): boolean;
}

export const fs: binding.Binding<Binding> = new binding.Binding<Binding>("fs");
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import * as binding from "./binding";

export interface Binding {
    ready(): boolean;
}

export const fs: Binding<Binding> = new binding.Binding<Binding>("fs");

=== dir ===
import * as binding from "./binding";

export interface Binding {
/// @generic.template symbol=Binding parameters=(this: Binding)
/// @type.symbol symbol=Binding type=Binding
/// @definition.interface symbol=Binding template=(this: Binding)
/// @definition.where symbol=Binding relation=satisfies left=this right=Binding
/// @definition.method symbol=Binding.ready source="ready(): boolean" slot=ready type=() => boolean

    ready(): boolean;
    /// @type.symbol symbol=Binding.ready source="ready(): boolean" type=() => boolean

}

export const fs: binding.Binding<Binding> = new binding.Binding<Binding>("fs");
/// @type.symbol symbol=fs source=fs type=binding.Binding<Binding>
/// @resolution.pattern source=fs kind=binding target=fs
/// @resolution.name source=binding.Binding target=binding.Binding
/// @resolution.name source=Binding target=Binding
/// @resolution.construct source="new binding.Binding<Binding>(\"fs\")" parameters=(string) arguments=(provided("fs") as string) return=binding.Binding<Binding> kind=class target=binding.Binding constructor=binding.Binding.symbol5 instance=binding.Binding<Binding>
/// @generic.instantiation id=binding.Binding.symbol5<Binding> template=binding.Binding.symbol5 arguments=(Binding)
/// @generic.instantiation id=binding.Binding<Binding> template=binding.Binding arguments=(Binding)
/// @resolution.name source=binding.Binding target=binding.Binding
/// @resolution.name source=Binding target=Binding
"#,
        r#"
"#,
    );
}

/// A class's callback field receives a handle parameter through a static caller.
#[test]
fn test_run_a_callback_field_with_a_handle_parameter() {
    let session = TestSession::single(
        r#"
class Source<T> {
    value: T | undefined;
}

class Reaction<T> {
    readonly run: (source: Source<T>) => void;

    constructor(run: (source: Source<T>) => void) {
        this.run = run;
    }

    static fire(reaction: Reaction<T>, source: Source<T>): void {
        reaction.run(source);
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Source<in out T> {
    value: T | undefined;
}

class Reaction<in out T> {
    readonly run: (source: Source<T>) => void;

    constructor(run: (source: Source<T>) => void) {
        this.run = run;
    }

    static fire(reaction: Reaction<T>, source: Source<T>): void {
        reaction.run(source);
    }
}

=== dir ===
class Source<T> {
/// @generic.template symbol=Source parameters=(in out T#1)
/// @type.symbol symbol=Source type=typeof Source
/// @definition.class symbol=Source template=(in out T#1)
/// @definition.field symbol=Source.value source="value: T | undefined" key=value type=T#1 | undefined
/// @type.symbol symbol=Source.T source=T type=T#1

    value: T | undefined;
    /// @type.symbol symbol=Source.value source="value: T | undefined" type=T#1 | undefined
    /// @resolution.name source=T target=Source.T

}

class Reaction<T> {
/// @generic.template symbol=Reaction parameters=(in out T#2)
/// @type.symbol symbol=Reaction type=typeof Reaction
/// @definition.class symbol=Reaction template=(in out T#2)
/// @definition.field symbol=Reaction.run source="readonly run: (source: Source<T>) => void" key=run type=(Source<T#2>) => void
/// @definition.method symbol=Reaction.constructor slot=constructor role=constructor type=(this: &'managed Reaction<T#2>, (Source<T#2>) => void) => Reaction<T#2>
/// @definition.method symbol=Reaction.fire slot=fire static=true type=(Reaction<T#2>, Source<T#2>) => void
/// @type.symbol symbol=Reaction.T source=T type=T#2

    readonly run: (source: Source<T>) => void;
    /// @type.symbol symbol=Reaction.run source="readonly run: (source: Source<T>) => void" type=(Source<T#2>) => void
    /// @type.symbol symbol=Reaction.source source="source: Source<T>" type=Source<T#2>
    /// @resolution.name source=Source target=Source
    /// @resolution.name source=T target=Reaction.T

    constructor(run: (source: Source<T>) => void) {
    /// @type.symbol symbol=Reaction.constructor type=(this: &'managed Reaction<T#2>, (Source<T#2>) => void) => Reaction<T#2>
    /// @type.symbol symbol=Reaction.constructor.this type=&'managed Reaction<T#2>
    /// @type.symbol symbol=Reaction.constructor.run source="run: (source: Source<T>) => void" type=(Source<T#2>) => void
    /// @type.symbol symbol=Reaction.constructor.source source="source: Source<T>" type=Source<T#2>
    /// @resolution.name source=Source target=Source
    /// @resolution.name source=T target=Reaction.T

        this.run = run;
        /// @resolution.receiver source=this kind=this declaration=Reaction type=&'managed Reaction<T#2>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.run kind=place
        /// @resolution.place source=this.run placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.run root=this keys=[run]
        /// @resolution.assignment source=this.run write="receiver=&'managed Reaction<T#2>, target=field(receiver=&'managed Reaction<T#2>, target=Reaction.run, type=(Source<T#2>) => void), type=(Source<T#2>) => void" type=(Source<T#2>) => void
        /// @resolution.name source=run target=Reaction.constructor.run
        /// @resolution.place source=run placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=run root=Reaction.constructor.run

    }

    static fire(reaction: Reaction<T>, source: Source<T>): void {
    /// @type.symbol symbol=Reaction.fire type=(Reaction<T#2>, Source<T#2>) => void
    /// @type.symbol symbol=Reaction.fire.reaction source="reaction: Reaction<T>" type=Reaction<T#2>
    /// @resolution.name source=Reaction target=Reaction
    /// @resolution.name source=T target=Reaction.T
    /// @type.symbol symbol=Reaction.fire.source source="source: Source<T>" type=Source<T#2>
    /// @resolution.name source=Source target=Source
    /// @resolution.name source=T target=Reaction.T

        reaction.run(source);
        /// @resolution.name source=reaction target=Reaction.fire.reaction
        /// @resolution.member source=reaction.run receiver=Reaction<T#2> type=(Source<T#2>) => void kind=field target_receiver=Reaction<T#2> key=run target=Reaction.run target_type=(Source<T#2>) => void
        /// @resolution.call source=reaction.run(source) parameters=(Source<T#2>) arguments=(provided(source) as Source<T#2>) return=void kind=expression target=expression
        /// @resolution.place source=reaction placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=reaction root=Reaction.fire.reaction
        /// @resolution.place source=reaction.run placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=reaction.run root=Reaction.fire.reaction keys=[run]
        /// @resolution.name source=source target=Reaction.fire.source
        /// @resolution.place source=source placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=source root=Reaction.fire.source

    }
}
"#,
        r#"
"#,
    );
}

/// Handles of every place flow into an unknown slot.
#[test]
fn test_pass_handles_to_an_unknown_parameter() {
    let session = TestSession::single(
        r#"
class Item {}

shared class SharedItem {}

function keep(value: unknown): void {}

function run(item: SharedItem): void {
    keep(new Item());
    keep(item);
    keep("text");
    keep(1);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Item {}

shared class SharedItem {}

function keep(value: unknown): void {}

function run(item: SharedItem): void {
    keep(new Item() as unknown);
    keep(item as unknown);
    keep("text" as unknown);
    keep(1 as unknown);
}

=== dir ===
class Item {}
/// @type.symbol symbol=Item source="class Item {}" type=typeof Item
/// @definition.class symbol=Item source="class Item {}"

shared class SharedItem {}
/// @type.symbol symbol=SharedItem source="shared class SharedItem {}" type=typeof SharedItem
/// @definition.class symbol=SharedItem source="shared class SharedItem {}"

function keep(value: unknown): void {}
/// @type.symbol symbol=keep source="function keep(value: unknown): void {}" type=(unknown) => void
/// @type.symbol symbol=keep.value source="value: unknown" type=unknown

function run(item: SharedItem): void {
/// @type.symbol symbol=run type=(SharedItem) => void
/// @type.symbol symbol=run.item source="item: SharedItem" type=SharedItem
/// @resolution.name source=SharedItem target=SharedItem

    keep(new Item());
    /// @resolution.name source=keep target=keep
    /// @resolution.call source="keep(new Item())" parameters=(unknown) arguments=(provided(new Item()) as unknown) return=void kind=symbol target=keep
    /// @resolution.construct source="new Item()" parameters=() return=Item kind=class target=Item constructor=default
    /// @resolution.name source=Item target=Item

    keep(item);
    /// @resolution.name source=keep target=keep
    /// @resolution.call source=keep(item) parameters=(unknown) arguments=(provided(item) as unknown) return=void kind=symbol target=keep
    /// @resolution.name source=item target=run.item
    /// @resolution.place source=item placement="shared" lifetime="frame" access="readonly"
    /// @resolution.access source=item root=run.item

    keep("text");
    /// @resolution.name source=keep target=keep
    /// @resolution.call source="keep(\"text\")" parameters=(unknown) arguments=(provided("text") as unknown) return=void kind=symbol target=keep

    keep(1);
    /// @resolution.name source=keep target=keep
    /// @resolution.call source=keep(1) parameters=(unknown) arguments=(provided(1) as unknown) return=void kind=symbol target=keep

}
"#,
        r#"
"#,
    );
}

/// A static factory forwards its string input into the constructor of a fresh object.
#[test]
fn test_forward_a_factory_input_into_a_constructor() {
    let session = TestSession::single(
        r#"
class Tag {
    readonly name: string;

    constructor(name: string) {
        this.name = name;
    }

    static make(name: string): Tag {
        new Tag(name)
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Tag {
    readonly name: string;

    constructor(name: string) {
        this.name = name;
    }

    static make(name: string): Tag {
        new Tag(name)
    }
}

=== dir ===
class Tag {
/// @type.symbol symbol=Tag type=typeof Tag
/// @definition.class symbol=Tag
/// @definition.field symbol=Tag.name source="readonly name: string" key=name type=string
/// @definition.method symbol=Tag.constructor slot=constructor role=constructor type=(this: &'managed Tag, string) => Tag
/// @definition.method symbol=Tag.make slot=make static=true type=(string) => Tag

    readonly name: string;
    /// @type.symbol symbol=Tag.name source="readonly name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=Tag.constructor type=(this: &'managed Tag, string) => Tag
    /// @type.symbol symbol=Tag.constructor.this type=&'managed Tag
    /// @type.symbol symbol=Tag.constructor.name source="name: string" type=string

        this.name = name;
        /// @resolution.receiver source=this kind=this declaration=Tag type=&'managed Tag
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.name kind=place
        /// @resolution.place source=this.name placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.name root=this keys=[name]
        /// @resolution.assignment source=this.name write="receiver=&'managed Tag, target=field(receiver=&'managed Tag, target=Tag.name, type=string), type=string" type=string
        /// @resolution.name source=name target=Tag.constructor.name
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=Tag.constructor.name

    }

    static make(name: string): Tag {
    /// @type.symbol symbol=Tag.make type=(string) => Tag
    /// @type.symbol symbol=Tag.make.name source="name: string" type=string
    /// @resolution.name source=Tag target=Tag

        new Tag(name)
        /// @resolution.construct source="new Tag(name)" parameters=(string) arguments=(provided(name) as string) return=Tag kind=class target=Tag constructor=Tag.constructor
        /// @resolution.name source=Tag target=Tag
        /// @resolution.name source=name target=Tag.make.name
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=Tag.make.name

    }
}
"#,
        r#"
"#,
    );
}

/// A free factory forwards its optional string inputs into a constructor.
#[test]
fn test_forward_optional_factory_inputs_into_a_constructor() {
    let session = TestSession::single(
        r#"
class Counter {
    readonly name: string;

    readonly unit: string | undefined;

    constructor(name: string, unit?: string) {
        this.name = name;
        this.unit = unit;
    }
}

export function counter(name: string, unit?: string): Counter {
    new Counter(name, unit)
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Counter {
    readonly name: string;

    readonly unit: string | undefined;

    constructor(name: string, unit?: string) {
        this.name = name;
        this.unit = unit;
    }
}

export function counter(name: string, unit?: string): Counter {
    new Counter(name, unit)
}

=== dir ===
class Counter {
/// @type.symbol symbol=Counter type=typeof Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.name source="readonly name: string" key=name type=string
/// @definition.field symbol=Counter.unit source="readonly unit: string | undefined" key=unit type=string | undefined
/// @definition.method symbol=Counter.constructor slot=constructor role=constructor type=(this: &'managed Counter, string, string | undefined?) => Counter

    readonly name: string;
    /// @type.symbol symbol=Counter.name source="readonly name: string" type=string

    readonly unit: string | undefined;
    /// @type.symbol symbol=Counter.unit source="readonly unit: string | undefined" type=string | undefined

    constructor(name: string, unit?: string) {
    /// @type.symbol symbol=Counter.constructor type=(this: &'managed Counter, string, string | undefined?) => Counter
    /// @type.symbol symbol=Counter.constructor.this type=&'managed Counter
    /// @type.symbol symbol=Counter.constructor.name source="name: string" type=string
    /// @type.symbol symbol=Counter.constructor.unit source="unit?: string" type=string | undefined

        this.name = name;
        /// @resolution.receiver source=this kind=this declaration=Counter type=&'managed Counter
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.name kind=place
        /// @resolution.place source=this.name placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.name root=this keys=[name]
        /// @resolution.assignment source=this.name write="receiver=&'managed Counter, target=field(receiver=&'managed Counter, target=Counter.name, type=string), type=string" type=string
        /// @resolution.name source=name target=Counter.constructor.name
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=Counter.constructor.name

        this.unit = unit;
        /// @resolution.receiver source=this kind=this declaration=Counter type=&'managed Counter
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.unit kind=place
        /// @resolution.place source=this.unit placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.unit root=this keys=[unit]
        /// @resolution.assignment source=this.unit write="receiver=&'managed Counter, target=field(receiver=&'managed Counter, target=Counter.unit, type=string | undefined), type=string | undefined" type=string | undefined
        /// @resolution.name source=unit target=Counter.constructor.unit
        /// @resolution.place source=unit placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=unit root=Counter.constructor.unit

    }
}

export function counter(name: string, unit?: string): Counter {
/// @type.symbol symbol=counter type=(string, string | undefined?) => Counter
/// @type.symbol symbol=counter.name source="name: string" type=string
/// @type.symbol symbol=counter.unit source="unit?: string" type=string | undefined
/// @resolution.name source=Counter target=Counter

    new Counter(name, unit)
    /// @resolution.construct source="new Counter(name, unit)" parameters=(string, string | undefined) arguments=(provided(name) as string, provided(unit) as string | undefined) return=Counter kind=class target=Counter constructor=Counter.constructor
    /// @resolution.name source=Counter target=Counter
    /// @resolution.name source=name target=counter.name
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=counter.name
    /// @resolution.name source=unit target=counter.unit
    /// @resolution.place source=unit placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=unit root=counter.unit

}
"#,
        r#"
"#,
    );
}

/// A constructor stores a readonly slice alias input in a field.
#[test]
fn test_store_a_readonly_slice_alias_input_in_a_field() {
    let session = TestSession::single(
        r#"
type Bytes = readonly [uint8];

class Reader {
    private readonly bytes: Bytes;

    private constructor(source: Bytes) {
        this.bytes = source;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bytes = readonly [uint8];

class Reader {
    private readonly bytes: Bytes;

    private constructor(source: Bytes) {
        this.bytes = source;
    }
}

=== dir ===
type Bytes = readonly [uint8];
/// @type.symbol symbol=Bytes source="type Bytes = readonly [uint8]" type=readonly Slice<uint8>
/// @definition.type symbol=Bytes source="type Bytes = readonly [uint8]" value=readonly Slice<uint8>

class Reader {
/// @type.symbol symbol=Reader type=typeof Reader
/// @definition.class symbol=Reader
/// @definition.field symbol=Reader.bytes source="private readonly bytes: Bytes" key=bytes visibility=private type=Bytes
/// @definition.method symbol=Reader.constructor slot=constructor visibility=private role=constructor type=(this: &'managed Reader, Bytes) => Reader

    private readonly bytes: Bytes;
    /// @type.symbol symbol=Reader.bytes source="private readonly bytes: Bytes" type=Bytes
    /// @resolution.name source=Bytes target=Bytes

    private constructor(source: Bytes) {
    /// @type.symbol symbol=Reader.constructor type=(this: &'managed Reader, Bytes) => Reader
    /// @type.symbol symbol=Reader.constructor.this type=&'managed Reader
    /// @type.symbol symbol=Reader.constructor.source source="source: Bytes" type=Bytes
    /// @resolution.name source=Bytes target=Bytes

        this.bytes = source;
        /// @resolution.receiver source=this kind=this declaration=Reader type=&'managed Reader
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.bytes kind=place
        /// @resolution.place source=this.bytes placement="local" lifetime="managed" access="readonly"
        /// @resolution.access source=this.bytes root=this keys=[bytes]
        /// @resolution.assignment source=this.bytes write="receiver=&'managed Reader, target=field(receiver=&'managed Reader, target=Reader.bytes, type=readonly Slice<uint8>), type=readonly Slice<uint8>" type=readonly Slice<uint8>
        /// @resolution.name source=source target=Reader.constructor.source
        /// @resolution.place source=source placement="local" lifetime="frame" access="readonly"
        /// @resolution.access source=source root=Reader.constructor.source

    }
}
"#,
        r#"
"#,
    );
}

/// A method returns a fresh object owned, built from its inputs and a field read through a borrow.
#[test]
fn test_return_a_fresh_object_owned_from_a_borrowed_receiver() {
    let session = TestSession::single(
        r#"
class Sink {}

class Counter {
    readonly name: string;

    readonly sink: readonly Sink | undefined;

    constructor(name: string, sink?: readonly Sink) {
        this.name = name;
        this.sink = sink;
    }
}

class Meter {
    private readonly sink: readonly Sink | undefined;

    constructor() {
        this.sink = undefined;
    }

    counter(&readonly this, name: string): ^Counter {
        new Counter(name, this.sink)
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Sink {}

class Counter {
    readonly name: string;

    readonly sink: readonly Sink | undefined;

    constructor(name: string, sink?: readonly Sink) {
        this.name = name;
        this.sink = sink;
    }
}

class Meter {
    private readonly sink: readonly Sink | undefined;

    constructor() {
        this.sink = undefined as readonly Sink | undefined;
    }

    counter(&readonly this, name: string): ^Counter {
        new Counter(name, this.sink)
    }
}

=== dir ===
class Sink {}
/// @type.symbol symbol=Sink source="class Sink {}" type=typeof Sink
/// @definition.class symbol=Sink source="class Sink {}"

class Counter {
/// @type.symbol symbol=Counter type=typeof Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.name source="readonly name: string" key=name type=string
/// @definition.field symbol=Counter.sink source="readonly sink: readonly Sink | undefined" key=sink type=readonly Sink | undefined
/// @definition.method symbol=Counter.constructor slot=constructor role=constructor type=(this: &'managed Counter, string, readonly Sink | undefined?) => Counter

    readonly name: string;
    /// @type.symbol symbol=Counter.name source="readonly name: string" type=string

    readonly sink: readonly Sink | undefined;
    /// @type.symbol symbol=Counter.sink source="readonly sink: readonly Sink | undefined" type=readonly Sink | undefined
    /// @resolution.name source=Sink target=Sink

    constructor(name: string, sink?: readonly Sink) {
    /// @type.symbol symbol=Counter.constructor type=(this: &'managed Counter, string, readonly Sink | undefined?) => Counter
    /// @type.symbol symbol=Counter.constructor.this type=&'managed Counter
    /// @type.symbol symbol=Counter.constructor.name source="name: string" type=string
    /// @type.symbol symbol=Counter.constructor.sink source="sink?: readonly Sink" type=readonly Sink | undefined
    /// @resolution.name source=Sink target=Sink

        this.name = name;
        /// @resolution.receiver source=this kind=this declaration=Counter type=&'managed Counter
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.name kind=place
        /// @resolution.place source=this.name placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.name root=this keys=[name]
        /// @resolution.assignment source=this.name write="receiver=&'managed Counter, target=field(receiver=&'managed Counter, target=Counter.name, type=string), type=string" type=string
        /// @resolution.name source=name target=Counter.constructor.name
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=Counter.constructor.name

        this.sink = sink;
        /// @resolution.receiver source=this kind=this declaration=Counter type=&'managed Counter
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.sink kind=place
        /// @resolution.place source=this.sink placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.sink root=this keys=[sink]
        /// @resolution.assignment source=this.sink write="receiver=&'managed Counter, target=field(receiver=&'managed Counter, target=Counter.sink, type=readonly Sink | undefined), type=readonly Sink | undefined" type=readonly Sink | undefined
        /// @resolution.name source=sink target=Counter.constructor.sink
        /// @resolution.place source=sink placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=sink root=Counter.constructor.sink

    }
}

class Meter {
/// @type.symbol symbol=Meter type=typeof Meter
/// @definition.class symbol=Meter
/// @definition.field symbol=Meter.sink source="private readonly sink: readonly Sink | undefined" key=sink visibility=private type=readonly Sink | undefined
/// @definition.method symbol=Meter.constructor slot=constructor role=constructor type=(this: &'managed Meter) => Meter
/// @definition.method symbol=Meter.counter slot=counter type=<Meter.counter.'a>(this: &Meter.counter.'a readonly Meter, string) => ^Counter

    private readonly sink: readonly Sink | undefined;
    /// @type.symbol symbol=Meter.sink source="private readonly sink: readonly Sink | undefined" type=readonly Sink | undefined
    /// @resolution.name source=Sink target=Sink

    constructor() {
    /// @type.symbol symbol=Meter.constructor type=(this: &'managed Meter) => Meter
    /// @type.symbol symbol=Meter.constructor.this type=&'managed Meter

        this.sink = undefined;
        /// @resolution.receiver source=this kind=this declaration=Meter type=&'managed Meter
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.sink kind=place
        /// @resolution.place source=this.sink placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.sink root=this keys=[sink]
        /// @resolution.assignment source=this.sink write="receiver=&'managed Meter, target=field(receiver=&'managed Meter, target=Meter.sink, type=readonly Sink | undefined), type=readonly Sink | undefined" type=readonly Sink | undefined

    }

    counter(&readonly this, name: string): ^Counter {
    /// @generic.template symbol=Meter.counter parameters=('a)
    /// @type.symbol symbol=Meter.counter type=<Meter.counter.'a>(this: &Meter.counter.'a readonly Meter, string) => ^Counter
    /// @type.symbol symbol=Meter.counter.this source="&readonly this" type=&Meter.counter.'a readonly Meter
    /// @type.symbol symbol=Meter.counter.name source="name: string" type=string
    /// @resolution.name source=Counter target=Counter

        new Counter(name, this.sink)
        /// @resolution.construct source="new Counter(name, this.sink)" parameters=(string, readonly Sink | undefined) arguments=(provided(name) as string, provided(this.sink) as readonly Sink | undefined) return=^Counter kind=class target=Counter constructor=Counter.constructor
        /// @resolution.name source=Counter target=Counter
        /// @resolution.name source=name target=Meter.counter.name
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=Meter.counter.name
        /// @resolution.member source=this.sink receiver=&Meter.counter.'a readonly Meter type=readonly Sink | undefined kind=field target_receiver=&Meter.counter.'a readonly Meter key=sink target=Meter.sink target_type=readonly Sink | undefined
        /// @resolution.receiver source=this kind=this declaration=Meter type=&Meter.counter.'a readonly Meter
        /// @resolution.place source=this placement=Meter.counter.'a lifetime=Meter.counter.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.sink placement=Meter.counter.'a lifetime=Meter.counter.'a access="readonly"
        /// @resolution.access source=this.sink root=this keys=[sink]

    }
}
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '^Counter' is not assignable to the method's 'this' type '&'managed Counter'"
/// @diagnostic.label line=23 column=9 span="new Counter(name, this.sink)" line_source="new Counter(name, this.sink)"
"#,
    );
}

/// A rest parameter of unknown values spreads into another rest parameter.
#[test]
fn test_spread_an_unknown_rest_parameter() {
    let session = TestSession::single(
        r#"
function inner(...values: unknown[]): void {}

function outer(...values: unknown[]): void {
    inner(...values);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function inner(...values: unknown[]): void {}

function outer(...values: unknown[]): void {
    inner(...values);
}

=== dir ===
function inner(...values: unknown[]): void {}
/// @type.symbol symbol=inner source="function inner(...values: unknown[]): void {}" type=(...unknown[]) => void
/// @type.symbol symbol=inner.values source="...values: unknown[]" type=unknown[]

function outer(...values: unknown[]): void {
/// @type.symbol symbol=outer type=(...unknown[]) => void
/// @type.symbol symbol=outer.values source="...values: unknown[]" type=unknown[]

    inner(...values);
    /// @resolution.name source=inner target=inner
    /// @resolution.call source=inner(...values) parameters=(unknown[]) arguments=(rest(spread(provided(...values) as unknown[], iterator=iterator#2(parameters=(), arguments=(), return=Iterator<unknown>), next=dynamic(Iterator<unknown> as Iterator<unknown>, Iterator.next)(parameters=(), arguments=(), return=IteratorResult<unknown, void>, regions=("managed" & "local"))) as unknown) pack=arrayFromOwnedSlice as unknown) return=void kind=symbol target=inner
    /// @generic.instantiation id=arrayFromOwnedSlice<unknown> template=arrayFromOwnedSlice arguments=(unknown)
    /// @generic.instantiation id=iterator#2<unknown> template=iterator#2 arguments=(unknown)
    /// @resolution.name source=values target=outer.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=outer.values

}
"#,
        r#"
"#,
    );
}

/// An extension of a local object newtype implements a requirement returning a string handle.
#[test]
fn test_implement_a_string_returning_requirement_on_a_local_newtype() {
    let session = TestSession::single(
        r#"
interface Display {
    display(this: &immutable this): ^string;
}

newtype Failure = {
    message?: string;
};

export extension of Failure implements Display {
    display(this: &immutable this): ^string {
        this.message ?? "failure"
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Display {
    display(this: &immutable this): ^string;
}

newtype Failure = {
    message?: string;
};

export extension of Failure implements Display {
    display(this: &immutable this): ^string {
        this.message ?? "failure"
    }
}

=== dir ===
interface Display {
/// @generic.template symbol=Display parameters=(this: Display)
/// @type.symbol symbol=Display type=Display
/// @definition.interface symbol=Display template=(this: Display)
/// @definition.where symbol=Display relation=satisfies left=this right=Display
/// @definition.method symbol=Display.display source="display(this: &immutable this): ^string" slot=display type=<Display.display.'a>(this: &Display.display.'a immutable this) => ^string

    display(this: &immutable this): ^string;
    /// @generic.template symbol=Display.display parent=template#0 parameters=('a)
    /// @type.symbol symbol=Display.display source="display(this: &immutable this): ^string" type=<Display.display.'a>(this: &Display.display.'a immutable this) => ^string
    /// @type.symbol symbol=Display.display.this source="this: &immutable this" type=&Display.display.'a immutable this

}

newtype Failure = {
/// @type.symbol symbol=Failure type=Failure
/// @definition.newtype symbol=Failure backing={ message?: string } constructors=[({ message?: string }) => Failure]

    message?: string;
    /// @type.symbol symbol=Failure.message source="message?: string" type=string

};

export extension of Failure implements Display {
/// @definition.extension symbol=<module>#2 form=exported target=Failure
/// @definition.implements symbol=<module>#2 source=Display target=Display
/// @definition.method symbol=display slot=display type=<display.'a>(this: &display.'a immutable Failure) => ^string
/// @definition.conformance symbol=<module>#2 member=display requirement=Display.display
/// @resolution.name source=Failure target=Failure
/// @resolution.name source=Display target=Display

    display(this: &immutable this): ^string {
    /// @generic.template symbol=display parent=template#1 parameters=('a)
    /// @type.symbol symbol=display type=<display.'a>(this: &display.'a immutable Failure) => ^string
    /// @type.symbol symbol=display.this source="this: &immutable this" type=&display.'a immutable Failure

        this.message ?? "failure"
        /// @resolution.member source=this.message receiver=&display.'a immutable Failure type=string | undefined kind=field target_receiver=&display.'a immutable Failure adjustments=(newtype.payload(Failure, &display.'a immutable { message?: string })) key=message target_type=string | undefined
        /// @resolution.operator source="this.message ?? \"failure\"" type=^string operator="??" kind=builtin operands=[this.message as string | undefined families=(string | undefined), "failure" as "failure" families=(string)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&display.'a immutable Failure
        /// @resolution.place source=this placement=display.'a lifetime=display.'a access="immutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.message placement=display.'a lifetime=display.'a access="immutable"
        /// @resolution.access source=this.message root=this keys=[message]

    }
}
"#,
        r#"
/// @diagnostic.error id=return-not-assignable message="type 'string' is not assignable to the declared result type '^string'"
/// @diagnostic.label line=12 column=9 span="this.message" line_source="this.message ?? \"failure\""
"#,
    );
}

/// A class's cell field holds an owned deque replaced through a readonly receiver.
#[test]
fn test_replace_an_owned_deque_in_a_cell_field_through_a_readonly_receiver() {
    let session = TestSession::single(
        r#"
import { Deque } from "destack:collections";
import { Cell } from "destack:memory";
import { Ready } from "destack:async";

class State<T> {
    private readonly values: Cell<Deque<Ready<T>>>;

    constructor() {
        this.values = Cell.new(Deque.new());
    }

    push(&readonly this, value: T): void {
        let values = this.values.replace(Deque.new());
        values.pushBack(Ready { value });
        this.values.set(values);
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Ready } from "destack:async";
import { Deque } from "destack:collections";
import { Cell } from "destack:memory";

class State<in out T> {
    private readonly values: Cell<Deque<Ready<T>>>;

    constructor() {
        this.values = Cell.new<Deque<Ready<T>>>(Deque.new<Ready<T>>() as Deque<Ready<T>>);
    }

    push(&readonly this, value: T): void {
        let values: Deque<Ready<T>> = this.values.replace<Deque<Ready<T>>, 'a>(
            Deque.new<Ready<T>>() as Deque<Ready<T>>,
        );
        values.pushBack<Ready<T>, "managed">(Ready<T> { value });
        this.values.set<Deque<Ready<T>>, 'a>(values);
    }
}

=== dir ===
import { Deque } from "destack:collections";
import { Cell } from "destack:memory";
import { Ready } from "destack:async";

class State<T> {
/// @generic.template symbol=State parameters=(in out T)
/// @type.symbol symbol=State type=typeof State
/// @definition.class symbol=State template=(in out T)
/// @definition.field symbol=State.values source="private readonly values: Cell<Deque<Ready<T>>>" key=values visibility=private type=Cell<Deque<Ready<T>>>
/// @definition.method symbol=State.constructor slot=constructor role=constructor type=(this: &'managed State<T>) => State<T>
/// @definition.method symbol=State.push slot=push type=<State.push.'a>(this: &State.push.'a readonly State<T>, T) => void
/// @type.symbol symbol=State.T source=T type=T

    private readonly values: Cell<Deque<Ready<T>>>;
    /// @type.symbol symbol=State.values source="private readonly values: Cell<Deque<Ready<T>>>" type=Cell<Deque<Ready<T>>>
    /// @resolution.name source=Cell target=Cell
    /// @resolution.name source=Deque target=Deque
    /// @resolution.name source=Ready target=Ready
    /// @resolution.name source=T target=State.T

    constructor() {
    /// @type.symbol symbol=State.constructor type=(this: &'managed State<T>) => State<T>
    /// @type.symbol symbol=State.constructor.this type=&'managed State<T>

        this.values = Cell.new(Deque.new());
        /// @resolution.receiver source=this kind=this declaration=State type=&'managed State<T>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.values kind=place
        /// @resolution.place source=this.values placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.values root=this keys=[values]
        /// @resolution.assignment source=this.values write="receiver=&'managed State<T>, target=field(receiver=&'managed State<T>, target=State.values, type=Cell<Deque<Ready<T>>>), type=Cell<Deque<Ready<T>>>" type=Cell<Deque<Ready<T>>>
        /// @resolution.name source=Cell target=Cell
        /// @resolution.member source=Cell.new receiver=Cell type=(T#2) => Cell<T#2> kind=symbol target_receiver=Cell target=new#2
        /// @resolution.call source=Cell.new(Deque.new()) parameters=(Deque<Ready<T>>) arguments=(provided(Deque.new()) as Deque<Ready<T>>) return=Cell<Deque<Ready<T>>> kind=symbol target=new#2 instance=Cell<Deque<Ready<T>>>.<extension#2>.new#2
        /// @generic.instantiation id=new#2<Deque<Ready<T>>> template=new#2 arguments=(Deque<Ready<T>>) owner=State.constructor
        /// @resolution.name source=Deque target=Deque
        /// @resolution.member source=Deque.new receiver=typeof Deque type=() => ^Deque<T#4> kind=symbol target_receiver=typeof Deque target=new
        /// @resolution.call source=Deque.new() parameters=() return=^Deque<Ready<T>> kind=symbol target=new instance=Deque<Ready<T>>.<extension#4>.new
        /// @generic.instantiation id=new<Ready<T>> template=new arguments=(Ready<T>) owner=State.constructor

    }

    push(&readonly this, value: T): void {
    /// @generic.template symbol=State.push parent=template#0 parameters=('a)
    /// @type.symbol symbol=State.push type=<State.push.'a>(this: &State.push.'a readonly State<T>, T) => void
    /// @type.symbol symbol=State.push.this source="&readonly this" type=&State.push.'a readonly State<T>
    /// @type.symbol symbol=State.push.value source="value: T" type=T
    /// @resolution.name source=T target=State.T

        let values = this.values.replace(Deque.new());
        /// @type.symbol symbol=State.push.values source=values type=Deque<Ready<T>>
        /// @resolution.pattern source=values kind=binding target=State.push.values
        /// @resolution.member source=this.values receiver=&State.push.'a readonly State<T> type=Cell<Deque<Ready<T>>> kind=field target_receiver=&State.push.'a readonly State<T> key=values target=State.values target_type=Cell<Deque<Ready<T>>>
        /// @resolution.member source=this.values.replace receiver=Cell<Deque<Ready<T>>> type=<replace.'a>(this: &replace.'a readonly Cell<Deque<Ready<T>>>, Deque<Ready<T>>) => Deque<Ready<T>> kind=symbol target_receiver=Cell<Deque<Ready<T>>> target=replace
        /// @resolution.call source=this.values.replace(Deque.new()) parameters=(Deque<Ready<T>>) arguments=(provided(Deque.new()) as Deque<Ready<T>>) return=Deque<Ready<T>> regions=(State.push.'a) kind=symbol target=replace receiver=Cell<Deque<Ready<T>>> adjustments=(borrow(&State.push.'a readonly Cell<Deque<Ready<T>>>)) instance=Cell<Deque<Ready<T>>>.<extension#2>.replace<State.push.'a>
        /// @resolution.receiver source=this kind=this declaration=State type=&State.push.'a readonly State<T>
        /// @resolution.place source=this placement=State.push.'a lifetime=State.push.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.values placement=State.push.'a lifetime=State.push.'a access="readonly"
        /// @resolution.access source=this.values root=this keys=[values]
        /// @generic.instantiation id="replace<Deque<Ready<T>>, State.push.'a>" template=replace arguments=(Deque<Ready<T>>, State.push.'a) owner=State.push
        /// @generic.instantiation id=replace<Deque<Ready<T>>> template=replace arguments=(Deque<Ready<T>>) owner=State.push
        /// @resolution.name source=Deque target=Deque
        /// @resolution.member source=Deque.new receiver=typeof Deque type=() => ^Deque<T#4> kind=symbol target_receiver=typeof Deque target=new
        /// @resolution.call source=Deque.new() parameters=() return=^Deque<Ready<T>> kind=symbol target=new instance=Deque<Ready<T>>.<extension#4>.new
        /// @generic.instantiation id=new<Ready<T>> template=new arguments=(Ready<T>) owner=State.push

        values.pushBack(Ready { value });
        /// @resolution.name source=values target=State.push.values
        /// @resolution.member source=values.pushBack receiver=Deque<Ready<T>> type=<pushBack.'a>(this: &pushBack.'a Deque<Ready<T>>, Ready<T>) => void kind=symbol target_receiver=Deque<Ready<T>> target=pushBack
        /// @resolution.call source="values.pushBack(Ready { value })" parameters=(Ready<T>) arguments=(provided(Ready { value }) as Ready<T>) return=void regions=("managed" & "local") kind=symbol target=pushBack receiver=Deque<Ready<T>> adjustments=(borrow(&'managed Deque<Ready<T>>)) instance="Deque<Ready<T>>.<extension#4>.pushBack<\"managed\" & \"local\">"
        /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=values root=State.push.values
        /// @generic.instantiation id="pushBack<Ready<T>, \"managed\" & \"local\">" template=pushBack arguments=(Ready<T>, "managed" & "local") owner=State.push
        /// @generic.instantiation id=pushBack<Ready<T>> template=pushBack arguments=(Ready<T>) owner=State.push
        /// @resolution.name source=Ready target=Ready
        /// @resolution.name source=value target=State.push.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=State.push.value

        this.values.set(values);
        /// @resolution.member source=this.values receiver=&State.push.'a readonly State<T> type=Cell<Deque<Ready<T>>> kind=field target_receiver=&State.push.'a readonly State<T> key=values target=State.values target_type=Cell<Deque<Ready<T>>>
        /// @resolution.member source=this.values.set receiver=Cell<Deque<Ready<T>>> type=<set.'a>(this: &set.'a readonly Cell<Deque<Ready<T>>>, Deque<Ready<T>>) => void kind=symbol target_receiver=Cell<Deque<Ready<T>>> target=set
        /// @resolution.call source=this.values.set(values) parameters=(Deque<Ready<T>>) arguments=(provided(values) as Deque<Ready<T>>) return=void regions=(State.push.'a) kind=symbol target=set receiver=Cell<Deque<Ready<T>>> adjustments=(borrow(&State.push.'a readonly Cell<Deque<Ready<T>>>)) instance=Cell<Deque<Ready<T>>>.<extension#2>.set<State.push.'a>
        /// @resolution.receiver source=this kind=this declaration=State type=&State.push.'a readonly State<T>
        /// @resolution.place source=this placement=State.push.'a lifetime=State.push.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.values placement=State.push.'a lifetime=State.push.'a access="readonly"
        /// @resolution.access source=this.values root=this keys=[values]
        /// @generic.instantiation id="set<Deque<Ready<T>>, State.push.'a>" template=set arguments=(Deque<Ready<T>>, State.push.'a) owner=State.push
        /// @generic.instantiation id=set<Deque<Ready<T>>> template=set arguments=(Deque<Ready<T>>) owner=State.push
        /// @resolution.name source=values target=State.push.values
        /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=values root=State.push.values

    }
}
"#,
        r#"
"#,
    );
}

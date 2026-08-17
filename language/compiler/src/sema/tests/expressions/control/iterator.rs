use crate::tests::{DirRows, TestSession};

#[test]
fn test_for_of_binds_array_elements() {
    let session = TestSession::single(
        r#"
const values: int32[] = [1, 2, 3];

for (const value of values) {
    value satisfies int32;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const values: int32[] = [1, 2, 3];

for (const value of values) {
    value satisfies int32;
}

=== dir ===
const values: int32[] = [1, 2, 3];
/// @type.symbol symbol=values source=values type=int32[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int32> template=collections.array.Array arguments=(int32)
/// @generic.instance id=memory.init.MaybeUninit<int32> template=memory.init.MaybeUninit arguments=(int32)
/// @generic.instance id=memory.raw.dangling<memory.init.MaybeUninit<int32>> template=memory.raw.dangling arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.Unique<Slice<memory.init.MaybeUninit<int32>>> template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<int32>>)
/// @generic.instance id=memory.unique.empty<memory.init.MaybeUninit<int32>> template=memory.unique.empty arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.emptyUniqueSlice<memory.init.MaybeUninit<int32>> template=memory.unique.emptyUniqueSlice arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.uniqueSliceFromRaw<memory.init.MaybeUninit<int32>> template=memory.unique.uniqueSliceFromRaw arguments=(memory.init.MaybeUninit<int32>)
/// @type.node source=[1, 2, 3] type=int32[]
/// @resolution.call source=[1, 2, 3] parameters=(&collections.array.arrayFromSlice.'a readonly Slice<collections.array.arrayFromSlice.T>) arguments=(rest(1, 2, 3) as int32) return=int32[] kind=symbol target=collections.array.arrayFromSlice instance=collections.array.arrayFromSlice<int32>
/// @generic.instantiation id=collections.array.arrayFromSlice<int32> template=collections.array.arrayFromSlice arguments=(int32)
/// @generic.instance id=collections.array.arrayFromSlice<int32> template=collections.array.arrayFromSlice arguments=(int32)
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3

for (const value of values) {
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=values type=int32[]
/// @resolution.name source=values target=values
/// @resolution.access source=values root=values

    value satisfies int32;
    /// @type.node source="value satisfies int32" type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=value root=value

}
"#,
    );
}

#[test]
fn test_for_of_rejects_non_iterable_receiver() {
    let session = TestSession::single(
        r#"
for (const value of 1) {
    value;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
for (const value of 1) {
    value;
}

=== dir ===
for (const value of 1) {
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

    value;
    /// @type.node source=value type=<error>
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=value root=value

}
"#,
        r#"
/// @diagnostic.error id=for-of-source-not-iterable message="for-of source must be iterable"
/// @diagnostic.label line=2 column=1 span="for" line_source="for (const value of 1) {"
"#,
    );
}

#[test]
fn test_for_in_binds_property_names_as_string() {
    let session = TestSession::single(
        r#"
const target = { a: 1, b: 2 };

for (const key in target) {
    key satisfies string;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const target: { a: float64; b: float64 } = { a: 1, b: 2 };

for (const key in target) {
    key satisfies string;
}

=== dir ===
const target = { a: 1, b: 2 };
/// @type.symbol symbol=target source=target type={ a: float64; b: float64 }
/// @resolution.pattern source=target kind=binding target=target
/// @type.node source={ a: 1, b: 2 } type={ a: float64; b: float64 }
/// @type.node source=1 type=1
/// @type.node source=2 type=2

for (const key in target) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=target type={ a: float64; b: float64 }
/// @resolution.name source=target target=target
/// @resolution.access source=target root=target

    key satisfies string;
    /// @type.node source="key satisfies string" type=string
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key
    /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=key root=key

}
"#,
    );
}

#[test]
fn test_for_in_rejects_literal_key_union_binding() {
    let session = TestSession::single(
        r#"
const target = { a: 1, b: 2 };

for (const key in target) {
    key satisfies "a" | "b";
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const target: { a: float64; b: float64 } = { a: 1, b: 2 };

for (const key in target) {
    key satisfies "a" | "b";
}

=== dir ===
const target = { a: 1, b: 2 };
/// @type.symbol symbol=target source=target type={ a: float64; b: float64 }
/// @resolution.pattern source=target kind=binding target=target
/// @type.node source={ a: 1, b: 2 } type={ a: float64; b: float64 }
/// @type.node source=1 type=1
/// @type.node source=2 type=2

for (const key in target) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=target type={ a: float64; b: float64 }
/// @resolution.name source=target target=target
/// @resolution.access source=target root=target

    key satisfies "a" | "b";
    /// @type.node source="key satisfies \"a\" | \"b\"" type=string
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key
    /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=key root=key

}
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'string' does not satisfy '\"a\" | \"b\"'"
/// @diagnostic.label line=5 column=9 span="satisfies" line_source="key satisfies \"a\" | \"b\";"
"#,
    );
}

#[test]
fn test_for_in_binds_union_property_names_as_string() {
    let session = TestSession::single(
        r#"
declare const target: { a: int32 } | { b: int32 };

for (const key in target) {
    key satisfies string;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const target: { a: int32 } | { b: int32 };

for (const key in target) {
    key satisfies string;
}

=== dir ===
declare const target: { a: int32 } | { b: int32 };
/// @type.symbol symbol=target source=target type={ a: int32 } | { b: int32 }
/// @resolution.pattern source=target kind=binding target=target

for (const key in target) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=target type={ a: int32 } | { b: int32 }
/// @resolution.name source=target target=target
/// @resolution.access source=target root=target

    key satisfies string;
    /// @type.node source="key satisfies string" type=string
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key
    /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=key root=key

}
"#,
    );
}

#[test]
fn test_for_in_accepts_readonly_object_receiver() {
    let session = TestSession::single(
        r#"
const target = { a: 1, b: 2 };

for (const key in &readonly target) {
    key satisfies string;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const target: { a: float64; b: float64 } = { a: 1, b: 2 };

for (const key in &readonly target) {
    key satisfies string;
}

=== dir ===
const target = { a: 1, b: 2 };
/// @type.symbol symbol=target source=target type={ a: float64; b: float64 }
/// @resolution.pattern source=target kind=binding target=target
/// @type.node source={ a: 1, b: 2 } type={ a: float64; b: float64 }
/// @type.node source=1 type=1
/// @type.node source=2 type=2

for (const key in &readonly target) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source="&readonly target" type=&'static readonly { a: float64; b: float64 }
/// @type.node source=target type={ a: float64; b: float64 }
/// @resolution.name source=target target=target
/// @resolution.place source=target placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=target root=target

    key satisfies string;
    /// @type.node source="key satisfies string" type=string
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key
    /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=key root=key

}
"#,
    );
}

#[test]
fn test_for_in_rejects_primitive_receiver() {
    let session = TestSession::single(
        r#"
for (const key in 1) {
    key;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
for (const key in 1) {
    key;
}

=== dir ===
for (const key in 1) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=1 type=1

    key;
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key
    /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=key root=key

}
"#,
        r#"
/// @diagnostic.error id=for-in-source-not-object-shaped message="for-in source must be object-shaped"
/// @diagnostic.label line=2 column=1 span="for" line_source="for (const key in 1) {"
"#,
    );
}

#[test]
fn test_for_in_rejects_unknown_receiver() {
    let session = TestSession::single(
        r#"
declare const target: unknown;

for (const key in target) {
    key;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const target: Dynamic<unknown>;

for (const key in target) {
    key;
}

=== dir ===
declare const target: unknown;
/// @type.symbol symbol=target source=target type=Dynamic<unknown>
/// @resolution.pattern source=target kind=binding target=target

for (const key in target) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=target type=Dynamic<unknown>
/// @resolution.name source=target target=target
/// @resolution.access source=target root=target

    key;
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key
    /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=key root=key

}
"#,
        r#"
/// @diagnostic.error id=for-in-source-not-object-shaped message="for-in source must be object-shaped"
/// @diagnostic.label line=4 column=1 span="for" line_source="for (const key in target) {"
"#,
    );
}

#[test]
fn test_for_in_rejects_array_receiver() {
    let session = TestSession::single(
        r#"
for (const key in [1, 2, 3]) {
    key;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
for (const key in [1, 2, 3]) {
    key;
}

=== dir ===
for (const key in [1, 2, 3]) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=[1, 2, 3] type=float64[]
/// @resolution.call source=[1, 2, 3] parameters=(&collections.array.arrayFromSlice.'a readonly Slice<collections.array.arrayFromSlice.T>) arguments=(rest(1, 2, 3) as float64) return=float64[] kind=symbol target=collections.array.arrayFromSlice instance=collections.array.arrayFromSlice<float64>
/// @generic.instantiation id=collections.array.arrayFromSlice<float64> template=collections.array.arrayFromSlice arguments=(float64)
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3

    key;
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key
    /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=key root=key

}
"#,
        r#"
/// @diagnostic.error id=for-in-source-not-object-shaped message="for-in source must be object-shaped"
/// @diagnostic.label line=2 column=1 span="for" line_source="for (const key in [1, 2, 3]) {"
"#,
    );
}

#[test]
fn test_for_in_accepts_struct_receiver() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

declare const point: Point;

for (const key in point) {
    key satisfies string;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

declare const point: Point;

for (const key in point) {
    key satisfies string;
}

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

declare const point: Point;
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point

for (const key in point) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=point type=Point
/// @resolution.name source=point target=point
/// @resolution.access source=point root=point

    key satisfies string;
    /// @type.node source="key satisfies string" type=string
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key
    /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=key root=key

}
"#,
    );
}

#[test]
fn test_for_in_accepts_class_receiver() {
    let session = TestSession::single(
        r#"
class User {
    name: string = "";
}

declare const user: User;

for (const key in user) {
    key satisfies string;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string = "";
}

declare const user: User;

for (const key in user) {
    key satisfies string;
}

=== dir ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string = \"\"" key=name type=string

    name: string = "";
    /// @type.symbol symbol=User.name source="name: string = \"\"" type=string
    /// @type.node source="\"\"" type=""

}

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

for (const key in user) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=user type=User
/// @resolution.name source=user target=user
/// @resolution.access source=user root=user

    key satisfies string;
    /// @type.node source="key satisfies string" type=string
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key
    /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=key root=key

}
"#,
    );
}

#[test]
fn test_for_in_accepts_optional_fields() {
    let session = TestSession::single(
        r#"
declare const target: { name?: string; active: boolean };

for (const key in target) {
    key satisfies string;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const target: { name?: string; active: boolean };

for (const key in target) {
    key satisfies string;
}

=== dir ===
declare const target: { name?: string; active: boolean };
/// @type.symbol symbol=target source=target type={ name?: string; active: boolean }
/// @resolution.pattern source=target kind=binding target=target

for (const key in target) {
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key
/// @type.node source=target type={ name?: string; active: boolean }
/// @resolution.name source=target target=target
/// @resolution.access source=target root=target

    key satisfies string;
    /// @type.node source="key satisfies string" type=string
    /// @type.node source=key type=string
    /// @resolution.name source=key target=key
    /// @resolution.place source=key placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=key root=key

}
"#,
    );
}

#[test]
fn test_for_of_binding_accepts_compound_assignment() {
    let session = TestSession::single(
        r#"
function visit(values: int32[]): void {
    for (let value of values) {
        value += 1;
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function visit(values: int32[]): void {
    for (let value of values) {
        value += 1;
    }
}

=== dir ===
function visit(values: int32[]): void {
/// @type.symbol symbol=visit type=(int32[]) => void
/// @generic.instance id=Array<int32> template=collections.array.Array arguments=(int32)
/// @generic.instance id=memory.init.MaybeUninit<int32> template=memory.init.MaybeUninit arguments=(int32)
/// @generic.instance id=memory.raw.dangling<memory.init.MaybeUninit<int32>> template=memory.raw.dangling arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.Unique<Slice<memory.init.MaybeUninit<int32>>> template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<int32>>)
/// @generic.instance id=memory.unique.empty<memory.init.MaybeUninit<int32>> template=memory.unique.empty arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.emptyUniqueSlice<memory.init.MaybeUninit<int32>> template=memory.unique.emptyUniqueSlice arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.uniqueSliceFromRaw<memory.init.MaybeUninit<int32>> template=memory.unique.uniqueSliceFromRaw arguments=(memory.init.MaybeUninit<int32>)
/// @type.symbol symbol=visit.values source="values: int32[]" type=int32[]

    for (let value of values) {
    /// @type.symbol symbol=visit.value source=value type=int32
    /// @resolution.pattern source=value kind=binding target=visit.value
    /// @resolution.name source=values target=visit.values
    /// @resolution.access source=values root=visit.values

        value += 1;
        /// @resolution.name source=value target=visit.value
        /// @resolution.operator source="value += 1" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 1 as int32 families=(integer)]
        /// @resolution.pattern.assign source=value kind=place
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.assignment source=value read=binding(visit.value) write=binding(visit.value) type=int32
        /// @resolution.access source=value root=visit.value

    }
}
"#,
    );
}

#[test]
fn test_for_of_accepts_an_iterator_through_the_blanket_iterable() {
    let session = TestSession::single(
        r#"
function total(values: int32[]): int32 {
    let sum: int32 = 0;
    for (const (index, value) of values.iterator().enumerate()) {
        sum = sum + value;
    }
    return sum;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function total(values: int32[]): int32 {
    let sum: int32 = 0;
    for (const (index, value) of values.iterator<int32>().enumerate<int32>()) {
        sum = sum + value;
    }
    return sum;
}

=== dir ===
function total(values: int32[]): int32 {
/// @type.symbol symbol=total type=(int32[]) => int32
/// @generic.instance id=Array<int32> template=collections.array.Array arguments=(int32)
/// @generic.instance id=memory.init.MaybeUninit<int32> template=memory.init.MaybeUninit arguments=(int32)
/// @generic.instance id=memory.raw.dangling<memory.init.MaybeUninit<int32>> template=memory.raw.dangling arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.Unique<Slice<memory.init.MaybeUninit<int32>>> template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<int32>>)
/// @generic.instance id=memory.unique.empty<memory.init.MaybeUninit<int32>> template=memory.unique.empty arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.emptyUniqueSlice<memory.init.MaybeUninit<int32>> template=memory.unique.emptyUniqueSlice arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.uniqueSliceFromRaw<memory.init.MaybeUninit<int32>> template=memory.unique.uniqueSliceFromRaw arguments=(memory.init.MaybeUninit<int32>)
/// @type.symbol symbol=total.values source="values: int32[]" type=int32[]

    let sum: int32 = 0;
    /// @type.symbol symbol=total.sum source=sum type=int32
    /// @resolution.pattern source=sum kind=binding target=total.sum

    for (const (index, value) of values.iterator().enumerate()) {
    /// @resolution.pattern source=(index, value) kind=tuple fields=(total.index, total.value)
    /// @type.symbol symbol=total.index source=index type=isize
    /// @resolution.pattern source=index kind=binding target=total.index
    /// @type.symbol symbol=total.value source=value type=int32
    /// @resolution.pattern source=value kind=binding target=total.value
    /// @resolution.name source=values target=total.values
    /// @resolution.member source=values.iterator receiver=int32[] type=(this: int32[]) => iter.iterator.Iterator<int32> kind=symbol target_receiver=int32[] target=collections.array.iterator#2
    /// @resolution.member source=values.iterator().enumerate receiver=iter.iterator.Iterator<int32> type=(this: iter.iterator.Iterator<int32>) => iter.iterator.EnumeratedIterator<iter.iterator.Iterator<int32>, int32> kind=symbol target_receiver=iter.iterator.Iterator<int32> target=iter.iterator.Iterator.enumerate
    /// @resolution.call source=values.iterator() parameters=() return=iter.iterator.Iterator<int32> kind=symbol target=collections.array.iterator#2 receiver=int32[] instance=Array<int32>.<extension#3>.iterator#2
    /// @resolution.call source=values.iterator().enumerate() parameters=() return=iter.iterator.EnumeratedIterator<iter.iterator.Iterator<int32>, int32> kind=symbol target=iter.iterator.Iterator.enumerate receiver=iter.iterator.Iterator<int32> instance=iter.iterator.Iterator<int32>.enumerate
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=total.values
    /// @generic.instantiation id=collections.array.iterator#2<int32> template=collections.array.iterator#2 arguments=(int32)
    /// @generic.instantiation id=iter.iterator.Iterator.enumerate<int32> template=iter.iterator.Iterator.enumerate arguments=(int32)
    /// @generic.instance id="iter.iterator.DropIterator<iter.iterator.Iterator<int32>, int32>" template=iter.iterator.DropIterator arguments=(iter.iterator.Iterator<int32>, int32)
    /// @generic.instance id="iter.iterator.DropWhileIterator<iter.iterator.Iterator<int32>, int32>" template=iter.iterator.DropWhileIterator arguments=(iter.iterator.Iterator<int32>, int32)
    /// @generic.instance id="iter.iterator.EnumeratedIterator<iter.iterator.Iterator<int32>, int32>" template=iter.iterator.EnumeratedIterator arguments=(iter.iterator.Iterator<int32>, int32)
    /// @generic.instance id="iter.iterator.FilterIterator<iter.iterator.Iterator<int32>, int32>" template=iter.iterator.FilterIterator arguments=(iter.iterator.Iterator<int32>, int32)
    /// @generic.instance id="iter.iterator.InspectIterator<iter.iterator.Iterator<int32>, int32>" template=iter.iterator.InspectIterator arguments=(iter.iterator.Iterator<int32>, int32)
    /// @generic.instance id="iter.iterator.IteratorResult<int32, iter.iterator.Iterator<int32>.Return>" template=iter.iterator.IteratorResult arguments=(int32, iter.iterator.Iterator<int32>.Return)
    /// @generic.instance id="iter.iterator.IteratorResult<int32, void>" template=iter.iterator.IteratorResult arguments=(int32, void)
    /// @generic.instance id="iter.iterator.PeekableIterator<iter.iterator.Iterator<int32>, int32>" template=iter.iterator.PeekableIterator arguments=(iter.iterator.Iterator<int32>, int32)
    /// @generic.instance id="iter.iterator.TakeIterator<iter.iterator.Iterator<int32>, int32>" template=iter.iterator.TakeIterator arguments=(iter.iterator.Iterator<int32>, int32)
    /// @generic.instance id="iter.iterator.TakeWhileIterator<iter.iterator.Iterator<int32>, int32>" template=iter.iterator.TakeWhileIterator arguments=(iter.iterator.Iterator<int32>, int32)
    /// @generic.instance id=collections.array.iterator#2<int32> template=collections.array.iterator#2 arguments=(int32)
    /// @generic.instance id=iter.iterator.Iterator.enumerate<int32> template=iter.iterator.Iterator.enumerate arguments=(int32)
    /// @generic.instance id=iter.iterator.Iterator<int32> template=iter.iterator.Iterator arguments=(int32)
    /// @generic.instance id=iter.iterator.IteratorReturn<iter.iterator.Iterator<int32>.Return> template=iter.iterator.IteratorReturn arguments=(iter.iterator.Iterator<int32>.Return)
    /// @generic.instance id=iter.iterator.IteratorReturn<void> template=iter.iterator.IteratorReturn arguments=(void)
    /// @generic.instance id=iter.iterator.IteratorYield<int32> template=iter.iterator.IteratorYield arguments=(int32)

        sum = sum + value;
        /// @resolution.name source=sum target=total.sum
        /// @resolution.pattern.assign source=sum kind=place
        /// @resolution.access source=sum root=total.sum
        /// @resolution.assignment source=sum write=binding(total.sum) type=int32
        /// @resolution.name source=sum target=total.sum
        /// @resolution.operator source="sum + value" type=int32 operator="+" kind=builtin operands=[sum as int32 families=(integer), value as int32 families=(integer)]
        /// @resolution.place source=sum placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=sum root=total.sum
        /// @resolution.name source=value target=total.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="readonly"
        /// @resolution.access source=value root=total.value

    }
    return sum;
    /// @resolution.name source=sum target=total.sum
    /// @resolution.place source=sum placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=sum root=total.sum

}
"#,
    );
}

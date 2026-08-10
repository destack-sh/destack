use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_derives_struct_equality_field_wise() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

function requireEqual<T: Equal>(value: T): T {
    return value;
}

const value = requireEqual(Point { x: 1, y: 2 });
value satisfies Point;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

function requireEqual<T: Equal>(value: T): T {
    return value;
}

const value: Point = requireEqual<Point>(Point { x: 1, y: 2 });
value satisfies Point;

=== checked ===
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

function requireEqual<T: Equal>(value: T): T {
/// @generic.template symbol=requireEqual parameters=(T: Equal)
/// @type.symbol symbol=requireEqual type=<T: Equal>(T) => T
/// @type.symbol symbol=requireEqual.T source="T: Equal" type=T
/// @resolution.name source=Equal target=ops.equality.Equal
/// @type.symbol symbol=requireEqual.value source="value: T" type=T
/// @resolution.name source=T target=requireEqual.T
/// @resolution.name source=T target=requireEqual.T

    return value;
    /// @resolution.name source=value target=requireEqual.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=requireEqual.value

}

const value = requireEqual(Point { x: 1, y: 2 });
/// @type.symbol symbol=value source=value type=Point
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source="requireEqual(Point { x: 1, y: 2 })" parameters=(Point) arguments=(provided(Point { x: 1, y: 2 }) as Point) return=Point kind=symbol target=requireEqual instance=requireEqual<Point>
/// @generic.instance source="requireEqual(Point { x: 1, y: 2 })" id=requireEqual<Point>
/// @resolution.name source=Point target=Point

value satisfies Point;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.name source=Point target=Point

/// @generic.instance id=requireEqual<Point> template=requireEqual arguments=(Point)
"#,
    );
}

#[test]
fn test_check_blocks_total_equality_on_float_field() {
    let session = TestSession::single(
        r#"
struct Sample {
    label: string;
    weight: float64;
}

function requireEqual<T: Equal>(value: T): T {
    return value;
}

function requirePartial<T: PartialEqual>(value: T): T {
    return value;
}

const partial = requirePartial(Sample { label: "a", weight: 1.0 });
const total = requireEqual(Sample { label: "a", weight: 1.0 });
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Sample {
    label: string;
    weight: float64;
}

function requireEqual<T: Equal>(value: T): T {
    return value;
}

function requirePartial<T: PartialEqual>(value: T): T {
    return value;
}

const partial: Sample = requirePartial<Sample>(Sample { label: "a", weight: 1.0 });
const total: Sample = requireEqual<Sample>(Sample { label: "a", weight: 1.0 });

=== checked ===
struct Sample {
/// @type.symbol symbol=Sample type=Sample
/// @definition.struct symbol=Sample
/// @definition.field symbol=Sample.label source="label: string" key=label type=string
/// @definition.field symbol=Sample.weight source="weight: float64" key=weight type=float64

    label: string;
    /// @type.symbol symbol=Sample.label source="label: string" type=string

    weight: float64;
    /// @type.symbol symbol=Sample.weight source="weight: float64" type=float64

}

function requireEqual<T: Equal>(value: T): T {
/// @generic.template symbol=requireEqual parameters=(T#1: Equal)
/// @type.symbol symbol=requireEqual type=<T#1: Equal>(T#1) => T#1
/// @type.symbol symbol=requireEqual.T source="T: Equal" type=T#1
/// @resolution.name source=Equal target=ops.equality.Equal
/// @type.symbol symbol=requireEqual.value source="value: T" type=T#1
/// @resolution.name source=T target=requireEqual.T
/// @resolution.name source=T target=requireEqual.T

    return value;
    /// @resolution.name source=value target=requireEqual.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=requireEqual.value

}

function requirePartial<T: PartialEqual>(value: T): T {
/// @generic.template symbol=requirePartial parameters=(T#2: PartialEqual)
/// @type.symbol symbol=requirePartial type=<T#2: PartialEqual>(T#2) => T#2
/// @type.symbol symbol=requirePartial.T source="T: PartialEqual" type=T#2
/// @resolution.name source=PartialEqual target=ops.equality.PartialEqual
/// @type.symbol symbol=requirePartial.value source="value: T" type=T#2
/// @resolution.name source=T target=requirePartial.T
/// @resolution.name source=T target=requirePartial.T

    return value;
    /// @resolution.name source=value target=requirePartial.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=requirePartial.value

}

const partial = requirePartial(Sample { label: "a", weight: 1.0 });
/// @type.symbol symbol=partial source=partial type=Sample
/// @resolution.pattern source=partial kind=binding target=partial
/// @resolution.name source=requirePartial target=requirePartial
/// @resolution.call source="requirePartial(Sample { label: \"a\", weight: 1.0 })" parameters=(Sample) arguments=(provided(Sample { label: "a", weight: 1.0 }) as Sample) return=Sample kind=symbol target=requirePartial instance=requirePartial<Sample>
/// @generic.instance source="requirePartial(Sample { label: \"a\", weight: 1.0 })" id=requirePartial<Sample>
/// @resolution.name source=Sample target=Sample

const total = requireEqual(Sample { label: "a", weight: 1.0 });
/// @type.symbol symbol=total source=total type=Sample
/// @resolution.pattern source=total kind=binding target=total
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source="requireEqual(Sample { label: \"a\", weight: 1.0 })" parameters=(Sample) arguments=(provided(Sample { label: "a", weight: 1.0 }) as Sample) return=Sample kind=symbol target=requireEqual instance=requireEqual<Sample>
/// @generic.instance source="requireEqual(Sample { label: \"a\", weight: 1.0 })" id=requireEqual<Sample>
/// @resolution.name source=Sample target=Sample

/// @generic.instance id=requireEqual<Sample> template=requireEqual arguments=(Sample)
/// @generic.instance id=requirePartial<Sample> template=requirePartial arguments=(Sample)
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Sample' does not satisfy 'Equal'"
/// @diagnostic.label line=16 column=15 span="requireEqual(Sample { label: \"a\", weight: 1.0 })" line_source="const total = requireEqual(Sample { label: \"a\", weight: 1.0 });"
/// @diagnostic.related line=7 column=23 span="T" line_source="function requireEqual<T: Equal>(value: T): T {" message="required by this bound on 'T'"
/// @diagnostic.note message="'Equal' reduces to 'Equal<this>'"
"#,
    );
}

#[test]
fn test_check_derives_recursive_struct_clone() {
    let session = TestSession::single(
        r#"
struct Node {
    value: int32;
    next: Box<Node> | null;
}

function requireClone<T: Clone>(value: T): T {
    return value;
}

const node = requireClone(Node { value: 1, next: null });
node satisfies Node;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Node {
    value: int32;
    next: Box<Node> | null;
}

function requireClone<T: Clone>(value: T): T {
    return value;
}

const node: Node = requireClone<Node>(Node { value: 1, next: null as Box<Node> | null });
node satisfies Node;

=== checked ===
struct Node {
/// @type.symbol symbol=Node type=Node
/// @definition.struct symbol=Node
/// @definition.field symbol=Node.next source="next: Box<Node> | null" key=next type=Box<Node> | null
/// @definition.field symbol=Node.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Node.value source="value: int32" type=int32

    next: Box<Node> | null;
    /// @type.symbol symbol=Node.next source="next: Box<Node> | null" type=Box<Node> | null
    /// @resolution.name source=Box target=memory.box.Box
    /// @resolution.name source=Node target=Node

}

function requireClone<T: Clone>(value: T): T {
/// @generic.template symbol=requireClone parameters=(T: Clone)
/// @type.symbol symbol=requireClone type=<T: Clone>(T) => T
/// @type.symbol symbol=requireClone.T source="T: Clone" type=T
/// @resolution.name source=Clone target=memory.capability.Clone
/// @type.symbol symbol=requireClone.value source="value: T" type=T
/// @resolution.name source=T target=requireClone.T
/// @resolution.name source=T target=requireClone.T

    return value;
    /// @resolution.name source=value target=requireClone.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=requireClone.value

}

const node = requireClone(Node { value: 1, next: null });
/// @type.symbol symbol=node source=node type=Node
/// @resolution.pattern source=node kind=binding target=node
/// @resolution.name source=requireClone target=requireClone
/// @resolution.call source="requireClone(Node { value: 1, next: null })" parameters=(Node) arguments=(provided(Node { value: 1, next: null }) as Node) return=Node kind=symbol target=requireClone instance=requireClone<Node>
/// @generic.instance source="requireClone(Node { value: 1, next: null })" id=requireClone<Node>
/// @resolution.name source=Node target=Node

node satisfies Node;
/// @resolution.name source=node target=node
/// @resolution.place source=node placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=node root=node
/// @resolution.name source=Node target=Node

/// @generic.instance id=Box<Node> template=memory.box.Box arguments=(Node)
/// @generic.instance id=requireClone<Node> template=requireClone arguments=(Node)
"#,
    );
}

#[test]
fn test_check_derives_struct_hash_without_floats() {
    let session = TestSession::single(
        r#"
struct Key {
    id: int32;
    name: string;
}

struct Measure {
    value: float64;
}

function requireHash<T: Hash>(value: T): T {
    return value;
}

const key = requireHash(Key { id: 1, name: "a" });
const measure = requireHash(Measure { value: 1.0 });
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Key {
    id: int32;
    name: string;
}

struct Measure {
    value: float64;
}

function requireHash<T: Hash>(value: T): T {
    return value;
}

const key: Key = requireHash<Key>(Key { id: 1, name: "a" });
const measure: Measure = requireHash<Measure>(Measure { value: 1.0 });

=== checked ===
struct Key {
/// @type.symbol symbol=Key type=Key
/// @definition.struct symbol=Key
/// @definition.field symbol=Key.id source="id: int32" key=id type=int32
/// @definition.field symbol=Key.name source="name: string" key=name type=string

    id: int32;
    /// @type.symbol symbol=Key.id source="id: int32" type=int32

    name: string;
    /// @type.symbol symbol=Key.name source="name: string" type=string

}

struct Measure {
/// @type.symbol symbol=Measure type=Measure
/// @definition.struct symbol=Measure
/// @definition.field symbol=Measure.value source="value: float64" key=value type=float64

    value: float64;
    /// @type.symbol symbol=Measure.value source="value: float64" type=float64

}

function requireHash<T: Hash>(value: T): T {
/// @generic.template symbol=requireHash parameters=(T: Hash)
/// @type.symbol symbol=requireHash type=<T: Hash>(T) => T
/// @type.symbol symbol=requireHash.T source="T: Hash" type=T
/// @resolution.name source=Hash target=ops.hash.Hash
/// @type.symbol symbol=requireHash.value source="value: T" type=T
/// @resolution.name source=T target=requireHash.T
/// @resolution.name source=T target=requireHash.T

    return value;
    /// @resolution.name source=value target=requireHash.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=requireHash.value

}

const key = requireHash(Key { id: 1, name: "a" });
/// @type.symbol symbol=key source=key type=Key
/// @resolution.pattern source=key kind=binding target=key
/// @resolution.name source=requireHash target=requireHash
/// @resolution.call source="requireHash(Key { id: 1, name: \"a\" })" parameters=(Key) arguments=(provided(Key { id: 1, name: "a" }) as Key) return=Key kind=symbol target=requireHash instance=requireHash<Key>
/// @generic.instance source="requireHash(Key { id: 1, name: \"a\" })" id=requireHash<Key>
/// @resolution.name source=Key target=Key

const measure = requireHash(Measure { value: 1.0 });
/// @type.symbol symbol=measure source=measure type=Measure
/// @resolution.pattern source=measure kind=binding target=measure
/// @resolution.name source=requireHash target=requireHash
/// @resolution.call source="requireHash(Measure { value: 1.0 })" parameters=(Measure) arguments=(provided(Measure { value: 1.0 }) as Measure) return=Measure kind=symbol target=requireHash instance=requireHash<Measure>
/// @generic.instance source="requireHash(Measure { value: 1.0 })" id=requireHash<Measure>
/// @resolution.name source=Measure target=Measure

/// @generic.instance id=requireHash<Key> template=requireHash arguments=(Key)
/// @generic.instance id=requireHash<Measure> template=requireHash arguments=(Measure)
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Measure' does not satisfy 'Hash'"
/// @diagnostic.label line=16 column=17 span="requireHash(Measure { value: 1.0 })" line_source="const measure = requireHash(Measure { value: 1.0 });"
/// @diagnostic.related line=11 column=22 span="T" line_source="function requireHash<T: Hash>(value: T): T {" message="required by this bound on 'T'"
"#,
    );
}

#[test]
fn test_check_requires_written_derive_for_compare() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

function requireCompare<T: Compare>(value: T): T {
    return value;
}

const value = requireCompare(Point { x: 1, y: 2 });
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

function requireCompare<T: Compare>(value: T): T {
    return value;
}

const value: Point = requireCompare<Point>(Point { x: 1, y: 2 });

=== checked ===
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

function requireCompare<T: Compare>(value: T): T {
/// @generic.template symbol=requireCompare parameters=(T: Compare)
/// @type.symbol symbol=requireCompare type=<T: Compare>(T) => T
/// @type.symbol symbol=requireCompare.T source="T: Compare" type=T
/// @resolution.name source=Compare target=ops.comparison.Compare
/// @type.symbol symbol=requireCompare.value source="value: T" type=T
/// @resolution.name source=T target=requireCompare.T
/// @resolution.name source=T target=requireCompare.T

    return value;
    /// @resolution.name source=value target=requireCompare.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=requireCompare.value

}

const value = requireCompare(Point { x: 1, y: 2 });
/// @type.symbol symbol=value source=value type=Point
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=requireCompare target=requireCompare
/// @resolution.call source="requireCompare(Point { x: 1, y: 2 })" parameters=(Point) arguments=(provided(Point { x: 1, y: 2 }) as Point) return=Point kind=symbol target=requireCompare instance=requireCompare<Point>
/// @generic.instance source="requireCompare(Point { x: 1, y: 2 })" id=requireCompare<Point>
/// @resolution.name source=Point target=Point

/// @generic.instance id=requireCompare<Point> template=requireCompare arguments=(Point)
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Point' does not satisfy 'Compare'"
/// @diagnostic.label line=11 column=15 span="requireCompare(Point { x: 1, y: 2 })" line_source="const value = requireCompare(Point { x: 1, y: 2 });"
/// @diagnostic.related line=7 column=25 span="T" line_source="function requireCompare<T: Compare>(value: T): T {" message="required by this bound on 'T'"
/// @diagnostic.note message="'Compare' reduces to 'Compare<this>'"
"#,
    );
}

#[test]
fn test_check_equates_classes_by_identity() {
    let session = TestSession::single(
        r#"
class Session {
    id: int32 = 0;
}

function requireEqual<T: Equal>(value: T): T {
    return value;
}

function requireHash<T: Hash>(value: T): T {
    return value;
}

const session = requireEqual(new Session());
const hashed = requireHash(new Session());
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Session {
    id: int32 = 0;
}

function requireEqual<T: Equal>(value: T): T {
    return value;
}

function requireHash<T: Hash>(value: T): T {
    return value;
}

const session: Session = requireEqual<Session>(new Session());
const hashed: Session = requireHash<Session>(new Session());

=== checked ===
class Session {
/// @type.symbol symbol=Session type=Session
/// @definition.class symbol=Session
/// @definition.field symbol=Session.id source="id: int32 = 0" key=id type=int32

    id: int32 = 0;
    /// @type.symbol symbol=Session.id source="id: int32 = 0" type=int32

}

function requireEqual<T: Equal>(value: T): T {
/// @generic.template symbol=requireEqual parameters=(T#1: Equal)
/// @type.symbol symbol=requireEqual type=<T#1: Equal>(T#1) => T#1
/// @type.symbol symbol=requireEqual.T source="T: Equal" type=T#1
/// @resolution.name source=Equal target=ops.equality.Equal
/// @type.symbol symbol=requireEqual.value source="value: T" type=T#1
/// @resolution.name source=T target=requireEqual.T
/// @resolution.name source=T target=requireEqual.T

    return value;
    /// @resolution.name source=value target=requireEqual.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=requireEqual.value

}

function requireHash<T: Hash>(value: T): T {
/// @generic.template symbol=requireHash parameters=(T#2: Hash)
/// @type.symbol symbol=requireHash type=<T#2: Hash>(T#2) => T#2
/// @type.symbol symbol=requireHash.T source="T: Hash" type=T#2
/// @resolution.name source=Hash target=ops.hash.Hash
/// @type.symbol symbol=requireHash.value source="value: T" type=T#2
/// @resolution.name source=T target=requireHash.T
/// @resolution.name source=T target=requireHash.T

    return value;
    /// @resolution.name source=value target=requireHash.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=requireHash.value

}

const session = requireEqual(new Session());
/// @type.symbol symbol=session source=session type=Session
/// @resolution.pattern source=session kind=binding target=session
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source="requireEqual(new Session())" parameters=(Session) arguments=(provided(new Session()) as Session) return=Session kind=symbol target=requireEqual instance=requireEqual<Session>
/// @generic.instance source="requireEqual(new Session())" id=requireEqual<Session>
/// @resolution.construct source="new Session()" parameters=() return=Session kind=class target=Session constructor=default
/// @resolution.name source=Session target=Session

const hashed = requireHash(new Session());
/// @type.symbol symbol=hashed source=hashed type=Session
/// @resolution.pattern source=hashed kind=binding target=hashed
/// @resolution.name source=requireHash target=requireHash
/// @resolution.call source="requireHash(new Session())" parameters=(Session) arguments=(provided(new Session()) as Session) return=Session kind=symbol target=requireHash instance=requireHash<Session>
/// @generic.instance source="requireHash(new Session())" id=requireHash<Session>
/// @resolution.construct source="new Session()" parameters=() return=Session kind=class target=Session constructor=default
/// @resolution.name source=Session target=Session

/// @generic.instance id=requireEqual<Session> template=requireEqual arguments=(Session)
/// @generic.instance id=requireHash<Session> template=requireHash arguments=(Session)
"#,
    );
}

#[test]
fn test_check_reports_unsatisfiable_written_derive_at_declaration() {
    let session = TestSession::single(
        r#"
@derive(Equal)
struct Sample {
    weight: float64;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
@derive(Equal)
struct Sample {
    weight: float64;
}

=== checked ===
@derive(Equal)
/// @resolution.name source=derive target=decorator.derive.derive
/// @resolution.name source=Equal target=ops.equality.Equal

struct Sample {
/// @type.symbol symbol=Sample type=Sample
/// @definition.struct symbol=Sample
/// @definition.implements symbol=Sample source=Equal target=Equal<this>
/// @definition.field symbol=Sample.weight source="weight: float64" key=weight type=float64

    weight: float64;
    /// @type.symbol symbol=Sample.weight source="weight: float64" type=float64

}
"#,
        r#"
/// @diagnostic.error id=interface-not-implemented message="type 'Sample' does not implement interface 'Equal<this>'"
/// @diagnostic.label line=2 column=9 span="Equal" line_source="@derive(Equal)"
"#,
    );
}

#[test]
fn test_check_replaces_auto_set_with_written_derives() {
    let session = TestSession::single(
        r#"
@derive(Debug)
struct Point {
    x: int32;
}

function requireEqual<T: Equal>(value: T): T {
    return value;
}

const value = requireEqual(Point { x: 1 });
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
@derive(Debug)
struct Point {
    x: int32;
}

function requireEqual<T: Equal>(value: T): T {
    return value;
}

const value: Point = requireEqual<Point>(Point { x: 1 });

=== checked ===
@derive(Debug)
/// @resolution.name source=derive target=decorator.derive.derive
/// @resolution.name source=Debug target=ops.format.Debug

struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.implements symbol=Point source=Debug target=Debug
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

function requireEqual<T: Equal>(value: T): T {
/// @generic.template symbol=requireEqual parameters=(T: Equal)
/// @type.symbol symbol=requireEqual type=<T: Equal>(T) => T
/// @type.symbol symbol=requireEqual.T source="T: Equal" type=T
/// @resolution.name source=Equal target=ops.equality.Equal
/// @type.symbol symbol=requireEqual.value source="value: T" type=T
/// @resolution.name source=T target=requireEqual.T
/// @resolution.name source=T target=requireEqual.T

    return value;
    /// @resolution.name source=value target=requireEqual.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=requireEqual.value

}

const value = requireEqual(Point { x: 1 });
/// @type.symbol symbol=value source=value type=Point
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source="requireEqual(Point { x: 1 })" parameters=(Point) arguments=(provided(Point { x: 1 }) as Point) return=Point kind=symbol target=requireEqual instance=requireEqual<Point>
/// @generic.instance source="requireEqual(Point { x: 1 })" id=requireEqual<Point>
/// @resolution.name source=Point target=Point

/// @generic.instance id=requireEqual<Point> template=requireEqual arguments=(Point)
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Point' does not satisfy 'Equal'"
/// @diagnostic.label line=11 column=15 span="requireEqual(Point { x: 1 })" line_source="const value = requireEqual(Point { x: 1 });"
/// @diagnostic.related line=7 column=23 span="T" line_source="function requireEqual<T: Equal>(value: T): T {" message="required by this bound on 'T'"
/// @diagnostic.note message="'Equal' reduces to 'Equal<this>'"
"#,
    );
}

#[test]
fn test_check_dispatches_equality_through_partial_equal() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

class Session {
    id: int32 = 0;
}

const a = Point { x: 1 };
const b = Point { x: 2 };
const same = a == b;
same satisfies boolean;

const s1 = new Session();
const s2 = new Session();
const csame = s1 == s2;
const cstrict = s1 === s2;
csame satisfies boolean;
cstrict satisfies boolean;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

class Session {
    id: int32 = 0;
}

const a: Point = Point { x: 1 };
const b: Point = Point { x: 2 };
const same: boolean = a == b;
same satisfies boolean;

const s1: Session = new Session();
const s2: Session = new Session();
const csame: boolean = s1 == s2;
const cstrict: boolean = s1 === s2;
csame satisfies boolean;
cstrict satisfies boolean;

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

class Session {
/// @type.symbol symbol=Session type=Session
/// @definition.class symbol=Session
/// @definition.field symbol=Session.id source="id: int32 = 0" key=id type=int32

    id: int32 = 0;
    /// @type.symbol symbol=Session.id source="id: int32 = 0" type=int32

}

const a = Point { x: 1 };
/// @type.symbol symbol=a source=a type=Point
/// @resolution.pattern source=a kind=binding target=a
/// @resolution.name source=Point target=Point

const b = Point { x: 2 };
/// @type.symbol symbol=b source=b type=Point
/// @resolution.pattern source=b kind=binding target=b
/// @resolution.name source=Point target=Point

const same = a == b;
/// @type.symbol symbol=same source=same type=boolean
/// @resolution.pattern source=same kind=binding target=same
/// @resolution.name source=a target=a
/// @resolution.operator source="a == b" type=boolean operator="==" kind=builtin operands=[a as Point, b as Point]
/// @resolution.place source=a placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=a root=a
/// @resolution.name source=b target=b
/// @resolution.place source=b placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=b root=b

same satisfies boolean;
/// @resolution.name source=same target=same
/// @resolution.place source=same placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=same root=same

const s1 = new Session();
/// @type.symbol symbol=s1 source=s1 type=Session
/// @resolution.pattern source=s1 kind=binding target=s1
/// @resolution.construct source="new Session()" parameters=() return=Session kind=class target=Session constructor=default
/// @resolution.name source=Session target=Session

const s2 = new Session();
/// @type.symbol symbol=s2 source=s2 type=Session
/// @resolution.pattern source=s2 kind=binding target=s2
/// @resolution.construct source="new Session()" parameters=() return=Session kind=class target=Session constructor=default
/// @resolution.name source=Session target=Session

const csame = s1 == s2;
/// @type.symbol symbol=csame source=csame type=boolean
/// @resolution.pattern source=csame kind=binding target=csame
/// @resolution.name source=s1 target=s1
/// @resolution.operator source="s1 == s2" type=boolean operator="==" kind=builtin operands=[s1 as Session, s2 as Session]
/// @resolution.place source=s1 placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=s1 root=s1
/// @resolution.name source=s2 target=s2
/// @resolution.place source=s2 placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=s2 root=s2

const cstrict = s1 === s2;
/// @type.symbol symbol=cstrict source=cstrict type=boolean
/// @resolution.pattern source=cstrict kind=binding target=cstrict
/// @resolution.name source=s1 target=s1
/// @resolution.operator source="s1 === s2" type=boolean operator="===" kind=builtin operands=[s1 as Session, s2 as Session]
/// @resolution.place source=s1 placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=s1 root=s1
/// @resolution.name source=s2 target=s2
/// @resolution.place source=s2 placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=s2 root=s2

csame satisfies boolean;
/// @resolution.name source=csame target=csame
/// @resolution.place source=csame placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=csame root=csame

cstrict satisfies boolean;
/// @resolution.name source=cstrict target=cstrict
/// @resolution.place source=cstrict placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=cstrict root=cstrict
"#,
    );
}

#[test]
fn test_check_rejects_strict_equality_on_value_types() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

const a = Point { x: 1 };
const b = Point { x: 2 };
const strict = a === b;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

const a: Point = Point { x: 1 };
const b: Point = Point { x: 2 };
const strict = a === b;

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

const a = Point { x: 1 };
/// @type.symbol symbol=a source=a type=Point
/// @resolution.pattern source=a kind=binding target=a
/// @resolution.name source=Point target=Point

const b = Point { x: 2 };
/// @type.symbol symbol=b source=b type=Point
/// @resolution.pattern source=b kind=binding target=b
/// @resolution.name source=Point target=Point

const strict = a === b;
/// @type.symbol symbol=strict source=strict type=<error>
/// @resolution.pattern source=strict kind=binding target=strict
/// @resolution.name source=a target=a
/// @resolution.place source=a placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=a root=a
/// @resolution.name source=b target=b
/// @resolution.place source=b placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=b root=b
"#,
        r#"
/// @diagnostic.error id=no-strict-identity message="value type 'Point' has no identity, compare with '=='"
/// @diagnostic.label line=8 column=18 span="===" line_source="const strict = a === b;"
"#,
    );
}

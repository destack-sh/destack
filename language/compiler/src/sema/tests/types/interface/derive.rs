use crate::tests::{DirRows, TestSession};

/// A struct derives equality from each of its fields.
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

    session.assert_dir(
        "main.tspp",
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

function requireEqual<T: Equal>(value: T): T {
/// @generic.template symbol=requireEqual parameters=(T: Equal)
/// @type.symbol symbol=requireEqual type=<T: Equal>(T) => T
/// @type.symbol symbol=requireEqual.T source="T: Equal" type=T
/// @resolution.name source=Equal target=Equal
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
/// @generic.instantiation id=requireEqual<Point> template=requireEqual arguments=(Point)
/// @generic.instance id=PartialEqual.equal<Point> template=PartialEqual.equal arguments=()
/// @generic.instance id=requireEqual<Point> template=requireEqual arguments=(Point)
/// @resolution.name source=Point target=Point

value satisfies Point;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @resolution.name source=Point target=Point

/// @generic.template symbol=PartialEqual.equal parameters=('a, 'b)
/// @type.symbol symbol=PartialEqual.equal type=<PartialEqual.equal.'a, PartialEqual.equal.'b>(this: &PartialEqual.equal.'a immutable Point, &PartialEqual.equal.'b immutable Point) => boolean
/// @type.symbol symbol=PartialEqual.equal.other type=&PartialEqual.equal.'b immutable Point
/// @generic.instance id="equal#1<int32, PartialEqual.equal.'a, PartialEqual.equal.'b>" template=equal#1 arguments=(int32, PartialEqual.equal.'a, PartialEqual.equal.'b)
"#,
    );
}

/// A struct with a float field derives partial equality and rejects total equality.
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
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

=== dir ===
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
/// @resolution.name source=Equal target=Equal
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
/// @resolution.name source=PartialEqual target=PartialEqual
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
/// @generic.instantiation id=requirePartial<Sample> template=requirePartial arguments=(Sample)
/// @resolution.name source=Sample target=Sample

const total = requireEqual(Sample { label: "a", weight: 1.0 });
/// @type.symbol symbol=total source=total type=Sample
/// @resolution.pattern source=total kind=binding target=total
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source="requireEqual(Sample { label: \"a\", weight: 1.0 })" parameters=(Sample) arguments=(provided(Sample { label: "a", weight: 1.0 }) as Sample) return=Sample kind=symbol target=requireEqual instance=requireEqual<Sample>
/// @generic.instantiation id=requireEqual<Sample> template=requireEqual arguments=(Sample)
/// @resolution.name source=Sample target=Sample
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Sample' does not satisfy 'Equal<this>'"
/// @diagnostic.label line=16 column=15 span="requireEqual(Sample { label: \"a\", weight: 1.0 })" line_source="const total = requireEqual(Sample { label: \"a\", weight: 1.0 });"
/// @diagnostic.related line=7 column=23 span="T" line_source="function requireEqual<T: Equal>(value: T): T {" message="required by this bound on 'T'"
"#,
    );
}

/// A recursive struct derives Clone.
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

    session.assert_dir(
        "main.tspp",
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

=== dir ===
struct Node {
/// @type.symbol symbol=Node type=Node
/// @definition.struct symbol=Node
/// @definition.field symbol=Node.next source="next: Box<Node> | null" key=next type=Box<Node> | null
/// @definition.field symbol=Node.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Node.value source="value: int32" type=int32

    next: Box<Node> | null;
    /// @type.symbol symbol=Node.next source="next: Box<Node> | null" type=Box<Node> | null
    /// @generic.instance id=Box<Node> template=Box arguments=(Node)
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=Node target=Node

}

function requireClone<T: Clone>(value: T): T {
/// @generic.template symbol=requireClone parameters=(T: Clone)
/// @type.symbol symbol=requireClone type=<T: Clone>(T) => T
/// @type.symbol symbol=requireClone.T source="T: Clone" type=T
/// @resolution.name source=Clone target=Clone
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
/// @generic.instantiation id=requireClone<Node> template=requireClone arguments=(Node)
/// @generic.instance id=Clone.clone#1<Node> template=Clone.clone#1 arguments=()
/// @generic.instance id=requireClone<Node> template=requireClone arguments=(Node)
/// @resolution.name source=Node target=Node

node satisfies Node;
/// @resolution.name source=node target=node
/// @resolution.place source=node placement="local" lifetime="static" access="immutable"
/// @resolution.access source=node root=node
/// @resolution.name source=Node target=Node

/// @generic.template symbol=Clone.clone#1 parameters=('a)
/// @generic.template symbol=Clone.clone#2 parameters=('a)
/// @type.symbol symbol=Clone.clone#1 type=<Clone.clone#1.'a>(this: &Clone.clone#1.'a immutable Node) => ^Node
/// @type.symbol symbol=Clone.clone#2 type=<Clone.clone#2.'a>(this: &Clone.clone#2.'a immutable (Box<Node> | null)) => ^(Box<Node> | null)
/// @generic.instance id="Clone.clone#2<Box<Node> | null>" template=Clone.clone#2 arguments=()
/// @generic.instance id="clone<Node, Clone.clone#2.'a>" template=clone arguments=(Node, Clone.clone#2.'a)
/// @generic.instance id="clone<int32, Clone.clone#1.'a>" template=clone arguments=(int32, Clone.clone#1.'a)
"#,
    );
}

/// A struct derives Hash when no field holds a float.
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
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

=== dir ===
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
/// @resolution.name source=Hash target=Hash
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
/// @generic.instantiation id=requireHash<Key> template=requireHash arguments=(Key)
/// @resolution.name source=Key target=Key

const measure = requireHash(Measure { value: 1.0 });
/// @type.symbol symbol=measure source=measure type=Measure
/// @resolution.pattern source=measure kind=binding target=measure
/// @resolution.name source=requireHash target=requireHash
/// @resolution.call source="requireHash(Measure { value: 1.0 })" parameters=(Measure) arguments=(provided(Measure { value: 1.0 }) as Measure) return=Measure kind=symbol target=requireHash instance=requireHash<Measure>
/// @generic.instantiation id=requireHash<Measure> template=requireHash arguments=(Measure)
/// @resolution.name source=Measure target=Measure
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Measure' does not satisfy 'Hash'"
/// @diagnostic.label line=16 column=17 span="requireHash(Measure { value: 1.0 })" line_source="const measure = requireHash(Measure { value: 1.0 });"
/// @diagnostic.related line=11 column=22 span="T" line_source="function requireHash<T: Hash>(value: T): T {" message="required by this bound on 'T'"
"#,
    );
}

/// Compare holds only where a derive list names it.
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
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

function requireCompare<T: Compare>(value: T): T {
/// @generic.template symbol=requireCompare parameters=(T: Compare)
/// @type.symbol symbol=requireCompare type=<T: Compare>(T) => T
/// @type.symbol symbol=requireCompare.T source="T: Compare" type=T
/// @resolution.name source=Compare target=Compare
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
/// @generic.instantiation id=requireCompare<Point> template=requireCompare arguments=(Point)
/// @resolution.name source=Point target=Point
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Point' does not satisfy 'Compare<this>'"
/// @diagnostic.label line=11 column=15 span="requireCompare(Point { x: 1, y: 2 })" line_source="const value = requireCompare(Point { x: 1, y: 2 });"
/// @diagnostic.related line=7 column=25 span="T" line_source="function requireCompare<T: Compare>(value: T): T {" message="required by this bound on 'T'"
"#,
    );
}

/// A class derives equality and hashing from its identity.
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

    session.assert_dir(
        "main.tspp",
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

=== dir ===
class Session {
/// @type.symbol symbol=Session type=typeof Session
/// @definition.class symbol=Session
/// @definition.field symbol=Session.id source="id: int32 = 0" key=id type=int32

    id: int32 = 0;
    /// @type.symbol symbol=Session.id source="id: int32 = 0" type=int32

}

function requireEqual<T: Equal>(value: T): T {
/// @generic.template symbol=requireEqual parameters=(T#1: Equal)
/// @type.symbol symbol=requireEqual type=<T#1: Equal>(T#1) => T#1
/// @type.symbol symbol=requireEqual.T source="T: Equal" type=T#1
/// @resolution.name source=Equal target=Equal
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
/// @resolution.name source=Hash target=Hash
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
/// @generic.instantiation id=requireEqual<Session> template=requireEqual arguments=(Session)
/// @generic.instance id=PartialEqual.equal<Session> template=PartialEqual.equal arguments=()
/// @generic.instance id=requireEqual<Session> template=requireEqual arguments=(Session)
/// @resolution.construct source="new Session()" parameters=() return=Session kind=class target=Session constructor=default
/// @resolution.name source=Session target=Session

const hashed = requireHash(new Session());
/// @type.symbol symbol=hashed source=hashed type=Session
/// @resolution.pattern source=hashed kind=binding target=hashed
/// @resolution.name source=requireHash target=requireHash
/// @resolution.call source="requireHash(new Session())" parameters=(Session) arguments=(provided(new Session()) as Session) return=Session kind=symbol target=requireHash instance=requireHash<Session>
/// @generic.instantiation id=requireHash<Session> template=requireHash arguments=(Session)
/// @generic.instance id=Hash.hash<Session> template=Hash.hash arguments=()
/// @generic.instance id=requireHash<Session> template=requireHash arguments=(Session)
/// @resolution.construct source="new Session()" parameters=() return=Session kind=class target=Session constructor=default
/// @resolution.name source=Session target=Session

/// @generic.template symbol=Hash.hash parameters=('a, 'b)
/// @generic.template symbol=PartialEqual.equal parameters=('a, 'b)
/// @type.symbol symbol=Hash.hash type=<Hash.hash.'a, Hash.hash.'b>(this: &Hash.hash.'a immutable Session, &Hash.hash.'b Hasher) => void
/// @type.symbol symbol=Hash.hash.state type=&Hash.hash.'b Hasher
/// @type.symbol symbol=PartialEqual.equal type=<PartialEqual.equal.'a, PartialEqual.equal.'b>(this: &PartialEqual.equal.'a immutable Session, &PartialEqual.equal.'b immutable Session) => boolean
/// @type.symbol symbol=PartialEqual.equal.other type=&PartialEqual.equal.'b immutable Session
/// @generic.instance id="Cast.truncate<int32, usize>" template=Cast.truncate arguments=(int32, usize)
/// @generic.instance id="equal#1<int32, PartialEqual.equal.'a, PartialEqual.equal.'b>" template=equal#1 arguments=(int32, PartialEqual.equal.'a, PartialEqual.equal.'b)
/// @generic.instance id="hash#1<int32, Hash.hash.'a, Hash.hash.'b>" template=hash#1 arguments=(int32, Hash.hash.'a, Hash.hash.'b)
/// @generic.instance id="truncateInt<int32, usize>" template=truncateInt arguments=(int32, usize)
"#,
    );
}

/// A derive the fields cannot satisfy reports a diagnostic at the declaration.
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
@derive(Equal)
struct Sample {
    weight: float64;
}

=== dir ===
@derive(Equal)
/// @resolution.name source=derive target=derive
/// @resolution.name source=Equal target=Equal

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

/// Reject derive arguments that do not name derivable interfaces.
#[test]
fn test_reject_non_interface_derive_argument() {
    let session = TestSession::single(
        r#"
const value = 1;

@derive(value)
struct Point {
    x: int32;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const value: 1 = 1;

@derive(value)
struct Point {
    x: int32;
}

=== dir ===
const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value

@derive(value)
/// @resolution.name source=derive target=derive
/// @resolution.name source=value target=value

struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}
"#,
        r#"
/// @diagnostic.error id=invalid-derive-interface message="derive argument must name a derivable interface"
/// @diagnostic.label line=4 column=9 span="value" line_source="@derive(value)"
"#,
    );
}

/// Reject a derivable interface selected more than once.
#[test]
fn test_reject_duplicate_derive_interface() {
    let session = TestSession::single(
        r#"
@derive(Clone, Clone)
struct Point {
    x: int32;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
@derive(Clone, Clone)
struct Point {
    x: int32;
}

=== dir ===
@derive(Clone, Clone)
/// @resolution.name source=derive target=derive
/// @resolution.name source=Clone target=Clone
/// @resolution.name source=Clone target=Clone

struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}
"#,
        r#"
/// @diagnostic.error id=duplicate-derive-interface message="duplicate derive interface 'Clone'"
/// @diagnostic.label line=2 column=16 span="Clone" line_source="@derive(Clone, Clone)"
"#,
    );
}

/// Apply no interfaces from an invalid derive list.
#[test]
fn test_invalid_derive_list_applies_no_interfaces() {
    let session = TestSession::single(
        r#"
const invalid = 1;

@derive(Clone, invalid)
struct Point {
    x: int32;
}

function requireClone<T: Clone>(value: T): void {}
function requireEqual<T: Equal>(value: T): void {}

requireClone(Point { x: 1 });
requireEqual(Point { x: 1 });
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
const invalid: 1 = 1;

@derive(Clone, invalid)
struct Point {
    x: int32;
}

function requireClone<T: Clone>(value: T): void {}
function requireEqual<T: Equal>(value: T): void {}

requireClone<Point>(Point { x: 1 });
requireEqual<Point>(Point { x: 1 });

=== dir ===
const invalid = 1;
/// @type.symbol symbol=invalid source=invalid type=1
/// @resolution.pattern source=invalid kind=binding target=invalid

@derive(Clone, invalid)
/// @resolution.name source=derive target=derive
/// @resolution.name source=Clone target=Clone
/// @resolution.name source=invalid target=invalid

struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

function requireClone<T: Clone>(value: T): void {}
/// @generic.template symbol=requireClone parameters=(T#1: Clone)
/// @type.symbol symbol=requireClone source="function requireClone<T: Clone>(value: T): void {}" type=<T#1: Clone>(T#1) => void
/// @type.symbol symbol=requireClone.T source="T: Clone" type=T#1
/// @resolution.name source=Clone target=Clone
/// @type.symbol symbol=requireClone.value source="value: T" type=T#1
/// @resolution.name source=T target=requireClone.T

function requireEqual<T: Equal>(value: T): void {}
/// @generic.template symbol=requireEqual parameters=(T#2: Equal)
/// @type.symbol symbol=requireEqual source="function requireEqual<T: Equal>(value: T): void {}" type=<T#2: Equal>(T#2) => void
/// @type.symbol symbol=requireEqual.T source="T: Equal" type=T#2
/// @resolution.name source=Equal target=Equal
/// @type.symbol symbol=requireEqual.value source="value: T" type=T#2
/// @resolution.name source=T target=requireEqual.T

requireClone(Point { x: 1 });
/// @resolution.name source=requireClone target=requireClone
/// @resolution.call source="requireClone(Point { x: 1 })" parameters=(Point) arguments=(provided(Point { x: 1 }) as Point) return=void kind=symbol target=requireClone instance=requireClone<Point>
/// @generic.instantiation id=requireClone<Point> template=requireClone arguments=(Point)
/// @resolution.name source=Point target=Point

requireEqual(Point { x: 1 });
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source="requireEqual(Point { x: 1 })" parameters=(Point) arguments=(provided(Point { x: 1 }) as Point) return=void kind=symbol target=requireEqual instance=requireEqual<Point>
/// @generic.instantiation id=requireEqual<Point> template=requireEqual arguments=(Point)
/// @resolution.name source=Point target=Point
"#, r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Point' does not satisfy 'Clone'"
/// @diagnostic.label line=12 column=1 span="requireClone(Point { x: 1 })" line_source="requireClone(Point { x: 1 });"
/// @diagnostic.related line=9 column=23 span="T" line_source="function requireClone<T: Clone>(value: T): void {}" message="required by this bound on 'T'"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Point' does not satisfy 'Equal<this>'"
/// @diagnostic.label line=13 column=1 span="requireEqual(Point { x: 1 })" line_source="requireEqual(Point { x: 1 });"
/// @diagnostic.related line=10 column=23 span="T" line_source="function requireEqual<T: Equal>(value: T): void {}" message="required by this bound on 'T'"
/// @diagnostic.error id=invalid-derive-interface message="derive argument must name a derivable interface"
/// @diagnostic.label line=4 column=16 span="invalid" line_source="@derive(Clone, invalid)"
"#);
}

/// A written derive list bounds the interfaces a struct conforms to.
#[test]
fn test_limit_derived_interfaces_to_the_written_derive_list() {
    let session = TestSession::single(
        r#"
@derive(Copy)
struct Point {
    x: int32;
}

function requireClone<T: Clone>(value: T): T {
    return value;
}

function requireEqual<T: Equal>(value: T): T {
    return value;
}

const cloned = requireClone(Point { x: 1 });
const value = requireEqual(Point { x: 1 });
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
@derive(Copy)
struct Point {
    x: int32;
}

function requireClone<T: Clone>(value: T): T {
    return value;
}

function requireEqual<T: Equal>(value: T): T {
    return value;
}

const cloned: Point = requireClone<Point>(Point { x: 1 });
const value: Point = requireEqual<Point>(Point { x: 1 });

=== dir ===
@derive(Copy)
/// @resolution.name source=derive target=derive
/// @resolution.name source=Copy target=Copy

struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.implements symbol=Point source=Copy target=Copy
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

function requireClone<T: Clone>(value: T): T {
/// @generic.template symbol=requireClone parameters=(T#1: Clone)
/// @type.symbol symbol=requireClone type=<T#1: Clone>(T#1) => T#1
/// @type.symbol symbol=requireClone.T source="T: Clone" type=T#1
/// @resolution.name source=Clone target=Clone
/// @type.symbol symbol=requireClone.value source="value: T" type=T#1
/// @resolution.name source=T target=requireClone.T
/// @resolution.name source=T target=requireClone.T

    return value;
    /// @resolution.name source=value target=requireClone.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=requireClone.value

}

function requireEqual<T: Equal>(value: T): T {
/// @generic.template symbol=requireEqual parameters=(T#2: Equal)
/// @type.symbol symbol=requireEqual type=<T#2: Equal>(T#2) => T#2
/// @type.symbol symbol=requireEqual.T source="T: Equal" type=T#2
/// @resolution.name source=Equal target=Equal
/// @type.symbol symbol=requireEqual.value source="value: T" type=T#2
/// @resolution.name source=T target=requireEqual.T
/// @resolution.name source=T target=requireEqual.T

    return value;
    /// @resolution.name source=value target=requireEqual.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=requireEqual.value

}

const cloned = requireClone(Point { x: 1 });
/// @type.symbol symbol=cloned source=cloned type=Point
/// @resolution.pattern source=cloned kind=binding target=cloned
/// @resolution.name source=requireClone target=requireClone
/// @resolution.call source="requireClone(Point { x: 1 })" parameters=(Point) arguments=(provided(Point { x: 1 }) as Point) return=Point kind=symbol target=requireClone instance=requireClone<Point>
/// @generic.instantiation id=requireClone<Point> template=requireClone arguments=(Point)
/// @resolution.name source=Point target=Point

const value = requireEqual(Point { x: 1 });
/// @type.symbol symbol=value source=value type=Point
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source="requireEqual(Point { x: 1 })" parameters=(Point) arguments=(provided(Point { x: 1 }) as Point) return=Point kind=symbol target=requireEqual instance=requireEqual<Point>
/// @generic.instantiation id=requireEqual<Point> template=requireEqual arguments=(Point)
/// @resolution.name source=Point target=Point
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Point' does not satisfy 'Equal<this>'"
/// @diagnostic.label line=16 column=15 span="requireEqual(Point { x: 1 })" line_source="const value = requireEqual(Point { x: 1 });"
/// @diagnostic.related line=11 column=23 span="T" line_source="function requireEqual<T: Equal>(value: T): T {" message="required by this bound on 'T'"
"#,
    );
}

/// An equality expression dispatches through the PartialEqual conformance.
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

    session.assert_dir(
        "main.tspp",
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
const same: boolean = a == (b as &'static immutable Point);
same satisfies boolean;

const s1: Session = new Session();
const s2: Session = new Session();
const csame: boolean = s1 == (s2 as &'managed immutable Session);
const cstrict: boolean = s1 === s2;
csame satisfies boolean;
cstrict satisfies boolean;

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

class Session {
/// @type.symbol symbol=Session type=typeof Session
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
/// @resolution.operator source="a == b" type=boolean operator="==" kind=call parameters=(&'static immutable Point) arguments=(provided(b) as &'static immutable Point) return=boolean regions=("static" & "local", "static" & "local") kind=symbol target=PartialEqual.equal receiver=Point adjustments=(borrow(&'static immutable Point)) instance="PartialEqual<Point>.equal<\"static\" & \"local\", \"static\" & \"local\">"
/// @resolution.place source=a placement="local" lifetime="static" access="immutable"
/// @resolution.access source=a root=a
/// @generic.instantiation id="PartialEqual.equal<Point, Point, \"static\" & \"local\", \"static\" & \"local\">" template=PartialEqual.equal arguments=(Point, "static" & "local", "static" & "local")
/// @generic.instance id=PartialEqual.equal#1<Point> template=PartialEqual.equal#1 arguments=()
/// @resolution.name source=b target=b
/// @resolution.place source=b placement="local" lifetime="static" access="immutable"
/// @resolution.access source=b root=b

same satisfies boolean;
/// @resolution.name source=same target=same
/// @resolution.place source=same placement="local" lifetime="static" access="immutable"
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
/// @resolution.operator source="s1 == s2" type=boolean operator="==" kind=call parameters=(&'managed immutable Session) arguments=(provided(s2) as &'managed immutable Session) return=boolean regions=("managed" & "local", "managed" & "local") kind=symbol target=PartialEqual.equal receiver=Session adjustments=(borrow(&'managed immutable Session)) instance="PartialEqual<Session>.equal<\"managed\" & \"local\", \"managed\" & \"local\">"
/// @resolution.place source=s1 placement="local" lifetime="static" access="immutable"
/// @resolution.access source=s1 root=s1
/// @generic.instantiation id="PartialEqual.equal<Session, Session, \"managed\" & \"local\", \"managed\" & \"local\">" template=PartialEqual.equal arguments=(Session, "managed" & "local", "managed" & "local")
/// @generic.instance id=PartialEqual.equal#2<Session> template=PartialEqual.equal#2 arguments=()
/// @resolution.name source=s2 target=s2
/// @resolution.place source=s2 placement="local" lifetime="static" access="immutable"
/// @resolution.access source=s2 root=s2

const cstrict = s1 === s2;
/// @type.symbol symbol=cstrict source=cstrict type=boolean
/// @resolution.pattern source=cstrict kind=binding target=cstrict
/// @resolution.name source=s1 target=s1
/// @resolution.operator source="s1 === s2" type=boolean operator="===" kind=builtin operands=[s1 as Session, s2 as Session]
/// @resolution.place source=s1 placement="local" lifetime="static" access="immutable"
/// @resolution.access source=s1 root=s1
/// @resolution.name source=s2 target=s2
/// @resolution.place source=s2 placement="local" lifetime="static" access="immutable"
/// @resolution.access source=s2 root=s2

csame satisfies boolean;
/// @resolution.name source=csame target=csame
/// @resolution.place source=csame placement="local" lifetime="static" access="immutable"
/// @resolution.access source=csame root=csame

cstrict satisfies boolean;
/// @resolution.name source=cstrict target=cstrict
/// @resolution.place source=cstrict placement="local" lifetime="static" access="immutable"
/// @resolution.access source=cstrict root=cstrict

/// @generic.template symbol=PartialEqual.equal#1 parameters=('a, 'b)
/// @generic.template symbol=PartialEqual.equal#2 parameters=('a, 'b)
/// @type.symbol symbol=PartialEqual.equal#1 type=<PartialEqual.equal#1.'a, PartialEqual.equal#1.'b>(this: &PartialEqual.equal#1.'a immutable Point, &PartialEqual.equal#1.'b immutable Point) => boolean
/// @type.symbol symbol=PartialEqual.equal#2 type=<PartialEqual.equal#2.'a, PartialEqual.equal#2.'b>(this: &PartialEqual.equal#2.'a immutable Session, &PartialEqual.equal#2.'b immutable Session) => boolean
/// @type.symbol symbol=PartialEqual.equal.other#1 type=&PartialEqual.equal#1.'b immutable Point
/// @type.symbol symbol=PartialEqual.equal.other#2 type=&PartialEqual.equal#2.'b immutable Session
/// @generic.instance id="equal#1<int32, PartialEqual.equal#1.'a, PartialEqual.equal#1.'b>" template=equal#1 arguments=(int32, PartialEqual.equal#1.'a, PartialEqual.equal#1.'b)
/// @generic.instance id="equal#1<int32, PartialEqual.equal#2.'a, PartialEqual.equal#2.'b>" template=equal#1 arguments=(int32, PartialEqual.equal#2.'a, PartialEqual.equal#2.'b)
"#,
    );
}

/// Strict equality on a value type reports a diagnostic.
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

const a: Point = Point { x: 1 };
const b: Point = Point { x: 2 };
const strict = a === b;

=== dir ===
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
/// @resolution.place source=a placement="local" lifetime="static" access="immutable"
/// @resolution.access source=a root=a
/// @resolution.name source=b target=b
/// @resolution.place source=b placement="local" lifetime="static" access="immutable"
/// @resolution.access source=b root=b
"#,
        r#"
/// @diagnostic.error id=no-strict-identity message="value type 'Point' has no identity, compare with '=='"
/// @diagnostic.label line=8 column=18 span="===" line_source="const strict = a === b;"
"#,
    );
}

/// A Clone bound reports a diagnostic once a later argument fixes the parameter.
#[test]
fn test_reject_a_clone_bound_on_a_type_argument_inferred_later() {
    let session = TestSession::single(
        r#"
struct Blocker {
    run: ^Function<(), void, "once">;
}

struct Holder<Value> {
    value: Value;
}

declare function hold<Value>(): Holder<Value>;
declare function holdBlocker(): Holder<Blocker>;

function requireClone<T: Clone>(value: T, seed: T): T {
    return value;
}

const cloned = requireClone(hold(), holdBlocker());
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
struct Blocker {
    run: ^(() => void);
}

struct Holder<out Value> {
    value: Value;
}

declare function hold<Value>(): Holder<Value>;
declare function holdBlocker(): Holder<Blocker>;

function requireClone<T: Clone>(value: T, seed: T): T {
    return value;
}

const cloned: Holder<Blocker> = requireClone<Holder<Blocker>>(hold<Blocker>(), holdBlocker());

=== dir ===
struct Blocker {
/// @type.symbol symbol=Blocker type=Blocker
/// @definition.struct symbol=Blocker
/// @definition.field symbol=Blocker.run source="run: ^Function<(), void, \"once\">" key=run type=^Function<(), void, "once">

    run: ^Function<(), void, "once">;
    /// @type.symbol symbol=Blocker.run source="run: ^Function<(), void, \"once\">" type=^Function<(), void, "once">
    /// @resolution.name source=Function target=Function

}

struct Holder<Value> {
/// @generic.template symbol=Holder parameters=(out Value#1)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out Value#1)
/// @definition.field symbol=Holder.value source="value: Value" key=value type=Value#1
/// @type.symbol symbol=Holder.Value source=Value type=Value#1

    value: Value;
    /// @type.symbol symbol=Holder.value source="value: Value" type=Value#1
    /// @resolution.name source=Value target=Holder.Value

}

declare function hold<Value>(): Holder<Value>;
/// @generic.template symbol=hold parameters=(Value#2)
/// @type.symbol symbol=hold source="declare function hold<Value>(): Holder<Value>" type=<Value#2>() => Holder<Value#2>
/// @type.symbol symbol=hold.Value source=Value type=Value#2
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Value target=hold.Value

declare function holdBlocker(): Holder<Blocker>;
/// @type.symbol symbol=holdBlocker source="declare function holdBlocker(): Holder<Blocker>" type=() => Holder<Blocker>
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Blocker target=Blocker

function requireClone<T: Clone>(value: T, seed: T): T {
/// @generic.template symbol=requireClone parameters=(T: Clone)
/// @type.symbol symbol=requireClone type=<T: Clone>(T, T) => T
/// @type.symbol symbol=requireClone.T source="T: Clone" type=T
/// @resolution.name source=Clone target=Clone
/// @type.symbol symbol=requireClone.value source="value: T" type=T
/// @resolution.name source=T target=requireClone.T
/// @type.symbol symbol=requireClone.seed source="seed: T" type=T
/// @resolution.name source=T target=requireClone.T
/// @resolution.name source=T target=requireClone.T

    return value;
    /// @resolution.name source=value target=requireClone.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=requireClone.value

}

const cloned = requireClone(hold(), holdBlocker());
/// @type.symbol symbol=cloned source=cloned type=Holder<Blocker>
/// @resolution.pattern source=cloned kind=binding target=cloned
/// @resolution.name source=requireClone target=requireClone
/// @resolution.call source="requireClone(hold(), holdBlocker())" parameters=(Holder<Blocker>, Holder<Blocker>) arguments=(provided(hold()) as Holder<Blocker>, provided(holdBlocker()) as Holder<Blocker>) return=Holder<Blocker> kind=symbol target=requireClone instance=requireClone<Holder<Blocker>>
/// @generic.instantiation id=requireClone<Holder<Blocker>> template=requireClone arguments=(Holder<Blocker>)
/// @resolution.name source=hold target=hold
/// @resolution.call source=hold() parameters=() return=Holder<Blocker> kind=symbol target=hold instance=hold<Blocker>
/// @generic.instantiation id=hold<Blocker> template=hold arguments=(Blocker)
/// @resolution.name source=holdBlocker target=holdBlocker
/// @resolution.call source=holdBlocker() parameters=() return=Holder<Blocker> kind=symbol target=holdBlocker
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Holder<Blocker>' does not satisfy 'Clone'"
/// @diagnostic.label line=17 column=16 span="requireClone(hold(), holdBlocker())" line_source="const cloned = requireClone(hold(), holdBlocker());"
/// @diagnostic.related line=13 column=23 span="T" line_source="function requireClone<T: Clone>(value: T, seed: T): T {" message="required by this bound on 'T'"
"#,
    );
}

/// A Clone bound on a struct holding a once function field reports a diagnostic.
#[test]
fn test_reject_a_clone_bound_on_a_struct_with_a_once_function_field() {
    let session = TestSession::single(
        r#"
struct Blocker {
    run: ^Function<(), void, "once">;
}

struct Holder<Value> {
    value: Value;
}

declare const held: Holder<Blocker>;

function requireClone<T: Clone>(value: T): T {
    return value;
}

const cloned = requireClone(held);
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
struct Blocker {
    run: ^(() => void);
}

struct Holder<out Value> {
    value: Value;
}

declare const held: Holder<Blocker>;

function requireClone<T: Clone>(value: T): T {
    return value;
}

const cloned: Holder<Blocker> = requireClone<Holder<Blocker>>(held);

=== dir ===
struct Blocker {
/// @type.symbol symbol=Blocker type=Blocker
/// @definition.struct symbol=Blocker
/// @definition.field symbol=Blocker.run source="run: ^Function<(), void, \"once\">" key=run type=^Function<(), void, "once">

    run: ^Function<(), void, "once">;
    /// @type.symbol symbol=Blocker.run source="run: ^Function<(), void, \"once\">" type=^Function<(), void, "once">
    /// @resolution.name source=Function target=Function

}

struct Holder<Value> {
/// @generic.template symbol=Holder parameters=(out Value)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out Value)
/// @definition.field symbol=Holder.value source="value: Value" key=value type=Value
/// @type.symbol symbol=Holder.Value source=Value type=Value

    value: Value;
    /// @type.symbol symbol=Holder.value source="value: Value" type=Value
    /// @resolution.name source=Value target=Holder.Value

}

declare const held: Holder<Blocker>;
/// @type.symbol symbol=held source=held type=Holder<Blocker>
/// @resolution.pattern source=held kind=binding target=held
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Blocker target=Blocker

function requireClone<T: Clone>(value: T): T {
/// @generic.template symbol=requireClone parameters=(T: Clone)
/// @type.symbol symbol=requireClone type=<T: Clone>(T) => T
/// @type.symbol symbol=requireClone.T source="T: Clone" type=T
/// @resolution.name source=Clone target=Clone
/// @type.symbol symbol=requireClone.value source="value: T" type=T
/// @resolution.name source=T target=requireClone.T
/// @resolution.name source=T target=requireClone.T

    return value;
    /// @resolution.name source=value target=requireClone.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=requireClone.value

}

const cloned = requireClone(held);
/// @type.symbol symbol=cloned source=cloned type=Holder<Blocker>
/// @resolution.pattern source=cloned kind=binding target=cloned
/// @resolution.name source=requireClone target=requireClone
/// @resolution.call source=requireClone(held) parameters=(Holder<Blocker>) arguments=(provided(held) as Holder<Blocker>) return=Holder<Blocker> kind=symbol target=requireClone instance=requireClone<Holder<Blocker>>
/// @generic.instantiation id=requireClone<Holder<Blocker>> template=requireClone arguments=(Holder<Blocker>)
/// @resolution.name source=held target=held
/// @resolution.place source=held placement="local" lifetime="static" access="immutable"
/// @resolution.access source=held root=held
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Holder<Blocker>' does not satisfy 'Clone'"
/// @diagnostic.label line=16 column=16 span="requireClone(held)" line_source="const cloned = requireClone(held);"
/// @diagnostic.related line=12 column=23 span="T" line_source="function requireClone<T: Clone>(value: T): T {" message="required by this bound on 'T'"
"#,
    );
}

/// A Clone bound on a struct holding a repeatable function field resolves.
#[test]
fn test_accept_a_clone_bound_on_a_struct_with_a_repeatable_function_field() {
    let session = TestSession::single(
        r#"
struct Handler {
    run: () => void;
}

function requireClone<T: Clone>(value: T): T {
    return value;
}

declare const handler: Handler;

const cloned = requireClone(handler);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Handler {
    run: () => void;
}

function requireClone<T: Clone>(value: T): T {
    return value;
}

declare const handler: Handler;

const cloned: Handler = requireClone<Handler>(handler);

=== dir ===
struct Handler {
/// @type.symbol symbol=Handler type=Handler
/// @definition.struct symbol=Handler
/// @definition.field symbol=Handler.run source="run: () => void" key=run type=() => void

    run: () => void;
    /// @type.symbol symbol=Handler.run source="run: () => void" type=() => void

}

function requireClone<T: Clone>(value: T): T {
/// @generic.template symbol=requireClone parameters=(T: Clone)
/// @type.symbol symbol=requireClone type=<T: Clone>(T) => T
/// @type.symbol symbol=requireClone.T source="T: Clone" type=T
/// @resolution.name source=Clone target=Clone
/// @type.symbol symbol=requireClone.value source="value: T" type=T
/// @resolution.name source=T target=requireClone.T
/// @resolution.name source=T target=requireClone.T

    return value;
    /// @resolution.name source=value target=requireClone.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=requireClone.value

}

declare const handler: Handler;
/// @type.symbol symbol=handler source=handler type=Handler
/// @resolution.pattern source=handler kind=binding target=handler
/// @resolution.name source=Handler target=Handler

const cloned = requireClone(handler);
/// @type.symbol symbol=cloned source=cloned type=Handler
/// @resolution.pattern source=cloned kind=binding target=cloned
/// @resolution.name source=requireClone target=requireClone
/// @resolution.call source=requireClone(handler) parameters=(Handler) arguments=(provided(handler) as Handler) return=Handler kind=symbol target=requireClone instance=requireClone<Handler>
/// @generic.instantiation id=requireClone<Handler> template=requireClone arguments=(Handler)
/// @resolution.name source=handler target=handler
/// @resolution.place source=handler placement="local" lifetime="static" access="immutable"
/// @resolution.access source=handler root=handler
"#,
        r#"
"#,
    );
}

use crate::tests::{DirRows, TestSession};

/// Reduce a written owned form on a struct to its bare payload.
#[test]
fn test_reduce_written_owned_form_on_struct_binding() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

declare const point: ^Point;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

declare const point: Point;

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

declare const point: ^Point;
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point
"#,
    );
}

/// Reduce stacked owned forms on a struct to the bare payload.
#[test]
fn test_reduce_stacked_owned_forms_on_struct() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

declare const point: ^^Point;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

declare const point: Point;

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

declare const point: ^^Point;
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point
"#,
    );
}

/// Reduce an owned form around a readonly struct payload to the readonly view.
#[test]
fn test_reduce_owned_form_around_readonly_payload() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

declare const point: ^readonly Point;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

declare const point: readonly Point;

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

declare const point: ^readonly Point;
/// @type.symbol symbol=point source=point type=Readonly<Point>
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point
"#,
    );
}

/// Keep a written owned form on a class value.
#[test]
fn test_keep_owned_form_on_class_value() {
    let session = TestSession::single(
        r#"
class Bag {
    size: int32 = 0;
}

declare const bag: ^Bag;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Bag {
    size: int32 = 0;
}

declare const bag: ^Bag;

=== dir ===
class Bag {
/// @type.symbol symbol=Bag type=Bag
/// @definition.class symbol=Bag
/// @definition.field symbol=Bag.size source="size: int32 = 0" key=size type=int32

    size: int32 = 0;
    /// @type.symbol symbol=Bag.size source="size: int32 = 0" type=int32

}

declare const bag: ^Bag;
/// @type.symbol symbol=bag source=bag type=Owned<Bag>
/// @resolution.pattern source=bag kind=binding target=bag
/// @resolution.name source=Bag target=Bag
"#,
    );
}

/// Keep an owned form created by substitution beneath an imported borrow constructor.
#[test]
fn test_keep_substituted_owned_form_beneath_borrow() {
    let session = TestSession::builder()
        .module(
            "dep.ds",
            r#"
export struct Point {
    x: int32;
}

export declare function share<T>(value: T): &'static ^T;
"#,
        )
        .module(
            "main.ds",
            r#"
import { Point, share } from "./dep.ds";

declare const point: Point;
const borrowed = share(point);

borrowed satisfies Borrowed<Point, "static" & "local", "mutable">;
"#,
        )
        .build();

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Point, share } from "./dep.ds";

declare const point: Point;
const borrowed: Borrowed<^Point, "static", "mutable"> = share<Point>(point);

borrowed satisfies Borrowed<Point, "static" & "local", "mutable">;

=== dir ===
import { Point, share } from "./dep.ds";

declare const point: Point;
/// @type.symbol symbol=point source=point type=dep.Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=dep.Point

const borrowed = share(point);
/// @type.symbol symbol=borrowed source=borrowed type=&'static Owned<dep.Point>
/// @resolution.pattern source=borrowed kind=binding target=borrowed
/// @resolution.name source=share target=dep.share
/// @resolution.call source=share(point) parameters=(dep.Point) arguments=(provided(point) as dep.Point) return=&'static Owned<dep.Point> kind=symbol target=dep.share instance=dep.share<dep.Point>
/// @generic.instantiation id=dep.share<dep.Point> template=dep.share arguments=(dep.Point)
/// @generic.instance id=dep.share<dep.Point> template=dep.share arguments=(dep.Point)
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=point root=point

borrowed satisfies Borrowed<Point, "static" & "local", "mutable">;
/// @resolution.name source=borrowed target=borrowed
/// @resolution.place source=borrowed placement="static" lifetime="static" access="mutable"
/// @resolution.access source=borrowed root=borrowed
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Point target=dep.Point
"#,
    );
}

/// Reduce a managed constructor restating its class payload default.
#[test]
fn test_reduce_managed_constructor_on_class_payload() {
    let session = TestSession::single(
        r#"
class Bag {
    size: int32 = 0;
}

type Boxed = Managed<Bag>;

declare const boxed: Boxed;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Bag {
    size: int32 = 0;
}

type Boxed = Managed<Bag>;

declare const boxed: local Bag;

=== dir ===
class Bag {
/// @type.symbol symbol=Bag type=Bag
/// @definition.class symbol=Bag
/// @definition.field symbol=Bag.size source="size: int32 = 0" key=size type=int32

    size: int32 = 0;
    /// @type.symbol symbol=Bag.size source="size: int32 = 0" type=int32

}

type Boxed = Managed<Bag>;
/// @type.symbol symbol=Boxed source="type Boxed = Managed<Bag>" type=local Bag
/// @definition.type symbol=Boxed source="type Boxed = Managed<Bag>" value=local Bag
/// @resolution.name source=Managed target=Managed
/// @resolution.name source=Bag target=Bag

declare const boxed: Boxed;
/// @type.symbol symbol=boxed source=boxed type=local Bag
/// @resolution.pattern source=boxed kind=binding target=boxed
/// @resolution.name source=Boxed target=Boxed
"#,
    );
}

/// Keep a managed box over an explicitly owned class payload.
#[test]
fn test_keep_managed_box_over_owned_class_payload() {
    let session = TestSession::single(
        r#"
import { Managed, Owned } from "destack:memory";

class Bag {
    size: int32 = 0;
}

declare const boxed: Managed<Owned<Bag>>;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Managed, Owned } from "destack:memory";

class Bag {
    size: int32 = 0;
}

declare const boxed: local ^Bag;

=== dir ===
import { Managed, Owned } from "destack:memory";

class Bag {
/// @type.symbol symbol=Bag type=Bag
/// @definition.class symbol=Bag
/// @definition.field symbol=Bag.size source="size: int32 = 0" key=size type=int32

    size: int32 = 0;
    /// @type.symbol symbol=Bag.size source="size: int32 = 0" type=int32

}

declare const boxed: Managed<Owned<Bag>>;
/// @type.symbol symbol=boxed source=boxed type=local Owned<Bag>
/// @resolution.pattern source=boxed kind=binding target=boxed
/// @resolution.name source=Managed target=Managed
/// @resolution.name source=Owned target=Owned
/// @resolution.name source=Bag target=Bag
"#,
    );
}

/// Reduce a generic managed slot only where the argument family is managed.
#[test]
fn test_reduce_generic_managed_slot_by_argument_family() {
    let session = TestSession::single(
        r#"
import { Managed } from "destack:memory";

class Bag {
    size: int32 = 0;
}

struct Point {
    x: int32;
}

type Slot<T> = Managed<T>;

declare const shared: Slot<Bag>;
declare const boxed: Slot<Point>;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Managed } from "destack:memory";

class Bag {
    size: int32 = 0;
}

struct Point {
    x: int32;
}

type Slot<T> = Managed<T>;

declare const shared: local Bag;
declare const boxed: local Point;

=== dir ===
import { Managed } from "destack:memory";

class Bag {
/// @type.symbol symbol=Bag type=Bag
/// @definition.class symbol=Bag
/// @definition.field symbol=Bag.size source="size: int32 = 0" key=size type=int32

    size: int32 = 0;
    /// @type.symbol symbol=Bag.size source="size: int32 = 0" type=int32

}

struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

type Slot<T> = Managed<T>;
/// @generic.template symbol=Slot parameters=(T)
/// @type.symbol symbol=Slot source="type Slot<T> = Managed<T>" type=local T
/// @definition.type symbol=Slot source="type Slot<T> = Managed<T>" template=(T) value=local T
/// @type.symbol symbol=Slot.T source=T type=T
/// @resolution.name source=Managed target=Managed
/// @resolution.name source=T target=Slot.T

declare const shared: Slot<Bag>;
/// @type.symbol symbol=shared source=shared type=local Bag
/// @resolution.pattern source=shared kind=binding target=shared
/// @resolution.name source=Slot target=Slot
/// @resolution.name source=Bag target=Bag

declare const boxed: Slot<Point>;
/// @type.symbol symbol=boxed source=boxed type=local Point
/// @resolution.pattern source=boxed kind=binding target=boxed
/// @resolution.name source=Slot target=Slot
/// @resolution.name source=Point target=Point
"#,
    );
}

/// Keep stacked raw layers and reborrow stacked borrow constructors.
#[test]
fn test_layer_stacked_raw_and_reborrow_stacked_borrows() {
    let session = TestSession::single(
        r#"
import { Borrowed, Raw } from "destack:memory";

declare const nested: Borrowed<Borrowed<int32>>;
declare const doubled: Raw<Raw<int32>>;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Borrowed, Raw } from "destack:memory";

declare const nested: &'static int32;
declare const doubled: **int32;

=== dir ===
import { Borrowed, Raw } from "destack:memory";

declare const nested: Borrowed<Borrowed<int32>>;
/// @type.symbol symbol=nested source=nested type=&'static constant int32
/// @resolution.pattern source=nested kind=binding target=nested
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Borrowed target=Borrowed

declare const doubled: Raw<Raw<int32>>;
/// @type.symbol symbol=doubled source=doubled type=Raw<Raw<int32>>
/// @resolution.pattern source=doubled kind=binding target=doubled
/// @resolution.name source=Raw target=Raw
/// @resolution.name source=Raw target=Raw
"#,
    );
}

/// Reduce a managed constructor stacked over an explicit managed box.
#[test]
fn test_reduce_stacked_managed_constructor() {
    let session = TestSession::single(
        r#"
import { Managed } from "destack:memory";

struct Point {
    x: int32;
}

declare const boxed: Managed<Managed<Point>>;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Managed } from "destack:memory";

struct Point {
    x: int32;
}

declare const boxed: local Point;

=== dir ===
import { Managed } from "destack:memory";

struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

declare const boxed: Managed<Managed<Point>>;
/// @type.symbol symbol=boxed source=boxed type=local Point
/// @resolution.pattern source=boxed kind=binding target=boxed
/// @resolution.name source=Managed target=Managed
/// @resolution.name source=Managed target=Managed
/// @resolution.name source=Point target=Point
"#,
    );
}

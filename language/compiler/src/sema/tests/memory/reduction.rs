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
        "main.tspp",
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
        "main.tspp",
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
        "main.tspp",
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
/// @type.symbol symbol=point source=point type=readonly Point
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class Bag {
    size: int32 = 0;
}

declare const bag: ^Bag;

=== dir ===
class Bag {
/// @type.symbol symbol=Bag type=typeof Bag
/// @definition.class symbol=Bag
/// @definition.field symbol=Bag.size source="size: int32 = 0" key=size type=int32

    size: int32 = 0;
    /// @type.symbol symbol=Bag.size source="size: int32 = 0" type=int32

}

declare const bag: ^Bag;
/// @type.symbol symbol=bag source=bag type=^Bag
/// @resolution.pattern source=bag kind=binding target=bag
/// @resolution.name source=Bag target=Bag
"#,
    );
}

/// Collapse an owned form created by substitution beneath an imported borrow constructor.
#[test]
fn test_collapse_a_substituted_owned_form_beneath_a_borrow() {
    let session = TestSession::builder()
        .module(
            "dep.tspp",
            r#"
export struct Point {
    x: int32;
}

export declare function share<T>(value: T): &'static ^T;
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Point, share } from "./dep.tspp";

declare const point: Point;
const borrowed = share(point);

borrowed satisfies Borrowed<Point, "static", "mutable">;
"#,
        )
        .build();

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Point, share } from "./dep.tspp";

declare const point: Point;
const borrowed: &'static Point = share<Point>(point);

borrowed satisfies Borrowed<Point, "static", "mutable">;

=== dir ===
import { Point, share } from "./dep.tspp";

declare const point: Point;
/// @type.symbol symbol=point source=point type=dep.Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=dep.Point

const borrowed = share(point);
/// @type.symbol symbol=borrowed source=borrowed type=&'static dep.Point
/// @resolution.pattern source=borrowed kind=binding target=borrowed
/// @resolution.name source=share target=dep.share
/// @resolution.call source=share(point) parameters=(dep.Point) arguments=(provided(point) as dep.Point) return=&'static dep.Point kind=symbol target=dep.share instance=dep.share<dep.Point>
/// @generic.instantiation id=dep.share<dep.Point> template=dep.share arguments=(dep.Point)
/// @generic.instance id=dep.share<dep.Point> template=dep.share arguments=(dep.Point) dependents=("local")
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point

borrowed satisfies Borrowed<Point, "static", "mutable">;
/// @resolution.name source=borrowed target=borrowed
/// @resolution.place source=borrowed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=borrowed root=borrowed
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Point target=dep.Point
"#,
    );
}
/// Keep stacked raw layers and reborrow stacked borrow constructors.
#[test]
fn test_layer_stacked_raw_and_reborrow_stacked_borrows() {
    let session = TestSession::single(
        r#"
import { Borrowed, Raw } from "tspp:memory";

declare const nested: Borrowed<Borrowed<int32>>;
declare const doubled: Raw<Raw<int32>>;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Borrowed, Raw } from "tspp:memory";

declare const nested: &'static int32;
declare const doubled: **int32;

=== dir ===
import { Borrowed, Raw } from "tspp:memory";

declare const nested: Borrowed<Borrowed<int32>>;
/// @type.symbol symbol=nested source=nested type=&'static int32
/// @resolution.pattern source=nested kind=binding target=nested
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=Borrowed target=Borrowed

declare const doubled: Raw<Raw<int32>>;
/// @type.symbol symbol=doubled source=doubled type=**int32
/// @resolution.pattern source=doubled kind=binding target=doubled
/// @resolution.name source=Raw target=Raw
/// @resolution.name source=Raw target=Raw
"#,
    );
}

use crate::tests::{DirRows, TestSession};

#[test]
fn test_mutable_array_rejects_covariant_element_flow() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

declare const circles: Circle[];
const shapes: Shape[] = circles;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

declare const circles: Circle[];
const shapes: Shape[] = circles;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare const circles: Circle[];
/// @type.symbol symbol=circles source=circles type=Array<Circle>
/// @resolution.name source=Circle target=Circle

const shapes: Shape[] = circles;
/// @type.symbol symbol=shapes source=shapes type=Array<Shape>
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=circles target=circles
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'Array<Circle>' is not assignable to type 'Array<Shape>'"
/// @diagnostic.label line=6 column=25 span="circles" line_source="const shapes: Shape[] = circles;"
"#,
    );
}

#[test]
fn test_readonly_array_accepts_covariant_element_flow() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

declare const circles: Circle[];
const shapes: readonly Shape[] = circles;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Shape {}
class Circle extends Shape {}

declare const circles: Circle[];
const shapes: readonly Shape[] = circles;

=== checked ===
class Shape {}
/// @type.symbol symbol=Shape source="class Shape {}" type=Shape
/// @definition.class symbol=Shape source="class Shape {}"

class Circle extends Shape {}
/// @type.symbol symbol=Circle source="class Circle extends Shape {}" type=Circle
/// @definition.class symbol=Circle source="class Circle extends Shape {}"
/// @definition.extends symbol=Circle source=Shape target=Shape
/// @resolution.name source=Shape target=Shape

declare const circles: Circle[];
/// @type.symbol symbol=circles source=circles type=Array<Circle>
/// @resolution.name source=Circle target=Circle

const shapes: readonly Shape[] = circles;
/// @type.symbol symbol=shapes source=shapes type=readonly Array<Shape>
/// @resolution.name source=Shape target=Shape
/// @resolution.name source=circles target=circles
"#,
    );
}

#[test]
fn test_mutable_object_rejects_covariant_property_flow() {
    let session = TestSession::single(
        r#"
declare const point: { x: 1 };
const widened: { x: float64 } = point;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const point: { x: 1 };
const widened: { x: float64 } = point;

=== checked ===
declare const point: { x: 1 };
/// @type.symbol symbol=point source=point type={ x: 1 }

const widened: { x: float64 } = point;
/// @type.symbol symbol=widened source=widened type={ x: float64 }
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '{ x: 1 }' is not assignable to type '{ x: float64 }'"
/// @diagnostic.label line=3 column=33 span="point" line_source="const widened: { x: float64 } = point;"
"#,
    );
}

#[test]
fn test_readonly_object_accepts_covariant_property_flow() {
    let session = TestSession::single(
        r#"
declare const point: { x: 1 };
const widened: { readonly x: float64 } = point;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const point: { x: 1 };
const widened: { readonly x: float64 } = point;

=== checked ===
declare const point: { x: 1 };
/// @type.symbol symbol=point source=point type={ x: 1 }

const widened: { readonly x: float64 } = point;
/// @type.symbol symbol=widened source=widened type={ readonly x: float64 }
/// @resolution.name source=point target=point
"#,
    );
}

#[test]
fn test_function_parameters_flow_contravariantly() {
    let session = TestSession::single(
        r#"
class Shape {}
class Circle extends Shape {}

declare const useShape: (shape: Shape) => void;
const useCircle: (circle: Circle) => void = useShape;

declare const useCircle2: (circle: Circle) => void;
const useShape2: (shape: Shape) => void = useCircle2;
"#,
    );

    session.assert_dir_checked_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error code=EC200 message="type '(Circle) => void' is not assignable to type '(Shape) => void'"
/// @diagnostic.label line=9 column=43 span="useCircle2" line_source="const useShape2: (shape: Shape) => void = useCircle2;"
"#,
    );
}

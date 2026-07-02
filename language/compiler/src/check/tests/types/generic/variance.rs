use crate::tests::TestSession;

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

    session.assert_dir_checked_diagnostics(
        "main.ds",
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

    session.assert_dir_checked_diagnostics("main.ds", "");
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

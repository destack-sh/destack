use crate::tests::{DirRows, TestSession};

#[test]
fn test_template_literal_infer_extracts_segment() {
    let session = TestSession::single(
        r#"
type Segment<T> = T extends `/${infer Name}` ? Name : never;
type Name = Segment<"/api">;

declare const name: Name;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
type Segment<T> = T extends `/${infer Name}` ? Name : never;
/// @generic.template symbol=Segment parameters=[T]
/// @type.symbol symbol=Segment type=T extends `/${infer Name}` ? Name : never

type Name = Segment<"/api">;
/// @resolution.name source=Segment target=Segment
/// @generic.instance source="Segment<\"/api\">" id="Segment<\"/api\">"
/// @type.symbol symbol=Name type="api"

declare const name: Name;
/// @resolution.name source=Name target=Name
/// @type.symbol symbol=name type="api"

/// @generic.instance id="Segment<\"/api\">" symbol=Segment arguments=["/api"]
"#,
    );
}

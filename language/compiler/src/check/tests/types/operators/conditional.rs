use crate::tests::{DirRows, TestSession};

#[test]
fn test_conditional_type_selects_matching_branch() {
    let session = TestSession::single(
        r#"
type Select<T> = T extends string ? string : int32;

declare const text: Select<string>;
declare const number: Select<boolean>;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
type Select<T> = T extends string ? string : int32;
/// @generic.template symbol=Select parameters=[T]
/// @type.symbol symbol=Select type=T extends string ? string : int32

declare const text: Select<string>;
/// @resolution.name source=Select target=Select
/// @generic.application source="Select<string>" id=Select<string>
/// @type.symbol symbol=text type=string

declare const number: Select<boolean>;
/// @resolution.name source=Select target=Select
/// @generic.application source="Select<boolean>" id=Select<boolean>
/// @type.symbol symbol=number type=int32

/// @generic.application id=Select<string> symbol=Select arguments=[string]
/// @generic.application id=Select<boolean> symbol=Select arguments=[boolean]
"#,
    );
}

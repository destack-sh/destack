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
=== annotated ===
type Select<T> = T extends string ? string : int32;

declare const text: Select<string>;
declare const number: Select<boolean>;

=== checked ===
type Select<T> = T extends string ? string : int32;
/// @generic.template source=declaration parameters=[T]
/// @type.symbol symbol=Select source="type Select<T> = T extends string ? string : int32" type=T extends string ? string : int32
/// @definition.type symbol=Select source="type Select<T> = T extends string ? string : int32" template=LocalGenericTemplateId(0) value=T extends string ? string : int32
/// @type.symbol symbol=Select.T source=T type=T
/// @resolution.name source=T target=Select.T

declare const text: Select<string>;
/// @type.symbol symbol=text source=text type=Select<string>
/// @resolution.name source=Select target=Select

declare const number: Select<boolean>;
/// @type.symbol symbol=number source=number type=Select<boolean>
/// @resolution.name source=Select target=Select
"#,
    );
}

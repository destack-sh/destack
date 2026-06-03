use crate::tests::{DirRows, TestSession};

#[test]
fn test_mapped_type_projects_each_source_key() {
    let session = TestSession::single(
        r#"
type Flags<T> = { [K in keyof T]: boolean };
type Actual = Flags<{ name: string; age: int32 }>;

declare const value: Actual;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
type Flags<T> = { [K in keyof T]: boolean };
/// @generic.template symbol=Flags parameters=[T]
/// @type.symbol symbol=Flags type={ [K in keyof T]: boolean }

type Actual = Flags<{ name: string; age: int32 }>;
/// @resolution.name source=Flags target=Flags
/// @generic.instance source="Flags<{ name: string; age: int32 }>" id="Flags<{ name: string; age: int32 }>"
/// @type.symbol symbol=Actual type={ name: boolean; age: boolean }

declare const value: Actual;
/// @resolution.name source=Actual target=Actual
/// @type.symbol symbol=value type={ name: boolean; age: boolean }

/// @generic.instance id="Flags<{ name: string; age: int32 }>" symbol=Flags arguments=[{ name: string; age: int32 }]
"#,
    );
}

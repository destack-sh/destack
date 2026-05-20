use crate::tests::{DirRows, TestSession};

#[test]
fn test_export_records_local_value() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
export let value: number = 1;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries(),
        r#"
export let value: number = 1;
/// @export.local key=value source=value

/// @export.summary exports=1
"#,
    );
}

#[test]
fn test_export_records_local_alias() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
let value = 1;
export { value as renamed };
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries(),
        r#"
let value = 1;
export { value as renamed };
/// @export.local key=renamed source=value

/// @export.summary exports=1
"#,
    );
}

#[test]
fn test_export_uses_latest_local_binding() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
let value = 1;
let value = 2;
export { value };
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries(),
        r#"
let value = 1;
let value = 2;
export { value };
/// @export.local key=value source=value#2

/// @export.summary exports=1
"#,
    );
}

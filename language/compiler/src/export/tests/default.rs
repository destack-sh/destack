use crate::tests::{DirRows, TestSession};

#[test]
fn test_export_records_default_function() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
export default function main(): number {
    return 1;
}
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries().with_export_stats(),
        r#"
export default function main(): number {
/// @export.local key=<default> source=main

    return 1;
}

/// @export.summary exports=1
/// @export.stats roots=1 expressions=visibility:1,export:1 symbols=scanned:2
"#,
    );
}

#[test]
fn test_export_records_default_expression() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
export default 1;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries().with_export_stats(),
        r#"
export default 1;
/// @export.local key=<default> source=symbol1

/// @export.summary exports=1
/// @export.stats roots=1 expressions=visibility:1,export:1 symbols=scanned:2
"#,
    );
}

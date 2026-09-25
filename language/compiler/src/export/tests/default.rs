use crate::tests::{DirRows, TestSession};

#[test]
fn test_export_records_default_function() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
export default function main(): number {
    return 1;
}
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.tspp",
        DirRows::exports().with_summaries(),
        r#"
export default function main(): number {
/// @export.local key=<default> symbols=[main]

    return 1;
}

/// @export.summary exports=1
"#,
    );
}

#[test]
fn test_export_records_default_expression() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
export default 1;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.tspp",
        DirRows::exports().with_summaries(),
        r#"
export default 1;
/// @export.local key=<default> symbols=[symbol1]

/// @export.summary exports=1
"#,
    );
}

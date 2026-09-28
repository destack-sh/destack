use crate::tests::TestSession;

/// A closure keeps the narrowing of a captured binding nothing writes, and rejects one a write reaches.
#[test]
fn test_keep_the_narrowing_of_a_binding_a_closure_captures() {
    let session = TestSession::single(
        r#"
declare function later(run: () => void): void;

function kept(name: string | undefined): void {
    if (name != undefined) {
        later(() => {
            const length: isize = name.length;
        });
    }
}

function rebound(source: string | undefined): void {
    let name = source;
    if (name != undefined) {
        later(() => {
            const length: isize = name.length;
        });
        name = undefined;
    }
}
"#,
    );

    session.assert_diagnostics(
        session.dir_checked_key("main.tspp"),
        r#"
/// @diagnostic.error id=stale-captured-narrowing message="'name' is narrowed in a closure while its binding changes elsewhere"
/// @diagnostic.label line=16 column=35 span="name" line_source="const length: isize = name.length;"
"#,
    );
}

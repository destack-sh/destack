use crate::tests::{DirRows, TestSession};

/// A required pattern rejects the absent result and binds the present payload.
#[test]
fn test_bind_required_patterns_at_the_present_type() {
    let session = TestSession::single(
        r#"
declare function parse(text: string): int32 | undefined;

function read(text: string): int32 {
    const value! = parse(text);
    return value;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function parse(text: string): int32 | undefined;

function read(text: string): int32 {
    const value! = parse(text);
    return value;
}

=== dir ===
declare function parse(text: string): int32 | undefined;
/// @type.symbol symbol=parse source="declare function parse(text: string): int32 | undefined" type=(string) => int32 | undefined

function read(text: string): int32 {
/// @type.symbol symbol=read type=(string) => int32
/// @type.symbol symbol=read.text source="text: string" type=string

    const value! = parse(text);
    /// @type.symbol symbol=read.value source=value type=int32
    /// @resolution.pattern source=value kind=binding target=read.value
    /// @resolution.pattern source=value! kind=must pattern=pattern
    /// @resolution.name source=parse target=parse
    /// @resolution.call source=parse(text) parameters=(string) arguments=(provided(text) as string) return=int32 | undefined kind=symbol target=parse
    /// @resolution.name source=text target=read.text
    /// @resolution.place source=text placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=text root=read.text

    return value;
    /// @resolution.name source=value target=read.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=value root=read.value

}
"#,
    );
}

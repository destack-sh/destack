use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_reports_fresh_object_excess_properties() {
    let session = TestSession::single(
        r#"
type Person = { name: string };

const value: Person = { name: "Ada", extra: true };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
type Person = { name: string };
/// @type.symbol symbol=Person type={ name: string }

const value: Person = { name: "Ada", extra: true };
/// @resolution.name source=Person target=Person
/// @type.node source="{ name: \"Ada\", extra: true }" type={ name: "Ada"; extra: true }
/// @type.symbol symbol=value type=Person

"#,
        r#"
/// @diagnostic.error code=EC205 message="excess property 'extra'"
/// @diagnostic.label line=4 column=39 source="const value: Person = { name: \"Ada\", extra: true };"
"#);
}

#[test]
fn test_check_allows_non_fresh_object_extra_properties() {
    let session = TestSession::single(
        r#"
type Person = { name: string };

const source = { name: "Ada", extra: true };
const value: Person = source;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
type Person = { name: string };
/// @type.symbol symbol=Person type={ name: string }

const source = { name: "Ada", extra: true };
/// @type.node source="{ name: \"Ada\", extra: true }" type={ name: string; extra: boolean }
/// @type.symbol symbol=source type={ name: string; extra: boolean }

const value: Person = source;
/// @resolution.name source=Person target=Person
/// @resolution.name source=source target=source
/// @type.symbol symbol=value type=Person
"#,
    );
}

#[test]
fn test_check_keeps_generic_object_literals_fresh_for_inference_only() {
    let session = TestSession::single(
        r#"
function keep<T: { name: string }>(value: T): T {
    return value;
}

const value = keep({ name: "Ada", extra: true });
const extra = value.extra;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
function keep<T: { name: string }>(value: T): T {
/// @generic.slot symbol=keep.T index=0 kind=type constraint={ name: string }
/// @type.symbol symbol=keep type=<T: { name: string }>(T) => T

    return value;
}

const value = keep({ name: "Ada", extra: true });
/// @resolution.name source=keep target=keep
/// @type.node source="{ name: \"Ada\", extra: true }" type={ name: string; extra: boolean }
/// @resolution.call source="keep({ name: \"Ada\", extra: true })" parameters=[{ name: string; extra: boolean }] return={ name: string; extra: boolean } kind=symbol target=keep instance="keep<{ name: string; extra: boolean }>"
/// @instance.application source="keep({ name: \"Ada\", extra: true })" id="keep<{ name: string; extra: boolean }>"
/// @type.symbol symbol=value type={ name: string; extra: boolean }

const extra = value.extra;
/// @resolution.name source=value target=value
/// @resolution.member source=value.extra receiver={ name: string; extra: boolean } kind=symbol target=value.extra
/// @type.symbol symbol=extra type=boolean

/// @instance.entry id="keep<{ name: string; extra: boolean }>" symbol=keep arguments=[{ name: string; extra: boolean }]
"#);
}

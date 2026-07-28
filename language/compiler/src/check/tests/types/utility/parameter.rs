use crate::tests::{DirRows, TestSession};

#[test]
fn test_parameters_extracts_argument_tuple() {
    let session = TestSession::single(
        r#"
type Args = Parameters<(name: string, count: number) => boolean>;

const ok: Args = ("Ada", 1);
ok satisfies (string, number);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Args = Parameters<(name: string, count: number) => boolean>;

const ok: Args = ("Ada", 1);
ok satisfies (string, number);

=== checked ===
type Args = Parameters<(name: string, count: number) => boolean>;
/// @type.symbol symbol=Args source="type Args = Parameters<(name: string, count: number) => boolean>" type=Parameters<Function<(string, float64), boolean>> reduced=(string, float64)
/// @definition.type symbol=Args source="type Args = Parameters<(name: string, count: number) => boolean>" value=Parameters<Function<(string, float64), boolean>> reduced=(string, float64)
/// @resolution.name source=Parameters target=types.function.Parameters

const ok: Args = ("Ada", 1);
/// @type.symbol symbol=ok source=ok type=Args reduced=(string, float64)
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Args target=Args

ok satisfies (string, number);
/// @resolution.name source=ok target=ok
/// @resolution.place source=ok placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=ok root=ok

/// @generic.instance id="Parameters<Function<(string, float64), boolean>>" template=types.function.Parameters arguments=(Function<(string, float64), boolean>)
"#,
    );
}

#[test]
fn test_parameters_preserves_optional_parameters() {
    let session = TestSession::single(
        r#"
type Args = Parameters<(name: string, count?: number) => boolean>;

const short: Args = ("Ada",);
const full: Args = ("Ada", 1);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Args = Parameters<(name: string, count?: number) => boolean>;

const short: Args = ("Ada",) as (string, float64 | undefined?);
const full: Args = ("Ada", 1 as float64 | undefined);

=== checked ===
type Args = Parameters<(name: string, count?: number) => boolean>;
/// @type.symbol symbol=Args source="type Args = Parameters<(name: string, count?: number) => boolean>" type=Parameters<Function<(string, float64 | undefined?), boolean>> reduced=(string, float64 | undefined?)
/// @definition.type symbol=Args source="type Args = Parameters<(name: string, count?: number) => boolean>" value=Parameters<Function<(string, float64 | undefined?), boolean>> reduced=(string, float64 | undefined?)
/// @resolution.name source=Parameters target=types.function.Parameters

const short: Args = ("Ada",);
/// @type.symbol symbol=short source=short type=Args reduced=(string, float64 | undefined?)
/// @resolution.pattern source=short kind=binding target=short
/// @resolution.name source=Args target=Args

const full: Args = ("Ada", 1);
/// @type.symbol symbol=full source=full type=Args reduced=(string, float64 | undefined?)
/// @resolution.pattern source=full kind=binding target=full
/// @resolution.name source=Args target=Args

/// @generic.instance id="Parameters<Function<(string, float64 | undefined?), boolean>>" template=types.function.Parameters arguments=(Function<(string, float64 | undefined?), boolean>)
"#,
    );
}

#[test]
fn test_parameters_preserves_rest_parameters() {
    let session = TestSession::single(
        r#"
type Args = Parameters<(name: string, ...flags: boolean[]) => void>;

const ok: Args = ("Ada", true, false);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Args = Parameters<(name: string, ...flags: boolean[]) => void>;

const ok: Args = ("Ada", true, false) as (string, ...boolean[]);

=== checked ===
type Args = Parameters<(name: string, ...flags: boolean[]) => void>;
/// @type.symbol symbol=Args source="type Args = Parameters<(name: string, ...flags: boolean[]) => void>" type=Parameters<Function<(string, ...boolean[]), void>> reduced=(string, ...boolean[])
/// @definition.type symbol=Args source="type Args = Parameters<(name: string, ...flags: boolean[]) => void>" value=Parameters<Function<(string, ...boolean[]), void>> reduced=(string, ...boolean[])
/// @resolution.name source=Parameters target=types.function.Parameters

const ok: Args = ("Ada", true, false);
/// @type.symbol symbol=ok source=ok type=Args reduced=(string, ...boolean[])
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Args target=Args

/// @generic.instance id="Parameters<Function<(string, ...boolean[]), void>>" template=types.function.Parameters arguments=(Function<(string, ...boolean[]), void>)
"#,
    );
}

#[test]
fn test_parameters_rejects_wrong_argument_types() {
    let session = TestSession::single(
        r#"
type Args = Parameters<(name: string, count: number) => boolean>;

const bad: Args = ("Ada", "one");
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Args = Parameters<(name: string, count: number) => boolean>;

const bad: Args = ("Ada", "one");

=== checked ===
type Args = Parameters<(name: string, count: number) => boolean>;
/// @type.symbol symbol=Args source="type Args = Parameters<(name: string, count: number) => boolean>" type=Parameters<Function<(string, float64), boolean>> reduced=(string, float64)
/// @definition.type symbol=Args source="type Args = Parameters<(name: string, count: number) => boolean>" value=Parameters<Function<(string, float64), boolean>> reduced=(string, float64)
/// @resolution.name source=Parameters target=types.function.Parameters

const bad: Args = ("Ada", "one");
/// @type.symbol symbol=bad source=bad type=Args reduced=(string, float64)
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Args target=Args

/// @generic.instance id="Parameters<Function<(string, float64), boolean>>" template=types.function.Parameters arguments=(Function<(string, float64), boolean>)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"one\"' is not assignable to type 'float64'"
/// @diagnostic.label line=4 column=27 span="\"one\"" line_source="const bad: Args = (\"Ada\", \"one\");"
/// @diagnostic.related line=4 column=12 span="Args" line_source="const bad: Args = (\"Ada\", \"one\");" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in element 1"
"#,
    );
}

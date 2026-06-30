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
/// @type.symbol symbol=Args source="type Args = Parameters<(name: string, count: number) => boolean>" type=types.function.Parameters<Function<(string, float64), boolean>> reduced=(string, float64)
/// @definition.type symbol=Args source="type Args = Parameters<(name: string, count: number) => boolean>" value=types.function.Parameters<Function<(string, float64), boolean>> reduced=(string, float64)
/// @resolution.name source=Parameters target=types.function.Parameters

const ok: Args = ("Ada", 1);
/// @type.symbol symbol=ok source=ok type=Args reduced=(string, float64)
/// @resolution.name source=Args target=Args

ok satisfies (string, number);
/// @resolution.name source=ok target=ok

/// @generic.instance id="types.function.Parameters<Function<(string, float64), boolean>>" template=types.function.Parameters arguments=(Function<(string, float64), boolean>)
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

const short: Args = ("Ada",);
const full: Args = ("Ada", 1);

=== checked ===
type Args = Parameters<(name: string, count?: number) => boolean>;
/// @type.symbol symbol=Args source="type Args = Parameters<(name: string, count?: number) => boolean>" type=types.function.Parameters<Function<(string, float64), boolean>> reduced=(string, float64?)
/// @definition.type symbol=Args source="type Args = Parameters<(name: string, count?: number) => boolean>" value=types.function.Parameters<Function<(string, float64), boolean>> reduced=(string, float64?)
/// @resolution.name source=Parameters target=types.function.Parameters

const short: Args = ("Ada",);
/// @type.symbol symbol=short source=short type=Args reduced=(string, float64?)
/// @resolution.name source=Args target=Args

const full: Args = ("Ada", 1);
/// @type.symbol symbol=full source=full type=Args reduced=(string, float64?)
/// @resolution.name source=Args target=Args

/// @generic.instance id="types.function.Parameters<Function<(string, float64), boolean>>" template=types.function.Parameters arguments=(Function<(string, float64), boolean>)
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

const ok: Args = ("Ada", true, false);

=== checked ===
type Args = Parameters<(name: string, ...flags: boolean[]) => void>;
/// @type.symbol symbol=Args source="type Args = Parameters<(name: string, ...flags: boolean[]) => void>" type=types.function.Parameters<Function<(string, ...boolean[]), void>> reduced=(string, ...boolean[])
/// @definition.type symbol=Args source="type Args = Parameters<(name: string, ...flags: boolean[]) => void>" value=types.function.Parameters<Function<(string, ...boolean[]), void>> reduced=(string, ...boolean[])
/// @resolution.name source=Parameters target=types.function.Parameters

const ok: Args = ("Ada", true, false);
/// @type.symbol symbol=ok source=ok type=Args reduced=(string, ...boolean[])
/// @resolution.name source=Args target=Args

/// @generic.instance id="types.function.Parameters<Function<(string, ...boolean[]), void>>" template=types.function.Parameters arguments=(Function<(string, ...boolean[]), void>)
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
/// @type.symbol symbol=Args source="type Args = Parameters<(name: string, count: number) => boolean>" type=types.function.Parameters<Function<(string, float64), boolean>> reduced=(string, float64)
/// @definition.type symbol=Args source="type Args = Parameters<(name: string, count: number) => boolean>" value=types.function.Parameters<Function<(string, float64), boolean>> reduced=(string, float64)
/// @resolution.name source=Parameters target=types.function.Parameters

const bad: Args = ("Ada", "one");
/// @type.symbol symbol=bad source=bad type=Args reduced=(string, float64)
/// @resolution.name source=Args target=Args

/// @generic.instance id="types.function.Parameters<Function<(string, float64), boolean>>" template=types.function.Parameters arguments=(Function<(string, float64), boolean>)
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"one\"' is not assignable to type 'float64'"
/// @diagnostic.label line=4 column=27 span="\"one\"" line_source="const bad: Args = (\"Ada\", \"one\");"
"#,
    );
}

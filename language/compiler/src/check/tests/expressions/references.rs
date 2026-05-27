use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_records_local_name_resolution() {
    let session = TestSession::single(
        r#"
const value = 1;
const copy = value;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value = 1;
/// @type.symbol symbol=value type=1
/// @type.node source=1 type=1

const copy = value;
/// @type.symbol symbol=copy type=1
/// @type.node source=value type=1
/// @resolution.name source=value target=value
"#,
    );
}

#[test]
fn test_check_resolves_profile_global_bindings_as_names() {
    let session = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "compiler": {
        "globals": ["globals.ds"]
    }
}
"#,
        )
        .module(
            "globals.ds",
            r#"
global {
    const answer: int32 = 42;
}
"#,
        )
        .module(
            "main.ds",
            r#"
const value = answer;
"#,
        )
        .build();

    session.assert_dir_checked_many(
        &["globals.ds", "main.ds"],
        DirRows::checked().with_reference_types(),
        r#"
=== globals.ds ===
global {
    const answer: int32 = 42;
    /// @type.symbol symbol=answer type=int32
    /// @type.node source=42 type=int32

}

=== main.ds ===
const value = answer;
/// @type.symbol symbol=value type=int32
/// @type.node source=answer type=int32
/// @resolution.name source=answer target=globals.answer
"#,
    );
}

#[test]
fn test_check_types_profile_import_meta_fields() {
    let session = TestSession::single(
        r#"
const runtime = import.meta.runtime;
const platform = import.meta.platform;
const debug = import.meta.debug;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const runtime = import.meta.runtime;
/// @type.symbol symbol=runtime type="destack" | "js"
/// @type.node source=import.meta type={ readonly runtime: "destack" | "js"; readonly platform: "unknown" | "windows" | "macos" | "linux" | "freebsd" | "openbsd" | "netbsd" | "dragonfly" | "solaris" | "illumos" | "haiku" | "fuchsia" | "redox" | "hermit" | "none"; readonly debug: boolean }
/// @type.node source=import.meta.runtime type="destack" | "js"
/// @resolution.member source=import.meta.runtime receiver={ readonly runtime: "destack" | "js"; readonly platform: "unknown" | "windows" | "macos" | "linux" | "freebsd" | "openbsd" | "netbsd" | "dragonfly" | "solaris" | "illumos" | "haiku" | "fuchsia" | "redox" | "hermit" | "none"; readonly debug: boolean } kind=field key=runtime

const platform = import.meta.platform;
/// @type.symbol symbol=platform type="unknown" | "windows" | "macos" | "linux" | "freebsd" | "openbsd" | "netbsd" | "dragonfly" | "solaris" | "illumos" | "haiku" | "fuchsia" | "redox" | "hermit" | "none"
/// @type.node source=import.meta type={ readonly runtime: "destack" | "js"; readonly platform: "unknown" | "windows" | "macos" | "linux" | "freebsd" | "openbsd" | "netbsd" | "dragonfly" | "solaris" | "illumos" | "haiku" | "fuchsia" | "redox" | "hermit" | "none"; readonly debug: boolean }
/// @type.node source=import.meta.platform type="unknown" | "windows" | "macos" | "linux" | "freebsd" | "openbsd" | "netbsd" | "dragonfly" | "solaris" | "illumos" | "haiku" | "fuchsia" | "redox" | "hermit" | "none"
/// @resolution.member source=import.meta.platform receiver={ readonly runtime: "destack" | "js"; readonly platform: "unknown" | "windows" | "macos" | "linux" | "freebsd" | "openbsd" | "netbsd" | "dragonfly" | "solaris" | "illumos" | "haiku" | "fuchsia" | "redox" | "hermit" | "none"; readonly debug: boolean } kind=field key=platform

const debug = import.meta.debug;
/// @type.symbol symbol=debug type=boolean
/// @type.node source=import.meta type={ readonly runtime: "destack" | "js"; readonly platform: "unknown" | "windows" | "macos" | "linux" | "freebsd" | "openbsd" | "netbsd" | "dragonfly" | "solaris" | "illumos" | "haiku" | "fuchsia" | "redox" | "hermit" | "none"; readonly debug: boolean }
/// @type.node source=import.meta.debug type=boolean
/// @resolution.member source=import.meta.debug receiver={ readonly runtime: "destack" | "js"; readonly platform: "unknown" | "windows" | "macos" | "linux" | "freebsd" | "openbsd" | "netbsd" | "dragonfly" | "solaris" | "illumos" | "haiku" | "fuchsia" | "redox" | "hermit" | "none"; readonly debug: boolean } kind=field key=debug
"#,
    );
}

#[test]
fn test_check_types_module_import_meta_fields() {
    let session = TestSession::single(
        r#"
module {
    const role = "server";
    const labels = {
        feature: ["search"] as const,
    };
}

const role = import.meta.role;
const features = import.meta.labels.feature;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
module {
    const role = "server";
    /// @type.symbol symbol=role#1 type="server"
    /// @type.node source="\"server\"" type="server"

    const labels = {
    /// @type.symbol symbol=labels type={ feature: readonly ["search"] }
    /// @type.node type={ feature: readonly ["search"] }

        feature: ["search"] as const,
        /// @type.node source="[\"search\"] as const" type=readonly ["search"]

    };
}

const role = import.meta.role;
/// @type.symbol symbol=role#2 type="server"
/// @type.node source=import.meta type={ readonly runtime: "destack" | "js"; readonly platform: "unknown" | "windows" | "macos" | "linux" | "freebsd" | "openbsd" | "netbsd" | "dragonfly" | "solaris" | "illumos" | "haiku" | "fuchsia" | "redox" | "hermit" | "none"; readonly debug: boolean; readonly role: "server"; readonly labels: { feature: readonly ["search"] } }
/// @type.node source=import.meta.role type="server"
/// @resolution.member source=import.meta.role receiver={ readonly runtime: "destack" | "js"; readonly platform: "unknown" | "windows" | "macos" | "linux" | "freebsd" | "openbsd" | "netbsd" | "dragonfly" | "solaris" | "illumos" | "haiku" | "fuchsia" | "redox" | "hermit" | "none"; readonly debug: boolean; readonly role: "server"; readonly labels: { feature: readonly ["search"] } } kind=field key=role

const features = import.meta.labels.feature;
/// @type.symbol symbol=features type=readonly ["search"]
/// @type.node source=import.meta type={ readonly runtime: "destack" | "js"; readonly platform: "unknown" | "windows" | "macos" | "linux" | "freebsd" | "openbsd" | "netbsd" | "dragonfly" | "solaris" | "illumos" | "haiku" | "fuchsia" | "redox" | "hermit" | "none"; readonly debug: boolean; readonly role: "server"; readonly labels: { feature: readonly ["search"] } }
/// @type.node source=import.meta.labels type={ feature: readonly ["search"] }
/// @type.node source=import.meta.labels.feature type=readonly ["search"]
/// @resolution.member source=import.meta.labels receiver={ readonly runtime: "destack" | "js"; readonly platform: "unknown" | "windows" | "macos" | "linux" | "freebsd" | "openbsd" | "netbsd" | "dragonfly" | "solaris" | "illumos" | "haiku" | "fuchsia" | "redox" | "hermit" | "none"; readonly debug: boolean; readonly role: "server"; readonly labels: { feature: readonly ["search"] } } kind=field key=labels
/// @resolution.member source=import.meta.labels.feature receiver={ feature: readonly ["search"] } kind=field key=feature
"#,
    );
}

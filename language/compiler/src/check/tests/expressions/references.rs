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
        DirRows::checked(),
        r#"
const value = 1;
/// @type.symbol symbol=value type=1

const copy = value;
/// @resolution.name source=value target=value
/// @type.symbol symbol=copy type=1
"#,
    );
}

#[test]
fn test_check_resolves_imported_global_bindings_as_names() {
    let session = TestSession::new()
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
import "./globals.ds";

const value = answer;
"#,
        )
        .build();

    session.assert_dir_checked_many(
        &["globals.ds", "main.ds"],
        DirRows::checked(),
        r#"
=== globals.ds ===
global {
    const answer: int32 = 42;
    /// @type.symbol symbol=answer type=int32
}


=== main.ds ===
import "./globals.ds";

const value = answer;
/// @resolution.name source=answer target=globals.answer
/// @type.symbol symbol=value type=int32
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
        DirRows::checked(),
        r#"
const runtime = import.meta.runtime;
/// @resolution.member source=import.meta.runtime receiver=import.meta kind=intrinsic target=import.meta.runtime
/// @type.symbol symbol=runtime type="destack" | "js"

const platform = import.meta.platform;
/// @resolution.member source=import.meta.platform receiver=import.meta kind=intrinsic target=import.meta.platform
/// @type.symbol symbol=platform type=Platform

const debug = import.meta.debug;
/// @resolution.member source=import.meta.debug receiver=import.meta kind=intrinsic target=import.meta.debug
/// @type.symbol symbol=debug type=boolean
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
        DirRows::checked(),
        r#"
module {
    const role = "server";
    /// @type.symbol symbol=role#1 type="server"

    const labels = {
        feature: ["search"] as const,
    };
    /// @type.symbol symbol=labels type={ feature: readonly ["search"] }
}

const role = import.meta.role;
/// @resolution.member source=import.meta.role receiver=import.meta kind=intrinsic target=import.meta.role
/// @type.symbol symbol=role#2 type="server"

const features = import.meta.labels.feature;
/// @resolution.member source=import.meta.labels receiver=import.meta kind=intrinsic target=import.meta.labels
/// @resolution.member source=import.meta.labels.feature receiver={ feature: readonly ["search"] } kind=direct target=import.meta.labels.feature
/// @type.symbol symbol=features type=readonly ["search"]
"#,
    );
}

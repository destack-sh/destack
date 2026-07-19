use crate::tests::{DirRows, TestSession};

#[test]
fn test_star_export_exposes_reexported_binding() {
    let session = TestSession::builder()
        .module(
            "source.ds",
            r#"
export const value: int32 = 1;
"#,
        )
        .module(
            "index.ds",
            r#"
export * from "./source.ds";
"#,
        )
        .module(
            "main.ds",
            r#"
import { value } from "./index.ds";

const direct = value;
"#,
        )
        .build();

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { value } from "./index.ds";

const direct: int32 = value;

=== checked ===
import { value } from "./index.ds";

const direct = value;
/// @type.symbol symbol=direct source=direct type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=source.value
"#,
    );
}

#[test]
fn test_namespace_export_exposes_member_binding() {
    let session = TestSession::builder()
        .module(
            "source.ds",
            r#"
export const value: int32 = 1;
"#,
        )
        .module(
            "index.ds",
            r#"
export * as source from "./source.ds";
"#,
        )
        .module(
            "main.ds",
            r#"
import { source } from "./index.ds";

const namespaced = source.value;
"#,
        )
        .build();

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { source } from "./index.ds";

const namespaced: int32 = source.value;

=== checked ===
import { source } from "./index.ds";

const namespaced = source.value;
/// @type.symbol symbol=namespaced source=namespaced type=int32
/// @type.node source=source.value type=int32
/// @resolution.name source=source.value target=source.value
"#,
    );
}

#[test]
fn test_unresolved_reference_names_the_declaring_sibling() {
    let session = TestSession::builder()
        .module(
            "util.ds",
            r#"
export const helper: int32 = 1;
export const sibling: int32 = 2;
"#,
        )
        .module(
            "main.ds",
            r#"
import { helper } from "./util.ds";

const first = helper;
const second = sibling;
"#,
        )
        .build();

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { helper } from "./util.ds";

const first: int32 = helper;
const second = sibling;

=== checked ===
import { helper } from "./util.ds";

const first = helper;
/// @type.symbol symbol=first source=first type=int32
/// @resolution.name source=helper target=util.helper

const second = sibling;
/// @type.symbol symbol=second source=second type=<error>
"#,
        r#"
/// @diagnostic.error id=unresolved-reference message="cannot find 'sibling'"
/// @diagnostic.label line=5 column=16 span="sibling" line_source="const second = sibling;"
/// @diagnostic.related file="util.ds" message="'sibling' is declared in this module"
/// @diagnostic.help message="import 'sibling' from that module"
"#,
    );
}

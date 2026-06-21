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

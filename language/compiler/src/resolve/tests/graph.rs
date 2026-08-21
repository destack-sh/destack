use crate::tests::TestSession;

#[test]
fn test_module_graph_tracks_import_edges() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { value } from "./dependency";

export const result = value;
"#,
        )
        .module("dependency.ds", "export const value: int32 = 1;")
        .build();

    compiler.assert_module_graph(
        &["main.ds", "dependency.ds"],
        r#"
module dependency.ds -> []
module main.ds -> [dependency.ds]
"#,
    );
}

#[test]
fn test_module_graph_keeps_cyclic_import_edges() {
    let compiler = TestSession::builder()
        .module(
            "a.ds",
            r#"
import { valueB } from "./b";

export const valueA: int32 = valueB;
"#,
        )
        .module(
            "b.ds",
            r#"
import { valueA } from "./a";

export const valueB: int32 = valueA;
"#,
        )
        .build();

    compiler.assert_module_graph(
        &["a.ds", "b.ds"],
        r#"
module a.ds -> [b.ds]
module b.ds -> [a.ds]
"#,
    );
}

#[test]
fn test_module_graph_retains_unused_import_reachability() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { value } from "./dependency";
"#,
        )
        .module(
            "dependency.ds",
            r#"
const seed = 1;
export const value = seed;
"#,
        )
        .build();

    compiler.assert_module_graph(
        &["main.ds", "dependency.ds"],
        r#"
module dependency.ds -> []
module main.ds -> [dependency.ds]
"#,
    );
}

#[test]
fn test_module_graph_edges_follow_extension_imports() {
    let compiler = TestSession::builder()
        .module(
            "point.ds",
            r#"
export struct Point {
    x: int32;
}
"#,
        )
        .module(
            "extension.ds",
            r#"
import { Point } from "./point";

export extension of Point {
    value(): int32 {
        return this.x;
    }
}
"#,
        )
        .build();

    compiler.assert_module_graph(
        &["point.ds", "extension.ds"],
        r#"
module extension.ds -> [point.ds]
module point.ds -> []
"#,
    );
}

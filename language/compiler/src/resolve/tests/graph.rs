use crate::tests::TestSession;

#[test]
fn test_module_graph_tracks_import_edges() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { value } from "./dependency";

export const result = value;
"#,
        )
        .module("dependency.tspp", "export const value: int32 = 1;")
        .build();

    compiler.assert_module_graph(
        &["main.tspp", "dependency.tspp"],
        r#"
module dependency.tspp -> []
module main.tspp -> [dependency.tspp]
"#,
    );
}

#[test]
fn test_module_graph_keeps_cyclic_import_edges() {
    let compiler = TestSession::builder()
        .module(
            "a.tspp",
            r#"
import { valueB } from "./b";

export const valueA: int32 = valueB;
"#,
        )
        .module(
            "b.tspp",
            r#"
import { valueA } from "./a";

export const valueB: int32 = valueA;
"#,
        )
        .build();

    compiler.assert_module_graph(
        &["a.tspp", "b.tspp"],
        r#"
module a.tspp -> [b.tspp]
module b.tspp -> [a.tspp]
"#,
    );
}

#[test]
fn test_module_graph_retains_unused_import_reachability() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { value } from "./dependency";
"#,
        )
        .module(
            "dependency.tspp",
            r#"
const seed = 1;
export const value = seed;
"#,
        )
        .build();

    compiler.assert_module_graph(
        &["main.tspp", "dependency.tspp"],
        r#"
module dependency.tspp -> []
module main.tspp -> [dependency.tspp]
"#,
    );
}

#[test]
fn test_module_graph_edges_follow_extension_imports() {
    let compiler = TestSession::builder()
        .module(
            "point.tspp",
            r#"
export struct Point {
    x: int32;
}
"#,
        )
        .module(
            "extension.tspp",
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
        &["point.tspp", "extension.tspp"],
        r#"
module extension.tspp -> [point.tspp]
module point.tspp -> []
"#,
    );
}

use crate::tests::TestSession;

#[test]
fn test_component_graph_keeps_written_reference_out_of_inference() {
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

    compiler.assert_component_graph(
        &["main.ds", "dependency.ds"],
        r#"
reference-component [dependency.ds]
reference-component [main.ds]
inference-component [dependency.ds]
inference-component [main.ds]
reference dependency.ds -> []
reference main.ds -> [dependency.ds]
inference dependency.ds -> []
inference main.ds -> []
"#,
    );
}

#[test]
fn test_component_graph_tracks_inferred_reference() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { value } from "./dependency";

export const result = value;
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

    compiler.assert_component_graph(
        &["main.ds", "dependency.ds"],
        r#"
reference-component [dependency.ds]
reference-component [main.ds]
inference-component [dependency.ds]
inference-component [main.ds]
reference dependency.ds -> []
reference main.ds -> [dependency.ds]
inference dependency.ds -> []
inference main.ds -> [dependency.ds]
"#,
    );
}

#[test]
fn test_component_graph_groups_mutual_inference() {
    let compiler = TestSession::builder()
        .module(
            "a.ds",
            r#"
import { valueB } from "./b";

export const valueA = valueB;
"#,
        )
        .module(
            "b.ds",
            r#"
import { valueA } from "./a";

export const valueB = valueA;
"#,
        )
        .build();

    compiler.assert_component_graph(
        &["a.ds", "b.ds"],
        r#"
reference-component [a.ds, b.ds]
inference-component [a.ds, b.ds]
reference a.ds -> [b.ds]
reference b.ds -> [a.ds]
inference a.ds -> [b.ds]
inference b.ds -> [a.ds]
"#,
    );
}

#[test]
fn test_component_graph_keeps_written_cycle_out_of_inference() {
    let compiler = TestSession::builder()
        .module(
            "a.ds",
            r#"
import * as b from "./b";

export interface A {
    other: b.B;
}
"#,
        )
        .module(
            "b.ds",
            r#"
import * as a from "./a";

export interface B {
    other: a.A;
}
"#,
        )
        .build();

    compiler.assert_component_graph(
        &["a.ds", "b.ds"],
        r#"
reference-component [a.ds, b.ds]
inference-component [a.ds]
inference-component [b.ds]
reference a.ds -> [b.ds]
reference b.ds -> [a.ds]
inference a.ds -> []
inference b.ds -> []
"#,
    );
}

#[test]
fn test_component_graph_retains_unused_import_reachability() {
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

    compiler.assert_component_graph(
        &["main.ds", "dependency.ds"],
        r#"
reference-component [dependency.ds]
reference-component [main.ds]
inference-component [dependency.ds]
inference-component [main.ds]
reference dependency.ds -> []
reference main.ds -> [dependency.ds]
inference dependency.ds -> []
inference main.ds -> []
"#,
    );
}

#[test]
fn test_component_graph_tracks_inferred_namespace_object() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import * as dependency from "./dependency";

export const result = dependency;
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

    compiler.assert_component_graph(
        &["main.ds", "dependency.ds"],
        r#"
reference-component [dependency.ds]
reference-component [main.ds]
inference-component [dependency.ds]
inference-component [main.ds]
reference dependency.ds -> []
reference main.ds -> [dependency.ds]
inference dependency.ds -> []
inference main.ds -> [dependency.ds]
"#,
    );
}

#[test]
fn test_component_graph_tracks_inferred_target_extension() {
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
    value() {
        return this.x;
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Point } from "./point";

declare const point: Point;
export const value = point.value();
"#,
        )
        .build();

    compiler.assert_component_graph(
        &["point.ds", "extension.ds", "main.ds"],
        r#"
reference-component [extension.ds]
reference-component [main.ds]
reference-component [point.ds]
inference-component [extension.ds]
inference-component [main.ds]
inference-component [point.ds]
reference extension.ds -> [point.ds]
reference main.ds -> [point.ds]
reference point.ds -> []
inference extension.ds -> []
inference main.ds -> [extension.ds]
inference point.ds -> []
"#,
    );
}

#[test]
fn test_component_graph_tracks_inferred_interface_implementation() {
    let compiler = TestSession::builder()
        .module(
            "contract.ds",
            r#"
export interface Contract {
    value(): int32;
}
"#,
        )
        .module(
            "implementation.ds",
            r#"
import { Contract } from "./contract";
import { Value } from "./value";

export extension ContractImplementation of Value implements Contract {
    value() {
        return 1;
    }
}
"#,
        )
        .module(
            "value.ds",
            r#"
export struct Value {}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Contract } from "./contract";
import { Value } from "./value";

declare function read<T: Contract>(value: T): int32;
declare const value: Value;
export const result = read(value);
"#,
        )
        .build();

    compiler.assert_component_graph(
        &["contract.ds", "implementation.ds", "main.ds", "value.ds"],
        r#"
reference-component [contract.ds]
reference-component [implementation.ds]
reference-component [main.ds]
reference-component [value.ds]
inference-component [contract.ds]
inference-component [implementation.ds]
inference-component [main.ds]
inference-component [value.ds]
reference contract.ds -> []
reference implementation.ds -> [contract.ds, value.ds]
reference main.ds -> [contract.ds, value.ds]
reference value.ds -> []
inference contract.ds -> []
inference implementation.ds -> []
inference main.ds -> [implementation.ds]
inference value.ds -> []
"#,
    );
}

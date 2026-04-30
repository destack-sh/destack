use crate::{
    DestackFormatOptions, assert_format, assert_format_program_roundtrip_with_file_type,
    assert_format_roundtrip,
};
use destack_source::FileType;

/// Simple imports should stay stable.
#[test]
fn test_format_import() {
    assert_format!(
        r#"import "foo""#,
        r#"import "foo""#,
        |p| p.parse_expression(),
        DestackFormatOptions::default()
    );
}

/// Named import lists should normalize spacing.
#[test]
fn test_format_import_with_items_from() {
    assert_format!(
        r#"import {foo,bar,baz} from "foo""#,
        r#"import { foo, bar, baz } from "foo""#,
        |p| p.parse_expression(),
        DestackFormatOptions::default_with_line_width(60)
    );
}

/// Type-equals require imports should keep their explicit form.
#[test]
fn test_format_import_type_equals_require() {
    assert_format!(
        r#"import type React = require("react")"#,
        r#"import type React = require("react");"#,
        |p| p.parse_expression(),
        DestackFormatOptions::default()
    );
}

/// Export attributes should stay stable.
#[test]
fn test_format_export_with_attributes() {
    assert_format!(
        r#"export { foo } from "bar" with { mode: "strict" }"#,
        r#"export { foo } from "bar" with { mode: "strict" }"#,
        |p| p.parse_expression(),
        DestackFormatOptions::default()
    );
}

/// Default-plus-namespace imports should roundtrip cleanly.
#[test]
fn test_format_import_default_and_namespace_roundtrip() {
    assert_format_roundtrip!(
        r#"import a, * as b from "a""#,
        r#"import a, * as b from "a""#,
        FileType::JavaScript,
        |p| p.parse_expression(),
    );
}

/// Triple-slash reference directives should stay directive-shaped.
#[test]
fn test_format_triple_slash_reference_directives() {
    assert_format_program_roundtrip_with_file_type(
        r#"/// <reference no-default-lib="true"/>
/// <reference path="./types.d.ts"/>
/// <reference types="node"/>
/// <reference lib="dom" />

type Value=string
"#,
        r#"/// <reference no-default-lib="true"/>
/// <reference path="./types.d.ts"/>
/// <reference types="node"/>
/// <reference lib="dom" />

type Value = string;
"#,
        FileType::TypeScriptDeclaration,
        DestackFormatOptions::default(),
    );
}

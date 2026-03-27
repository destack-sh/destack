use crate::{
    DestackFormatOptions, TestFormatter, assert_format, assert_format_roundtrip_with_file_type,
};
use destack_source::FileType;

#[test]
fn test_format_import() {
    assert_format!(
        "import \"foo\"",
        "import \"foo\"",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_import_with_alias() {
    assert_format!(
        "import * as foo from \"foo\"",
        "import * as foo from \"foo\"",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_import_with_items_from() {
    assert_format!(
        "import {bar, baz} from \"foo\"",
        "import { bar, baz } from \"foo\"",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(60)
    );
}

#[test]
fn test_format_import_with_overflow() {
    let source = r#"import {
    StructuredObject,
    StructuredObjectOptions,
    StructuredObjectOptions2,
    StructuredObjectOptions3,
} from "lib""#;
    assert_format!(
        source,
        source,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(60)
    );
}

#[test]
fn test_format_export_glob() {
    assert_format!(
        r#"export * from "./foo""#,
        r#"export * from "./foo""#,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_import_with_default_and_block() {
    assert_format!(
        "import Default, { type Item } from \"foo\"",
        "import Default, { type Item } from \"foo\"",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_import_type_equals_require() {
    assert_format!(
        r#"import type React = require("react")"#,
        r#"import type React = require("react");"#,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_export_import_type_equals_require() {
    assert_format!(
        r#"export import type React = require("react")"#,
        r#"export import type React = require("react");"#,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_export_with_default_and_block() {
    assert_format!(
        "export { default, default as bar, foo } from \"foo\"",
        "export { default, default as bar, foo } from \"foo\"",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_export_with_attributes() {
    assert_format!(
        "export { foo } from \"bar\" with { mode: \"strict\" }",
        "export { foo } from \"bar\" with { mode: \"strict\" }",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_import_default_and_namespace_roundtrip() {
    assert_format_roundtrip_with_file_type(
        r#"import a, * as b from "a""#,
        r#"import a, * as b from "a""#,
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

#[test]
fn test_format_export_default_and_namespace_roundtrip() {
    assert_format_roundtrip_with_file_type(
        r#"export a, * as b from "mod""#,
        r#"export a, * as b from "mod""#,
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

#[test]
fn test_format_import_type_empty_items_roundtrip() {
    assert_format_roundtrip_with_file_type(
        r#"import type {} from "a""#,
        r#"import type {} from "a""#,
        FileType::TypeScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

#[test]
fn test_format_export_empty_items_with_target_roundtrip() {
    assert_format_roundtrip_with_file_type(
        r#"export {} from "a""#,
        r#"export {} from "a""#,
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

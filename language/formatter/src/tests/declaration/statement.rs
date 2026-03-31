use destack_ast::BlockContext;
use destack_source::FileType;
use destack_workspace::OrganizeImports;

use crate::{DestackFormatOptions, assert_format, assert_format_program};

/// Return formatter options with import organization enabled.
fn organize_imports_options() -> DestackFormatOptions {
    DestackFormatOptions {
        organize_imports: OrganizeImports::On,
        ..DestackFormatOptions::default()
    }
}

/// Semicolons should be inserted for non-tail statement expressions.
/// Tail expressions in value-position blocks should stay semicolonless.
#[test]
fn test_format_block_insert_semicolon() {
    assert_format!(
        r#"{
    let x = 1
    x

    if (x) {
        y
    } else {
        z()
    }
}"#,
        r#"{
    let x = 1;
    x;

    if (x) {
        y;
    } else {
        z();
    }
}"#,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default()
    );
}

/// Inline blocks should stay inline when used as expressions.
#[test]
fn test_format_block_inline() {
    assert_format!(
        r#"const x = if (y) { z } else { w }"#,
        r#"const x = if (y) { z } else { w }"#,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_tab()
    );
}

/// Statement-like blocks should expand into full blocks.
#[test]
fn test_format_block_statement_like() {
    assert_format!(
        r#"if (y) { z } else { w; }"#,
        r#"if (y) {
	z;
} else {
	w;
}"#,
        |p| p.eat_if(),
        DestackFormatOptions::default_tab()
    );
}

/// Declarations after expressions should not force an extra blank line.
#[test]
fn test_format_block_declaration_after_expression_no_forced_blank_line() {
    assert_format!(
        r#"{ x = 1; class A {} }"#,
        r#"{
	x = 1;
	class A {}
}"#,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default_tab()
    );
}

/// Side-effect imports should keep source order and stay above regular imports at program scope.
#[test]
fn test_format_program_import_sorting_preserves_side_effect_order() {
    assert_format_program!(
        r#"import "zeta"
import thing from "pkg"
import "alpha"
import fs from "node:fs""#,
        r#"import "zeta";
import "alpha";

import fs from "node:fs";

import thing from "pkg";
"#,
        FileType::Destack,
        organize_imports_options()
    );
}

/// Organized imports should insert blank lines between import groups at program scope.
#[test]
fn test_format_program_import_sorting_inserts_group_blank_lines() {
    assert_format_program!(
        r#"import rel from "./rel"
import pkg from "react"
import alias from "~/core"
import fs from "node:fs""#,
        r#"import fs from "node:fs";

import pkg from "react";

import alias from "~/core";

import rel from "./rel";
"#,
        FileType::Destack,
        organize_imports_options()
    );
}

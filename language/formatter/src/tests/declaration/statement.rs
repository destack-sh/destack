use destack_ast::BlockContext;
use destack_source::FileType;
use destack_workspace::OrganizeImports;

use crate::{
    DestackFormatOptions, assert_format, assert_format_program,
    assert_format_program_reference_widths, assert_format_program_roundtrip_with_file_type,
};

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

/// Let-else branches should format like other control heads with statement bodies.
#[test]
fn test_format_let_else_statement() {
    assert_format!(
        r#"let {x}=value else{return}"#,
        r#"let { x } = value else {
    return;
}"#,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
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

/// Source blank lines before statement comments should remain visible.
#[test]
fn test_format_program_preserves_blank_line_before_statement_comments() {
    assert_format_program!(
        r#"if (!process.stdout.isTTY) process.stdout._handle?.setBlocking?.(true);

// Call the Rust CLI first
const mode = runCli();
"#,
        r#"if (!process.stdout.isTTY) process.stdout._handle?.setBlocking?.(true);

// Call the Rust CLI first
const mode = runCli();
"#,
        FileType::TypeScript,
    );
}

/// Blank lines before parenthesized statement expressions should be preserved.
#[test]
fn test_format_program_preserves_blank_line_before_parenthesized_iife() {
    assert_format_program_reference_widths(
        r#"const a = 1

;(function() {
  const b = 2;
})()

foo();
[1,2,3]; // prettier-ignore

bar();
[4,5,6]; // oxfmt-ignore

baz();
[7,8,9]"#,
        FileType::JavaScript,
        &[(
            80,
            r#"const a = 1;

(function () {
  const b = 2;
})();

foo();
[1,2,3]; // prettier-ignore

bar();
[4,5,6]; // oxfmt-ignore

baz();
[7, 8, 9];
"#,
        )],
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

/// Decorator comments should stay attached to decorated class declarations.
#[test]
fn test_format_decorator_comments() {
    assert_format_program_reference_widths(
        r#"// test.ts
import { Component } from "@angular/core";

@Component({
  selector: "my-component", // test
})
export class AppMyComponent {}

@Component({
  selector: "my-component", // test
})
export default class AppMyComponent {}
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"// test.ts
import { Component } from "@angular/core";

@Component({
  selector: "my-component", // test
})
export class AppMyComponent {}

@Component({
  selector: "my-component", // test
})
export default class AppMyComponent {}
"#,
            ),
            (
                100,
                r#"// test.ts
import { Component } from "@angular/core";

@Component({
  selector: "my-component", // test
})
export class AppMyComponent {}

@Component({
  selector: "my-component", // test
})
export default class AppMyComponent {}
"#,
            ),
        ],
    );
}

/// Export-head comments should stay attached after `export`.
#[test]
fn test_format_export_head_comments() {
    assert_format_program_reference_widths(
        r#"export /* keep */ class A {}
export /* keep */ default class B {}
export /* keep */ function c() {}
export /* keep */ type T = string
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"export /* keep */ class A {}
export default /* keep */ class B {}
export /* keep */ function c() {}
export /* keep */ type T = string;
"#,
            ),
            (
                100,
                r#"export /* keep */ class A {}
export default /* keep */ class B {}
export /* keep */ function c() {}
export /* keep */ type T = string;
"#,
            ),
        ],
    );
}

/// Ignore directive aliases should preserve the following raw statement text.
#[test]
fn test_format_program_ignore_directive_aliases_roundtrip() {
    assert_format_program_roundtrip_with_file_type(
        r#"const keepFormatted = 1;

// fmt-ignore
const fmtIgnored   =  [  1,2,3 ]

// format-ignore
const formatIgnored   =  {  alpha:1,  beta:2 }

// prettier-ignore
const prettierIgnored   =  call(  alpha,  beta )

// oxfmt-ignore
const oxfmtIgnored   =  source /* hop */ ?. ( "value" )

// deno-fmt-ignore
const denoIgnored   =  foo?.( "value" )

// biome-ignore format: keep raw
const biomeIgnored   =  run(  first,  second )

// fmt-ignore-start
const rangeIgnoredA   =  [  4,5,6 ]
const rangeIgnoredB   =  {  gamma:3,  delta:4 }
// fmt-ignore-end

const keepFormattedToo = 2;
"#,
        r#"const keepFormatted = 1;

// fmt-ignore
const fmtIgnored   =  [  1,2,3 ]

// format-ignore
const formatIgnored   =  {  alpha:1,  beta:2 }

// prettier-ignore
const prettierIgnored   =  call(  alpha,  beta )

// oxfmt-ignore
const oxfmtIgnored   =  source /* hop */ ?. ( "value" )

// deno-fmt-ignore
const denoIgnored   =  foo?.( "value" )

// biome-ignore format: keep raw
const biomeIgnored   =  run(  first,  second )

// fmt-ignore-start
const rangeIgnoredA   =  [  4,5,6 ]
const rangeIgnoredB   =  {  gamma:3,  delta:4 }
// fmt-ignore-end

const keepFormattedToo = 2;
"#,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

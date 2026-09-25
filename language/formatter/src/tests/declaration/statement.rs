use tspp_source::FileType;

use crate::{
    TsppFormatOptions, assert_format, assert_format_program,
    assert_format_program_reference_widths, assert_format_program_roundtrip_with_file_type,
    parse_first_expression,
};

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
        y
    } else {
        z()
    }
}"#,
        parse_first_expression,
        TsppFormatOptions::default()
    );
}

#[test]
fn test_format_placed_binding_declaration() {
    assert_format_program!(
        r#"shared const registry: Registry = new Registry()
"#,
        r#"shared const registry: Registry = new Registry();
"#,
        FileType::Tspp,
    );
}

/// Tagged struct literal statements should not need object-literal disambiguation.
#[test]
fn test_format_block_tagged_struct_literal_statement() {
    assert_format_program!(
        r#"struct OsPathBytes { kind: "bytes"; bytes: PathBytes; }
function writePath(data: PathBytes) {
    OsPathBytes {
        kind: "bytes",
        bytes: data,
    }
    done();
}
"#,
        r#"struct OsPathBytes {
    kind: "bytes";
    bytes: PathBytes;
}
function writePath(data: PathBytes) {
    OsPathBytes {
        kind: "bytes",
        bytes: data,
    };
    done();
}
"#,
        FileType::Tspp,
    );
}

/// Tagged struct literal tail expressions should remain expression-valued.
#[test]
fn test_format_block_tagged_struct_literal_tail_expression() {
    assert_format_program!(
        r#"struct OsPathBytes { kind: "bytes"; bytes: PathBytes; }
function osPathBytes(data: PathBytes): OsPath {
    OsPathBytes {
        kind: "bytes",
        bytes: data,
    }
}
"#,
        r#"struct OsPathBytes {
    kind: "bytes";
    bytes: PathBytes;
}
function osPathBytes(data: PathBytes): OsPath {
    OsPathBytes {
        kind: "bytes",
        bytes: data,
    }
}
"#,
        FileType::Tspp,
    );
}

/// Module decorators should stay above empty module declarations.
#[test]
fn test_format_module_declaration_decorators() {
    assert_format_program!(
        r#"@noManaged
@noHeap
module {}
"#,
        r#"@noManaged
@noHeap
module {}
"#,
        FileType::Tspp,
    );
}

/// Final semicolons in value-capable blocks should preserve statement position.
#[test]
fn test_format_block_preserves_terminal_statement_semicolon() {
    assert_format_program!(
        r#"function run(): void {
    done();
}

const visit = () => {
    consume();
}
"#,
        r#"function run(): void {
    done();
}

const visit = () => {
    consume();
};
"#,
        FileType::Tspp,
    );
}

/// Final expressions in value-capable blocks should stay semicolonless.
#[test]
fn test_format_block_preserves_terminal_expression_without_semicolon() {
    assert_format_program!(
        r#"function run(): Status {
    done()
}

const visit = () => {
    consume()
}
"#,
        r#"function run(): Status {
    done()
}

const visit = () => {
    consume()
};
"#,
        FileType::Tspp,
    );
}

/// Preserve comments owned by semicolonless control and value expressions.
#[test]
fn test_format_semicolonless_statement_comments() {
    assert_format_program!(
        r#"function value(): Status {
  if (ready) {
    primary
  } else {
    secondary
  } // conditional
  fallback // tail
}
"#,
        r#"function value(): Status {
    if (ready) {
        primary
    } else {
        secondary
    } // conditional
    fallback // tail
}
"#,
        FileType::Tspp,
    );
}

/// Void function bodies should keep terminal expressions statement-position.
#[test]
fn test_format_void_function_body_inserts_terminal_semicolon() {
    assert_format_program!(
        r#"function run(): void {
    done()
}
"#,
        r#"function run(): void {
    done();
}
"#,
        FileType::Tspp,
    );
}

/// Method tail expressions should preserve value position unless the method mode is statement-only.
#[test]
fn test_format_method_body_preserves_terminal_expression_without_semicolon() {
    assert_format_program!(
        r#"class Box {
    constructor() {
        initialize()
    }

    get value(): number {
        this.current
    }

    set value(next: number) {
        this.current = next
    }

    read(): number {
        this.current
    }
}
"#,
        r#"class Box {
    constructor() {
        initialize();
    }

    get value(): number {
        this.current
    }

    set value(next: number) {
        this.current = next;
    }

    read(): number {
        this.current
    }
}
"#,
        FileType::Tspp,
    );
}

/// Object method tail expressions should preserve value position.
#[test]
fn test_format_object_method_body_preserves_terminal_expression_without_semicolon() {
    assert_format_program!(
        r#"const tools = {
    read(): number {
        current()
    },
}
"#,
        r#"const tools = {
    read(): number {
        current()
    },
};
"#,
        FileType::Tspp,
    );
}

/// Extension getter tail expressions should preserve value position.
#[test]
fn test_format_extension_getter_ternary_tail_without_semicolon() {
    assert_format_program!(
        r#"extension<T, const N: number> of SmallArray<T, N> implements
    Sequence<T>,
    Index<usize>,
    IndexSet<usize, T>,
    Iterable<T>,
    Iterable<&T>,
    Iterable<&readonly T>,
    Iterable<&T>,
    From<Iterable<T>>,
    FromIterator<T>,
    Extend<T, "mutable">,
    Default,
    From<Array<T>>,
    From<FixedArray<T, N>> {
    get capacity(): usize {
        this.spillStorage == undefined ? (N as usize) : this.spillCapacity
    }
}
"#,
        r#"extension<T, const N: number> of SmallArray<T, N>
    implements
        Sequence<T>,
        Index<usize>,
        IndexSet<usize, T>,
        Iterable<T>,
        Iterable<&T>,
        Iterable<&readonly T>,
        Iterable<&T>,
        From<Iterable<T>>,
        FromIterator<T>,
        Extend<T, "mutable">,
        Default,
        From<Array<T>>,
        From<FixedArray<T, N>>
{
    get capacity(): usize {
        this.spillStorage == undefined ? (N as usize) : this.spillCapacity
    }
}
"#,
        FileType::Tspp,
    );
}

/// Inline blocks should stay inline when used as expressions.
#[test]
fn test_format_block_inline() {
    assert_format!(
        r#"const x = if (y) { z } else { w }"#,
        r#"const x = if (y) { z } else { w }"#,
        parse_first_expression,
        TsppFormatOptions::default_tab()
    );
}

/// Statement-like blocks should expand into full blocks.
#[test]
fn test_format_block_statement_like() {
    assert_format!(
        r#"if (y) { z } else { w; }"#,
        r#"if (y) {
	z
} else {
	w;
}"#,
        parse_first_expression,
        TsppFormatOptions::default_tab()
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
        parse_first_expression,
        TsppFormatOptions::default()
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
        parse_first_expression,
        TsppFormatOptions::default_tab()
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
        FileType::Tspp,
    );
}

/// Adjacent declaration doc comments should stay with the following declaration.
#[test]
fn test_format_program_keeps_adjacent_doc_comments_leading() {
    assert_format_program!(
        r#"/**
 * Deno's `sessionStorage` API operates similarly to the {@linkcode localStorage} API.
 *
 * @example
 * ```ts
 * const value = sessionStorage.getItem("key");
 * console.log(value); // Output: "value"
 * ```
 */
declare let sessionStorage: Storage;
/** @category Cache */
/** Provides access to the Cache API. */
declare let caches: CacheStorage;
"#,
        r#"/// Deno's `sessionStorage` API operates similarly to the {@linkcode localStorage} API.
///
/// @example
/// ```ts
/// const value = sessionStorage.getItem("key");
/// console.log(value); // Output: "value"
/// ```
declare let sessionStorage: Storage;
/// @category Cache
/// Provides access to the Cache API.
declare let caches: CacheStorage;
"#,
        FileType::TsppDeclaration,
    );
}

/// Documentation prose should keep authored line breaks between sentences.
#[test]
fn test_format_doc_comment_keeps_authored_line_breaks() {
    assert_format_program!(
        r#"/// Atomically replaces the value at `ptr` with `value`.
/// Returns the previous value.
const value = 1;
"#,
        r#"/// Atomically replaces the value at `ptr` with `value`.
/// Returns the previous value.
const value = 1;
"#,
        FileType::Tspp,
    );
}

/// Documentation prose should wrap overlong lines without merging the next authored line.
#[test]
fn test_format_doc_comment_wraps_overlong_authored_line() {
    assert_format_program!(
        r#"/// Atomically replaces the value at `ptr` with `value` while holding the global runtime lock for the full call duration.
/// Returns the previous value.
const value = 1;
"#,
        r#"/// Atomically replaces the value at `ptr` with `value` while holding the global runtime lock for
/// the full call duration.
/// Returns the previous value.
const value = 1;
"#,
        FileType::Tspp,
    );
}

/// Declaration comments without delayed semicolons should stay leading.
#[test]
fn test_format_program_keeps_next_declaration_doc_comment_leading() {
    assert_format_program!(
        r#"declare const first: string
/** doc */
declare const second: string
"#,
        r#"declare const first: string;
/// doc
declare const second: string;
"#,
        FileType::TsppDeclaration,
    );
}

/// Global blocks should omit redundant `declare` modifiers in declaration files.
#[test]
fn test_format_global_omits_redundant_declare_modifier() {
    assert_format_program!(
        r#"declare global {
    let Buffer: BufferConstructor;
}
"#,
        r#"global {
    let Buffer: BufferConstructor;
}
"#,
        FileType::TsppDeclaration,
    );
}

/// Ambient global blocks should retain `declare` in ordinary source files.
#[test]
fn test_format_global_prints_declare_modifier() {
    assert_format_program!(
        r#"declare global {
    let Buffer: BufferConstructor;
}
"#,
        r#"declare global {
    let Buffer: BufferConstructor;
}
"#,
        FileType::Tspp,
    );
}

/// Blank lines before parenthesized statement expressions should be preserved.
#[test]
fn test_format_program_preserves_blank_line_before_parenthesized_iife() {
    assert_format_program_reference_widths(
        r#"const a = 1

;(() => {
  const b = 2;
})()

foo();
[1,2,3]; // prettier-ignore

bar();
[4,5,6]; // oxfmt-ignore

baz();
[7,8,9]"#,
        FileType::Tspp,
        &[(
            80,
            r#"const a = 1;

(() => {
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
        FileType::Tspp,
        TsppFormatOptions::default()
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
        FileType::Tspp,
        TsppFormatOptions::default()
    );
}

/// Decorator comments should stay attached to decorated class declarations.
#[test]
fn test_format_decorator_comments() {
    assert_format_program_reference_widths(
        r#"// test.tspp
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
        FileType::Tspp,
        &[
            (
                80,
                r#"// test.tspp
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
                r#"// test.tspp
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
        FileType::Tspp,
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
        FileType::Tspp,
        TsppFormatOptions::default(),
    );
}

/// Prefix ignore directives should preserve a semicolon on the ignored statement line.
#[test]
fn test_format_program_ignore_directive_preserves_same_line_semicolon() {
    assert_format_program_roundtrip_with_file_type(
        r#"// format-ignore
run(  alpha,  beta  );
const formatted =  1
"#,
        r#"// format-ignore
run(  alpha,  beta  );
const formatted = 1;
"#,
        FileType::Tspp,
        TsppFormatOptions::default(),
    );
}

/// Prefix ignore directives should not absorb a semicolon from the next line.
#[test]
fn test_format_program_ignore_directive_keeps_next_line_semicolon_separate() {
    assert_format_program_roundtrip_with_file_type(
        r#"// format-ignore
run(  alpha,  beta  )
;
const formatted =  1
"#,
        r#"// format-ignore
run(  alpha,  beta  )
;
const formatted = 1;
"#,
        FileType::Tspp,
        TsppFormatOptions::default(),
    );
}

/// Trailing ignore directives should attach across statement separator trivia.
#[test]
fn test_format_program_trailing_ignore_preserves_same_line_semicolon() {
    assert_format_program_roundtrip_with_file_type(
        r#"run(  alpha  ); // format-ignore
const formatted =  1
"#,
        r#"run(  alpha  ); // format-ignore
const formatted = 1;
"#,
        FileType::Tspp,
        TsppFormatOptions::default(),
    );
}

/// File ignore directives should only apply before the first source token.
#[test]
fn test_format_program_file_ignore_requires_file_prefix_position() {
    assert_format_program_roundtrip_with_file_type(
        r#"// format-ignore-file
const raw   =  [  1,2,3 ]
"#,
        r#"// format-ignore-file
const raw   =  [  1,2,3 ]
"#,
        FileType::Tspp,
        TsppFormatOptions::default(),
    );

    assert_format_program_roundtrip_with_file_type(
        r#"const formatted =  1
// format-ignore-file
const alsoFormatted =  [  1,2,3 ]
"#,
        r#"const formatted = 1;
// format-ignore-file
const alsoFormatted = [1, 2, 3];
"#,
        FileType::Tspp,
        TsppFormatOptions::default(),
    );
}

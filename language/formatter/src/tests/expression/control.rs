use crate::{
    DestackFormatOptions, assert_format, assert_format_program,
    assert_format_program_reference_widths,
};
use destack_source::FileType;

#[test]
fn test_format_match_expression_cases() {
    assert_format!(
        "match (x) { 1 => 2; 3 => 4 }",
        "match (x) {\n\t1 => 2\n\t3 => 4\n}",
        |p| p.eat_match(),
        DestackFormatOptions::default_tab()
    );
}

#[test]
fn test_format_match_with_block_case_and_guard() {
    assert_format!(
        "match (value) { Pattern if (cond) => { const X = 1; } }",
        "match (value) {\n\tPattern if (cond) => {\n\t\tconst X = 1;\n\t}\n}",
        |p| p.eat_match(),
        DestackFormatOptions::default_tab()
    );
}

#[test]
fn test_format_switch_expression_cases() {
    assert_format!(
        "switch (x) { case 1: 2; case 3: 4 }",
        "switch (x) {\n\tcase 1: 2;\n\tcase 3: 4;\n}",
        |p| p.eat_match(),
        DestackFormatOptions::default_tab()
    );
}

#[test]
fn test_format_switch_with_default_case() {
    assert_format!(
        "switch (x) { case 1: \"one\"; default: \"other\" }",
        "switch (x) {\n\tcase 1: \"one\";\n\tdefault: \"other\";\n}",
        |p| p.eat_match(),
        DestackFormatOptions::default_tab()
    );
}

#[test]
fn test_format_switch_with_block() {
    assert_format!(
        "switch (value) { case 1: { const x = 1; } }",
        "switch (value) {\n\tcase 1: {\n\t\tconst x = 1;\n\t}\n}",
        |p| p.eat_match(),
        DestackFormatOptions::default_tab()
    );
}

/// Match arm annotations should stay on their own line above the arm.
#[test]
fn test_format_match_with_annotated_arm() {
    assert_format_program!(
        r#"match (result) { @cold Err(e) => handle(e); Ok(v) => v }"#,
        r#"match (result) {
    @cold
    Err(e) => handle(e)
    Ok(v) => v
}
"#,
        FileType::Destack,
    );
}

/// Else-branch comments should stay attached to the correct branch shell.
#[test]
fn test_format_if_else_comments() {
    assert_format_program_reference_widths(
        r#"if (true) {}

// comment1
else if (false) {}

// comment2

else {}
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"if (true) {
}

// comment1
else if (false) {
}

// comment2
else {
}
"#,
            ),
            (
                100,
                r#"if (true) {
}

// comment1
else if (false) {
}

// comment2
else {
}
"#,
            ),
        ],
    );
}

/// Yield comments should preserve the inner wrapper only when it is semantically needed.
#[test]
fn test_format_yield_type_comments() {
    assert_format_program_reference_widths(
        r#"function *t1() {
    yield (
        // comment
        a as any
    );
}

function *t2() {
    yield (
        // comment
        a as any
    ) + 1;
}
function *t3() {
    yield (
        // comment
        a as any
    ) ? 0 : 1;
}
function *t4() {
    yield (
        // comment
        a as any
    ).b;
}
function *t5() {
    yield (
        // comment
        a as any
    )[a];
}
function *t6() {
    yield (
        // comment
        a as any
    )();
}
function *t7() {
    yield (
        // comment
        a as any
    )``;
}
function *t8() {
    yield (
        // comment
        a as any
    ) as any;
}
function *t9() {
    yield (
        // comment
        a as any
    ) satisfies any;
}
function *t10() {
    yield (
        // comment
        a as any
    )!;
}
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"function* t1() {
  yield (
    // comment
    a as any
  );
}

function* t2() {
  yield (
    // comment
    (a as any) + 1
  );
}
function* t3() {
  yield (
    // comment
    (a as any)
      ? 0
      : 1
  );
}
function* t4() {
  yield (
    // comment
    (a as any).b
  );
}
function* t5() {
  yield (
    // comment
    (a as any)[a]
  );
}
function* t6() {
  yield (
    // comment
    (a as any)()
  );
}
function* t7() {
  yield (
    // comment
    (a as any)``
  );
}
function* t8() {
  yield (
    // comment
    a as any as any
  );
}
function* t9() {
  yield (
    // comment
    a as any satisfies any
  );
}
function* t10() {
  yield (
    // comment
    (a as any)!
  );
}
"#,
            ),
            (
                100,
                r#"function* t1() {
  yield (
    // comment
    a as any
  );
}

function* t2() {
  yield (
    // comment
    (a as any) + 1
  );
}
function* t3() {
  yield (
    // comment
    (a as any)
      ? 0
      : 1
  );
}
function* t4() {
  yield (
    // comment
    (a as any).b
  );
}
function* t5() {
  yield (
    // comment
    (a as any)[a]
  );
}
function* t6() {
  yield (
    // comment
    (a as any)()
  );
}
function* t7() {
  yield (
    // comment
    (a as any)``
  );
}
function* t8() {
  yield (
    // comment
    a as any as any
  );
}
function* t9() {
  yield (
    // comment
    a as any satisfies any
  );
}
function* t10() {
  yield (
    // comment
    (a as any)!
  );
}
"#,
            ),
        ],
    );
}

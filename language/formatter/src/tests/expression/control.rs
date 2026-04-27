use crate::{
    DestackFormatOptions, assert_format, assert_format_program,
    assert_format_program_reference_widths,
};
use destack_source::FileType;

#[test]
fn test_format_match_expression_cases() {
    assert_format!(
        "match (x) { 1 => 2; 3 => 4 }",
        r#"match (x) {
	1 => 2
	3 => 4
}"#,
        |p| p.eat_match(),
        DestackFormatOptions::default_tab()
    );
}

/// Yielded member chains should keep separator comments inside the chain indentation.
#[test]
fn test_format_yield_member_separator_comment() {
    assert_format_program_reference_widths(
        r#"function *a() {
  yield task
    // No extra parens
    .run();
}
"#,
        FileType::JavaScript,
        &[
            (
                80,
                r#"function* a() {
  yield task
    // No extra parens
    .run();
}
"#,
            ),
            (
                100,
                r#"function* a() {
  yield task
    // No extra parens
    .run();
}
"#,
            ),
        ],
    );
}

/// For-loop assignments should only parenthesize the condition slot.
#[test]
fn test_format_for_assignment_slots() {
    assert_format_program!(
        r#"for (i = 0; foo = bar; i += 1) {}"#,
        r#"for (i = 0; (foo = bar); i += 1) {}
"#,
        FileType::TypeScript
    );
}

#[test]
fn test_format_match_with_block_case_and_guard() {
    assert_format!(
        "match (value) { Pattern if (cond) => { const X = 1; } }",
        r#"match (value) {
	Pattern if (cond) => {
		const X = 1;
	}
}"#,
        |p| p.eat_match(),
        DestackFormatOptions::default_tab()
    );
}

#[test]
fn test_format_switch_expression_cases() {
    assert_format!(
        "switch (x) { case 1: 2; case 3: 4 }",
        r#"switch (x) {
	case 1:
		2;
	case 3:
		4;
}"#,
        |p| p.eat_match(),
        DestackFormatOptions::default_tab()
    );
}

#[test]
fn test_format_switch_with_default_case() {
    assert_format!(
        "switch (x) { case 1: \"one\"; default: \"other\" }",
        r#"switch (x) {
	case 1:
		"one";
	default:
		"other";
}"#,
        |p| p.eat_match(),
        DestackFormatOptions::default_tab()
    );
}

#[test]
fn test_format_switch_with_block() {
    assert_format!(
        "switch (value) { case 1: { const x = 1; } }",
        r#"switch (value) {
	case 1: {
		const x = 1;
	}
}"#,
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

/// Else-branch comments should stay attached to the correct branch.
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
function *t11() {
    yield (
        /* keep */ // comment
        a as any
    ) + 1;
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
function* t11() {
  yield (
    /* keep */ // comment
    (a as any) + 1
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
function* t11() {
  yield (
    /* keep */ // comment
    (a as any) + 1
  );
}
"#,
            ),
        ],
    );
}

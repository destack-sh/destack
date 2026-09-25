use crate::{
    TsppFormatOptions, assert_format, assert_format_program,
    assert_format_program_reference_widths, assert_format_program_roundtrip_with_file_type,
    parse_first_expression,
};
use tspp_source::FileType;

#[test]
fn test_format_match_expression_cases() {
    assert_format!(
        "match (x) { 1 => 2; 3 => 4 }",
        r#"match (x) {
	1 => 2
	3 => 4
}"#,
        parse_first_expression,
        TsppFormatOptions::default_tab()
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
        FileType::Tspp,
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
        FileType::Tspp
    );
}

/// Keep a short for-loop condition independent from expanded neighboring slots.
#[test]
fn test_format_for_condition_groups_independently() {
    assert_format_program_reference_widths(
        r#"for (let lane: usize = firstLaneOffsetThatForcesTheInitializerToBreak(); lane < N; lane += strideThatForcesTheUpdateToBreak()) {}
"#,
        FileType::Tspp,
        &[(
            80,
            r#"for (
  let lane: usize = firstLaneOffsetThatForcesTheInitializerToBreak();
  lane < N;
  lane += strideThatForcesTheUpdateToBreak()
) {}
"#,
        )],
    );
}

/// Format one while binding condition as a logical operand sequence.
#[test]
fn test_format_while_binding_condition() {
    assert_format_program!(
        r#"while(let value! = queue.tryPop()&&value.isReady()){process(value);}"#,
        r#"while (let value! = queue.tryPop() && value.isReady()) {
    process(value);
}
"#,
        FileType::Tspp
    );
}

/// Preserve grouping required by one compound while-condition operand.
#[test]
fn test_format_grouped_while_binding_condition_operand() {
    assert_format_program!(
        r#"while((ready||retry)&&let value! = queue.tryPop()){process(value);}"#,
        r#"while ((ready || retry) && let value! = queue.tryPop()) {
    process(value);
}
"#,
        FileType::Tspp
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
        parse_first_expression,
        TsppFormatOptions::default_tab()
    );
}

/// Format one grouped binding condition in a match guard.
#[test]
fn test_format_match_binding_guard() {
    assert_format_program!(
        r#"match(value){text if((ready||retry)&&let parsed! = parse(text)&&parsed>0)=>parsed}"#,
        r#"match (value) {
    text if ((ready || retry) && let parsed! = parse(text) && parsed > 0) => parsed
}
"#,
        FileType::Tspp
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
        parse_first_expression,
        TsppFormatOptions::default_tab()
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
        parse_first_expression,
        TsppFormatOptions::default_tab()
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
        parse_first_expression,
        TsppFormatOptions::default_tab()
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
        FileType::Tspp,
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
        FileType::Tspp,
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

/// Block comments before `else` should stay on the explicit block boundary.
#[test]
fn test_format_if_else_block_boundary_comment() {
    assert_format_program_roundtrip_with_file_type(
        r#"if (ready) { run() } /* keep-boundary */ else { stop() }
"#,
        r#"if (ready) {
    run()
} /* keep-boundary */ else {
    stop()
}
"#,
        FileType::Tspp,
        TsppFormatOptions::default(),
    );
}

/// Line comments before `else` should stay after the consequent block.
#[test]
fn test_format_if_else_line_boundary_comment() {
    assert_format_program_roundtrip_with_file_type(
        r#"if (ready) { run() } // keep-boundary
else { stop() }
"#,
        r#"if (ready) {
    run()
} // keep-boundary
else {
    stop()
}
"#,
        FileType::Tspp,
        TsppFormatOptions::default(),
    );
}

/// Trailing comments should not expand brace-free branches.
#[test]
fn test_format_if_else_statement_boundary_comment() {
    assert_format_program_roundtrip_with_file_type(
        r#"if (ready) run() // keep-run
else stop()
"#,
        r#"if (ready) run(); // keep-run
else stop();
"#,
        FileType::Tspp,
        TsppFormatOptions::default(),
    );
}

/// Block comments before `else` should not expand a brace-free alternate body.
#[test]
fn test_format_if_else_statement_block_boundary_comment() {
    assert_format_program_roundtrip_with_file_type(
        r#"if (ready) run()
/* keep-boundary */
else stop()
"#,
        r#"if (ready) run();
/* keep-boundary */ else stop();
"#,
        FileType::Tspp,
        TsppFormatOptions::default(),
    );
}

/// Line comments before `else` should not expand a brace-free alternate body.
#[test]
fn test_format_if_else_statement_line_boundary_comment() {
    assert_format_program_roundtrip_with_file_type(
        r#"if (ready) run()
// keep-boundary
else stop()
"#,
        r#"if (ready) run();
// keep-boundary
else stop();
"#,
        FileType::Tspp,
        TsppFormatOptions::default(),
    );
}

/// Yield comments should preserve the inner wrapper only when it is semantically needed.
#[test]
fn test_format_yield_type_comments() {
    assert_format_program_reference_widths(
        r#"function *t1() {
    yield (
        // comment
        a as unknown
    );
}

function *t2() {
    yield (
        // comment
        a as unknown
    ) + 1;
}
function *t3() {
    yield (
        // comment
        a as unknown
    ) ? 0 : 1;
}
function *t4() {
    yield (
        // comment
        a as unknown
    ).b;
}
function *t5() {
    yield (
        // comment
        a as unknown
    )[a];
}
function *t6() {
    yield (
        // comment
        a as unknown
    )();
}
function *t7() {
    yield (
        // comment
        a as unknown
    )``;
}
function *t8() {
    yield (
        // comment
        a as unknown
    ) as unknown;
}
function *t9() {
    yield (
        // comment
        a as unknown
    ) satisfies unknown;
}
function *t10() {
    yield (
        // comment
        a as unknown
    )!;
}
function *t11() {
    yield (
        /* keep */ // comment
        a as unknown
    ) + 1;
}
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"function* t1() {
  yield (
    // comment
    a as unknown
  );
}

function* t2() {
  yield (
    // comment
    (a as unknown) + 1
  );
}
function* t3() {
  yield (
    // comment
    (a as unknown)
      ? 0
      : 1
  );
}
function* t4() {
  yield (
    // comment
    (a as unknown).b
  );
}
function* t5() {
  yield (
    // comment
    (a as unknown)[a]
  );
}
function* t6() {
  yield (
    // comment
    (a as unknown)()
  );
}
function* t7() {
  yield (
    // comment
    (a as unknown)``
  );
}
function* t8() {
  yield (
    // comment
    a as unknown as unknown
  );
}
function* t9() {
  yield (
    // comment
    a as unknown satisfies unknown
  );
}
function* t10() {
  yield (
    // comment
    (a as unknown)!
  );
}
function* t11() {
  yield (
    /* keep */ // comment
    (a as unknown) + 1
  );
}
"#,
            ),
            (
                100,
                r#"function* t1() {
  yield (
    // comment
    a as unknown
  );
}

function* t2() {
  yield (
    // comment
    (a as unknown) + 1
  );
}
function* t3() {
  yield (
    // comment
    (a as unknown)
      ? 0
      : 1
  );
}
function* t4() {
  yield (
    // comment
    (a as unknown).b
  );
}
function* t5() {
  yield (
    // comment
    (a as unknown)[a]
  );
}
function* t6() {
  yield (
    // comment
    (a as unknown)()
  );
}
function* t7() {
  yield (
    // comment
    (a as unknown)``
  );
}
function* t8() {
  yield (
    // comment
    a as unknown as unknown
  );
}
function* t9() {
  yield (
    // comment
    a as unknown satisfies unknown
  );
}
function* t10() {
  yield (
    // comment
    (a as unknown)!
  );
}
function* t11() {
  yield (
    /* keep */ // comment
    (a as unknown) + 1
  );
}
"#,
            ),
        ],
    );
}

/// Break values print bare, with parentheses only around lone identifiers.
#[test]
fn test_format_break_value_forms() {
    assert_format_program!(
        r#"const a = loop { break 10; };
const b = loop { break (result); };
const c = loop { break count * 2; };
const d = outer: loop { break outer: "done"; };
"#,
        r#"const a = loop {
    break 10;
};
const b = loop {
    break (result);
};
const c = loop {
    break count * 2;
};
const d = outer: loop {
    break outer: "done";
};
"#,
        FileType::Tspp
    );
}

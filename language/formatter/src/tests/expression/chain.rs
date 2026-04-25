use crate::{
    DestackFormatOptions, assert_format_program_reference_widths,
    assert_format_program_roundtrip_with_file_type,
};
use destack_source::FileType;

/// Nested helper calls in broken sequence expressions should still stay flat when they fit.
#[test]
fn test_format_member_call_arguments_stay_flat_in_broken_sequence() {
    assert_format_program_roundtrip_with_file_type(
        r#"function f() {
  return (
    (_this$prop3 = babelHelpers.classPrivateFieldGet2(_prop2, this)),
    // This forces the sequence group to expand without forcing the helper call.
    // The helper call should keep its own fitting decision.
    class Inner {}
  );
}
"#,
        r#"function f() {
    return (
        (_this$prop3 = babelHelpers.classPrivateFieldGet2(_prop2, this)),
        // This forces the sequence group to expand without forcing the helper call.
        // The helper call should keep its own fitting decision.
        class Inner {}
    );
}
"#,
        FileType::JavaScript,
        DestackFormatOptions::default_with_line_width(100),
    );
}

/// Member chains should split after the base head.
#[test]
fn test_format_member_chain_splits_after_base_head() {
    assert_format_program_reference_widths(
        r#"const result = api.getClient().getService().fetchAll().map((x) => x.id)
"#,
        FileType::Destack,
        &[(
            30,
            r#"const result = api
  .getClient()
  .getService()
  .fetchAll()
  .map((x) => x.id);
"#,
        )],
    );
}

/// Long generic member chains should keep the chosen split head shape.
#[test]
fn test_format_member_chain_with_generic_call_arguments() {
    assert_format_program_reference_widths(
        r#"const defaultColorDecoratorsEnablement = accessor.get(IConfigurationService).getValue<"auto" | "always" | "never">("longlonglonglonglonglonglonglonglong")
"#,
        FileType::TypeScript,
        &[(
            60,
            r#"const defaultColorDecoratorsEnablement = accessor
  .get(IConfigurationService)
  .getValue<"auto" | "always" | "never">(
    "longlonglonglonglonglonglonglonglong",
  );
"#,
        )],
    );
}

/// Optional chains should break with the operator leading each continuation line.
#[test]
fn test_format_optional_chain_breaks_with_leading_operators() {
    assert_format_program_reference_widths(
        r#"const value = dataSource?.getClient()?.getUser(id)?.profile?.name
"#,
        FileType::Destack,
        &[(
            35,
            r#"const value = dataSource
  ?.getClient()
  ?.getUser(id)?.profile?.name;
"#,
        )],
    );
}

/// Blank lines between chain groups should be preserved.
#[test]
fn test_format_member_chain_preserves_blank_lines() {
    assert_format_program_reference_widths(
        r#"Promise.all(writeIconFiles)

  // TO DO -- END
  .then(() => writeRegistry())
"#,
        FileType::TypeScript,
        &[(
            80,
            r#"Promise.all(writeIconFiles)

  // TO DO -- END
  .then(() => writeRegistry());
"#,
        )],
    );
}

/// Member instantiations should stay inline when the chain still fits.
#[test]
fn test_format_member_instantiation_chain_stays_inline() {
    assert_format_program_roundtrip_with_file_type(
        r#"api.getService().getFactory<number>
"#,
        r#"api.getService().getFactory<number>;
"#,
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(25),
    );
}

/// Short statement-position heads should merge the first chain group.
#[test]
fn test_format_member_chain_merges_short_statement_head() {
    assert_format_program_roundtrip_with_file_type(
        r#"obj.method() /* step 1 */ .transform() /* step 2 */ .result()
"#,
        r#"obj.method() /* step 1 */
    .transform() /* step 2 */
    .result();
"#,
        FileType::Destack,
        DestackFormatOptions::default_with_line_width(20),
    );
}

/// Short statement-position call heads should keep the first call on the head line.
#[test]
fn test_format_member_chain_merges_short_statement_call_head() {
    assert_format_program_roundtrip_with_file_type(
        r#"data.filter(x => x.valid) /* now map */ .map(x => x.value)
"#,
        r#"data.filter((x) => x.valid) /* now map */
    .map((x) => x.value);
"#,
        FileType::Destack,
        DestackFormatOptions::default_with_line_width(50),
    );
}

/// Computed first hops should break after the separator comment when width is tight.
#[test]
fn test_format_member_chain_breaks_computed_first_hop_with_separator_comment() {
    assert_format_program_roundtrip_with_file_type(
        r#"source /* before-index */ [key].call()
"#,
        r#"source /* before-index */[
    key
].call();
"#,
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(20),
    );
}

/// Blank lines after terminal calls should not survive onto the next member hop.
#[test]
fn test_format_member_chain_elides_blank_line_after_terminal_call() {
    assert_format_program_roundtrip_with_file_type(
        r#"const x = fn()

.c1()
"#,
        r#"const x = fn().c1();
"#,
        FileType::JavaScript,
        DestackFormatOptions::default_with_line_width(80),
    );
}

/// Tagged template arguments should not count as simple chain arguments.
#[test]
fn test_format_member_chain_breaks_for_tagged_template_argument() {
    assert_format_program_reference_widths(
        r#"const value = utc("time_updated").notNull().default(sql`CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)`);
"#,
        FileType::TypeScript,
        &[(
            60,
            r#"const value = utc("time_updated")
  .notNull()
  .default(
    sql`CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)`,
  );
"#,
        )],
    );
}

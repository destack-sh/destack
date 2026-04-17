use crate::chain::MemberChain;
use crate::{
    DestackFormatOptions, TestFormatter, assert_format_program_reference_widths,
    assert_format_program_roundtrip_with_file_type,
};
use destack_ast::Expression;
use destack_parser::ParserOptions;
use destack_source::FileType;

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

/// Short declarator heads keep short instantiation chains inline at the reference width.
#[test]
fn test_format_member_instantiation_chain_stays_inline() {
    assert_format_program_roundtrip_with_file_type(
        r#"const value = api.getService().getFactory<number>
"#,
        r#"const value = api.getService().getFactory<number>;
"#,
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(25),
    );
}

/// Static member instantiations should not create extra tail groups.
#[test]
fn test_member_instantiation_chain_tail_group_shape() {
    let (formatter, expression_id) = TestFormatter::parse_with_file_type(
        r#"service.getService().getFactory<number>"#,
        FileType::TypeScript,
        |parser| parser.eat_expression(ParserOptions::default()),
    )
    .expect("parse member instantiation chain");
    let context = formatter.context(DestackFormatOptions::default_with_line_width(25));

    let expression = context.tree.get(expression_id);
    let tail_group_count =
        MemberChain::tail_group_count(&context, expression_id).expect("build member chain");

    assert!(matches!(expression, Expression::Instantiation { .. }));
    assert_eq!(tail_group_count, 1);
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

/// Computed first hops should still merge with the head when only the computed gap has a comment.
#[test]
fn test_format_member_chain_merges_computed_first_hop_with_boundary_comment() {
    assert_format_program_roundtrip_with_file_type(
        r#"source /* before-index */ [key].call()
"#,
        r#"source /* before-index */[key].call();
"#,
        FileType::TypeScript,
        DestackFormatOptions::default_with_line_width(20),
    );
}

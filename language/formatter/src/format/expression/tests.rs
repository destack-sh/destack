use crate::format::directive::any_ignore_range_for_nodes;
use crate::format::expression::{
    ParenthesizedUnwrapMode, expression_has_complex_callback, is_assignment_left_target,
    should_unwrap_parenthesized,
};
use crate::format::operator::flatten_type_binary_expression;
use crate::{
    Annotation, DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions, TestFormatter,
    assert_format, assert_format_idempotent_with_file_type, assert_format_output_eq,
    assert_format_program_idempotent_with_file_type,
    assert_format_program_roundtrip_with_file_type, assert_format_roundtrip_with_file_type,
    statement_list,
};
use destack_ast::{
    Argument, BinaryOperator, BlockContext, Declaration, DeclarationDescriptor, Expression,
    LocalNodeId, NodeParentIndex, NodeTree, NodeType,
};
use destack_source::{FileType, LanguageType};
use destack_workspace::{FormatterOptions, QuoteProperty, QuoteStyle};

/// Build a formatter context for expression classifier assertions.
fn context_from_formatter(formatter: &TestFormatter) -> DestackFormatContext<'_> {
    DestackFormatContext::new(
        DestackFormatOptions::default(),
        DestackFormatArtifacts {
            file: &formatter.file,
            tree: &formatter.tree,
            tokens: &formatter.tokens,
            side_tokens: &formatter.side_tokens,
            side_span: &formatter.side_span,
            strings: &formatter.strings,
            parents: NodeParentIndex::from_tree(&formatter.tree),
        },
    )
}

/// Build the JavaScript options used by fixture-derived tests.
fn javascript_fixture_format_options() -> DestackFormatOptions {
    let formatter_options = FormatterOptions::default()
        .with_indent_width(2)
        .with_line_width(80)
        .with_quote_style(QuoteStyle::Double);
    DestackFormatOptions::from_formatter_options(formatter_options, LanguageType::JavaScript)
}

/// Build the TypeScript options used by fixture-derived tests.
fn typescript_fixture_format_options() -> DestackFormatOptions {
    let formatter_options = FormatterOptions::default()
        .with_indent_width(2)
        .with_line_width(80)
        .with_quote_style(QuoteStyle::Double);
    DestackFormatOptions::from_formatter_options(formatter_options, LanguageType::TypeScript)
}

const OPTIONAL_CALL_NO_ARGUMENT_SOURCE: &str = r#"call// 101
()
call
// 102
()
call(// 103
  )
call
  (// 104
  )
call(
  // 105
  )
call// 106
?.(
  )
call// 107
?.(
  )
call?.// 108
(
  )

call/* 201 */
()
call
/* 202 */
()
call/* 203 */()
call(/* 204 */
  )
call(
  /* 205 */
  )
call/* 206 */?.
()
call/* 207 */
?.
()
call
/* 208 */?.
()

call/* 209 */
?.
()
call?./* 210 */
()
call
?./* 211 */
()
call
?.
/* 212 */
()
"#;

const TYPESCRIPT_AS_SOURCE: &str = r#"const name = (description as DescriptionObject).name || (description as string);
this.isTabActionBar((e.target || e.srcElement) as HTMLElement);
(originalError ? wrappedError(errMsg, originalError) : Error(errMsg)) as InjectionError;
'current' in (props.pagination as Object);
('current' in props.pagination) as Object;
start + (yearSelectTotal as number);
(start + yearSelectTotal) as number;
scrollTop > (visibilityHeight as number);
(scrollTop > visibilityHeight) as number;
export default class Column<T> extends (RcTable.Column as React.ComponentClass<ColumnProps<T>,ColumnProps<T>,ColumnProps<T>,ColumnProps<T>>) {}
export const MobxTypedForm = class extends (Form as { new (): any }) {}
export abstract class MobxTypedForm1 extends (Form as { new (): any }) {}
({}) as {};
function*g() {
  const test = (yield 'foo') as number;
}
async function g1() {
  const test = (await 'foo') as number;
}
({}) as X;
() => ({}) as X;
const state = JSON.stringify({
  next: window.location.href,
  nonce,
} as State);

(foo.bar as Baz) = [bar];
(foo.bar as any)++;

(bValue as boolean) ? 0 : -1;
<boolean>bValue ? 0 : -1;

const value1 = thisIsAReallyReallyReallyReallyReallyLongIdentifier as SomeInterface;
const value2 = thisIsAnIdentifier as thisIsAReallyReallyReallyReallyReallyReallyReallyReallyReallyReallyReallyLongInterface;
const value3 = thisIsAReallyLongIdentifier as (SomeInterface | SomeOtherInterface);
const value4 = thisIsAReallyLongIdentifier as { prop1: string, prop2: number, prop3: number }[];
const value5 = thisIsAReallyReallyReallyReallyReallyReallyReallyReallyReallyLongIdentifier as [string, number];

const iter1 = createIterator(this.controller, child, this.tag as SyncFunctionComponent);
const iter2 = createIterator(self.controller, child, self.tag as SyncFunctionComponent);
"#;

/// Find the first parenthesized expression whose inner expression satisfies a predicate.
fn find_parenthesized_expression_by_inner(
    tree: &NodeTree,
    mut predicate: impl FnMut(&Expression) -> bool,
) -> (LocalNodeId<Expression>, LocalNodeId<Expression>) {
    for raw_node_id in 0..tree.next_id() {
        if tree.get_node_type(raw_node_id) != NodeType::Expression {
            continue;
        }

        let expression_id = LocalNodeId::<Expression>::new(raw_node_id);
        let Expression::Parenthesized { expression } = tree.get(expression_id) else {
            continue;
        };
        let inner_expression = tree.get(*expression);
        if predicate(inner_expression) {
            return (expression_id, *expression);
        }
    }

    panic!("expected parenthesized expression matching predicate");
}

/// Simple expressions should stay on one line.
#[test]
fn test_format_expression_simple() {
    assert_format!(
        "1 + 2 * 3 - a / b % c",
        "1 + 2 * 3 - a / b % c",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_tab()
    );
}

/// Parenthesized expressions should retain their parentheses.
#[test]
fn test_format_expression_parenthesized() {
    assert_format!(
        "(((1 + 2) * 3) - a / (b % c))",
        "((1 + 2) * 3 - a / (b % c))",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_tab()
    );
}

/// Empty parenthesis/arguments/tuplestuples should be respected.
#[test]
fn test_format_expression_nested_empty_parenthesis() {
    assert_format!(
        "(((())))",
        "(((())))",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// Empty parenthesis/arguments/tuplestuples should be respected.
#[test]
fn test_format_expression_nested_empty_arguments() {
    assert_format!(
        "foo<()>(((())))",
        "foo<()>(((())))",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_expression_struct_literal_trivial() {
    assert_format!(
        "({ a: 1, ...B })",
        "({ a: 1, ...B })",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_expression_struct_literal_spread() {
    assert_format!(
        "Foo { ...B }",
        "Foo { ...B }",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_assignment_target_detection() {
    let source = "({ className, unfurl: unfurlAttrr, ...attrs } = { className: \"name\", unfurl: \"unfurl\", others: [1, 2, 3] })";
    let (formatter, _expression_id) =
        TestFormatter::parse(source, |p| p.eat_expression(Default::default()))
            .expect("parse assignment target source");

    let context = context_from_formatter(&formatter);

    let mut found_assignment_target = false;
    for raw_node_id in 0..formatter.tree.next_id() {
        if formatter.tree.get_node_type(raw_node_id) != NodeType::Expression {
            continue;
        }

        let object_expression_id = LocalNodeId::<Expression>::new(raw_node_id);
        let Expression::ObjectExpression { properties, .. } =
            formatter.tree.get(object_expression_id)
        else {
            continue;
        };

        if properties.len() != 3 {
            continue;
        }

        if is_assignment_left_target(&context, object_expression_id) {
            found_assignment_target = true;
            break;
        }
    }

    assert!(found_assignment_target);
}

#[test]
fn test_format_expression_if_ternary() {
    assert_format!(
        "true ? 1 : 2",
        "true ? 1 : 2",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_expression_index_call_mixed_postfix() {
    assert_format!(
        "x?.[f]?.[2]?.(a, b)",
        "x?.[f]?.[2]?.(a, b)",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// Instantiation expressions should retain static arguments.
#[test]
fn test_format_expression_instantiation() {
    assert_format!(
        "f<number>",
        "f<number>",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// New expressions should drop redundant parentheses for simple member callees.
#[test]
fn test_format_new_expression_drops_simple_member_parentheses() {
    assert_format!(
        "new (a.b)()",
        "new a.b()",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// New expressions should keep optional member callee parentheses.
#[test]
fn test_format_new_expression_keeps_optional_member_parentheses() {
    assert_format!(
        "new (a?.b)()",
        "new (a?.b)()",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// Call expressions should keep optional member callee parentheses.
#[test]
fn test_format_call_expression_keeps_optional_member_parentheses() {
    assert_format!(
        "(a?.b)()",
        "(a?.b)()",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// Member expressions should keep optional member object parentheses.
#[test]
fn test_format_member_expression_keeps_optional_member_parentheses() {
    assert_format!(
        "(a?.b).c",
        "(a?.b).c",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// New expressions should wrap call member callees as a unit.
#[test]
fn test_format_new_expression_wraps_call_member_callee() {
    assert_format!(
        "new (X()).y",
        "new (X().y)()",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_class_first_member_no_blank_with_consistent_quote_props() {
    let mut options = DestackFormatOptions::default();
    options.indent_width = 2;
    options.line_width = 80;
    options.quote_style = QuoteStyle::Double;
    options.quote_props = QuoteProperty::Consistent;

    assert_format_program_roundtrip_with_file_type(
        "// Class with no quotes needed\nclass A {\n  a = \"a\";\n}\n\n// Class with quotes preserved\nclass B {\n  'b' = \"b\";\n}\n",
        "// Class with no quotes needed\nclass A {\n  a = \"a\";\n}\n\n// Class with quotes preserved\nclass B {\n  b = \"b\";\n}\n",
        FileType::JavaScript,
        options,
    );
}

#[test]
fn test_format_jsx_comment_between_statements_stays_own_line_prefix() {
    let mut options = DestackFormatOptions::default();
    options.indent_width = 2;
    options.line_width = 80;
    options.quote_style = QuoteStyle::Double;

    assert_format_program_roundtrip_with_file_type(
        "[\n  {\n    baz: () => {\n      return <Foo />;\n    },\n  },\n];\n\n// Simpler attribute case\n<Component />;\n",
        "[\n  {\n    baz: () => {\n      return <Foo />;\n    },\n  },\n];\n\n// Simpler attribute case\n<Component />;\n",
        FileType::JavaScriptXml,
        options,
    );
}

#[test]
fn test_format_assignment_seam_inline_block_comment_roundtrip() {
    assert_format_program_roundtrip_with_file_type(
        "value = /* seam */ other;\n",
        "value = /* seam */ other;\n",
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

#[test]
fn test_format_declarator_assignment_seam_inline_block_comment_roundtrip() {
    assert_format_program_roundtrip_with_file_type(
        "let value = /* seam */ other;\n",
        "let value = /* seam */ other;\n",
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Assignment seams with closure type-cast docs stay inline and idempotent.
#[test]
fn test_format_assignment_seam_inline_doc_comment_roundtrip() {
    assert_format_program_roundtrip_with_file_type(
        "foo = (/** @type {!Baz} */ (baz).bar);\n",
        "foo = /** @type {!Baz} */ (baz).bar;\n",
        FileType::JavaScript,
        javascript_fixture_format_options(),
    );
}

/// Parenthesized member objects with boundary comments should not unwrap.
#[test]
fn test_parenthesis_rules_reject_member_object_boundary_comment() {
    let source = "(value /* boundary */).member";
    let (formatter, _) = TestFormatter::parse(source, |p| p.eat_expression(Default::default()))
        .expect("parse member expression");
    let context = context_from_formatter(&formatter);
    let (parenthesized_id, inner_expression_id) =
        find_parenthesized_expression_by_inner(&formatter.tree, |_| true);

    assert!(!should_unwrap_parenthesized(
        &context,
        parenthesized_id,
        inner_expression_id,
        ParenthesizedUnwrapMode::MemberObject,
    ));
}

/// Parenthesized closure-cast member objects should unwrap.
#[test]
fn test_parenthesis_rules_allow_closure_cast_member_object_unwrap() {
    let source = "(/** @type {array} */ numberOrString).map((x) => x)";
    let (formatter, _) = TestFormatter::parse(source, |p| p.eat_expression(Default::default()))
        .expect("parse member expression");
    let context = context_from_formatter(&formatter);
    let (parenthesized_id, inner_expression_id) =
        find_parenthesized_expression_by_inner(&formatter.tree, |_| true);

    assert!(should_unwrap_parenthesized(
        &context,
        parenthesized_id,
        inner_expression_id,
        ParenthesizedUnwrapMode::MemberObject,
    ));
}

/// Parenthesized ordinary-comment member objects should unwrap.
#[test]
fn test_parenthesis_rules_allow_ordinary_comment_member_object_unwrap() {
    let source = "(/* ordinary */ source).next()";
    let (formatter, _) = TestFormatter::parse(source, |p| p.eat_expression(Default::default()))
        .expect("parse member expression");
    let context = context_from_formatter(&formatter);
    let (parenthesized_id, inner_expression_id) =
        find_parenthesized_expression_by_inner(&formatter.tree, |_| true);

    assert!(should_unwrap_parenthesized(
        &context,
        parenthesized_id,
        inner_expression_id,
        ParenthesizedUnwrapMode::MemberObject,
    ));
}

/// Type-cast seams should keep trailing callsite comments on their own line.
#[test]
fn test_format_type_cast_node_keeps_terminal_line_comment() {
    let mut options = DestackFormatOptions::default();
    options.indent_width = 2;
    options.line_width = 80;
    options.quote_style = QuoteStyle::Double;

    assert_format_program_roundtrip_with_file_type(
        "!left &&\n/** @type {boolean} */\n(\n  /** @type {Identifier} */\n  (a) === \"call\" ||\n    /** @type {Identifier} */\n    (b) === \"bind\"\n//  ^^^^^^^^^^^^^^ No need to wrap with parentheses here because the type cast node is already wrapped with parentheses.\n) && right;\n\n/** @type {Number} */ (a + b)();\n//                    ^^^^^^^ No need to wrap with parentheses here because the type cast node is already wrapped with parentheses.\n",
        "!left &&\n  /** @type {boolean} */\n  (\n    /** @type {Identifier} */\n    (a) === \"call\" ||\n      /** @type {Identifier} */\n      (b) === \"bind\"\n    //  ^^^^^^^^^^^^^^ No need to wrap with parentheses here because the type cast node is already wrapped with parentheses.\n  ) &&\n  right;\n\n/** @type {Number} */ (a + b)();\n//                    ^^^^^^^ No need to wrap with parentheses here because the type cast node is already wrapped with parentheses.\n",
        FileType::JavaScript,
        options,
    );
}

/// Decorated class expressions in extends heads should keep explicit parentheses.
#[test]
fn test_format_decorated_class_expression_keeps_extends_parentheses() {
    let source = "class Derived extends (@decorator class Base {}) {}";
    let expected = "class Derived extends (\n    @decorator\n    class Base {}\n) {}";
    let (formatter, expression_id) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
            p.eat_expression(Default::default())
        })
        .expect("parse decorated class extends expression");

    let formatted = formatter.format(&expression_id, DestackFormatOptions::default());
    assert_eq!(formatted, expected);
}

/// Class extends assignment heads should keep explicit grouping wrappers.
#[test]
fn test_format_class_extends_assignment_keeps_parentheses_idempotent() {
    assert_format_idempotent_with_file_type(
        "class A extends (b = c) {}",
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

/// Anonymous class extends update heads should keep explicit grouping wrappers.
#[test]
fn test_format_anonymous_class_extends_update_keeps_parentheses_idempotent() {
    assert_format_idempotent_with_file_type(
        "x = class extends (++b) {}",
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

/// Class extends object heads should keep explicit grouping wrappers.
#[test]
fn test_format_class_extends_object_keeps_parentheses_idempotent() {
    assert_format_idempotent_with_file_type(
        "class A extends ({}) {}",
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

/// Function expression member receivers should keep explicit grouping wrappers.
#[test]
fn test_format_function_expression_member_keeps_parentheses_idempotent() {
    assert_format_idempotent_with_file_type(
        "(function() {}).length",
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

/// Class expression member receivers should keep explicit grouping wrappers.
#[test]
fn test_format_class_expression_member_keeps_parentheses_idempotent() {
    assert_format_idempotent_with_file_type(
        "(class {}).a",
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

/// Function expression optional-call receivers should keep explicit grouping wrappers.
#[test]
fn test_format_function_expression_optional_call_keeps_parentheses_idempotent() {
    assert_format_idempotent_with_file_type(
        "(function() {})?.()",
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

/// Broken chains should keep non-null tails attached to the preceding call expression.
#[test]
fn test_format_non_null_chain_keeps_call_tail_attached_idempotent() {
    let source = r#"{
  const secondType = sourceCode.getNodeByRangeIndex1234(second.range[0])!.type
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Broken chains should keep consecutive non-null tails attached to the preceding call expression.
#[test]
fn test_format_double_non_null_chain_keeps_call_tail_attached_idempotent() {
    let source = r#"{
  const secondType = sourceCode.getNodeByRangeIndex1234(second.range[0])!!.type
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Broken optional chains should keep non-null tails attached to the preceding optional call.
#[test]
fn test_format_optional_non_null_chain_keeps_call_tail_attached_idempotent() {
    let source = r#"{
  const secondType = sourceCode?.getNodeByRangeIndex1234(second.range[0])!.type
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Parenthesized optional chains should keep non-null tails before the closing wrapper.
#[test]
fn test_format_parenthesized_optional_non_null_chain_keeps_tail_inside_wrapper_idempotent() {
    let source = r#"{
  const secondType = (sourceCode?.getNodeByRangeIndex1234(second.range[0])!).type
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Object-expression chain bases should keep explicit grouping in statement positions.
#[test]
fn test_format_non_null_object_chain_base_keeps_grouping_idempotent() {
    let source = r#"{
  if (a) ({ a, ...b }).a()!.c()
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Multiline type-assertion roots should keep non-null and member tails attached on one line.
#[test]
fn test_format_non_null_static_member_chain_after_multiline_type_assertion_has_expected_layout() {
    let source = r#"(<IJSONSchema>(
  compoundConfigurationsSchema.items
)).oneOf![1].properties!.folder.enum = folderNames;
"#;

    assert_format_program_roundtrip_with_file_type(
        source,
        source,
        FileType::TypeScript,
        typescript_fixture_format_options(),
    );
}

/// Heritage heads with optional and non-null member chains should stay parse-safe and idempotent.
#[test]
fn test_format_heritage_member_expression_like_non_null_chain_is_idempotent() {
    let source = r#"class A_long_long_long_long_long_long_long_long_name1
  extends eslint.Rule.RuleModule {}

class Short
  extends eslint.Rule.RuleModule {}

class A_long_long_long_long_long_long_long_long_name12
  extends eslint.Rule?.RuleModule {}

class A_long_long_long_long_long_long_long_long_name12
  extends eslint.Rule.RuleModule! {}
class A_long_long_long_long_long_long_long_long_name12
  extends eslint?.Rule.RuleModule! {}
class A_long_long_long_long_long_long_long_long_name12
  extends eslint.Rule.RuleModule!! {}

interface A_long_long_long_long_long_long_long_long_name2
  extends eslint.Rule.RuleModule {}

class A_long_long_long_long_long_long_long_long_name2
  implements eslint.Rule.RuleModule {}

class A_long_long_long_long_long_long_long_long_name3
  extends eslint.Rule.RuleModule
  implements eslint.Rule.RuleModule {}
"#;

    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        typescript_fixture_format_options(),
    );
}

/// Decorated class arguments should be visible to argument annotation profiling.
#[test]
fn test_call_argument_profile_detects_decorated_class_argument() {
    let source = "use((@decorator class {}))";
    let (formatter, expression_id) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
            p.eat_expression(Default::default())
        })
        .expect("parse decorated class argument expression");
    let context = context_from_formatter(&formatter);

    let Expression::Call {
        dynamic_arguments, ..
    } = formatter.tree.get(expression_id)
    else {
        panic!("expected call expression");
    };
    assert_eq!(dynamic_arguments.len(), 1);

    let profile = context.argument_annotation_cache(dynamic_arguments[0]);
    assert!(
        profile.has_prefix_annotation,
        "expected decorated class argument to report prefix annotation"
    );
}

/// Parenthesized new callees with optional chains should not unwrap.
#[test]
fn test_parenthesis_rules_reject_optional_new_callee_unwrap() {
    let source = "new (value?.member)()";
    let (formatter, _) = TestFormatter::parse(source, |p| p.eat_expression(Default::default()))
        .expect("parse new expression");
    let context = context_from_formatter(&formatter);
    let (parenthesized_id, inner_expression_id) =
        find_parenthesized_expression_by_inner(&formatter.tree, |inner_expression| {
            matches!(
                inner_expression,
                Expression::Member { .. } | Expression::PrivateMember { .. }
            )
        });

    assert!(!should_unwrap_parenthesized(
        &context,
        parenthesized_id,
        inner_expression_id,
        ParenthesizedUnwrapMode::NewMemberCallee,
    ));
}

/// Sparse arrays should preserve elision slots.
#[test]
fn test_format_array_expression_sparse_elisions() {
    assert_format!(
        "[,,]",
        "[,,]",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
    assert_format!(
        "[,]",
        "[,]",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
    assert_format!(
        "[1,,]",
        "[1,,]",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
    assert_format!(
        "[1,,3]",
        "[1,, 3]",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// Empty new arguments should keep boundary comments inside parentheses.
#[test]
fn test_format_new_expression_empty_argument_comment() {
    assert_format!(
        "new require(/* comment */)",
        "new require(/* comment */)",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// Member expressions should unwrap redundant call object parentheses.
#[test]
fn test_format_member_expression_unwraps_parenthesized_call_object() {
    assert_format!(
        "(require(\"x\")).TraceEntryPointsPlugin",
        "require('x').TraceEntryPointsPlugin",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_member_expression_keeps_parenthesized_integer_object() {
    assert_format!(
        "(1).toString()",
        "(1).toString()",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// Tree children with map callbacks that return trees force a break.
#[test]
fn test_tree_child_map_callback_breaks() {
    let input = "<List>{items.map((item) => <Item key={item.id} />)}</List>";
    let (formatter, expression_id) =
        TestFormatter::parse(input, |p| p.eat_expression(Default::default()))
            .expect("parse tree expression");

    // find the tree child expression
    let Expression::TreeExpression { elements, .. } = formatter.tree.get(expression_id) else {
        panic!("expected tree expression");
    };
    let elements = elements.as_ref().expect("expected elements");
    let child_id = elements.first().expect("expected child element");
    let Argument::Positional { value, .. } = formatter.tree.get(*child_id) else {
        panic!("expected positional child");
    };

    // build a context to run the helper on
    let context = context_from_formatter(&formatter);

    assert!(expression_has_complex_callback(&context, *value));
}

/// Const on borrows normalizes to readonly in type formatting.
#[test]
fn test_format_type_const_borrow_normalizes_to_readonly() {
    assert_format!("&const Foo", "&readonly Foo", |p| p
        .eat_expression(Default::default()));
}

/// Const on pointers normalizes to readonly in type formatting.
#[test]
fn test_format_type_const_pointer_normalizes_to_readonly() {
    assert_format!(
        "type T = *const Foo",
        "*readonly Foo",
        |p| p.eat_type(&p.mark(), DeclarationDescriptor::default()),
        |tree: &NodeTree, expr_id| {
            let expression = tree.get(expr_id);
            let Expression::Declaration(declaration_id) = expression else {
                panic!("expected type declaration expression");
            };

            let declaration = tree.get(*declaration_id);
            let Declaration::Type { value, .. } = declaration else {
                panic!("expected type declaration");
            };

            *value
        },
        DestackFormatOptions::default()
    );
}

/// Type aliases preserve borrowed references.
#[test]
fn test_format_type_alias_preserves_borrowed_reference() {
    let (formatter, expression_id) = TestFormatter::parse("type Borrowed = &Buffer", |p| {
        p.eat_expression(Default::default())
    })
    .expect("parse type alias with borrowed reference");
    let Expression::Declaration(declaration_id) = formatter.tree.get(expression_id) else {
        panic!("expected type declaration expression");
    };
    let Declaration::Type { value, .. } = formatter.tree.get(*declaration_id) else {
        panic!("expected type declaration");
    };
    if !matches!(formatter.tree.get(*value), Expression::ReferenceOf { .. }) {
        panic!(
            "unexpected type alias value expression: {:?}",
            formatter.tree.get(*value)
        );
    }

    assert_format!(
        "type Borrowed = &Buffer",
        "type Borrowed = &Buffer;",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// Type aliases preserve readonly borrowed references.
#[test]
fn test_format_type_alias_preserves_readonly_borrowed_reference() {
    assert_format!(
        "type Borrowed = &readonly Buffer",
        "type Borrowed = &readonly Buffer;",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_member_call_chain_line() {
    assert_format!(
        "call().followed().by().many().calls()",
        "call().followed().by().many().calls()",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_tab_with_line_width(100)
    );
}

#[test]
fn test_format_member_call_chain_retains_breaks() {
    assert_format!(
        "call()\n\t.followed()\n\t.by()\n\t.many()\n\t.calls()",
        "call()\n\t.followed()\n\t.by()\n\t.many()\n\t.calls()",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_tab_with_line_width(20)
    );
}

#[test]
fn test_format_member_call_chain_breaks() {
    assert_format!(
        "call().followed().by().many().calls()",
        "call()\n\t.followed()\n\t.by()\n\t.many()\n\t.calls()",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_tab_with_line_width(20)
    );
}

#[test]
fn test_format_member_call_chain_breaks_with_maybe_and_index() {
    assert_format!(
        "call().followed()?.by()[0]?.many()?.calls()",
        "call()\n\t.followed()\n\t?.by()[0]\n\t?.many()\n\t?.calls()",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_tab_with_line_width(20)
    );
}

#[test]
fn test_format_path_member_call_chain_breaks() {
    assert_format!(
        "long.base.path.followed().by().many().calls()",
        "long.base\n\t.path.followed()\n\t.by()\n\t.many()\n\t.calls()",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_tab_with_line_width(20)
    );
}

#[test]
fn test_format_index_member_chain_breaks() {
    assert_format!(
        "identifier1.identifier2.identifier3[indexA].identifier4[indexB]?.[indexC][indexD]",
        "identifier1\n\t.identifier2\n\t.identifier3[indexA]\n\t.identifier4[indexB]\n\t?.[indexC][indexD]",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_tab_with_line_width(20)
    );
}

/// Member chains break before long boundary comments when optional calls follow.
#[test]
fn test_format_member_chain_breaks_before_long_boundary_comment_with_optional_call() {
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format!(
        "this.getParameters /* xxxxxxxxxxxxxxxxxxxxxxxxxxxx */\n  ?.();",
        "this\n  .getParameters /* xxxxxxxxxxxxxxxxxxxxxxxxxxxx */\n  ?.()",
        |p| p.eat_expression(Default::default()),
        options
    );
}

/// Inline optional call seams keep short boundary star comments on the same line.
#[test]
fn test_format_optional_call_keeps_inline_boundary_star_comment() {
    assert_format!(
        "alert /* comment */?.(\"value\")",
        "alert /* comment */?.(\"value\")",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(80)
    );
}

/// Optional call comments between `?.` and `(` should move before the optional operator.
#[test]
fn test_format_optional_call_operator_parenthesis_comment_moves_before_optional_operator() {
    let source = "call?./* 210 */()\ncall\n?./* 211 */()\n";
    let expected = "call /* 210 */?.();\ncall /* 211 */?.();\n";
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(4);
    assert_format_program_roundtrip_with_file_type(source, expected, FileType::JavaScript, options);
}

/// JS/TSX parse mode should normalize optional-call seam comments the same way.
#[test]
fn test_format_optional_call_operator_parenthesis_comment_moves_before_optional_operator_jsx_mode()
{
    let source = "call/* 209 */?.()\ncall?./* 210 */()\ncall\n?./* 211 */()\ncall? /* 212 */.()\n";
    let expected =
        "call /* 209 */?.();\ncall /* 210 */?.();\ncall /* 211 */?.();\ncall /* 212 */?.();\n";
    let formatter_options = FormatterOptions::default()
        .with_indent_width(2)
        .with_line_width(80)
        .with_quote_style(QuoteStyle::Double);
    let options = DestackFormatOptions::from_formatter_options(
        formatter_options,
        LanguageType::JavaScriptXml,
    );
    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScriptXml,
        options,
    );
}

/// Return statements should keep own-line callee comments before optional calls.
#[test]
fn test_format_return_optional_call_callee_line_comment_is_idempotent() {
    let source = r#"function x() {
  return func2
    //comment
    ?.bar();
}"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        javascript_fixture_format_options(),
    );
}

/// Call arguments should keep same-line head comments after `(`.
#[test]
fn test_format_call_argument_head_line_comment_is_idempotent() {
    let source = r#"call( // comment
  value
);"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        javascript_fixture_format_options(),
    );
}

/// New-expression arguments should keep same-line head comments after `(`.
#[test]
fn test_format_new_argument_head_line_comment_is_idempotent() {
    let source = r#"new Factory( // comment
  value
);"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        javascript_fixture_format_options(),
    );
}

/// Parenthesized return values should keep own-line callee comments before optional calls.
#[test]
fn test_format_return_parenthesized_optional_call_callee_line_comment_is_idempotent() {
    let source = r#"function f() {
  return (
    foo
      // comment
      ?.bar()
  );
}"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        javascript_fixture_format_options(),
    );
}

/// No-argument optional-call fixture should stay idempotent in JS/TSX mode.
#[test]
fn test_format_optional_call_no_argument_fixture_is_idempotent_jsx_mode() {
    let source = OPTIONAL_CALL_NO_ARGUMENT_SOURCE;
    let formatter_options = FormatterOptions::default()
        .with_indent_width(2)
        .with_line_width(80)
        .with_quote_style(QuoteStyle::Double);
    let options = DestackFormatOptions::from_formatter_options(
        formatter_options,
        LanguageType::JavaScriptXml,
    );
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScriptXml, options);
}

/// Optional chains keep short member tails with the optional call that introduces them.
#[test]
fn test_format_optional_chain_keeps_short_member_tail_with_optional_call() {
    assert_format!(
        "dataSource?.getClient()?.getUser(id)?.profile?.name",
        "dataSource\n    ?.getClient()\n    ?.getUser(id)?.profile?.name",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(35)
    );
}

/// Chains wrap instantiation prefixes before member tails.
#[test]
fn test_format_chain_wraps_instantiation_prefix_before_member_tail() {
    assert_format!(
        "api.getFactory<number>.name",
        "(api.getFactory<number>).name",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// Instantiation prefixes before index tails use relational spacing.
#[test]
fn test_format_chain_instantiation_before_index_uses_relational_spacing() {
    assert_format!(
        "providers[\"main\"]<Factory>[0]",
        "providers[\"main\"] < Factory > [0]",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// Parenthesized instantiation call wrappers can drop when they are redundant.
#[test]
fn test_format_parenthesized_instantiation_call_callee_wrapper_drops() {
    assert_format!(
        "(makeFactory<number>)(config)",
        "makeFactory<number>(config)",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// Single non-interpolated template literal snapshot calls break at the member hop.
#[test]
fn test_format_chain_call_breaks_before_template_literal_snapshot_member() {
    assert_format!(
        "expect(genCode(createVNodeCall(null, \"`div`\", mockProps))).toMatchInlineSnapshot(`\n  `)",
        "expect(genCode(createVNodeCall(null, \"`div`\", mockProps)))\n    .toMatchInlineSnapshot(`\n  `)",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// Chain layout keeps call-like argument wrappers compact while the inner chain breaks.
#[test]
fn test_format_chain_layout_promotes_head_in_call_like_argument() {
    assert_format!(
        "render(foo.bar.getResource(id).map(transform).finalize())",
        "render(foo.bar.getResource(id)\n    .map(transform)\n    .finalize())",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(30)
    );
}

/// Chain layout breaks at `=` when a long rhs chain cannot fit inline.
#[test]
fn test_format_chain_layout_respects_assignment_rhs_width() {
    assert_format!(
        "veryLongBindingName = source.alpha.beta.gamma().delta().epsilon()",
        "veryLongBindingName =\n    source.alpha.beta.gamma().delta().epsilon()",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(40)
    );
}

#[test]
fn test_format_expression_tree_literal_without_arguments() {
    assert_format!(
        "<Entity/>",
        "<Entity />",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_expression_tree_literal_with_arguments() {
    assert_format!(
        "<Entity a={1} b = {2} />",
        "<Entity a={1} b={2} />",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_expression_tree_literal_parenthesized() {
    let source = r"(
    <Entity a={1} b={2}>
        <Entity a={1} b={2} />
    </Entity>
)";
    let expected = r"(
    <Entity a={1} b={2}>
        <Entity a={1} b={2} />
    </Entity>
)";
    assert_format!(
        source,
        expected,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_expression_tree_literal_nested() {
    let source = r#"<A x={4} y={4}>
    <B x="hey">
        <C>
            <D />
            2
        </C>
    </B>
</A>"#;
    let expected = r#"(
    <A x={4} y={4}>
        <B x="hey">
            <C>
                <D />
                2
            </C>
        </B>
    </A>
)"#;
    assert_format!(
        source,
        expected,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_expression_tree_literal_with_array_of_struct_element() {
    // multiline array attributes break the element
    let source = r#"<Menu
    items={[
        { to: "/posts" },
        { to: "/posts/$postId", params: { postId: "postId" } },
    ]}
/>"#;
    let expected = r#"(
    <Menu
        items={[
            { to: "/posts" },
            { to: "/posts/$postId", params: { postId: "postId" } },
        ]}
    />
)"#;
    assert_format!(
        source,
        expected,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_expression_call_with_struct_literal() {
    let source = r#"Destack.serve({
    fetch: function (req: Request) {
        return Response("Success!")
    },
    run: true,
})"#;
    assert_format!(
        source,
        r#"Destack.serve({
    fetch: function (req: Request) {
        return Response("Success!");
    },
    run: true,
})"#,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(40)
    );
}

#[test]
fn test_format_expression_let_call() {
    let input = r#"const ast = await parseAsync(text, {
    sourceFileName: "file",
    parserOpts: {
        plugins: ["typescript", "jsx"],
    },
    sourceType: "module",
    configFile: false,
    babelrc: false,
})"#;
    let expected = r#"const ast = await parseAsync(text, {
    sourceFileName: "file",
    parserOpts: {
        plugins: ["typescript", "jsx"],
    },
    sourceType: "module",
    configFile: false,
    babelrc: false,
})"#;
    assert_format!(
        input,
        expected,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(40)
    );
}

#[test]
fn test_format_call_single_lambda_argument_with_prefix_comment_breaks() {
    let source = "call(/* comment */\n  () => {\n    //\n  }\n)";
    let expected = "call(\n    /* comment */\n    () => {\n        //\n    },\n)";
    assert_format!(source, expected, |p| p.eat_expression(Default::default()));
}

#[test]
fn test_format_call_nested_arrow_boundary_comments() {
    let source = "call(\n  () /**/ => //\n    () /**/ => /**/\n      () /**/ => /**/ {\n        //\n      }\n)";
    let expected =
        "call(() /**/ =>\n    //\n    () /**/ =>\n    /**/\n    () /**/ => /**/ {\n    //\n})";
    assert_format!(source, expected, |p| p.eat_expression(Default::default()));
}

#[test]
fn test_format_chained_assignment() {
    assert_format!(
        "a = b = c = 1",
        "a = b = c = 1",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_chained_assignment_long() {
    assert_format!(
        "veryLongName = anotherLongName = thirdLongName = 42",
        "veryLongName = anotherLongName = thirdLongName = 42",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(30)
    );
}

#[test]
fn test_format_jsx_with_comment() {
    assert_format!(
        "<Container>{/* XOXO */}</Container>",
        "(\n    <Container>\n        {/* XOXO */}\n    </Container>\n)",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_jsx_conditional_child() {
    assert_format!(
        "<div>{loading && <Spinner />}</div>",
        "<div>{loading && <Spinner />}</div>",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_jsx_in_function_call() {
    assert_format!(
        "render(<App />)",
        "render(<App />)",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_deeply_nested_callbacks() {
    assert_format!(
        "fetch(url).then((res) => res.json()).then((data) => process(data))",
        "fetch(url)\n    .then((res) => res.json())\n    .then((data) => process(data))",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(40)
    );
}

#[test]
fn test_format_optional_chain_with_nullish() {
    assert_format!(
        r#"user?.profile?.name ?? "Anonymous""#,
        r#"user?.profile?.name ?? "Anonymous""#,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// Formats type conditionals with infer bindings.
#[test]
fn test_format_type_conditional_with_infer() {
    assert_format!(
        "type Result = T extends infer U ? U : never",
        "type Result = T extends infer U ? U : never;",
        |p| p.eat_expression(Default::default())
    );
}

/// Formats type conditionals with constrained infer bindings.
#[test]
fn test_format_type_conditional_with_constrained_infer() {
    assert_format!(
        "type Result = T extends infer U extends string ? U : never",
        "type Result = T extends infer U extends string ? U : never;",
        |p| p.eat_expression(Default::default())
    );
}

/// Formats nested conditional types with explicit parentheses.
#[test]
fn test_format_type_conditional_nested() {
    assert_format!(
        "type Result = T extends U ? (U extends V ? X : Y) : Z",
        "type Result = T extends U ? (U extends V ? X : Y) : Z;",
        |p| p.eat_expression(Default::default())
    );
}

/// Comments after conditional tests should keep trailing ownership across passes.
#[test]
fn test_format_type_conditional_test_trailing_line_comment_is_idempotent() {
    let source = r#"type A =
  B extends T // comment
    ? foo
    : bar;
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::TypeScript, options);
}

/// Comments after ternary tests should keep test-trailing ownership across passes.
#[test]
fn test_format_ternary_test_trailing_line_comment_is_idempotent() {
    let source = r#"var inspect = 4 === util.inspect.length // node <= 0.8.x
  ? (function (v, colors) {
      return util.inspect(v, void 0, void 0, colors);
    })
  : (
    // node > 0.8.x
    function (v, colors) {
      return util.inspect(v, { colors: colors });
    }
  );
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

/// Parenthesized ternary function branches with separator comments should be idempotent.
#[test]
fn test_format_ternary_parenthesized_function_branch_comments_are_idempotent() {
    let source = r#"var inspect = 4 === util.inspect.length
  ? (
  // node <= 0.8.x
  function (v, colors) {
    return util.inspect(v, void 0, void 0, colors);
  })
  : (// node > 0.8.x
  function (v, colors) {
    return util.inspect(v, { colors: colors });
  });
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

/// Repeated ternary function branches with separator comments should stay idempotent.
#[test]
fn test_format_repeated_ternary_parenthesized_function_branch_comments_are_idempotent() {
    let source = r#"var inspect = 4 === util.inspect.length
  ? (
  // node <= 0.8.x
  function (v, colors) {
    return util.inspect(v, void 0, void 0, colors);
  })
  : (// node > 0.8.x
  function (v, colors) {
    return util.inspect(v, { colors: colors });
  });

var inspect = 4 === util.inspect.length
  ? (
  // node <= 0.8.x
  function (v, colors) {
    return util.inspect(v, void 0, void 0, colors);
  })
  : (// node > 0.8.x
  function (v, colors) {
    return util.inspect(v, { colors: colors });
  });
"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        javascript_fixture_format_options(),
    );
}

/// Own-line comments before ternary question separators should stay idempotent.
#[test]
fn test_format_ternary_question_separator_own_line_comments_are_idempotent() {
    let source = r#"cond ?
  // comment
  <T>() => () => 1
  :
  // comment
  <T>() => () => 1;
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::TypeScript, options);
}

/// Formats type intersections with trailing operators in Destack.
#[test]
fn test_format_type_intersection_trailing_operator_destack() {
    assert_format!(
        "type Combined = HasName & HasAge & HasEmail",
        "type Combined = HasName &\n    HasAge &\n    HasEmail;",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(30)
    );
}

/// Formats mapped types with modifiers and key remaps.
#[test]
fn test_format_type_mapped_with_remap() {
    let source = r#"type Remap = { readonly [K in keyof T as `${K}`]-?: T[K] }"#;
    let expected = r#"type Remap = { readonly [K in keyof T as `${K}`]-?: T[K] };"#;
    assert_format!(source, expected, |p| p.eat_expression(Default::default()));
}

/// Formats mapped types with removal modifiers.
#[test]
fn test_format_type_mapped_with_removals() {
    let source = r#"type Mutable = { -readonly [K in keyof T]-?: T[K] }"#;
    let expected = r#"type Mutable = { -readonly [K in keyof T]-?: T[K] };"#;
    assert_format!(source, expected, |p| p.eat_expression(Default::default()));
}

/// Formats mapped types without modifiers.
#[test]
fn test_format_type_mapped_without_modifiers() {
    let source = r#"type Plain = { [K in keyof T]: T[K] }"#;
    let expected = r#"type Plain = { [K in keyof T]: T[K] };"#;
    assert_format!(source, expected, |p| p.eat_expression(Default::default()));
}

/// Formats mapped types with optional modifiers.
#[test]
fn test_format_type_mapped_with_optional() {
    let source = r#"type Optional = { [K in keyof T]?: T[K] }"#;
    let expected = r#"type Optional = { [K in keyof T]?: T[K] };"#;
    assert_format!(source, expected, |p| p.eat_expression(Default::default()));
}

/// Formats chained type index expressions.
#[test]
fn test_format_type_index() {
    let source = r#"type Value = T[K][P]"#;
    let expected = r#"type Value = T[K][P];"#;
    assert_format!(source, expected, |p| p.eat_expression(Default::default()));
}

/// Formats type template literals with single spans.
#[test]
fn test_format_type_template_literal() {
    let source = r#"type Key = `on${K}`"#;
    let expected = r#"type Key = `on${K}`;"#;
    assert_format!(source, expected, |p| p.eat_expression(Default::default()));
}

/// Formats type template literals with multiple spans.
#[test]
fn test_format_type_template_literal_multiple_spans() {
    let source = r#"type Key = `on${K}:${V}`"#;
    let expected = r#"type Key = `on${K}:${V}`;"#;
    assert_format!(source, expected, |p| p.eat_expression(Default::default()));
}

/// Normalizes template literal type unions to width-aware leading-pipe layout.
#[test]
fn test_format_type_template_literal_union_with_leading_pipe() {
    let source = r#"type T = `${
  | 'W'
  | 'I'
  | 'L'
  | 'L'
  | 'B'
  | 'R'
  | 'E'
    | 'A'
  | 'K'
}${'!' | '!!'}`"#;
    let expected = r#"type T = `${
    | 'W'
    | 'I'
    | 'L'
    | 'L'
    | 'B'
    | 'R'
    | 'E'
    | 'A'
    | 'K'}${'!' | "!!"}`;"#;
    assert_format!(source, expected, |p| p.eat_expression(Default::default()));
}

#[test]
fn test_template_literal_union_span_newline_signal() {
    let source = r#"type T = `${
  | 'W'
  | 'I'
  | 'L'
  | 'L'
  | 'B'
  | 'R'
  | 'E'
  | 'A'
  | 'K'
}${'!' | '!!'}`"#;

    let (formatter, expression_id) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
            p.eat_expression(Default::default())
        })
        .expect("parse template literal union");
    let context = context_from_formatter(&formatter);

    let Expression::Declaration(declaration_id) = formatter.tree.get(expression_id) else {
        panic!("expected declaration expression");
    };
    let Declaration::Type { value, .. } = formatter.tree.get(*declaration_id) else {
        panic!("expected type declaration");
    };
    let Expression::TypeTemplateLiteral { spans, .. } = formatter.tree.get(*value) else {
        panic!("expected type template literal");
    };

    assert_eq!(spans.len(), 2);
    assert!(
        context.node_has_newline(spans[0]),
        "first template span should keep multiline source signal"
    );
    assert!(
        !context.node_has_newline(spans[1]),
        "second template span should stay single-line in source signal"
    );
}

/// Normalizes grouped leading-union wrappers in destack syntax.
#[test]
fn test_format_type_union_preserves_grouped_leading_union_parentheses() {
    let source = "type C = | (| (| A | B))";
    let expected = "type C = ((A | B));";
    assert_format!(source, expected, |p| p.eat_expression(Default::default()));
}

/// Drops single-member leading union wrappers in parenthesized array element types.
#[test]
fn test_format_type_single_member_leading_union_parenthesized_array() {
    let source = "type Items = ( | number)[]";
    let expected = "type Items = number[];";
    assert_format!(source, expected, |p| p.eat_expression(Default::default()));
}

/// Formats type imports with qualifiers.
#[test]
fn test_format_type_import() {
    assert_format!(
        r#"type Imported = import("mod").Type"#,
        r#"type Imported = import("mod").Type;"#,
        |p| p.eat_expression(Default::default())
    );
}

/// Formats type imports without qualifiers.
#[test]
fn test_format_type_import_without_qualifier() {
    assert_format!(
        r#"type Imported = import("mod")"#,
        r#"type Imported = import("mod");"#,
        |p| p.eat_expression(Default::default())
    );
}

/// Keep import expression trailing comments at the argument boundary.
#[test]
fn test_format_import_expression_trailing_argument_comments() {
    let source = r#"
import(
  // comment1
  `../alias/${base}.js`
  // comment2
);

import(
  // comment1
  `../alias/${base}.js`,
  // comment2
  { with: { type: "json" }}
  // comment2
);
"#
    .trim_start();
    let expected = r#"
import(
    // comment1
    `../alias/${base}.js`
    // comment2
);

import(
    // comment1
    `../alias/${base}.js`,
    // comment2
    { with: { type: "json" } }
    // comment2
);
"#
    .trim_start();
    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Formats standalone infer expressions.
#[test]
fn test_format_type_infer_expression() {
    assert_format!("type Result = infer U", "type Result = infer U;", |p| p
        .eat_expression(Default::default()));
}

#[test]
fn test_format_async_arrow() {
    assert_format!(
        "async (event) => await processEvent(event)",
        "async (event) => await processEvent(event)",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_return_jsx_inline() {
    // short JSX returns stay inline
    assert_format!(
        "return <App />",
        "return <App />",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_return_jsx_multiline() {
    assert_format!(
        "return <App prop=\"value\" another=\"thing\" />",
        "return (\n    <App\n        prop=\"value\"\n        another=\"thing\"\n    />\n)",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(30)
    );
}

/// Return-adjacent leading comments should stay idempotent across binary, member, and tag forms.
#[test]
fn test_format_return_adjacent_leading_comments_are_idempotent() {
    let source = r#"function logical() {
  return (
    // Reason for 42
    42
  ) && 84;
}

function memberInside() {
  return (
    // Reason for a.b
    a.b
  ).c;
}

function memberInAndOutWithCalls() {
  return (
    // Reason for a
    aFunction.b()
  ).c.d();
}

function taggedTemplate() {
  return (
    // Reason for a
    a
  )`b`;
}
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

#[test]
fn test_format_nested_ternary() {
    assert_format!(
        "const x = isFirst ? firstValue : isSecond ? secondValue : defaultValue",
        "const x = isFirst\n    ? firstValue\n    : isSecond\n        ? secondValue\n        : defaultValue",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(50)
    );
}

/// Keeps call rhs attached to `=` while call arguments break internally.
#[test]
fn test_format_const_call_with_multiline_object_rhs() {
    assert_format!(
        "const server = createServer({ port: config.server.port, middleware: [corsMiddleware(config.server.cors), authMiddleware(), loggingMiddleware({ level: \"info\" })], routes: [userRouter] })",
        "const server = createServer({\n    port: config.server.port,\n    middleware: [\n        corsMiddleware(config.server.cors),\n        authMiddleware(),\n        loggingMiddleware({ level: \"info\" }),\n    ],\n    routes: [userRouter],\n})",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(70)
    );
}

/// Keeps chain rhs attached to `=` while chain operations break internally.
#[test]
fn test_format_export_const_chain_rhs_does_not_break_after_operator() {
    assert_format!(
        "export const userRouter =\n    Router\n    .create()\n    .get(\"/users\", async (ctx) => {\n        return Response.json(ctx);\n    })\n    .post(\"/users\", async (ctx) => {\n        return Response.created(ctx);\n    })",
        "export const userRouter = Router.create()\n    .get(\"/users\", async (ctx) => {\n        return Response.json(ctx);\n    })\n    .post(\"/users\", async (ctx) => {\n        return Response.created(ctx);\n    })",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default_with_line_width(80)
    );
}

/// Breaks generic call rhs values after `=` when operator seams are preferred.
#[test]
fn test_format_const_generic_call_rhs_breaks_after_operator() {
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format!(
        "const result = configurationService.getValue<Record<string, boolean>>(enalementSetting)",
        "const result =\n  configurationService.getValue<Record<string, boolean>>(enalementSetting)",
        |p| p.eat_expression(Default::default()),
        options
    );
}

/// Preserves break-after-operator layout for multiline generic call rhs values.
#[test]
fn test_format_const_generic_call_rhs_preserves_source_operator_break() {
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format!(
        "const result =\n  configurationService.getValue<Record<string, boolean>>(\n  enalementSetting\n)",
        "const result =\n  configurationService.getValue<Record<string, boolean>>(enalementSetting)",
        |p| p.eat_expression(Default::default()),
        options
    );
}

/// Keeps `=` inline for multiline object-like generic call rhs values.
#[test]
fn test_format_const_generic_call_with_multiline_type_argument_keeps_operator_inline() {
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format!(
        "const emitter = createGlobalEmitter<{\n  key: Extract<Event, { type: key }>\n}>()",
        "const emitter = createGlobalEmitter<{\n  key: Extract<Event, { type: key }>;\n}>()",
        |p| p.eat_expression(Default::default()),
        options
    );
}

#[test]
fn test_format_jsx_bracket_same_line_true() {
    let mut options = DestackFormatOptions::default_with_line_width(30);
    options.bracket_same_line = true;
    assert_format!(
        r#"<Button variant="primary" size="large" disabled />"#,
        "(\n    <Button\n        variant=\"primary\"\n        size=\"large\"\n        disabled />\n)",
        |p| p.eat_expression(Default::default()),
        options
    );
}

#[test]
fn test_format_jsx_bracket_same_line_false() {
    let mut options = DestackFormatOptions::default_with_line_width(30);
    options.bracket_same_line = false;
    assert_format!(
        r#"<Button variant="primary" size="large" disabled />"#,
        "(\n    <Button\n        variant=\"primary\"\n        size=\"large\"\n        disabled\n    />\n)",
        |p| p.eat_expression(Default::default()),
        options
    );
}

/// Prefix expressions inside postfix operators get parenthesized.
#[test]
fn test_format_await_inside_maybe_gets_parenthesized() {
    // this tests the case where we have Maybe { left: Await { expr } }
    // which should format as (await expr)? not await expr?
    assert_format!(
        "(await foo())?",
        "(await foo())?",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// Unary prefix inside maybe gets parenthesized.
#[test]
fn test_format_unary_inside_maybe_gets_parenthesized() {
    assert_format!(
        "(-x)?",
        "(-x)?",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_unary_await_expression_parenthesizes_operand() {
    assert_format!(
        "async () => !await foo()",
        "async () => !(await foo())",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// Postfix inside postfix doesn't need extra parentheses.
#[test]
fn test_format_postfix_inside_maybe_no_extra_parens() {
    assert_format!(
        "x.foo?",
        "x.foo?",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// Call expression inside maybe doesn't need parentheses.
#[test]
fn test_format_call_inside_maybe_no_parens() {
    assert_format!(
        "foo()?",
        "foo()?",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

/// Await? syntactic sugar stays as is.
#[test]
fn test_format_await_maybe_sugar() {
    assert_format!(
        "await? foo()",
        "await? foo()",
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_call_argument_function_parameter_line_comment_is_idempotent() {
    let source = "exportDefaultWhatever(function (\n  aaaaaaaaaaaString,  //\n  bbbbbbbbbbbString,\n  cccccccccccString,\n) {\n  return null;\n}, \"xyz\")";
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);

    let (first_test, first_expression) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
            p.eat_expression(Default::default())
        })
        .unwrap();
    let first_output = first_test.format(&first_expression, options.clone());

    let (second_test, second_expression) =
        TestFormatter::parse_with_file_type(&first_output, FileType::JavaScript, |p| {
            p.eat_expression(Default::default())
        })
        .unwrap();
    let second_output = second_test.format(&second_expression, options);

    assert_eq!(first_output, second_output);
}

#[test]
fn test_format_assignment_chain_with_member_call_is_idempotent() {
    let source = "bifornCringerMoshedPerplexSawder =\n  askTrovenaBeenaDependsRowans =\n  glimseGlyphsHazardNoopsTieTie =\n  x =\n  averredBathersBoxroomBuggyNurl =\n  anodyneCondosMal(sdsadsa,dasdas,asd(()=>sdf)).ateOverateRetinol =\n  annularCooeedSplicesWalksWayWay =\n    kochabCooieGameOnOboleUnweave;";
    let options = DestackFormatOptions::default_with_line_width(80);

    let (first_test, first_expression) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
            p.eat_expression(Default::default())
        })
        .unwrap();
    let first_output = first_test.format(&first_expression, options.clone());

    let (second_test, second_expression) =
        TestFormatter::parse_with_file_type(&first_output, FileType::JavaScript, |p| {
            p.eat_expression(Default::default())
        })
        .unwrap();
    let second_output = second_test.format(&second_expression, options);

    assert_eq!(first_output, second_output);
}

#[test]
fn test_format_assignment_chain_in_call_argument_is_idempotent() {
    let source = "call(\n  function() {\n    return 1;\n  },\n  askTrovenaBeenaDependsRowans = glimseGlyphsHazardNoopsTieTie = 200_000_000_000n\n)";
    let options = DestackFormatOptions::default_with_line_width(80);

    let (first_test, first_expression) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
            p.eat_expression(Default::default())
        })
        .unwrap();
    let first_output = first_test.format(&first_expression, options.clone());

    let (second_test, second_expression) =
        TestFormatter::parse_with_file_type(&first_output, FileType::JavaScript, |p| {
            p.eat_expression(Default::default())
        })
        .unwrap();
    let second_output = second_test.format(&second_expression, options);

    assert_eq!(first_output, second_output);
}

/// Inline block comments between binary operands and operators should keep stable spacing.
#[test]
fn test_format_binary_operator_inline_block_comment_spacing_is_idempotent() {
    let source = r#"{
a = b || /** Comment */
c;

a = b /** Comment */ ||
c;

a = b || /** TODO this is a very very very very long comment that makes it go > 80 columns */
c;

a = b /** TODO this is a very very very very long comment that makes it go > 80 columns */ ||
c;

a = b || /** TODO this is a very very very very long comment that makes it go > 80 columns */ c;

a = b && /** Comment */
c;

a = b /** Comment */ &&
c;

a = b && /** TODO this is a very very very very long comment that makes it go > 80 columns */
c;

a = b /** TODO this is a very very very very long comment that makes it go > 80 columns */ &&
c;

a = b && /** TODO this is a very very very very long comment that makes it go > 80 columns */ c;

a = b + /** Comment */
c;

a = b /** Comment */ +
c;

a = b + /** TODO this is a very very very very long comment that makes it go > 80 columns */
c;

a = b /** TODO this is a very very very very long comment that makes it go > 80 columns */ +
c;

a = b + /** TODO this is a very very very very long comment that makes it go > 80 columns */ c;
}"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);

    let (first_test, first_block) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScriptXml, |p| {
            p.eat_block(BlockContext::Expression)
        })
        .unwrap();
    let first_output = first_test.format(&first_block, options.clone());

    let (second_test, second_block) =
        TestFormatter::parse_with_file_type(&first_output, FileType::JavaScriptXml, |p| {
            p.eat_block(BlockContext::Expression)
        })
        .unwrap();
    let second_output = second_test.format(&second_block, options);
    let expected = r#"{
  a =
    b /** Comment */ || c;

  a =
    b /** Comment */ || c;

  a =
    b /** TODO this is a very very very very long comment that makes it go > 80 columns */ || c;

  a =
    b /** TODO this is a very very very very long comment that makes it go > 80 columns */ || c;

  a =
    b ||
    /** TODO this is a very very very very long comment that makes it go > 80 columns */ c;

  a =
    b /** Comment */ && c;

  a =
    b /** Comment */ && c;

  a =
    b /** TODO this is a very very very very long comment that makes it go > 80 columns */ && c;

  a =
    b /** TODO this is a very very very very long comment that makes it go > 80 columns */ && c;

  a =
    b &&
    /** TODO this is a very very very very long comment that makes it go > 80 columns */ c;

  a =
    b /** Comment */ + c;

  a =
    b /** Comment */ + c;

  a =
    b /** TODO this is a very very very very long comment that makes it go > 80 columns */ + c;

  a =
    b /** TODO this is a very very very very long comment that makes it go > 80 columns */ + c;

  a =
    b
    + /** TODO this is a very very very very long comment that makes it go > 80 columns */ c;
}"#;

    assert_eq!(first_output, expected);
    assert_eq!(
        first_output, second_output,
        "first output:\n{first_output}\n\nsecond output:\n{second_output}"
    );
}

#[test]
fn test_format_throw_parenthesized_sequence_with_comment_is_idempotent() {
    let source = r#"{
  function sequenceExpressionInside() {
    throw (
      // Reason for a
      a, b
    );
  }
}"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);

    let (first_test, first_block) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
            p.eat_block(BlockContext::Expression)
        })
        .unwrap();
    let first_output = first_test.format(&first_block, options.clone());

    assert!(
        first_output.contains("throw ("),
        "throw argument lost parentheses:\n{first_output}"
    );

    let (second_test, second_block) =
        TestFormatter::parse_with_file_type(&first_output, FileType::JavaScript, |p| {
            p.eat_block(BlockContext::Expression)
        })
        .unwrap();
    let second_output = second_test.format(&second_block, options);

    assert_format_output_eq(&first_output, &second_output);
}

#[test]
fn test_format_return_throw_parenthesized_sequence_with_inline_head_comment_is_idempotent() {
    let source = r#"function sequenceExpressionInside() {
  return ( // Reason for a
    a, b
  );
  throw ( // Reason for a
    a, b
  );
}"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

#[test]
fn test_format_return_parenthesized_sequence_with_own_line_comment_is_stable() {
    let source = r#"function sequenceExpressionInside() {
  return (
    // Reason for a
    (a, b)
  );
}"#;
    let expected = r#"function sequenceExpressionInside() {
  return (
    // Reason for a
    (a, b)
  );
}
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_roundtrip_with_file_type(source, expected, FileType::JavaScript, options);
}

#[test]
fn test_format_jsdoc_same_line_jsx_return_is_idempotent() {
    let source = r#"function multilineBlockSameLineJsx() {
  return (
    /**
     * JSX Same line
     */ <div></div>
  );
}"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScriptXml, options);
}

/// Satisfies seam comments on qualified rhs paths should become trailing expression comments.
#[test]
fn test_format_satisfies_seam_comment_keeps_qualified_type_argument_comment() {
    let source = "value satisfies // seam\nns.Record<A, B>";
    let expected = "value satisfies ns.Record<A, B> // seam";
    let (formatter, expression_id) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
            p.eat_expression(Default::default())
        })
        .expect("parse qualified satisfies seam source");

    let formatted = formatter.format(&expression_id, DestackFormatOptions::default());
    assert_eq!(formatted, expected);
}

/// Satisfies seam comments should remap into compact single-segment rhs type arguments.
#[test]
fn test_format_satisfies_seam_comment_keeps_single_segment_compact_remap() {
    let source = "value satisfies // seam\nRecord<A, B>";
    let expected = "value satisfies Record< // seam\n    A,\n    B\n>";
    let (formatter, expression_id) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
            p.eat_expression(Default::default())
        })
        .expect("parse single-segment satisfies seam source");

    let formatted = formatter.format(&expression_id, DestackFormatOptions::default());
    assert_eq!(formatted, expected);
}

/// Export seam comments should stay between export and declare declaration heads.
#[test]
fn test_format_export_seam_comment_keeps_declare_declaration_head() {
    let source = "export // seam\ndeclare function f(): void {}";
    let expected = "export // seam\ndeclare function f(): void {}";
    let (formatter, expression_id) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
            p.eat_expression(Default::default())
        })
        .expect("parse export declare seam source");

    let formatted = formatter.format(&expression_id, DestackFormatOptions::default());
    assert_eq!(formatted, expected);
}

/// Export seam comments should stay between export and async declaration heads.
#[test]
fn test_format_export_seam_comment_keeps_async_declaration_head() {
    let source = "export // seam\nasync function f() {}";
    let expected = "export // seam\nasync function f() {}";
    assert_format_roundtrip_with_file_type(
        source,
        expected,
        FileType::TypeScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

/// Semicolon-guard separator comments should keep stable blank-line boundaries.
#[test]
fn test_format_semicolon_guard_separator_comment_is_idempotent() {
    let source = r#"{
  /**
  @type {{
    bar: string[]
  }}
  */
  ({}).bar.forEach(doStuff);

  // 1

  /**
  @type {{
    bar: string[]
  }}
  */

  ({}).bar.forEach(doStuff);
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Directive seams before guarded parenthesized expressions should keep one blank line.
#[test]
fn test_format_semicolon_guard_directive_comment_keeps_single_blank_line() {
    let source = "{\n\"use strict\";\n\n// comment\n(() => {});\n}";
    let expected = "{\n    \"use strict\";\n\n    // comment\n    () => {};\n}";
    assert_format_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// If-statement semicolon-guard comments should keep blank separator lines stable.
#[test]
fn test_format_if_semicolon_guard_comment_blank_line_is_idempotent() {
    let source = r#"{
  if (1) foo

  // 11
  ;[]
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// If-else semicolon-guard comments should keep blank separator lines stable.
#[test]
fn test_format_if_else_semicolon_guard_comment_blank_line_is_idempotent() {
    let source = r#"{
  if (1) ; else foo

  // 11
  ;[]
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// For-statement semicolon-guard comments should keep blank separator lines stable.
#[test]
fn test_format_for_semicolon_guard_comment_blank_line_is_idempotent() {
    let source = r#"{
  for (;;) foo

  // 11
  ;[]
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// While-statement semicolon-guard comments should keep blank separator lines stable.
#[test]
fn test_format_while_semicolon_guard_comment_blank_line_is_idempotent() {
    let source = r#"{
  while (1) foo

  // 11
  ;[]
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Do-while semicolon-guard comments should keep blank separator lines stable.
#[test]
fn test_format_do_while_semicolon_guard_comment_blank_line_is_idempotent() {
    let source = r#"{
  do; while (1)

  // 11
  ;[]
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Unary-plus semicolon-guard comments should keep blank separator lines stable.
#[test]
fn test_format_unary_plus_semicolon_guard_comment_blank_line_is_idempotent() {
    let source = r#"{
  foo

  // 11
  ;+bar
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Unary-minus semicolon-guard comments should keep blank separator lines stable.
#[test]
fn test_format_unary_minus_semicolon_guard_comment_blank_line_is_idempotent() {
    let source = r#"{
  foo

  // 11
  ;-bar
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Inline block comments between closing delimiters and semicolons should stay on the left boundary.
#[test]
fn test_format_inline_block_comment_between_closing_paren_and_semicolon_is_idempotent() {
    let source = "!(() => 3) /* foo */;";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        javascript_fixture_format_options(),
    );
}

/// Declaration seams before guarded parenthesized calls should keep the expected output.
#[test]
fn test_format_declare_semicolon_guard_comment_parenthesized_call_output() {
    let source = "declare const PAGE_PATH: string\n  //<- THIS spaces\n;(()=>{})()\n";
    let expected = "declare const PAGE_PATH: string;\n  //<- THIS spaces\n(() => {})();\n";
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    let (formatter, roots) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse declaration semicolon guard source");
    let output = formatter.format(&statement_list(&roots), options);
    assert_format_output_eq(expected, output);
}

/// Block comments on semicolon-guard heads should keep one inline space before `(`.
#[test]
fn test_format_semicolon_guard_inline_block_comment_before_parenthesized_call_keeps_space() {
    let source = "const left = 1;\n/** @type {Number} */ (a + b)();\n";
    let expected = "const left = 1;\n/** @type {Number} */ (a + b)();\n";
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    let (formatter, roots) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse semicolon guard inline block comment source");
    let output = formatter.format(&statement_list(&roots), options);
    assert_format_output_eq(expected, output);
}

#[test]
fn test_format_file_header_comments_before_declaration_are_idempotent() {
    let source = r#"// TODO: upgrade parser
// class A {}

class C1 {
  get;
  x(){}
}
"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

#[test]
fn test_format_no_semi_do_while_blank_separator_is_idempotent() {
    let source = r#"do break; while (false)
if (true) do break; while (false)

if (true) 1; else 2
for (;;) ;
"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Long call-left binary expressions should keep one stable operator break layout.
#[test]
fn test_format_long_call_left_binary_expression_operator_break_is_idempotent() {
    let source = r#"const marker = true;

fooooooooooooooooooooooooooooooooooooooooooooooooooooooooo(aaaaaaaaaaaaaaaaaaa)
  + a;
"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        javascript_fixture_format_options(),
    );
}

/// Semicolon guard seams should remain stable with inline trivia before bracket heads.
#[test]
fn test_format_semicolon_guard_with_inline_comment_before_bracket_is_idempotent() {
    let source = r#"{
  foo

  // 11
  ; /* guard */ [bar]
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Semicolon guard seams should remain stable with inline trivia before unary-plus heads.
#[test]
#[ignore = "unresolved seam-order drift with inline block trivia before unary plus guard heads"]
fn test_format_semicolon_guard_with_inline_comment_before_unary_plus_is_idempotent() {
    let source = r#"{
  foo

  // 11
  ; /* guard */ +bar
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Return statement semicolon-guard comments should stay inside the function body.
#[test]
fn test_format_return_semicolon_guard_comments_stay_in_body() {
    let source = r#"function a() {
  return

  // 11
  ;[]

  return

  // 21
  ;foo

  // prettier-ignore
  return

  ;[]

  return /* comment */ ;
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

/// Control-head comments before empty statement bodies should stay idempotent.
#[test]
fn test_format_control_head_comment_before_empty_statement_is_idempotent() {
    let source = r#"{
  for(;;) // 34
  ;
  if(a) /* 35 */
  ;
  else /* 352 */
  ;
  while(a) /* 36 */
  ;
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Own-line control-head comments before empty statements should stay idempotent.
#[test]
fn test_format_control_head_own_line_comment_before_empty_statement_is_idempotent() {
    let source = r#"{
  do
  // 21
  ; while (1)

  if (a)
  // 25
  ;

  with (a)
  // 27
  ;
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Condition-tail line comments before empty `if` bodies should stay idempotent.
#[test]
fn test_format_if_condition_tail_line_comment_before_empty_body_is_idempotent() {
    let source = r#"if (Boolean(
  node.type === "ImportExpression" ||
  node.type === "TSImportType" ||
  node.type === "TSExternalModuleReference") // comment
);"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        javascript_fixture_format_options(),
    );
}

/// Call-head line comments in `if` conditions should stay idempotent.
#[test]
fn test_format_if_condition_call_head_line_comment_is_idempotent() {
    let source = r#"if (Boolean // comment
(
  node.type === "ImportExpression" ||
  node.type === "TSImportType" ||
  node.type === "TSExternalModuleReference"));"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        javascript_fixture_format_options(),
    );
}

/// Nested unary condition comments should stay idempotent.
#[test]
fn test_format_if_condition_nested_unary_comment_is_idempotent() {
    let source = r#"if (!(
  // comment
  !(node.type === "ImportExpression" ||
    node.type === "TSImportType" ||
    node.type === "TSExternalModuleReference")
)); // comment"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        javascript_fixture_format_options(),
    );
}

/// Unary-head line comments in `if` conditions should stay idempotent.
#[test]
fn test_format_if_condition_unary_head_line_comment_is_idempotent() {
    let source = r#"if (! // comment
!(
  node.type === "ImportExpression" ||
  node.type === "TSImportType" ||
  node.type === "TSExternalModuleReference"
)); // comment"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        javascript_fixture_format_options(),
    );
}

/// Mixed unary condition comment heads should stay idempotent.
#[test]
fn test_format_if_condition_mixed_unary_comment_heads_are_idempotent() {
    let source = r#"if ( // mixed-unary-a
!!(
  node.type === "ImportExpression" ||
  node.type === "TSImportType" ||
  node.type === "TSExternalModuleReference"));

if (! // mixed-unary-b
!(
  node.type === "ImportExpression" ||
  node.type === "TSImportType" ||
  node.type === "TSExternalModuleReference"));

if (!! // mixed-unary-c
(
  node.type === "ImportExpression" ||
  node.type === "TSImportType" ||
  node.type === "TSExternalModuleReference"));"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        javascript_fixture_format_options(),
    );
}

/// Labels with empty statement bodies and comments should stay idempotent.
#[test]
fn test_format_label_empty_statement_comments_are_idempotent() {
    let source = r#"a: /* comment */;
a: ;/* comment */
a /* comment */:;"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        javascript_fixture_format_options(),
    );
}

/// Control-head comments before non-block statement bodies should stay idempotent.
#[test]
fn test_format_control_head_comment_before_non_block_body_is_idempotent() {
    let source = r#"{
  for(;;) // 34
  foo();
  if(a) /* 35 */
  foo();
  else /* 352 */
  foo();
  while(a) /* 36 */
  foo();
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Inline head comments before update-expression statement bodies should stay idempotent.
#[test]
fn test_format_control_head_inline_star_comment_before_update_body_is_idempotent() {
    let source = r#"{
  if(true) /* 7 */ ++x;
  while(true) /* 7 */ ++x;
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// If-else block comments between heads and bodies should stay idempotent.
#[test]
fn test_format_if_else_head_body_block_comment_pair_is_idempotent() {
    let source = r#"{
  if(a) /* */ /*
  65 */
  {}
  else /* */ /*
  652 */
  {}
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// If heads with trailing condition comments should stay idempotent.
#[test]
fn test_format_if_head_condition_line_comments_are_idempotent() {
    let source = r#"{
  if(
    true
    // 1
  ) {}

  if(
    true // 5
    && true // 52
  ) {}
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Inline head comments before empty statements should stay idempotent.
#[test]
fn test_format_control_head_inline_star_comment_before_empty_statement_is_idempotent() {
    let source = r#"{
  if(a) /* 45 */
  ;
  while(a) /* 46 */
  ;
  with(a) /* 47 */
  ;
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Mixed control-head star comments before empty statements should stay idempotent.
#[test]
fn test_format_control_head_mixed_star_comments_before_empty_statement_are_idempotent() {
    let source = r#"{
  do /* 41 */
  ; while(1)
  for(a in b) /* 42 */
  ;
  for(a of b) /* 43 */
  ;
  if(a) /* 45 */
  ;
  else /* 452 */
  ;
  while(a) /* 46 */
  ;
  with(a) /* 47 */
  ;

  do /*
  51 */
  ; while(1)
  for(a in b) /*
  52 */
  ;
  for(a of b) /*
  53 */
  ;
  if(a) /*
  55 */
  ;
  else /*
  552 */
  ;
  while(a) /*
  56 */
  ;
  with(a) /*
  57 */
  ;
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Inline star comments before statement semicolons should stay idempotent.
#[test]
fn test_format_inline_star_comment_before_semicolon_statement_is_idempotent() {
    let source = r#"{
  var a = {}/* dangling */;
  var b = []/* dangling */;
  array = []/* array */;
  object = {}/* object */;
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Empty object and array literals should keep dangling comments inside delimiters.
#[test]
fn test_format_empty_container_dangling_comments_stay_inside_delimiters() {
    let source = r#"{
  var a = {/* dangling */};
  var b = {
    // dangling
  };
  var c = [/* dangling */];
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Program-leading comments and top-level blank seams should not introduce extra empty lines.
#[test]
fn test_format_program_leading_comments_and_top_level_blank_seam_stay_stable() {
    let source = r#"// Fixture-derived coverage for ignore directives and top-level seams.
function a() {
  const first = 1;

  // first seam
  const second = 2;

  // second seam
  const third = 3;
}

const response = {
  // oxfmt-ignore
  "_text": "Turn on the lights",
  intent: "lights",
};
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_roundtrip_with_file_type(source, source, FileType::JavaScript, options);
}

/// Blank lines before member-chain seam comments should remain attached and stable.
#[test]
fn test_format_member_chain_blank_line_before_seam_comment_is_stable() {
    let source = r#"Promise.all(writeIconFiles)
  // TO DO -- END
  .then(() => writeRegistry());

Promise.all(writeIconFiles)

  // TO DO -- END
  .then(() => writeRegistry());

Promise.all(writeIconFiles)
  // TO DO -- END

  .then(() => writeRegistry());
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_roundtrip_with_file_type(source, source, FileType::JavaScript, options);
}

/// Yield-chain own-line seam comments should stay indented on the chain continuation.
#[test]
fn test_format_yield_chain_blank_line_comment_keeps_continuation_indent() {
    let source = "function *a() {\n  yield task\n    // No extra parens\n    .run();\n}\n";
    let expected = "function* a() {\n  yield task\n    // No extra parens\n    .run();\n}\n";
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    let (formatter, roots) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse yield chain seam comment source");
    let output = formatter.format(&statement_list(&roots), options);
    assert_format_output_eq(expected, output);
}

/// Member-chain seam block comments should keep continuation shape across passes.
#[test]
fn test_format_member_chain_own_line_block_comment_keeps_continuation_shape() {
    let source = r#"_.a(a)
  /* very very very very very very very long such that it is longer than 80 columns */
  .a();
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_roundtrip_with_file_type(source, source, FileType::JavaScript, options);
}

/// Member-chain own-line ignore comments should stay on the member continuation.
#[test]
fn test_format_member_chain_ignore_comment_stays_on_member_continuation() {
    let source = r#"verylongidentifierthatwillwrap123123123123123(
  a.b
    // prettier-ignore
    // Some other comment here
    .c
);
"#;
    let expected = r#"verylongidentifierthatwillwrap123123123123123(
  a.b
    // prettier-ignore
    // Some other comment here
    .c,
);
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_roundtrip_with_file_type(source, expected, FileType::JavaScript, options);
}

/// Chain head block comments should not steal following member-chain seam comments across passes.
#[test]
fn test_format_member_chain_head_block_comment_then_flowfix_seam_is_idempotent() {
    let source = r#"_.a(a)
  /* very very very very very very very long such that it is longer than 80 columns */
  .a()

Something
  // $FlowFixMe(>=0.41.0)
  .getInstance(this.props.dao)
  .getters()
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

/// Parser should keep call argument member chains intact across ignore-comment seams.
#[test]
fn test_parse_member_chain_ignore_comment_inside_call_argument_is_call_expression() {
    let source = r#"verylongidentifierthatwillwrap123123123123123(
  a.b
    // prettier-ignore
    // Some other comment here
    .c
);
"#;
    let (formatter, roots) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
            .expect("parse member-chain ignore comment source");

    assert_eq!(roots.len(), 1);

    let statement_id = roots[0];
    let Expression::Statement(expression_id) = formatter.tree.get(statement_id) else {
        panic!("expected statement expression root");
    };

    let Expression::Call {
        left,
        dynamic_arguments,
        ..
    } = formatter.tree.get(*expression_id)
    else {
        panic!("expected call expression root");
    };
    assert!(!dynamic_arguments.is_empty());

    let first_argument = formatter.tree.get(dynamic_arguments[0]);
    let Argument::Positional { value, .. } = first_argument else {
        panic!("expected positional call argument");
    };
    assert!(
        matches!(
            formatter.tree.get(*value),
            Expression::Path { .. }
                | Expression::Member { .. }
                | Expression::PrivateMember { .. }
                | Expression::Index { .. }
        ),
        "expected member-like first argument, got {:?}",
        formatter.tree.get(*value)
    );
    assert!(
        matches!(
            formatter.tree.get(*left),
            Expression::Path { .. } | Expression::Member { .. } | Expression::PrivateMember { .. }
        ),
        "expected call callee path-like expression, got {:?}",
        formatter.tree.get(*left)
    );

    let context = context_from_formatter(&formatter);
    let has_argument_ignore_range =
        any_ignore_range_for_nodes(&context, dynamic_arguments, context.comment_tokens());
    assert!(
        !has_argument_ignore_range,
        "expected call argument list to avoid ignore-range short-circuit"
    );
}

/// Blank seams before `.` should collapse to one stable member-chain expression.
#[test]
fn test_format_member_chain_blank_seam_without_comment_collapses() {
    let source = "{\n  value\n\n  .prop;\n}";
    let expected = "{\n    value.prop;\n}";
    assert_format_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Call argument separator line comments should keep comma-before-comment ownership.
#[test]
fn test_format_call_argument_separator_line_comment_keeps_comma_before_comment() {
    let source = r#"call(
    () => {
        // ...
    }, //
    "good"
);
"#;
    let expected = r#"call(
    () => {
        // ...
    }, //
    "good",
);
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(4);
    assert_format_program_roundtrip_with_file_type(source, expected, FileType::JavaScript, options);
}

/// Separator comments after comma should not force ternary argument reflow in call lists.
#[test]
fn test_format_call_argument_inline_separator_comment_preserves_ternary_shape() {
    let source = r#"cb(
  overflowing ? "absolute top-0" : "relative", // sidebar custom changes - to contain the absolute sidebar below
  parameter,
);

cb(
  overflowing ? "absolute top-0" : "relative" /* */, // sidebar custom changes - to contain the absolute sidebar below
  parameter,
);
"#;
    let expected = source;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_roundtrip_with_file_type(source, expected, FileType::JavaScript, options);
}

/// End-of-line comments between call callees and `(` should format to call-tail comments.
#[test]
fn test_format_call_callee_head_line_comment_moves_to_call_tail() {
    let source = "call // C3\n()";
    let expected = "call(); // C3\n";
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(4);
    assert_format_program_roundtrip_with_file_type(source, expected, FileType::JavaScript, options);
}

/// Empty export clauses should keep `{}` when a line seam comment follows `export`.
#[test]
fn test_format_export_seam_comment_keeps_empty_export_clause() {
    let source = "export //comment\n{}";
    let expected = "//comment\nexport {}";
    let (formatter, expression_id) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
            p.eat_expression(Default::default())
        })
        .expect("parse export empty clause line seam source");
    let formatted = formatter.format(&expression_id, DestackFormatOptions::default());
    assert_format_output_eq(expected, &formatted);
}

/// Empty export clauses should keep `{}` when an inline block seam comment follows `export`.
#[test]
fn test_format_export_block_seam_comment_keeps_empty_export_clause() {
    let source = "export /* comment */ {}";
    let expected = "export {} /* comment */";
    let (formatter, expression_id) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
            p.eat_expression(Default::default())
        })
        .expect("parse export empty clause block seam source");
    let formatted = formatter.format(&expression_id, DestackFormatOptions::default());
    assert_format_output_eq(expected, &formatted);
}

/// Empty typed destructuring patterns should keep interior comments inside delimiters.
#[test]
fn test_format_typed_empty_destructuring_pattern_keeps_interior_comments() {
    let source = "const {\n  // bar\n  // baz\n}: Foo = expr";
    let expected = "const {\n    // bar\n    // baz\n}: Foo =\n    expr";
    assert_format_roundtrip_with_file_type(
        source,
        expected,
        FileType::TypeScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

/// Empty lambda parameter lists should keep delimiter-interior comments inside `()`.
#[test]
fn test_format_empty_lambda_parameter_list_keeps_interior_comment() {
    let source = "const arrow = (\n/* function parameter */\n) => {}";
    let expected = "const arrow = (\n    /* function parameter */\n) => {}";
    assert_format_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

/// Empty method parameter lists should keep delimiter-interior comments inside `()`.
#[test]
fn test_format_empty_method_parameter_list_keeps_interior_comment() {
    let source = "const object = {\n  method(\n    /* object method parameter */\n  ) {},\n}";
    let expected =
        "const object = {\n    method(\n        /* object method parameter */\n    ) {},\n}";
    assert_format_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

/// Method rest parameters should never keep a trailing comma.
#[test]
fn test_format_method_rest_parameter_drops_trailing_comma() {
    let source = "({ method(...args) {} })";
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

/// Static override modifiers should keep parser-valid TypeScript order.
#[test]
fn test_format_typescript_static_override_modifier_order_is_idempotent() {
    let source = "class A extends B { static override foo: string }";
    assert_format_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

/// Empty explicit block statements in switch cases should be roundtrip-stable.
#[test]
fn test_format_switch_empty_block_case_is_idempotent() {
    let source = "switch (x) {\n  case x: {\n  }\n\n  case y: {\n  }\n}";
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

/// Trailing switch-head line comments should stay on the following case label seam.
#[test]
fn test_format_switch_head_comment_before_default_is_idempotent() {
    let source = "switch (1) { // comment1\n  default:\n}";
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_expression(Default::default()),
        DestackFormatOptions::default(),
    );
}

/// Switch default label line comments should stay parse-stable across statements.
#[test]
fn test_format_switch_default_label_line_comment_seams_are_idempotent() {
    let source = r#"
switch(1){default: // comment1
}

switch(2){default: // comment2
//comment2a
}
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Switch default label comments before explicit blocks should remain idempotent.
#[test]
fn test_format_switch_default_label_comments_before_blocks_are_idempotent() {
    let source = r#"
switch(x) {
  default: // comment
    {break;}
}

switch(x) {
  default: /* comment */
    {break;}
}

switch(x) {
  default:
    /* comment */ {
    break;}
}

switch(x) {
  default: /* comment */ {
    break;}
}
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Switch label comments after one blank line should keep the `case ...:` seam parse-stable.
#[test]
fn test_format_switch_label_blank_comment_seam_is_idempotent() {
    let source = "{\n  switch (true) {\n    case true:\n\n    // Good luck getting here\n    case false:\n  }\n}";
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Switch explicit block cases with trailing empty statements should collapse in one pass.
#[test]
fn test_format_switch_case_explicit_block_with_trailing_empty_statement_roundtrip() {
    let source = r#"
switch (error.code) {
  case ConfigurationEditingErrorCode.ERROR_INVALID_CONFIGURATION: {
    nls.localize("errorInvalidConfiguration", "Unable to write into settings. Correct errors/warnings in the file and try again.");
  };
}
"#
    .trim_start();
    let expected = r#"
switch (error.code) {
    case ConfigurationEditingErrorCode.ERROR_INVALID_CONFIGURATION: {
        nls.localize(
            "errorInvalidConfiguration",
            "Unable to write into settings. Correct errors/warnings in the file and try again.",
        );
    }
}
"#
    .trim_start();
    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Ignored labelled continue seams should remain parse-stable across formatter passes.
#[test]
fn test_format_ignore_labelled_continue_semicolon_guard_is_idempotent() {
    let source = "{\n  lbl: for (;;) {\n    if (condition) {\n      // prettier-ignore\n      continue                   lbl\n\n      // breaking comment\n      ;(possibleArray || []).sort()\n    }\n  }\n}";
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Multiline comments between `as`/`satisfies` and rhs types should stay on rhs seams.
#[test]
fn test_format_typescript_as_satisfies_multiline_seam_comments_are_idempotent() {
    let source = "{\n1 as\n/*comment*/\nFoo;\n1 satisfies\n/*comment*/\nFoo;\n}";
    assert_format_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Own-line comments after `as` should not collapse `a as` into one identifier.
#[test]
fn test_format_typescript_as_own_line_comment_preserves_operator_spacing() {
    let source = "functionArg = a as\n  // comment\n  TSESTree.ArrowFunctionExpression | TSESTree.ArrowFunctionExpression | TSESTree.FunctionExpression | undefined;\n";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Cast chains with trailing seam comments should stay parse-safe and idempotent.
#[test]
fn test_format_typescript_as_comment_chain_fixture_17407_is_idempotent() {
    let source = r#"
function getClassNameFromPrototypeMethod(container) {
  return ((container // a
    .left as PropertyAccessExpression) // b
    .expression as PropertyAccessExpression) // c
    .expression; // d
}
"#
    .trim_start();
    let options = DestackFormatOptions::default()
        .with_indent_width(2)
        .with_line_width(80);
    assert_format_program_idempotent_with_file_type(source, FileType::TypeScript, options);
}

/// Long parenthesized `as` unions should keep one stable rhs layout across passes.
#[test]
fn test_format_typescript_as_parenthesized_union_rhs_is_idempotent() {
    let source = r#"
const value1 = thisIsAReallyReallyReallyReallyReallyLongIdentifier as SomeInterface;
const value2 = thisIsAnIdentifier as thisIsAReallyReallyReallyReallyReallyReallyReallyReallyReallyReallyReallyLongInterface;
const value3 = thisIsAReallyLongIdentifier as (SomeInterface | SomeOtherInterface);
"#
    .trim_start();
    let options = DestackFormatOptions::default()
        .with_indent_width(2)
        .with_line_width(80);
    assert_format_program_idempotent_with_file_type(source, FileType::TypeScript, options);
}

/// TypeScript `as/as.ts` should stay idempotent for cast rhs layout.
#[test]
fn test_format_typescript_as_fixture_is_idempotent() {
    let source = TYPESCRIPT_AS_SOURCE;
    let options = DestackFormatOptions::default()
        .with_indent_width(2)
        .with_line_width(80);
    assert_format_program_idempotent_with_file_type(source, FileType::TypeScript, options);
}

/// Union property comments should not move semicolon ownership across passes.
#[test]
fn test_format_typescript_union_property_comments_keep_semicolon_position() {
    let source = "type T = {\n  prop:\n    // comment\n    | T1\n    // comment\n    | T2\n    // comment\n}\n";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Union prefix comments in parenthesized type annotations should stay idempotent.
#[test]
fn test_format_typescript_union_parenthesized_prefix_comments_are_idempotent() {
    let source = "let aa2: /*1*/ | /*2*/ C | /*3*/ D;\nlet aa3: /*1*/ | /*2*/ C | /*3*/ D /*4*/;\n";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Parenthesized constructor-function unions should remain stable across formatter passes.
#[test]
fn test_format_typescript_union_parenthesized_constructor_member_is_idempotent() {
    let source = "type Ctor = (new () => X) | Y;\n";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Type-indexed mapped types should keep stable inline layout after `=`.
#[test]
fn test_format_typescript_type_indexed_mapped_type_is_idempotent() {
    let source = "type NonFunctionPropertyNames<T> = { [K in keyof T]: T[K] extends Function ? never : K }[keyof T];\n";
    let options = DestackFormatOptions::default()
        .with_indent_width(2)
        .with_line_width(80);
    assert_format_program_idempotent_with_file_type(source, FileType::TypeScript, options);
}

/// Function-type union members should keep one outer grouping wrapper.
#[test]
fn test_format_typescript_union_function_member_keeps_single_outer_wrapper() {
    let source = "type T = number | ((arg: any) => void);\n";
    let expected = "type T = number | (arg: any) => void;\n";
    let options = DestackFormatOptions::default();
    let (formatter, roots) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse function-type union member source");
    let output = formatter.format(&statement_list(&roots), options);
    assert_format_output_eq(expected, output);

    assert_format_program_idempotent_with_file_type(
        expected,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Leading union comments after `=` should keep one stable placement across passes.
#[test]
fn test_format_typescript_union_head_comment_after_equals_is_idempotent() {
    let source = "type Aa1 = /*1*/ | /*2*/ C | D;\n";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Adjacent inline block comments in type unions should keep stable spacing across passes.
#[test]
fn test_format_typescript_union_adjacent_inline_block_comments_are_idempotent() {
    let source = r#"
type B1 = a /* 1 */ /* 2 */ | b;
type B2 = a /* 1 */ | /* 2 */ b;
type B3 = a | /* 1 */ /* 2 */ b;
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Leading-pipe union doc comments should keep stable owner placement across passes.
#[test]
fn test_format_typescript_union_leading_pipe_doc_comments_are_idempotent() {
    let source = r#"
type A1 =     | /**
     * octahedralRhinocerosTransformer
     */
    a
    | (b | c);

type A2 =     | /**
     * hippopotamicKangarooMutator
     */
    (a | b)
    | c;
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Leading-pipe union mixed comment seams should stay idempotent.
#[test]
fn test_format_typescript_union_leading_pipe_mixed_comment_seams_are_idempotent() {
    let source = r#"
// TODO[@fisker]: comments not attached correctly after first element
type A1 =
  | /**
   * 11
   */
  a
  | b

type A2 =
  | /**
   * 21
   */ a
  | b

type A3 =
  | // 31
  a
  |
  b;
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Parenthesized intersection seams around union operands should remain stable.
#[test]
fn test_format_typescript_union_parenthesized_intersection_comment_seams_are_idempotent() {
    let source = r#"
type A1 =
  // prettier-ignore
  (A | B)
  & (
    // prettier-ignore
    A | B
  )
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Deeply nested single-type union wrappers with comments should stay idempotent.
#[test]
fn test_format_typescript_union_single_type_nested_comments_are_idempotent() {
    let source = r#"
type A1 =
  | (
    | (
      | (
          | A
          // A comment to force break
          | B
        )
    )
  );
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Single-member leading-pipe wrappers with line comments should keep union members on rerun.
#[test]
fn test_format_typescript_union_single_type_line_comment_preserves_second_member() {
    let source = r#"
type A6 = /*1*/
  | A
  // A comment to force break
  | B;
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Parser output for leading-pipe unions should preserve both arms before formatting.
#[test]
fn test_parse_typescript_union_single_type_line_comment_keeps_binary_arms() {
    let source = "type A6 = /*1*/\n| A\n// A comment to force break\n| B;";
    let (formatter, roots) =
        TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
            .expect("parse union source");

    assert_eq!(roots.len(), 1);

    let declaration_expression_id = match formatter.tree.get(roots[0]) {
        Expression::Statement(expression_id) => *expression_id,
        _ => roots[0],
    };

    let Expression::Declaration(declaration_id) = formatter.tree.get(declaration_expression_id)
    else {
        panic!("expected type declaration expression");
    };
    let Declaration::Type { value, .. } = formatter.tree.get(*declaration_id) else {
        panic!("expected type declaration node");
    };

    let Expression::Binary {
        operator,
        left,
        right,
    } = formatter.tree.get(*value)
    else {
        panic!("expected binary union type value");
    };
    assert_eq!(*operator, BinaryOperator::ElementwiseOr);

    let context = context_from_formatter(&formatter);
    let flattened = flatten_type_binary_expression(&context, *value, *operator);
    assert_eq!(flattened.len(), 2);

    let Expression::Path { path, .. } = formatter.tree.get(*left) else {
        panic!("expected left union arm path");
    };
    assert_eq!(path.segments.len(), 1);
    assert_eq!(formatter.strings.get(path.segments[0]), "A");

    let Expression::Path { path, .. } = formatter.tree.get(*right) else {
        panic!("expected right union arm path");
    };
    assert_eq!(path.segments.len(), 1);
    assert_eq!(formatter.strings.get(path.segments[0]), "B");
}

/// Prettier fixture 18379 should stay idempotent for union-intersection seams.
#[test]
fn test_format_typescript_union_fixture_18379_is_idempotent() {
    let source = r#"
type A1 =
  (
    A | B // comment 1
  ) & (
    // comment2
    A | B
  )

type A2 =
  (
    A | B // prettier-ignore
  ) & (
    // prettier-ignore
    A | B
  )

type A1 =
  // comment 1
  (A | B)
  & (
    // comment2
    A | B
  )

type A1 =
  // prettier-ignore
  (A | B)
  & (
    // prettier-ignore
    A | B
  )
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Prettier single-type nested union fixture should keep line comments stable.
#[test]
fn test_format_typescript_union_single_type_fixture_is_idempotent() {
    let source = r#"
type A1 =
  | (
    | (
      | (
          | A
          // A comment to force break
          | B
        )
    )
  );
type A2 =
  | (
    | (
          | A
          // A comment to force break
          | B
        )
    | (
          | A
          // A comment to force break
          | B
        )
  );
type A3 =
  | ( | (
          | A
          // A comment to force break
          | B
        ) );
type A4 =
  | ( | ( | (
          | A
          // A comment to force break
          | B
        ) ) );
type A5 =
  | (
    | (
      | { key: string }
      | { key: string }
      | { key: string }
      | { key: string }
    )
    | { key: string }
    | { key: string }
  );
type A6 = | (
  /*1*/ | (
    | (
          | A
          // A comment to force break
          | B
        )
  )
  );

type B1 =
  | (
    & (
      (
          | A
          // A comment to force break
          | B
        )
    )
  );
type B2 =
  | (
    & (
      | (
        & (
          (
          | A
          // A comment to force break
          | B
        )
        )
      )
    )
  );
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Type intersections with assignment seam comments should keep stable value ownership.
#[test]
fn test_format_typescript_union_consistent_with_flow_comment_fixture_is_idempotent() {
    let source = r#"
type A3 = // dir, exp, arg, modifiers
  & [string]
  & [string, ExpressionNode]
  & [string, ExpressionNode, ExpressionNode]
  & [string, ExpressionNode, ExpressionNode, ObjectExpression]
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Leading-pipe type aliases with seam comments after equals should stay idempotent.
#[test]
fn test_format_typescript_union_consistent_with_flow_single_type_fixture_is_idempotent() {
    let source = r#"
type A6 = /*1*/
| (
  | (
    | (
          | A
          // A comment to force break
          | B
        )
  )
  );
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Parenthesized tuple union entries should keep one stable multiline layout.
#[test]
fn test_format_typescript_union_consistent_with_flow_within_tuple_fixture_is_idempotent() {
    let source = r#"
type D = [
  (AAAAAAAAAAAAAAAAAAAAAA | BBBBBBBBBBBBBBBBBBBBBB | CCCCCCCCCCCCCCCCCCCCCC | DDDDDDDDDDDDDDDDDDDDDD),
  (AAAAAAAAAAAAAAAAAAAAAA | BBBBBBBBBBBBBBBBBBBBBB | CCCCCCCCCCCCCCCCCCCCCC | DDDDDDDDDDDDDDDDDDDDDD)
]
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Multiline `as const` postfix comments should keep trailing semicolons after the comment.
#[test]
fn test_format_typescript_as_const_multiline_postfix_comment_keeps_semicolon_position() {
    let source = "{\n1 as /*\ncomment\n*/const;\n}";
    let expected = "{\n    1 as const /*\n    comment\n    */;\n}";
    assert_format_roundtrip_with_file_type(
        source,
        expected,
        FileType::TypeScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Comments after `=>` should stay with arrow bodies, not drift into signatures.
#[test]
fn test_format_typescript_arrow_body_multiline_comments_are_idempotent() {
    let source = "{\nconst fn3 = (): any => /*\nMultiple line\n*/\nnull;\nconst fn8 = () => /*\nMultiple line\n*/\nnull;\n}";
    assert_format_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Blank lines before assignment-following own-line comments should remain stable.
#[test]
fn test_format_assignment_followup_comment_blank_line_is_idempotent() {
    let source = "{\nsomething.veeeeeery.looooooooooooooooooooooooooong = some.other.rather.long.chain;\n\n// does not work if it ends with a function call\nsomething.veeeeeery.looooooooooooooooooooooooooong = some.other.rather.long.chain.functionCall();\n}";
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Own-line assignment seam JSDoc comments should keep rhs break shape after `=`.
#[test]
fn test_format_assignment_own_line_jsdoc_comment_breaks_after_operator() {
    let source = "{\n  sourcemap =\n  /** @type {'inline' | 'hidden' | 'sourcemap'} */ (\n      process.env.WORKER_MODE\n    ) || sourcemap;\n}\n";
    let expected = "{\n  sourcemap =\n    /** @type {'inline' | 'hidden' | 'sourcemap'} */ (\n      process.env.WORKER_MODE\n    ) || sourcemap;\n}\n";
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_roundtrip_with_file_type(source, expected, FileType::JavaScript, options);
}

/// Assignment targets with inline type-cast comments should not force rhs operator breaks.
#[test]
fn test_format_assignment_target_typecast_rest_comment_keeps_inline_rhs() {
    let source = "{\n[a, /** @type {string[]} */ ...rest3] = arr;\n}";
    let expected = "{\n    [a, /** @type {string[]} */ ...rest3] = arr;\n}";
    assert_format_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Declarator assignment seam line comments should lead complex rhs values after `=`.
#[test]
fn test_format_declarator_assignment_seam_line_comment_leads_complex_rhs() {
    let source = "{\nlet obj2 = // Comment\n{\n  key: \"val\"\n};\n\nlet obj6 = // Comment\n[\n  \"val\"\n];\n}";
    let expected = "{\n    let obj2 =\n        // Comment\n        {\n            key: \"val\",\n        };\n\n    let obj6 =\n        // Comment\n        [\"val\"];\n}";
    assert_format_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Declarator seam line comments before array rhs values should stay idempotent.
#[test]
fn test_format_declarator_assignment_array_rhs_seam_comment_is_idempotent() {
    let source = r#"{
    let obj6 =
        // Comment
        ["val"];
}"#;
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// JSX spread child line comments should not drift between `...` and the operand.
#[test]
fn test_format_jsx_spread_child_line_comments_are_idempotent() {
    let source = "{\n<div>{\n  //comment\n  ...a\n}</div>;\n\n<div>{//comment\n  ...a// comment\n}</div>;\n}";
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScriptXml,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// JSX empty expression containers with line comments should stay parseable and idempotent.
#[test]
fn test_format_jsx_empty_expression_line_comments_are_idempotent() {
    let source =
        "{\n<div>{// single line comment\n}</div>;\n\n<div>{// first\n// second\n}</div>;\n}";
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScriptXml,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// JSX member and index receivers should keep stable parenthesized postfix formatting.
#[test]
fn test_format_jsx_member_expression_receivers_are_idempotent() {
    let source = r#"
(<div>
  <a>foo</a>
</div>).method();
(<div>
  <a>foo</a>
</div>).property;
(<div>
  <a>foo</a>
</div>)["computed"]();
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScriptXml,
        DestackFormatOptions::default(),
    );
}

/// Array blank-line seams should stay before the next element, not before the next comma.
#[test]
fn test_format_array_blank_line_before_element_is_idempotent() {
    let source = "{\nconst values = [\n  1,\n\n  2,\n\n  // comment\n  3,\n];\n}";
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        options,
    );
}

/// Top-level array assignment seams should keep stable blank lines across semicolon insertion.
#[test]
fn test_format_top_level_array_assignment_blank_lines_are_idempotent() {
    let source = "{\na = [\n  1,\n]\n\nb = [\n  2,\n]\n}";
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        options,
    );
}

/// Program-level array assignment seams should keep stable blank lines across passes.
#[test]
fn test_format_program_top_level_array_assignment_blank_lines_are_idempotent() {
    let source = "a = [\n\n  1,\n  2,\n\n  3,\n]\n\nb = [\n  4,\n  5,\n]\n";
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

/// Arrow seams with inline comments should stay idempotent at program level.
#[test]
fn test_format_program_arrow_comment_before_arrow_is_idempotent() {
    let source = "a = () /* before arrow */ =>\nnull;\na = () => /* after arrow */\nnull;\na = (/* in parentheses */) =>\nnull;\n";
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

/// JSDoc comments after `=>` should stay attached to nested arrow bodies across passes.
#[test]
fn test_format_program_nested_arrow_jsdoc_after_arrow_is_idempotent() {
    let source = r#"const createIdFilter =
  (id) =>
    /** @param {any} s */
    (s) =>
      /** @param {string} id */
      s.id === id;
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

/// Nested arrow postfix block comments should remain local to their owning arrow segment.
#[test]
fn test_format_program_nested_arrow_postfix_block_comments_are_idempotent() {
    let source = r#"f((a) => ((b) => ((c) => (1 ? 2 : 3)/* b */ /* c */)) /* a */);
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

/// Semicolon-guard comments after loop bodies should stay idempotent.
#[test]
fn test_format_no_semi_for_statement_guard_comment_is_idempotent() {
    let source = "{\nfor (;;) foo\n\n// 11\n;[]\n\nfor (;;) foo\n\n// 21\n;foo\n}";
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        options,
    );
}

/// ASI guard comments before bracket starts should stay idempotent.
#[test]
fn test_format_no_semi_asi_guard_comment_is_idempotent() {
    let source = "{\n  let foo\n\n  // comment\n  ;[foo] = [1]\n}";
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2),
    );
}

/// No-semi guard comments after declarations should stay stable in program context.
#[test]
fn test_format_no_semi_program_guard_comment_after_declaration_is_idempotent() {
    let source = "let error = new Error(response.statusText);\n// comment\n[].response = response\n\nx;\n\n{\n  let foo\n\n  // comment\n  ;[foo] = [1]\n}\n";
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

/// No-semi fixture comments before control statements should stay before the control statement.
#[test]
fn test_format_no_semi_comment_before_if_statement_stays_outside_body() {
    let source = r#"
class X {} [1, 2, 3].forEach(fn)

// don't semicolon if it doesn't start statement

if (true) (() => {})()
"#
    .trim_start();
    let expected = r#"
class X {}
[1, 2, 3].forEach(fn);

// don't semicolon if it doesn't start statement

if (true) (() => {})();
"#
    .trim_start();
    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Operator-leading no-semi expressions should not oscillate across formatting passes.
#[test]
fn test_format_no_semi_operator_leading_binary_statements_are_idempotent() {
    let source =
        "1\n- 1\n\n1\n+ 1\n\n1\n/ 1\n\narr\n[0]\n\nfn\n(x)\n\n!1\n\n1\n< 1\n\ntag\n`string`\n";
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

/// Semicolon-terminated trailing block comments should stay on the left statement before ignore directives.
#[test]
fn test_format_no_semi_trailing_block_comment_before_ignore_is_idempotent() {
    let source = "{\nfor (a of b) foo; /* comment */\n\n// prettier-ignore\nfor (   a of   b) while   (   1)   foo (   )\n}";
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Semicolon-terminated trailing block comments at end of file should stay idempotent.
#[test]
fn test_format_no_semi_trailing_block_comment_at_eof_is_idempotent() {
    let source = "for (a of b) foo; /* comment */";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// For-of no-semi seams with trailing block comments should stay stable across full fixture flow.
#[test]
fn test_format_no_semi_for_of_fixture_slice_is_idempotent() {
    let source = "for (a of b) foo\n\n// 11\n;[]\n\nfor (a of b) foo\n\n// 21\n;foo\n\n// prettier-ignore\nfor (   a of   b)   foo (   )\n\n;[]\n\nfor (a of b) foo /* comment */ ;\n\n// prettier-ignore\nfor (   a of   b) while   (   1)   foo (   )\n\n;[]\n";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Export trailing block comments after semicolons should stay idempotent.
#[test]
fn test_format_export_trailing_block_comment_after_semicolon_is_idempotent() {
    let source = "export {}; /* comment */";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Mixed export comment seams should keep trailing block comment spacing stable.
#[test]
fn test_format_export_mixed_comment_seams_are_idempotent() {
    let source = "export //comment\n{}\n\nexport /* comment */ {};\n";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Export postfix block comments without source spacing should stay stable.
#[test]
fn test_format_export_postfix_block_comment_without_space_is_idempotent() {
    let source = "export {};\n\nexport {};/* comment */\n";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Prettier export comment fixture shape should stay idempotent end to end.
#[test]
fn test_format_export_comment_fixture_shape_is_idempotent() {
    let source = "export //comment\n{}\n\nexport /* comment */ {};\n\nconst foo = ''\nexport {\n  foo // comment\n}\n\nconst bar = ''\nexport {\n  // comment\n  bar\n}\n\nconst fooo = ''\nconst barr = ''\nexport {\n  fooo, // comment\n  barr, // comment\n}\n\nconst foooo = ''\nconst barrr = ''\nexport {\n  foooo,\n  barrr as  // comment\n\t\t baz,\n} from 'foo'\n\nconst fooooo = ''\nconst barrrr = ''\nexport {\n  fooooo,\n  barrrr as  // comment\n\t\t bazz,\n}\n";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Export alias comments after `as` should format as own-line specifier comments.
#[test]
fn test_format_export_alias_line_comment_after_as_has_expected_layout() {
    let source = "const foooo = ''\nconst barrr = ''\nexport {\n  foooo,\n  barrr as  // comment\n\t\t baz,\n} from 'foo'\n";
    let expected = "const foooo = \"\";\nconst barrr = \"\";\nexport {\n    foooo,\n    // comment\n    barrr as baz,\n} from \"foo\";\n";
    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Export and import alias separator blank lines before line comments should be idempotent.
#[test]
fn test_format_export_import_alias_blank_line_comment_seams_are_idempotent() {
    let source = r#"
const foo = ''
const bar = ''
export {
  foo,

  bar as // export-marker
  baz,
}

const alpha = ''
const beta = ''
import {
  alpha,

  beta as // import-marker
  gamma,
} from 'pkg'
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// For-loop arrays with boundary comments should keep trailing commas before own-line comments.
#[test]
fn test_format_for_loop_array_boundary_comment_keeps_trailing_comma_position() {
    let source = r#"
for (let i in [
  // comment1
  1, 2, 3
  // comment2
]);

for (let i of [
  // comment1
  1, 2, 3
  // comment2
]);
"#
    .trim_start();
    let expected = r#"
for (let i in [
    // comment1
    1, 2, 3,
    // comment2
]);

for (let i of [
    // comment1
    1, 2, 3,
    // comment2
]);
"#
    .trim_start();
    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Type tuple declaration seams should not gain unstable extra blank lines across passes.
#[test]
fn test_format_type_tuple_declaration_blank_seam_is_idempotent() {
    let source = r#"
type Foo5 = [
    /* comment1 */
];

type Foo6 = [
    /* comment1 */

    /* comment2 */
];
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Type union grouping around commented operands should stay idempotent.
#[test]
fn test_format_type_union_grouped_operand_comment_is_idempotent() {
    let source = r#"
export type a =
  // foo
  | foo1&foo2
  // bar
  | bar1&bar2
  // prettier-ignore
  | qux1&qux2;

export type b =
  // foo
  | foo1&foo2
  // bar
  | bar1&bar2
  // prettier-ignore
  | qux1&qux2
  // baz
  | baz1&baz2;
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Pattern tails with own-line comments should not accumulate extra commas across passes.
#[test]
fn test_format_typescript_pattern_tail_comments_do_not_accumulate_commas() {
    let source = r#"
function method({
  foo1,
  // bar = "bar",
  foo2
  // bazz = "bazz",
}: Foo) {}

function method([
  foo,
  // bar = "bar",
  foo2
  // bazz = "bazz",
]: Foo) {}
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::TypeScript,
        DestackFormatOptions::default(),
    );
}

/// Semicolon-terminated trailing line comments should keep left ownership before ignore directives.
#[test]
fn test_format_no_semi_trailing_line_comment_before_ignore_is_idempotent() {
    let source = "{\nfoo(); // 1\n// 2\n// prettier-ignore\nbar   (   )\n}";
    assert_format_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        |p| p.eat_block(BlockContext::Expression),
        DestackFormatOptions::default(),
    );
}

/// Binary operator seam comments before rhs operands should keep semicolon-tail ownership.
#[test]
fn test_format_binary_operator_seam_comment_before_rhs_is_idempotent() {
    let source = r#"
a = b + // Comment
c;

a = b + // TODO this is a very very very very long comment that makes it go > 80 columns
c;
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Blank seams before `else` comments should remain stable across passes.
#[test]
fn test_format_blank_seam_before_else_comment_is_idempotent() {
    let source = "if (a) {\n  foo();\n}\n\n// before else\nelse {\n  bar();\n}\n";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Single-argument call source newlines should not force unstable expanded call lists.
#[test]
fn test_format_single_argument_call_with_source_newline_is_idempotent() {
    let source = r#"
const run = (value) => {
    call(
        chain(value)
            .next()
            .done()
    );
};
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Single-argument closure type-cast callsites should stay stable across wrapper normalization.
#[test]
fn test_format_single_argument_closure_typecast_comment_call_is_idempotent() {
    let source = r#"
var newArray = test(/** @type {array} */ (numberOrString.map(x => x)));
var newArray = test(/** @type {array} */ ((numberOrString).map(x => x)));
"#
    .trim_start();
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScriptXml, options);
}

/// JSX callback head line comments should be preserved across formatting passes.
#[test]
fn test_format_jsx_callback_head_line_comment_is_preserved_and_idempotent() {
    let source = r#"
KEYPAD_NUMBERS.map(num => ( // Buttons 0-9
  <div />
));
"#
    .trim_start();
    let options = DestackFormatOptions::default();

    let (first_formatter, first_roots) =
        TestFormatter::parse_with_file_type(source, FileType::JavaScriptXml, |p| Ok(p.parse()))
            .expect("parse first-pass source");
    let first_context = context_from_formatter(&first_formatter);
    let marker_annotation_id = first_context
        .formatter_annotation_entries
        .iter()
        .enumerate()
        .find_map(|(entry_index, _)| {
            let annotation_id = LocalNodeId::<Annotation>::new(entry_index as u32);
            let Annotation::Comment { node, .. } = first_context.annotation(annotation_id) else {
                return None;
            };

            (first_context.comment_text(node).trim() == "Buttons 0-9").then_some(annotation_id)
        });
    let marker_annotation_id = marker_annotation_id
        .expect("expected formatter annotation for callback head line comment marker");
    let marker_owner_node = first_context
        .formatter_annotation_ids_by_node_id
        .iter()
        .enumerate()
        .find_map(|(node_index, annotation_ids)| {
            annotation_ids
                .iter()
                .any(|candidate| candidate.id == marker_annotation_id.id)
                .then_some(node_index)
        })
        .expect("expected owner node for callback head line comment marker");
    let marker_owner_node_type = first_context.tree.get_node_type(marker_owner_node as u32);
    let marker_position = first_context.annotation(marker_annotation_id).position();

    let first_output = first_formatter.format(&statement_list(&first_roots), options.clone());
    assert!(
        first_output.contains("Buttons 0-9"),
        "formatted output should keep callback head line comments: position={marker_position:?} owner={marker_owner_node_type:?} id={marker_owner_node}\n{first_output}"
    );

    let (second_formatter, second_roots) =
        TestFormatter::parse_with_file_type(&first_output, FileType::JavaScriptXml, |p| {
            Ok(p.parse())
        })
        .expect("parse second-pass source");
    let second_output = second_formatter.format(&statement_list(&second_roots), options);
    assert_format_output_eq(&first_output, &second_output);
}

/// JSX spread object literals with inner ignore comments should keep the comment inside the value.
#[test]
fn test_format_jsx_spread_object_ignore_comment_stays_inside_value() {
    let source = "a = <div {...{/* prettier-ignore */}}/>;\n";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScriptXml,
        DestackFormatOptions::default(),
    );
}

/// Closure-cast member wrappers should keep stable grouping across passes.
#[test]
fn test_format_closure_cast_member_wrapper_is_idempotent() {
    let source = r#"
var newArray = /** @type {array} */ (numberOrString).map((x) => x);
var newArray = /** @type {array} */ ((numberOrString)).map((x) => x);
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScriptXml,
        DestackFormatOptions::default(),
    );
}

/// Statement-end chain seams with comments should keep blank ownership stable.
#[test]
fn test_format_blank_seam_before_chain_comment_is_idempotent() {
    let source = "{\n  value\n\n  // chain seam\n  .prop();\n}\n";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Semicolon-guard array seams should keep blank ownership stable before comments.
#[test]
fn test_format_blank_seam_before_semicolon_guard_array_comment_is_idempotent() {
    let source = "{\n  if (a)\n    foo();\n\n  // guard\n  ;[x] = y;\n}\n";
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Logical conditions with line comments should keep separators on the next line.
#[test]
fn test_format_logical_condition_line_comments_are_idempotent() {
    let source = r#"
if (
  true // 5
  && true // 52
) {}

while (
  true // 5
  && true // 52
) {}
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Inline else-head line comments should stay attached to the else body seam.
#[test]
fn test_format_else_head_line_comment_is_idempotent() {
    let source = r#"
if (a) // 15
  foo();
else // 152
  foo();
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Multiline block and jsdoc comments in jsx expression containers should be idempotent.
#[test]
fn test_format_jsx_expression_multiline_comment_alignment_is_idempotent() {
    let source = r#"
<div>
  {a/* comment
*/
  }
</div>;

<div>
  {/**
   * JSDoc-y comment in JSX. I wonder what will happen to it?
  */ {}}
</div>;

<div>
  {
    /**
   * Another JSDoc comment in JSX.
  */
  }
</div>;
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScriptXml,
        DestackFormatOptions::default(),
    );
}

/// Empty statement and arithmetic comment seams should remain stable across passes.
#[test]
fn test_format_empty_statement_comment_seams_are_idempotent() {
    let source = r#"
a; /* a */ // b
; /* c */

foo; // first
;// second
;// third

function x() {
} // first
; // second

a = (
  b // 1
  + // 2
  c // 3
  + // 4
  d // 5
  + /* 6 */
  e // 7
);
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// Optional call seams with line comments should keep continuation syntax valid.
#[test]
fn test_format_optional_call_line_comment_seam_is_idempotent() {
    let source = r#"
render?.( // Warm any cache
  <ChildUpdates renderAnchor={true} anchorClassOn={true} />,
  container
);
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScriptXml,
        DestackFormatOptions::default(),
    );
}

/// Switch case labels with trailing line comments should keep `:` on the label line.
#[test]
fn test_format_switch_case_label_line_comment_is_idempotent() {
    let source = r#"
switch (foo) {
  case "bar": //comment
    doThing(); //comment

  case "baz":
    doOtherThing(); //comment
}
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

/// JSX text wrapping should keep spacing stable across formatter passes.
#[test]
fn test_format_jsx_text_wrap_spacing_is_idempotent() {
    let source = r#"
x =
  <div>
    before{stuff}after{stuff}after{stuff}after{stuff}after{stuff}after{stuff}{stuff}{stuff}after{stuff}after
  </div>;

single_expression_child_tags =
  <div>
    You currently have <strong>{dashboardStr}</strong> and <strong>{userStr}</strong>
  </div>;

convert_space_expressions =
  <div>{" "}</div>;
"#
    .trim_start();
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScriptXml,
        DestackFormatOptions::default(),
    );
}

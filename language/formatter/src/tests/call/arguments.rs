use crate::{
    DestackFormatOptions, TestFormatter, assert_format,
    assert_format_program_idempotent_with_file_type,
    assert_format_program_roundtrip_with_file_type,
};
use destack_source::FileType;

#[test]
fn test_format_argument_named() {
    assert_format!(
        "x: 1",
        "x: 1",
        |p| p.eat_tree_argument(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_argument_named_shorthand() {
    assert_format!(
        "x",
        "x",
        |p| p.eat_tree_argument(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_format_argument_positional() {
    assert_format!(
        "1",
        "1",
        |p| p.eat_tree_argument(),
        DestackFormatOptions::default()
    );
}

#[test]
fn test_multi_argument_call_virtual_separator_comment_formats_with_comma_before_comment() {
    let source = "call(
  value,
  other // marker
)";
    let expected = "call(
    value,
    other, // marker
);
";
    assert_format_program_roundtrip_with_file_type(
        source,
        expected,
        FileType::JavaScript,
        DestackFormatOptions::default(),
    );
}

#[test]
fn test_format_react_hook_separator_comment_cluster_is_idempotent() {
    let source = r#"useEffect(
  () => {
    console.log("some code", props.foo);
  }

  ,
  // We need to disable the eslint warning here,
  // because of some complicated reason.
  // eslint-disable line react-hooks/exhaustive-deps
  []
)"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScriptXml, options);
}

#[test]
fn test_format_trailing_collection_last_argument_expansion_is_idempotent() {
    let source =
        r#"func(one, two, three, four, five, six, seven, eig, is, this, too, long, yes, []);"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

/// Leading block callbacks with simple `this.#x` tails should stay inline.
#[test]
fn test_format_leading_block_callback_with_private_field_tail_stays_inline() {
    let source = r#"class Foo {
  #t: NodeJS.Timeout | undefined = undefined;
  #v: number;

  constructor(v: number) {
    this.#v = v;
  }

  start() {
    this.#t = setInterval(() => {
      console.log();
    }, this.#v);
  }
}
"#;
    let expected = source;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_roundtrip_with_file_type(source, expected, FileType::TypeScript, options);
}

/// Leading block callbacks with trailing collection tails should break.
#[test]
fn test_format_leading_block_callback_with_object_tail_breaks() {
    let source = r#"call(
  () => {
    return foo;
  },
  { a: 1, b: 2, c: 3 },
);
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_roundtrip_with_file_type(source, source, FileType::TypeScript, options);
}

/// Test-like calls with callback tails should keep grouped multiline layout.
#[test]
fn test_format_test_like_call_with_callback_tail_breaks() {
    let source = r#"test(
  code.replace((c) => ""),
  () => {},
);
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_roundtrip_with_file_type(source, source, FileType::JavaScript, options);
}

#[test]
fn test_format_trailing_collection_object_with_inner_comment_is_idempotent() {
    let source = r#"func(one, two, three, four, five, six, seven, eig, is, this, too, long, yes, {
  // Comments
});"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

#[test]
fn test_format_trailing_collection_last_argument_expansion_sequence_is_idempotent() {
    let source = r#"func(one, two, three, four, five, six, seven, eig, is, this, too, long, no, []);
func(one, two, three, four, five, six, seven, eig, is, this, too, long, yes, []);
func(one, two, three, four, five, six, seven, eig, is, this, too, long, yes, {
  // Comments
});"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

/// Single chain-valued arguments inside member chains should not force unstable expansion.
#[test]
fn test_format_member_chain_single_chain_argument_is_idempotent() {
    let source = r#"const sel = this.connections

  .concat(this.activities.concat(this.operators))
  .filter(x => x.selected);
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

/// Preserve-line call-argument clusters should keep statement spacing stable across passes.
#[test]
fn test_format_preserve_line_argument_cluster_spacing_is_idempotent() {
    let source = r#"differentArgTypes(

  () => {
    return true
  },

  isTrue ?
    doSomething() : 12,

);
moreArgTypes(

  [1, 2,
    3],

  {
    name: 'Hello World',
    age: 29
  },

  doSomething(

    // Hello world


    // Hello world again
    { name: 'Hello World', age: 34 },


    oneThing
      + anotherThing,

    // Comment

  ),

);"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

#[test]
fn test_format_preserve_line_argument_list_fixture_slice_is_idempotent() {
    let source = r#"comments(
  // Comment

  /* Some comments */
  short,
  /* Another comment */

  short2, // Even more comments
);

evenMoreArgTypes(
  doSomething(
    { name: "Hello World", age: 34 },

    true,
  ),

  14,
);
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

#[test]
fn test_preserve_line_even_more_and_apply_slice_is_idempotent() {
    let source = r#"evenMoreArgTypes(
  doSomething(
    { name: "Hello World", age: 34 },

    true,
  ),

  14,

  1 + 2 - 90 / 80,

  !98 * 60 - 90,
);

foo.apply(
  null,

  // Array here
  [1, 2],
);
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
}

#[test]
fn test_preserve_line_argument_list_jsx_mode_slice_is_idempotent() {
    let source = r#"comments(
  // Comment

  /* Some comments */
  short,
  /* Another comment */

  short2, // Even more comments

  /* Another comment */

  // Long Long Long Long Long Comment

  /* Long Long Long Long Long Comment */
  // Long Long Long Long Long Comment

  short3,
  // More comments
);

differentArgTypes(
  () => {
    return true;
  },

  isTrue ? doSomething() : 12,
);

moreArgTypes(
  [1, 2, 3],

  {
    name: "Hello World",
    age: 29,
  },

  doSomething(
    // Hello world

    // Hello world again
    { name: "Hello World", age: 34 },

    oneThing + anotherThing,

    // Comment
  ),
);

evenMoreArgTypes(
  doSomething(
    { name: "Hello World", age: 34 },

    true,
  ),

  14,

  1 + 2 - 90 / 80,

  !98 * 60 - 90,
);

foo.apply(
  null,

  // Array here
  [1, 2],
);

bar.on(
  "readable",

  () => {
    doStuff();
  },
);
"#;
    let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
    assert_format_program_idempotent_with_file_type(source, FileType::JavaScriptXml, options);
}

#[test]
fn test_format_optional_chain_single_boolean_argument_is_idempotent() {
    let source = r#"a = Boolean(
  a_long_long_long_long_condition || a_long_long_long_long_condition || a_long_long_long_long_condition,
)?.toString();"#;
    assert_format_program_idempotent_with_file_type(
        source,
        FileType::JavaScriptXml,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2),
    );
}

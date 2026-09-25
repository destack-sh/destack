use crate::{TsppFormatOptions, assert_format_program, assert_format_program_reference_widths};
use tspp_source::FileType;

/// Block callbacks with short cast tails should follow the grouped-last layout.
#[test]
fn test_format_grouped_last_argument_layout_with_short_cast_tail() {
    assert_format_program_reference_widths(
        r#"const x = [].reduce(() => {
  return "y";
}, {} as SomeType<OtherType>);
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"const x = [].reduce(() => {
  return "y";
}, {} as SomeType<OtherType>);
"#,
            ),
            (
                100,
                r#"const x = [].reduce(() => {
  return "y";
}, {} as SomeType<OtherType>);
"#,
            ),
        ],
    );
}

/// Call-only layout checks must not leak into `new` argument formatting.
#[test]
fn test_format_new_expression_does_not_use_test_call_layout() {
    assert_format_program!(
        r#"new test("description", () => { foo(); }, 100)
"#,
        r#"new test(
  "description",
  () => {
    foo();
  },
  100,
);
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}

/// Inferred call heads should format as ordinary compact call heads.
#[test]
fn test_format_inferred_call_head() {
    assert_format_program!(
        r#"const point: Point = _(1, 2);
"#,
        r#"const point: Point = _(1, 2);
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}

/// Mixed argument families should use the expected grouped-last layout.
#[test]
fn test_format_grouped_last_argument_layout() {
    assert_format_program_reference_widths(
        r#"// Don't group when both arguments are objects
call({ a: 1 }, { b: 2 });

// Don't group when both arguments are arrays
call([1, 2, 3], [4, 5, 6]);

// Don't group when both arguments are TSAsExpression
call(x as string, y as number);

// Don't group when both arguments are TSSatisfiesExpression
call(x satisfies Foo, y satisfies Bar);

// Don't group when both arguments are arrow functions
call(() => foo, () => bar);

// Don't group when both arguments are block arrow functions
call(() => { return foo; }, () => { return bar; });

// DO group when arguments are different types - object and array
call({ a: 1, b: 2, c: 3 }, [1, 2, 3, 4, 5, 6]);

// DO group when arguments are different types - array and object
call([1, 2, 3, 4, 5, 6], { a: 1, b: 2, c: 3 });

// DO group when first is arrow and second is object
call(() => { return foo; }, { a: 1, b: 2, c: 3 });

// DO group when first is object and second is arrow
call({ a: 1, b: 2, c: 3 }, () => { return foo; });
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"// Don't group when both arguments are objects
call({ a: 1 }, { b: 2 });

// Don't group when both arguments are arrays
call([1, 2, 3], [4, 5, 6]);

// Don't group when both arguments are TSAsExpression
call(x as string, y as number);

// Don't group when both arguments are TSSatisfiesExpression
call(x satisfies Foo, y satisfies Bar);

// Don't group when both arguments are arrow functions
call(
  () => foo,
  () => bar,
);

// Don't group when both arguments are block arrow functions
call(
  () => {
    return foo;
  },
  () => {
    return bar;
  },
);

// DO group when arguments are different types - object and array
call({ a: 1, b: 2, c: 3 }, [1, 2, 3, 4, 5, 6]);

// DO group when arguments are different types - array and object
call([1, 2, 3, 4, 5, 6], { a: 1, b: 2, c: 3 });

// DO group when first is arrow and second is object
call(
  () => {
    return foo;
  },
  { a: 1, b: 2, c: 3 },
);

// DO group when first is object and second is arrow
call({ a: 1, b: 2, c: 3 }, () => {
  return foo;
});
"#,
            ),
            (
                100,
                r#"// Don't group when both arguments are objects
call({ a: 1 }, { b: 2 });

// Don't group when both arguments are arrays
call([1, 2, 3], [4, 5, 6]);

// Don't group when both arguments are TSAsExpression
call(x as string, y as number);

// Don't group when both arguments are TSSatisfiesExpression
call(x satisfies Foo, y satisfies Bar);

// Don't group when both arguments are arrow functions
call(
  () => foo,
  () => bar,
);

// Don't group when both arguments are block arrow functions
call(
  () => {
    return foo;
  },
  () => {
    return bar;
  },
);

// DO group when arguments are different types - object and array
call({ a: 1, b: 2, c: 3 }, [1, 2, 3, 4, 5, 6]);

// DO group when arguments are different types - array and object
call([1, 2, 3, 4, 5, 6], { a: 1, b: 2, c: 3 });

// DO group when first is arrow and second is object
call(
  () => {
    return foo;
  },
  { a: 1, b: 2, c: 3 },
);

// DO group when first is object and second is arrow
call({ a: 1, b: 2, c: 3 }, () => {
  return foo;
});
"#,
            ),
        ],
    );
}

/// Named function callback arguments should follow the parameter layout.
#[test]
fn test_format_named_function_argument_layout() {
    assert_format_program_reference_widths(
        r#"useStableCallback(function useShowToast(
    ...args: Parameters<typeof toastService.addToastItem>
  ): void {
    toastService.addToastItem(...args);
  });
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"useStableCallback(function useShowToast(
  ...args: Parameters<typeof toastService.addToastItem>
): void {
  toastService.addToastItem(...args);
});
"#,
            ),
            (
                100,
                r#"useStableCallback(function useShowToast(
  ...args: Parameters<typeof toastService.addToastItem>
): void {
  toastService.addToastItem(...args);
});
"#,
            ),
        ],
    );
}

/// Blank lines between call arguments should stay broken out.
#[test]
fn test_format_preserves_blank_lines_between_call_arguments() {
    assert_format_program_reference_widths(
        r#"const value = call(
  foo,

  bar
)
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"const value = call(
  foo,

  bar,
);
"#,
            ),
            (
                100,
                r#"const value = call(
  foo,

  bar,
);
"#,
            ),
        ],
    );
}

/// Grouped first-argument layouts should treat simple generic and array cast tails like the formatter expects.
#[test]
fn test_format_grouped_first_argument_simple_cast_types() {
    assert_format_program_reference_widths(
        r#"const genericTail = call((
  alpha: AlphaType,
  beta: BetaType,
): Result => {
  return foo;
}, value as Foo<string>);

const genericStaticTail = call(<T, U>(
  alpha: AlphaType,
  beta: BetaType,
): Result => {
  return foo;
}, value as Foo<string>);

const arrayTail = call((
  alpha: AlphaType,
  beta: BetaType,
): Result => {
  return foo;
}, value as string[]);
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"const genericTail = call((alpha: AlphaType, beta: BetaType): Result => {
  return foo;
}, value as Foo<string>);

const genericStaticTail = call(
  <T, U>(alpha: AlphaType, beta: BetaType): Result => {
    return foo;
  },
  value as Foo<string>,
);

const arrayTail = call((alpha: AlphaType, beta: BetaType): Result => {
  return foo;
}, value as string[]);
"#,
            ),
            (
                100,
                r#"const genericTail = call((alpha: AlphaType, beta: BetaType): Result => {
  return foo;
}, value as Foo<string>);

const genericStaticTail = call(<T, U>(alpha: AlphaType, beta: BetaType): Result => {
  return foo;
}, value as Foo<string>);

const arrayTail = call((alpha: AlphaType, beta: BetaType): Result => {
  return foo;
}, value as string[]);
"#,
            ),
        ],
    );
}

/// Block arrow functions should group as the first argument when the tail is short.
#[test]
fn test_format_grouped_first_block_arrow_argument_layout() {
    assert_format_program_reference_widths(
        r#"const value = call(() => { return foo; }, bar);
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"const value = call(() => {
  return foo;
}, bar);
"#,
            ),
            (
                100,
                r#"const value = call(() => {
  return foo;
}, bar);
"#,
            ),
        ],
    );
}

/// Calls should let the outer call break before a complex single argument.
#[test]
fn test_format_nested_single_argument_call_layout() {
    assert_format_program_reference_widths(
        r#"const value = load(path.join(__dirname, "very-long-relative/path/that/forces/layout", "another-long-segment"));
"#,
        FileType::Tspp,
        &[
            (
                40,
                r#"const value = load(
  path.join(
    __dirname,
    "very-long-relative/path/that/forces/layout",
    "another-long-segment",
  ),
);
"#,
            ),
            (
                60,
                r#"const value = load(
  path.join(
    __dirname,
    "very-long-relative/path/that/forces/layout",
    "another-long-segment",
  ),
);
"#,
            ),
        ],
    );
}

/// Function-composition style calls should always break out all arguments.
#[test]
fn test_format_function_composition_call_arguments_broken_out() {
    assert_format_program_reference_widths(
        r#"const value = compose(sortBy((x) => x), flatten, map((x) => [x, x * 2]));
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"const value = compose(
  sortBy((x) => x),
  flatten,
  map((x) => [x, x * 2]),
);
"#,
            ),
            (
                100,
                r#"const value = compose(
  sortBy((x) => x),
  flatten,
  map((x) => [x, x * 2]),
);
"#,
            ),
        ],
    );
}

/// Test-style calls should keep the duration argument with the callback body.
#[test]
fn test_format_test_call_layout_with_duration() {
    assert_format_program_reference_widths(
        r#"test("formats the callback", () => {
  doThing();
}, 1000);
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"test("formats the callback", () => {
  doThing();
}, 1000);
"#,
            ),
            (
                100,
                r#"test("formats the callback", () => {
  doThing();
}, 1000);
"#,
            ),
        ],
    );
}

/// Two-argument test calls should keep expression-body callbacks in the direct layout.
#[test]
fn test_format_two_argument_test_call_keeps_expression_body_callback() {
    assert_format_program!(
        r#"it("name", (first, second) => first + second);
"#,
        r#"it("name", (first, second) =>
  first + second);
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(40).with_indent_width(2)
    );
}

/// Two-argument test calls should keep block arrow callbacks in the direct layout.
#[test]
fn test_format_two_argument_test_call_keeps_block_arrow_callback() {
    assert_format_program!(
        r#"it("name", (first, second) => { return first + second; });
"#,
        r#"it("name", (first, second) => {
  return first + second;
});
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(40).with_indent_width(2)
    );
}

/// Three-argument test calls should not use the direct layout for non-literal names.
#[test]
fn test_format_test_call_with_non_literal_name_stays_broken_out() {
    assert_format_program_reference_widths(
        r#"it(
  "a" + b,
  async () => {
    // code
  },
  30000,
);
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"it(
  "a" + b,
  async () => {
    // code
  },
  30000,
);
"#,
            ),
            (
                100,
                r#"it(
  "a" + b,
  async () => {
    // code
  },
  30000,
);
"#,
            ),
        ],
    );
}

/// Unit-test setup wrappers should keep the wrapper and callback in the direct layout.
#[test]
fn test_format_unit_test_setup_wrapper_keeps_direct_layout() {
    assert_format_program!(
        r#"beforeEach(async(() => { foo(); }));
"#,
        r#"beforeEach(async(() => {
  foo();
}));
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(40).with_indent_width(2)
    );
}

/// Three-argument hook calls should keep the ref, callback, and deps array in the direct layout.
#[test]
fn test_format_three_argument_hook_keeps_direct_layout() {
    assert_format_program!(
        r#"useImperativeHandle(ref, () => { foo(); }, [foo]);
"#,
        r#"useImperativeHandle(ref, () => {
  foo();
}, [foo]);
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(40).with_indent_width(2)
    );
}

/// Import-meta resolution calls should still follow member-chain breaking.
#[test]
fn test_format_import_meta_resolve_follows_member_chain_layout() {
    assert_format_program_reference_widths(
        r#"const url = import.meta.resolve("pkg/path");
"#,
        FileType::Tspp,
        &[
            (
                40,
                r#"const url = import.meta
  .resolve("pkg/path");
"#,
            ),
            (
                80,
                r#"const url = import.meta.resolve("pkg/path");
"#,
            ),
        ],
    );
}

/// Nested curried calls should break from the inside out.
#[test]
fn test_format_long_curried_call_breaks_from_inner_call() {
    assert_format_program_reference_widths(
        r#"const value = curriedFunction(firstArg)(secondArg)(thirdArg);
"#,
        FileType::Tspp,
        &[
            (
                30,
                r#"const value = curriedFunction(
  firstArg,
)(secondArg)(thirdArg);
"#,
            ),
            (
                40,
                r#"const value =
  curriedFunction(firstArg)(secondArg)(
    thirdArg,
  );
"#,
            ),
        ],
    );
}

/// Member receivers should not force call arguments onto separate lines.
#[test]
fn test_format_member_call_arguments_stay_grouped() {
    assert_format_program_reference_widths(
        r#"class Foo {
  private timer: NodeJS.Timeout | undefined = undefined;
  private value: number;

  constructor(v: number) {
    this.value = v;
  }

  start() {
    this.timer = setInterval(() => {
      console.log();
    }, this.value);
  }
}
"#,
        FileType::Tspp,
        &[
            (
                80,
                r#"class Foo {
  private timer: NodeJS.Timeout | undefined = undefined;
  private value: number;

  constructor(v: number) {
    this.value = v;
  }

  start() {
    this.timer = setInterval(() => {
      console.log();
    }, this.value);
  }
}
"#,
            ),
            (
                100,
                r#"class Foo {
  private timer: NodeJS.Timeout | undefined = undefined;
  private value: number;

  constructor(v: number) {
    this.value = v;
  }

  start() {
    this.timer = setInterval(() => {
      console.log();
    }, this.value);
  }
}
"#,
            ),
        ],
    );
}

/// Block arrow callbacks after spread arguments preserve their function head.
#[test]
fn test_format_spread_with_block_arrow_callback_argument() {
    assert_format_program!(
        r#"bar(...items, () => {
  return 1;
});
"#,
        r#"bar(...items, () => {
  return 1;
});
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}

/// Lambda callbacks should keep the hook-style callback and deps-array layout.
#[test]
fn test_format_lambda_callback_with_short_array_tail() {
    assert_format_program!(
        r#"const value = call(() => { return foo; }, [1, 2, 3]);
"#,
        r#"const value = call(() => {
  return foo;
}, [1, 2, 3]);
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(40).with_indent_width(2)
    );
}

/// Comment-only callback blocks must stay expanded in grouped-last call layout.
#[test]
fn test_format_grouped_last_comment_only_callback_block() {
    assert_format_program_reference_widths(
        r#"target(...argument, () => {
  // code
});
"#,
        FileType::Tspp,
        &[(
            80,
            r#"target(...argument, () => {
  // code
});
"#,
        )],
    );
}

/// Trailing line comments on the last argument must still expand the call.
#[test]
fn test_format_last_argument_trailing_line_comment_breaks_call() {
    assert_format_program!(
        r#"call(
  () => {
    // ...
  },
  "good" // trailing
)
"#,
        r#"call(
  () => {
    // ...
  },
  "good", // trailing
);
"#,
        FileType::Tspp,
        TsppFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}

use crate::{
    DestackFormatOptions, assert_format, assert_format_program, assert_format_program_idempotent,
    assert_format_program_reference_widths,
};
use destack_source::FileType;

/// Simple named tree arguments should stay stable.
#[test]
fn test_format_argument_named() {
    assert_format!(
        r#"x: 1"#,
        r#"x: 1"#,
        |p| p.eat_tree_argument(),
        DestackFormatOptions::default()
    );
}

/// React-hook separator comment clusters should stay idempotent.
#[test]
fn test_format_react_hook_separator_comment_cluster_is_idempotent() {
    assert_format_program_idempotent!(
        r#"useEffect(
  () => {
    console.log("some code", props.foo);
  }

  ,
  // We need to disable the eslint warning here,
  // because of some complicated reason.
  // eslint-disable line react-hooks/exhaustive-deps
  []
)"#,
        FileType::JavaScriptXml,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}

/// Multiline JSX arguments should force expanded multi-argument call layout.
#[test]
fn test_format_multiline_jsx_argument_forces_expanded_call_layout() {
    assert_format_program!(
        r#"const view = fn(bar, <div>
  <span />
</div>)
"#,
        r#"const view = fn(
  bar,
  <div>
    <span />
  </div>,
);
"#,
        FileType::JavaScriptXml,
        DestackFormatOptions::default_with_line_width(80).with_indent_width(2)
    );
}

/// Comments before type-argument calls should survive speculative call formatting.
#[test]
fn test_format_type_argument_call_comment_seams() {
    assert_format_program_reference_widths(
        r#"// Type arguments with speculative formatting should not lose comments
export const globalRegistry: $ZodRegistry = /*@__PURE__*/ registry();

// Comments before call expressions with type arguments should be preserved
const r = /* THIS */ f<Type>()
const s = /* comment */ foo<A | B | C>()
"#,
        FileType::TypeScript,
        &[
            (
                80,
                r#"// Type arguments with speculative formatting should not lose comments
export const globalRegistry: $ZodRegistry = /*@__PURE__*/ registry();

// Comments before call expressions with type arguments should be preserved
const r = /* THIS */ f<Type>();
const s = /* comment */ foo<A | B | C>();
"#,
            ),
            (
                100,
                r#"// Type arguments with speculative formatting should not lose comments
export const globalRegistry: $ZodRegistry = /*@__PURE__*/ registry();

// Comments before call expressions with type arguments should be preserved
const r = /* THIS */ f<Type>();
const s = /* comment */ foo<A | B | C>();
"#,
            ),
        ],
    );
}

/// Block callbacks with short cast tails should keep the expected grouped-first layout.
#[test]
fn test_format_typescript_grouped_first_argument_layout() {
    assert_format_program_reference_widths(
        r#"const x = [].reduce(() => {
  return "y";
}, {} as SomeType<OtherType>);
"#,
        FileType::TypeScript,
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

/// Mixed argument families should use the expected grouped-last layout.
#[test]
fn test_format_typescript_grouped_last_argument_layout() {
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

// Don't group when both arguments are function expressions
call(function() { return foo; }, function() { return bar; });

// DO group when arguments are different types - object and array
call({ a: 1, b: 2, c: 3 }, [1, 2, 3, 4, 5, 6]);

// DO group when arguments are different types - array and object
call([1, 2, 3, 4, 5, 6], { a: 1, b: 2, c: 3 });

// DO group when first is arrow and second is object
call(() => { return foo; }, { a: 1, b: 2, c: 3 });

// DO group when first is object and second is arrow
call({ a: 1, b: 2, c: 3 }, () => { return foo; });
"#,
        FileType::TypeScript,
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

// Don't group when both arguments are function expressions
call(
  function () {
    return foo;
  },
  function () {
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

// Don't group when both arguments are function expressions
call(
  function () {
    return foo;
  },
  function () {
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

/// Named function callback arguments should keep the expected inline call shell.
#[test]
fn test_format_typescript_named_function_argument_layout() {
    assert_format_program_reference_widths(
        r#"useStableCallback(function useShowToast(
    ...args: Parameters<typeof toastService.addToastItem>
  ): void {
    toastService.addToastItem(...args);
  });
"#,
        FileType::TypeScript,
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

# Call Groups

## Call Grouping

### multiple arrow arguments break

Arrow callbacks break to multiple lines when there are several.

```tspp:main.tspp
call(() => foo, () => bar)
```

```tspp expected
call(
    () => foo,
    () => bar,
);
```

### block callback with extra arguments breaks

Block-bodied callbacks expand when paired with other arguments.

```tspp:main.tspp
setTimeout(
    () => {
        // ...
    },
    timeout * Math.pow(1)
)
```

```tspp expected
setTimeout(
    () => {
        // ...
    },
    timeout * Math.pow(1),
);
```

### blank lines between arguments

Blank lines between arguments are preserved.

```tspp:main.tspp
call(
  () => {
    // ...
  },

  "good"
)
```

```tspp expected
call(
    () => {
        // ...
    },

    "good",
);
```

### trailing comment on last argument

Trailing comments stay attached to their argument.

```tspp:main.tspp
call(
  () => {
    // ...
  },
  "good" // trailing
)
```

```tspp expected
call(
    () => {
        // ...
    },
    "good", // trailing
);
```

### template literal argument

Template literal arguments keep their indentation.

```tspp:main.tspp
expect(genCode(createVNodeCall(null, "`div`", mockProps)))
  .toMatchInlineSnapshot(`
  `)
```

```tspp expected
expect(genCode(createVNodeCall(null, "`div`", mockProps))).toMatchInlineSnapshot(`
  `);
```

### optional call boundary line comment

Line comments between a callee and optional call stay on the full call expression.

```tspp:main.tspp
call // C4
?.()
```

```tspp expected
call?.(); // C4
```

### optional call separator block comment

Block comments between the callee and `?.` stay before the optional operator.

```tspp:main.tspp
alert /* comment */?.("value")
```

```tspp expected
alert /* comment */?.("value");
```

### optional call with inline comment argument

Inline block comments in empty optional call arguments stay inside `()`.

```tspp:main.tspp
call?.(/* argument comment */)
```

```tspp expected
call?.(/* argument comment */);
```

### empty call with line comment argument

Line comments in empty call arguments stay inside multiline `()`.

```tspp:main.tspp
call(
  // argument line comment
)
```

```tspp expected
call(
    // argument line comment
);
```

### empty optional call with line comment argument

Line comments in empty optional call arguments stay inside multiline `()`.

```tspp:main.tspp
call?.( // argument line comment
)
```

```tspp expected
call?.(
    // argument line comment
);
```

### last argument boundary comments

Comments around the final callback control whether the call stays grouped.

```tspp:main.tspp
call(editor /* comment */, () => {
  //
});
call(editor, /* comment */
  () => {
    //
  }
);
call(/* */ editor /* comment */, () => {
  //
});
call(/* comment */
  () => {
    //
  }
);
```

```tspp expected
call(editor /* comment */, () => {
    //
});
call(editor /* comment */, () => {
    //
});
call(/* */ editor /* comment */, () => {
    //
});
call(
    /* comment */
    () => {
        //
    },
);
```

### multiple named function expressions break

Named function expressions break to multiple lines when repeated.

```tspp:main.tspp
call(function first() { return foo; }, function second() { return bar; })
```

```tspp expected
call(
    function first() {
        return foo;
    },
    function second() {
        return bar;
    },
);
```

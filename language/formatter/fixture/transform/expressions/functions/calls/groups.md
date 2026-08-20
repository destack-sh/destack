# Call Groups

## Call Grouping

### multiple arrow arguments break

Arrow callbacks break to multiple lines when there are several.

```ds:main.ds
call(() => foo, () => bar)
```

```ds expected
call(
    () => foo,
    () => bar,
);
```

### block callback with extra arguments breaks

Block-bodied callbacks expand when paired with other arguments.

```ds:main.ds
setTimeout(
    () => {
        // ...
    },
    timeout * Math.pow(1)
)
```

```ds expected
setTimeout(
    () => {
        // ...
    },
    timeout * Math.pow(1),
);
```

### blank lines between arguments

Blank lines between arguments are preserved.

```ds:main.ds
call(
  () => {
    // ...
  },

  "good"
)
```

```ds expected
call(
    () => {
        // ...
    },

    "good",
);
```

### trailing comment on last argument

Trailing comments stay attached to their argument.

```ds:main.ds
call(
  () => {
    // ...
  },
  "good" // trailing
)
```

```ds expected
call(
    () => {
        // ...
    },
    "good", // trailing
);
```

### template literal argument

Template literal arguments keep their indentation.

```ds:main.ds
expect(genCode(createVNodeCall(null, "`div`", mockProps)))
  .toMatchInlineSnapshot(`
  `)
```

```ds expected
expect(genCode(createVNodeCall(null, "`div`", mockProps))).toMatchInlineSnapshot(`
  `);
```

### optional call boundary line comment

Line comments between a callee and optional call stay on the full call expression.

```ds:main.ds
call // C4
?.()
```

```ds expected
call?.(); // C4
```

### optional call separator block comment

Block comments between the callee and `?.` stay before the optional operator.

```ds:main.ds
alert /* comment */?.("value")
```

```ds expected
alert /* comment */?.("value");
```

### optional call with inline comment argument

Inline block comments in empty optional call arguments stay inside `()`.

```ds:main.ds
call?.(/* argument comment */)
```

```ds expected
call?.(/* argument comment */);
```

### empty call with line comment argument

Line comments in empty call arguments stay inside multiline `()`.

```ds:main.ds
call(
  // argument line comment
)
```

```ds expected
call(
    // argument line comment
);
```

### empty optional call with line comment argument

Line comments in empty optional call arguments stay inside multiline `()`.

```ds:main.ds
call?.( // argument line comment
)
```

```ds expected
call?.(
    // argument line comment
);
```

### last argument boundary comments

Comments around the final callback control whether the call stays grouped.

```ds:main.ds
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

```ds expected
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

```ds:main.ds
call(function first() { return foo; }, function second() { return bar; })
```

```ds expected
call(
    function first() {
        return foo;
    },
    function second() {
        return bar;
    },
);
```

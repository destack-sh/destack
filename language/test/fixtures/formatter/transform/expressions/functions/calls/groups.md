# Call Groups

## TypeScript Call Grouping

### multiple arrow arguments break

Arrow callbacks break to multiple lines when there are several.

```ts:main.ts
call(() => foo, () => bar)
```

```ts expected
call(
    () => foo,
    () => bar,
);
```

### block callback with extra arguments breaks

Block-bodied callbacks expand when paired with other arguments.

```ts:main.ts
setTimeout(
    () => {
        // ...
    },
    timeout * Math.pow(1)
)
```

```ts expected
setTimeout(
    () => {
        // ...
    },
    timeout * Math.pow(1),
);
```

### blank lines between arguments

Blank lines between arguments are preserved.

```ts:main.ts
call(
  () => {
    // ...
  },

  "good"
)
```

```ts expected
call(
    () => {
        // ...
    },

    "good",
);
```

### trailing comment on last argument

Trailing comments stay attached to their argument.

```ts:main.ts
call(
  () => {
    // ...
  },
  "good" // trailing
)
```

```ts expected
call(
    () => {
        // ...
    },
    "good", // trailing
);
```

### template literal argument

Template literal arguments keep their indentation.

```ts:main.ts
expect(genCode(createVNodeCall(null, "`div`", mockProps)))
  .toMatchInlineSnapshot(`
  `)
```

```ts expected
expect(genCode(createVNodeCall(null, "`div`", mockProps))).toMatchInlineSnapshot(`
  `);
```

### optional call boundary line comment

Line comments between a callee and optional call stay on the full call expression.

```ts:main.ts
call // C4
?.()
```

```ts expected
call?.(); // C4
```

### optional call separator block comment

Block comments between the callee and `?.` stay before the optional operator.

```ts:main.ts
alert /* comment */?.("value")
```

```ts expected
alert /* comment */?.("value");
```

### optional call with inline comment argument

Inline block comments in empty optional call arguments stay inside `()`.

```ts:main.ts
call?.(/* argument comment */)
```

```ts expected
call?.(/* argument comment */);
```

### empty call with line comment argument

Line comments in empty call arguments stay inside multiline `()`.

```ts:main.ts
call(
  // argument line comment
)
```

```ts expected
call(
    // argument line comment
);
```

### empty optional call with line comment argument

Line comments in empty optional call arguments stay inside multiline `()`.

```ts:main.ts
call?.( // argument line comment
)
```

```ts expected
call?.(
    // argument line comment
);
```

### last argument boundary comments

Comments around the final callback control whether the call stays grouped.

```ts:main.ts
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

```ts expected
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

### multiple function expressions break

Function expressions break to multiple lines when repeated.

```ts:main.ts
call(function () { return foo; }, function () { return bar; })
```

```ts expected
call(
    function () {
        return foo;
    },
    function () {
        return bar;
    },
);
```

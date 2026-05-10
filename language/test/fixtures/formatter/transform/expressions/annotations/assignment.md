# Assignment Annotations

## Assignment Boundaries

### assignment chain with ts-ignore marker

Assignment right side marker comments stay attached to the assigned expression.

```ts:main.ts
longVariableName1 = // @ts-ignore
(variable01 + veryLongVariableNameNumber2).method()
```

```ts expected
longVariableName1 = // @ts-ignore
    (variable01 + veryLongVariableNameNumber2).method();
```

### assignment to arrow with separator comments

Comments around assignment to arrow expressions stay attached to the assigned arrow.

```ts:main.ts
const handler = /* marker */

  // before-arrow

  () => {}
```

```ts expected
const handler =
    /* marker */

    // before-arrow

    () => {};
```

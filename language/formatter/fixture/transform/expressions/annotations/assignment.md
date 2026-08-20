# Assignment Annotations

## Assignment Boundaries

### assignment chain with ts-ignore marker

Assignment right side marker comments stay attached to the assigned expression.

```ds:main.ds
longVariableName1 = // @ts-ignore
(variable01 + veryLongVariableNameNumber2).method()
```

```ds expected
longVariableName1 = // @ts-ignore
    (variable01 + veryLongVariableNameNumber2).method();
```

### assignment to arrow with separator comments

Comments around assignment to arrow expressions stay attached to the assigned arrow.

```ds:main.ds
const handler = /* marker */

  // before-arrow

  () => {}
```

```ds expected
const handler =
    /* marker */

    // before-arrow

    () => {};
```

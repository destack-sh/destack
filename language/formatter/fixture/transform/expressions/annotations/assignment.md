# Assignment Annotations

## Assignment Boundaries

### assignment chain with ts-ignore marker

Assignment right side marker comments stay attached to the assigned expression.

```tspp:main.tspp
longVariableName1 = // @ts-ignore
(variable01 + veryLongVariableNameNumber2).method()
```

```tspp expected
longVariableName1 = // @ts-ignore
    (variable01 + veryLongVariableNameNumber2).method();
```

### assignment to arrow with separator comments

Comments around assignment to arrow expressions stay attached to the assigned arrow.

```tspp:main.tspp
const handler = /* marker */

  // before-arrow

  () => {}
```

```tspp expected
const handler =
    /* marker */

    // before-arrow

    () => {};
```

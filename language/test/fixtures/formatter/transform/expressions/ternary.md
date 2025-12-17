# Ternary Expressions

Tests for ternary (conditional) expression formatting.

## Basic Ternary

### ternary as variable value

Ternary can be assigned to a variable.

```ds
const result = x > 0 ? "positive" : "non-positive"
```

```ds expected
const result = x > 0 ? "positive" : "non-positive";
```

### ternary with identifiers in assignment

Ternary with identifier expressions stays on one line.

```ds
const value = isActive ? activeValue : inactiveValue
```

```ds expected
const value = isActive ? activeValue : inactiveValue;
```

## Line Breaking

### long ternary in assignment breaks

When a ternary exceeds line width, it breaks across lines.

```ds line-width=30
const el = isLoading ? <Spinner /> : <Content data={data} />
```

```ds expected
const el = isLoading
    ? <Spinner />
    : <Content data={data} />;
```

### ternary with long branches breaks

Long branches cause the ternary to break.

```ds line-width=40
const x = condition ? longConsequentValue : longAlternateValue
```

```ds expected
const x = condition
    ? longConsequentValue
    : longAlternateValue;
```

## Nested Ternary

### nested ternary in assignment

Nested ternaries format with proper indentation.

```ds line-width=50
const x = a ? b : c ? d : e
```

```ds expected
const x = a ? b : c ? d : e;
```

### deeply nested ternary breaks

Deep nesting breaks the outer ternary, inner stays on one line if it fits.

```ds line-width=50
const x = isFirst ? firstValue : isSecond ? secondValue : defaultValue
```

```ds expected
const x = isFirst
    ? firstValue
    : isSecond ? secondValue : defaultValue;
```

## Ternary with Complex Expressions

### ternary with function calls

Function calls work in ternary branches.

```ds
const result = valid ? process(data) : handleError(err)
```

```ds expected
const result = valid ? process(data) : handleError(err);
```

### ternary with object literals

Object literals can be ternary branches.

```ds
const config = isUser ? { type: "user" } : { type: "guest" }
```

```ds expected
const config = isUser ? { type: "user" } : { type: "guest" };
```

### ternary in function call

Ternary can be a function argument.

```ds
render(loading ? <Spinner /> : <Content />)
```

```ds expected
render(loading ? <Spinner /> : <Content />);
```

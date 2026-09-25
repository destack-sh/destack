# Ternary Expressions

Ternary fixtures cover conditional expression wrapping, comments, and nested branches.

## Ternary Forms

### ternary as variable value

Ternary can be assigned to a variable.

```tspp
const result = x > 0 ? "positive" : "non-positive"
```

```tspp expected
const result = x > 0 ? "positive" : "non-positive";
```

### ternary with identifiers in assignment

Ternary with identifier expressions stays on one line.

```tspp
const value = isActive ? activeValue : inactiveValue
```

```tspp expected
const value = isActive ? activeValue : inactiveValue;
```

### ternary with logical condition

Logical conditions remain inline when short.

```tspp
const value = ready && valid ? ok : fail
```

```tspp expected
const value = ready && valid ? ok : fail;
```

## Line Breaking

### long ternary in assignment breaks

When a ternary exceeds line width, it breaks across lines.

```tspp line-width=30
const el = isLoading ? <Spinner /> : <Content data={data} />
```

```tspp expected
const el = isLoading ? (
    <Spinner />
) : (
    <Content data={data} />
);
```

### ternary with long branches breaks

Long branches cause the ternary to break.

```tspp line-width=40
const x = condition ? longConsequentValue : longAlternateValue
```

```tspp expected
const x = condition
    ? longConsequentValue
    : longAlternateValue;
```

### ternary with nullish coalescing branch

Nullish coalescing branches keep their parentheses and indentation.

```tspp:main.tspp line-width=60
const value = options.singleRun ? "Infinity" : (options.cacheLifetime?.glob ?? DEFAULT_TSCONFIG_CACHE_DURATION_SECONDS)
```

```tspp expected
const value = options.singleRun
    ? "Infinity"
    : (options.cacheLifetime?.glob
          ?? DEFAULT_TSCONFIG_CACHE_DURATION_SECONDS);
```

## Nested Ternary

### nested ternary in assignment

Nested ternaries expand with stable indentation.

```tspp line-width=50
const x = a ? b : c ? d : e
```

```tspp expected
const x = a ? b : c ? d : e;
```

### deeply nested ternary breaks

Nested ternaries break at all levels with same indentation.

```tspp line-width=50
const x = isFirst ? firstValue : isSecond ? secondValue : defaultValue
```

```tspp expected
const x = isFirst
    ? firstValue
    : isSecond
      ? secondValue
      : defaultValue;
```

## Ternary Branch Expressions

### ternary with function calls

Function calls work in ternary branches.

```tspp
const result = valid ? process(data) : handleError(err)
```

```tspp expected
const result = valid ? process(data) : handleError(err);
```

### ternary with chained call in branch

Chained calls in ternary branches keep their indentation.

```tspp:main.tspp line-width=80
const result = id === null
  ? null
  : internal.getSuspenseCache(client).getFragmentRef(
      [id, options.fragment, cache.canonicalStringify(variables)],
      client,
      tslib.__assign(tslib.__assign({}, options), {
        variables: variables,
        from: id,
      }),
    )
```

```tspp expected
const result = id === null
    ? null
    : internal.getSuspenseCache(client).getFragmentRef(
          [id, options.fragment, cache.canonicalStringify(variables)],
          client,
          tslib.__assign(tslib.__assign({}, options), {
              variables: variables,
              from: id,
          }),
      );
```

### ternary with object literals

Object literals can be ternary branches.

```tspp
const config = isUser ? { type: "user" } : { type: "guest" }
```

```tspp expected
const config = isUser ? { type: "user" } : { type: "guest" };
```

### ternary in function call

Ternary can be a function argument.

```tspp
render(loading ? <Spinner /> : <Content />)
```

```tspp expected
render(loading ? <Spinner /> : <Content />);
```

### ternary as const assertion value

Ternary expressions keep grouping when asserted as const.

```tspp
const value = (true ? 1 : 2) as const
const checked = (enabled ? value : fallback) satisfies number
```

```tspp expected
const value = (true ? 1 : 2) as const;
const checked = (enabled ? value : fallback) satisfies number;
```

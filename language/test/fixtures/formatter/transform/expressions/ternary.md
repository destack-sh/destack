# Ternary Expressions

Ternary fixtures cover conditional expression layout, comments, and nested branches.

## Ternary Forms

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

### ternary with logical condition

Logical conditions remain inline when short.

```ds
const value = ready && valid ? ok : fail
```

```ds expected
const value = ready && valid ? ok : fail;
```

## Line Breaking

### long ternary in assignment breaks

When a ternary exceeds line width, it breaks across lines.

```ds line-width=30
const el = isLoading ? <Spinner /> : <Content data={data} />
```

```ds expected
const el = isLoading ? (
    <Spinner />
) : (
    <Content data={data} />
);
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

### ternary with nullish coalescing branch

Nullish coalescing branches keep their parentheses and indentation.

```ts:main.ts line-width=60
const value = options.singleRun ? "Infinity" : (options.cacheLifetime?.glob ?? DEFAULT_TSCONFIG_CACHE_DURATION_SECONDS)
```

```ts expected
const value = options.singleRun
    ? "Infinity"
    : (options.cacheLifetime?.glob ??
      DEFAULT_TSCONFIG_CACHE_DURATION_SECONDS);
```

## Nested Ternary

### nested ternary in assignment

Nested ternaries expand with stable indentation.

```ds line-width=50
const x = a ? b : c ? d : e
```

```ds expected
const x = a ? b : c ? d : e;
```

### deeply nested ternary breaks

Nested ternaries break at all levels with same indentation.

```ds line-width=50
const x = isFirst ? firstValue : isSecond ? secondValue : defaultValue
```

```ds expected
const x = isFirst
    ? firstValue
    : isSecond
      ? secondValue
      : defaultValue;
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

### ternary with chained call in branch

Chained calls in ternary branches keep their indentation.

```ts:main.ts line-width=80
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

```ts expected
const result =
    id === null
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

### ternary as const assertion value

Ternary expressions keep grouping when asserted as const.

```ds
const value = (true ? 1 : 2) as const
const checked = (enabled ? value : fallback) satisfies number
```

```ds expected
const value = (true ? 1 : 2) as const;
const checked = (enabled ? value : fallback) satisfies number;
```

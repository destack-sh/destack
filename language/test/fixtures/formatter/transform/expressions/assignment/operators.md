# Assignment Expressions

Assignment fixtures cover operator spacing, chaining, patterns, comments, and assignment targets.

## Assignment Forms

### assignment expression

Simple assignments keep spaces around the operator.

```ds
value = 1
```

```ds expected
value = 1;
```

### chained assignment

Chained assignments stay right-associative.

```ds
first = second = third
```

```ds expected
first = second = third;
```

### assignment expression call chain breaks after operator

Poorly breakable call chains should break after `=`.

```ts:main.ts line-width=20
result = api.namespace.member().tail()
```

```ts expected
result =
    api.namespace
        .member()
        .tail();
```

### call initializer breaks after the operator

Long call initializers break after `=`.

```ts:main.ts line-width=30
const veryLongPackageBindingName = loadPackage(jestPath)
```

```ts expected
const veryLongPackageBindingName =
    loadPackage(jestPath);
```

### interpolated template argument stays fluid

Interpolated template arguments should not trigger the poorly breakable shortcut.

```ts:main.ts line-width=30
const veryLongBindingName = namespace.foo(`hello ${name}`)
```

```ts expected
const veryLongBindingName =
    namespace.foo(
        `hello ${name}`,
    );
```

### string RHS breaks after operator

Long left-hand sides with string RHS values should break after `=`.

```ts:main.ts line-width=20
const veryLongVariableName = "value"
veryLongVariableName = "value"
```

```ts expected
const veryLongVariableName =
    "value";
veryLongVariableName =
    "value";
```

## Compound Assignments

### additive assignment

Compound assignments keep spaces around the operator.

```ds
count += 1
```

```ds expected
count += 1;
```

### logical assignments

Logical assignment operators format with spaces.

```ds
value ||= fallback
```

```ds expected
value ||= fallback;
```

### logical and assignment

Logical and assignment operators format with spaces.

```ds
value &&= compute()
```

```ds expected
value &&= compute();
```

### nullish coalescing assignment

Nullish coalescing assignment keeps spaces around the operator.

```ts:main.ts
value ??= fallback
```

```ts expected
value ??= fallback;
```

### bitwise assignment

Bitwise assignments keep spaces around the operator.

```ds
flags |= mask
```

```ds expected
flags |= mask;
```

### shift assignment

Shift assignments keep spaces around the operator.

```ds
flags <<= 1
```

```ds expected
flags <<= 1;
```

### non-null assignment drops extra parentheses

Non-null assertions do not require parentheses on assignment.

```ts:main.ts
(pendingSetRef.flags!) |= SchedulerJobFlags.DISPOSED
```

```ts expected
pendingSetRef.flags! |= SchedulerJobFlags.DISPOSED;
```

### type assertion assignments keep required parentheses

`as` and `satisfies` assertions keep parentheses when used as assignment targets.

```ts:main.ts
(pendingSetRef.flags as T) |= SchedulerJobFlags.DISPOSED
(pendingSetRef.flags satisfies T) |= SchedulerJobFlags.DISPOSED
```

```ts expected
(pendingSetRef.flags as T) |= SchedulerJobFlags.DISPOSED;
(pendingSetRef.flags satisfies T) |= SchedulerJobFlags.DISPOSED;
```

### wrapping and saturating assignments

Wrapping and saturating assignments keep spaces around the operator.

```ds
total +%= increment
```

```ds expected
total +%= increment;
```

```ds
count *|= multiplier
```

```ds expected
count *|= multiplier;
```

## Destructuring Assignments

### array rest assignment

Array rest patterns format with spread in assignment.

```ts:main.ts
[...rest] = arr
```

```ts expected
[...rest] = arr;
```

### object assignment defaults

Object assignment targets keep shorthand and property defaults.

```ds
({ x = fallback, y: z = other, [key]: target, ...rest } = value)
```

```ds expected
({ x = fallback, y: z = other, [key]: target, ...rest } = value);
```

### array assignment defaults

Array assignment targets keep elisions, defaults, and rest.

```ds
[first, , second = fallback, ...rest] = value
```

```ds expected
[first, , second = fallback, ...rest] = value;
```

### nested assignment defaults

Defaults inside nested object and array targets stay assignable.

```ds
({ a: { b = c } = d, e: [f = g] } = h)
```

```ds expected
({
    a: { b = c } = d,
    e: [f = g],
} = h);
```

### member assignment targets

Object assignment targets keep member and index targets.

```ds
({ value: object.property, [key]: target[index] } = source)
```

```ds expected
({ value: object.property, [key]: target[index] } = source);
```

### nested assignment targets

Nested assignment targets keep aliases, defaults, computed keys, and rest fields.

```ts:main.ts
({ a, b: { c = d }, [key]: target[index], ...rest } = source)
```

```ts expected
({
    a,
    b: { c = d },
    [key]: target[index],
    ...rest
} = source);
```

### array assignment targets

Array assignment targets keep elisions, defaults, nested targets, and rest fields.

```ts:main.ts
[first, , second = fallback, { value: object.property }, ...rest] = source
```

```ts expected
[first, , second = fallback, { value: object.property }, ...rest] = source;
```

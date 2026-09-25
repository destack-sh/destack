# Assignment Expressions

Assignment fixtures cover operator spacing, chaining, patterns, comments, and assignment targets.

## Assignment Forms

### assignment expression

Simple assignments keep spaces around the operator.

```tspp
value = 1
```

```tspp expected
value = 1;
```

### chained assignment

Chained assignments stay right-associative.

```tspp
first = second = third
```

```tspp expected
first = second = third;
```

### assignment expression call chain breaks after operator

Poorly breakable call chains should break after `=`.

```tspp:main.tspp line-width=20
result = api.namespace.member().tail()
```

```tspp expected
result =
    api.namespace
        .member()
        .tail();
```

### call initializer breaks after the operator

Long call initializers break after `=`.

```tspp:main.tspp line-width=30
const veryLongPackageBindingName = loadPackage(jestPath)
```

```tspp expected
const veryLongPackageBindingName =
    loadPackage(jestPath);
```

### interpolated template argument stays fluid

Interpolated template arguments should not trigger the poorly breakable shortcut.

```tspp:main.tspp line-width=30
const veryLongBindingName = namespace.foo(`hello ${name}`)
```

```tspp expected
const veryLongBindingName =
    namespace.foo(
        `hello ${name}`,
    );
```

### string RHS breaks after operator

Long left-hand sides with string RHS values should break after `=`.

```tspp:main.tspp line-width=20
const veryLongVariableName = "value"
veryLongVariableName = "value"
```

```tspp expected
const veryLongVariableName =
    "value";
veryLongVariableName =
    "value";
```

## Compound Assignments

### additive assignment

Compound assignments keep spaces around the operator.

```tspp
count += 1
```

```tspp expected
count += 1;
```

### logical assignments

Logical assignment operators format with spaces.

```tspp
value ||= fallback
```

```tspp expected
value ||= fallback;
```

### logical and assignment

Logical and assignment operators format with spaces.

```tspp
value &&= compute()
```

```tspp expected
value &&= compute();
```

### nullish coalescing assignment

Nullish coalescing assignment keeps spaces around the operator.

```tspp:main.tspp
value ??= fallback
```

```tspp expected
value ??= fallback;
```

### bitwise assignment

Bitwise assignments keep spaces around the operator.

```tspp
flags |= mask
```

```tspp expected
flags |= mask;
```

### shift assignment

Shift assignments keep spaces around the operator.

```tspp
flags <<= 1
```

```tspp expected
flags <<= 1;
```

### non-null assignment drops extra parentheses

Non-null assertions do not require parentheses on assignment.

```tspp:main.tspp
(pendingSetRef.flags!) |= SchedulerJobFlags.DISPOSED
```

```tspp expected
pendingSetRef.flags! |= SchedulerJobFlags.DISPOSED;
```

### type assertion assignments keep required parentheses

`as` and `satisfies` assertions keep parentheses when used as assignment targets.

```tspp:main.tspp
(pendingSetRef.flags as T) |= SchedulerJobFlags.DISPOSED
(pendingSetRef.flags satisfies T) |= SchedulerJobFlags.DISPOSED
```

```tspp expected
(pendingSetRef.flags as T) |= SchedulerJobFlags.DISPOSED;
(pendingSetRef.flags satisfies T) |= SchedulerJobFlags.DISPOSED;
```

## Destructuring Assignments

### array rest assignment

Array rest patterns format with spread in assignment.

```tspp:main.tspp
[...rest] = arr
```

```tspp expected
[...rest] = arr;
```

### object assignment defaults

Object assignment targets keep shorthand and property defaults.

```tspp
({ x = fallback, y: z = other, [key]: target, ...rest } = value)
```

```tspp expected
({ x = fallback, y: z = other, [key]: target, ...rest } = value);
```

### array assignment defaults

Array assignment targets keep elisions, defaults, and rest.

```tspp
[first, , second = fallback, ...rest] = value
```

```tspp expected
[first, , second = fallback, ...rest] = value;
```

### nested assignment defaults

Defaults inside nested object and array targets stay assignable.

```tspp
({ a: { b = c } = d, e: [f = g] } = h)
```

```tspp expected
({
    a: { b = c } = d,
    e: [f = g],
} = h);
```

### member assignment targets

Object assignment targets keep member and index targets.

```tspp
({ value: object.property, [key]: target[index] } = source)
```

```tspp expected
({ value: object.property, [key]: target[index] } = source);
```

### nested assignment targets

Nested assignment targets keep aliases, defaults, computed keys, and rest fields.

```tspp:main.tspp
({ a, b: { c = d }, [key]: target[index], ...rest } = source)
```

```tspp expected
({
    a,
    b: { c = d },
    [key]: target[index],
    ...rest
} = source);
```

### array assignment targets

Array assignment targets keep elisions, defaults, nested targets, and rest fields.

```tspp:main.tspp
[first, , second = fallback, { value: object.property }, ...rest] = source
```

```tspp expected
[first, , second = fallback, { value: object.property }, ...rest] = source;
```

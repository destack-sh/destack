# Type Binary Operators

Type binary fixtures cover `as`, `satisfies`, non-null assertions, and assertion grouping.

## Cast Expressions

### as cast expression

Cast operators keep spaces around `as`.

```tspp:main.tspp
const value = input as Foo
```

```tspp expected
const value = input as Foo;
```

### satisfies expression

Satisfies operators keep spaces around `satisfies`.

```tspp:main.tspp
const value = input satisfies Foo
```

```tspp expected
const value = input satisfies Foo;
```

## Nested Type Binaries

### call with type binary arguments

Type binary arguments stay grouped in calls.

```tspp:main.tspp
const value = call(input as Foo, other satisfies Bar)
```

```tspp expected
const value = call(input as Foo, other satisfies Bar);
```

### casted comparison in logical expression

Comparison casts inside logical expressions keep grouping parentheses.

```tspp:main.tspp
const ok = i < 0 || i >= length as number
```

```tspp expected
const ok = i < 0 || ((i >= length) as number);
```

### cast before arithmetic

Casts on the left side of arithmetic stay grouped.

```tspp:main.tspp
const last = length as number - 1
```

```tspp expected
const last = (length as number) - 1;
```

### satisfies before arithmetic

Satisfies expressions on the left side of arithmetic stay grouped.

```tspp:main.tspp
const last = (length satisfies number) - 1
```

```tspp expected
const last = (length satisfies number) - 1;
```

### cast as call callee

Cast expressions used as call callees keep grouping parentheses.

```tspp:main.tspp
const value = (value as Fn)()
```

```tspp expected
const value = (value as Fn)();
```

### cast as member object

Cast expressions used as member objects keep grouping parentheses.

```tspp:main.tspp
const value = (value as Box).property
```

```tspp expected
const value = (value as Box).property;
```

### chained assertions stay direct

Nested assertion chains stay direct when no parent context needs grouping.

```tspp:main.tspp
const value = input as unknown as Result
```

```tspp expected
const value = input as unknown as Result;
```

### chained assertion with union target

Nested assertions keep grouping when the inner target is a union.

```tspp:main.tspp
const value = ("ok" as string | number) as string
```

```tspp expected
const value = ("ok" as string | number) as string;
```

### assertion in ternary test

Assertions in ternary tests keep grouping parentheses.

```tspp:main.tspp
const value = (input as boolean) ? yes : no
```

```tspp expected
const value = (input as boolean) ? yes : no;
```

## Runtime Guards

### runtime type guard comments

Runtime type guard comments stay on the side of the operator they describe.

```tspp
const ok = value /* checked value */ is /* expected type */ string
```

```tspp expected
const ok = value /* checked value */ is /* expected type */ string;
```

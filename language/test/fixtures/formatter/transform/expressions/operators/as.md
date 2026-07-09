# Type Binary Operators

Type binary fixtures cover `as`, `satisfies`, non-null assertions, and assertion grouping.

## Cast Expressions

### as cast expression

Cast operators keep spaces around `as`.

```ds:main.ds
const value = input as Foo
```

```ds expected
const value = input as Foo;
```

### satisfies expression

Satisfies operators keep spaces around `satisfies`.

```ds:main.ds
const value = input satisfies Foo
```

```ds expected
const value = input satisfies Foo;
```

## Nested Type Binaries

### call with type binary arguments

Type binary arguments stay grouped in calls.

```ds:main.ds
const value = call(input as Foo, other satisfies Bar)
```

```ds expected
const value = call(input as Foo, other satisfies Bar);
```

### casted comparison in logical expression

Comparison casts inside logical expressions keep grouping parentheses.

```ds:main.ds
const ok = i < 0 || i >= length as number
```

```ds expected
const ok = i < 0 || ((i >= length) as number);
```

### cast before arithmetic

Casts on the left side of arithmetic stay grouped.

```ds:main.ds
const last = length as number - 1
```

```ds expected
const last = (length as number) - 1;
```

### satisfies before arithmetic

Satisfies expressions on the left side of arithmetic stay grouped.

```ds:main.ds
const last = (length satisfies number) - 1
```

```ds expected
const last = (length satisfies number) - 1;
```

### cast as call callee

Cast expressions used as call callees keep grouping parentheses.

```ds:main.ds
const value = (value as Fn)()
```

```ds expected
const value = (value as Fn)();
```

### cast as member object

Cast expressions used as member objects keep grouping parentheses.

```ds:main.ds
const value = (value as Box).property
```

```ds expected
const value = (value as Box).property;
```

### chained assertions stay direct

Nested assertion chains stay direct when no parent context needs grouping.

```ds:main.ds
const value = input as unknown as Result
```

```ds expected
const value = input as unknown as Result;
```

### chained assertion with union target

Nested assertions keep grouping when the inner target is a union.

```ds:main.ds
const value = ("ok" as string | number) as string
```

```ds expected
const value = ("ok" as string | number) as string;
```

### assertion in ternary test

Assertions in ternary tests keep grouping parentheses.

```ds:main.ds
const value = (input as boolean) ? yes : no
```

```ds expected
const value = (input as boolean) ? yes : no;
```

## Runtime Guards

### runtime type guard comments

Runtime type guard comments stay on the side of the operator they describe.

```ds
const ok = value /* checked value */ is /* expected type */ string
```

```ds expected
const ok = value /* checked value */ is /* expected type */ string;
```

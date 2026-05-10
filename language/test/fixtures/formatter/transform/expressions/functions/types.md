# Function Types

## Function Types

### function type alias

Function types keep parameter and return type spacing.

```ds
type Predicate = (value: string) => boolean
```

```ds expected
type Predicate = (value: string) => boolean;
```

### function type predicate return

Function type returns can narrow their parameter type.

```ds
type Is<T> = (value: any) => value is T
```

```ds expected
type Is<T> = (value: any) => value is T;
```

### function type predicate in conditional

Predicate function types stay parenthesized in conditional type constraints.

```ds
type Guarded<Actual> = Actual extends (value: any, ...args: any[]) => value is infer T ? T : never
```

```ds expected
type Guarded<Actual> = Actual extends ((value: any, ...args: any[]) => value is infer T)
    ? T
    : never;
```

### function type predicate comments

Comments before and after `is` stay inside the predicate return.

```ds
type Guard<T> = (value: unknown) => value /* value */ is /* type */ T
```

```ds expected
type Guard<T> = (value: unknown) => value /* value */ is /* type */ T;
```

### function type predicate comments in conditional

Predicate comments survive when the function type is the conditional constraint.

```ds
type PickGuard<Actual> = Actual extends (value: unknown) => value /* value */ is infer T ? T : never
```

```ds expected
type PickGuard<Actual> = Actual extends ((value: unknown) => value /* value */ is infer T)
    ? T
    : never;
```

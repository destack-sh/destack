# Template Remapping

## Template Remapping

### mapped key template remap

Mapped key remaps keep template literal keys attached to the `as` clause.

```tspp
type HandlerMap<T>={ [K in keyof T as `on-${K}`]:T[K] }
```

```tspp expected
type HandlerMap<T> = { [K in keyof T as `on-${K}`]: T[K] };
```

### inferred template part

Template infer clauses stay inline when the conditional fits.

```tspp
type PayloadName<E>=E extends `evt:${infer Name}` ? Name : never
```

```tspp expected
type PayloadName<E> = E extends `evt:${infer Name}` ? Name : never;
```

# Conditional Types

## distributivity and infer

### naked type parameters distribute across unions

Using a naked type parameter in the check position should evaluate each union member independently.

```ts
type Label<T> = T extends string ? `s:${T}` : `n:${T}`;

declare const value: Label<"a" | 1>;
value satisfies "s:a" | "n:1";
```

### wrapped type parameters do not distribute across unions

Wrapping the type parameter should force a single non-distributed comparison for the entire union.

```ts
type Label<T> = [T] extends [string] ? `s:${T}` : `n:${T}`;

declare const value: Label<"a" | 1>;
value satisfies `n:${"a" | 1}`;
```

### infer in tuple decomposition preserves positional unions

Tuple-pattern inference should preserve positionally inferred unions for each captured slot.

```ts
type First<T> = T extends [infer A, ...unknown[]] ? A : never;

declare const value: First<["a", 1] | ["b", 2]>;
value satisfies "a" | "b";
```

### repeated infer bindings require compatible matches

When the same `infer` variable appears twice, incompatible captures should collapse that branch to `never`.

```ts
type Repeat<T> = T extends `${infer A}-${infer A}` ? A : never;

declare const value: Repeat<"x-y">;
value satisfies "x";
```

- contains: not assignable

## ds tuple conditionals

### ds tuple infer extracts first tuple slot across unions

In `.ds`, tuple-pattern extraction should pull the first element type from each union member before rejoining.

```ds
type FirstSlot<T> = T extends (infer A, infer B) ? A : never;

declare const value: FirstSlot<(string, int32) | (boolean, int32)>;
value satisfies string | boolean;
```

### ds wrapped tuple conditionals disable distribution

In `.ds`, tuple-wrapped conditionals should disable union distribution and compare the full union at once.

```ds
type NonDist<T> = [T] extends [(string, int32)] ? "pair" : "other";

declare const value: NonDist<(string, int32) | (boolean, int32)>;
value satisfies "other";
```

### ds repeated tuple infer bindings reject incompatible tuple slots

In `.ds`, repeated tuple captures should reject unions where corresponding tuple slots do not unify.

```ds
type SameSlots<T> = T extends (infer A, infer A) ? A : never;

let bad: SameSlots<(string, int32)> = true;
```

- contains: not assignable

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

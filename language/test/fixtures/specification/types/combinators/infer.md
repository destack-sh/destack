# Conditional Infer Edges

## repeated bindings

### repeated tuple infer bindings require equal members

> A repeated `infer A` in tuple positions should unify both slots to one compatible type.

```ts
type EqualPair<T> = T extends [infer A, infer A] ? A : never;

declare const value: EqualPair<[1, 1]>;
value satisfies 1;
```

### repeated tuple infer bindings reject mismatched members

> If repeated tuple slots disagree, the conditional branch should fail instead of widening to a loose match.

```ts
type EqualPair<T> = T extends [infer A, infer A] ? A : never;

declare const value: EqualPair<[1, 2]>;
value satisfies 1;
```

- contains: not assignable

## nested infer

### nested function infer captures both argument and return spans

> Function-pattern inference should bind both the parameter slot and the return slot in one pass.

```ts
type SignatureSlots<T> = T extends (value: infer A) => infer R ? [A, R] : never;

declare const value: SignatureSlots<(value: "a" | "b") => "x" | "y">;
value[0] satisfies "a" | "b";
value[1] satisfies "x" | "y";
```

### recursive template infer collects path segments

> Recursive template decomposition should accumulate path segments in left-to-right declaration order.

```ts
type Segments<T> = T extends `${infer A}/${infer B}` ? [A, ...Segments<B>] : [T];

declare const value: Segments<"a/b/c">;
value[0] satisfies "a";
value[2] satisfies "c";
```

## distribution control

### distributive and wrapped conditional forms remain distinct

> Wrapping the subject in a tuple should disable distributivity and evaluate the union as one relation.

```ts
type Dist<T> = T extends string ? `s:${T}` : never;
type NonDist<T> = [T] extends [string] ? `s:${T}` : never;

declare const distributed: Dist<"a" | "b">;
distributed satisfies "s:a" | "s:b";

declare const nondistributed: NonDist<"a" | "b">;
nondistributed satisfies `s:${"a" | "b"}`;
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

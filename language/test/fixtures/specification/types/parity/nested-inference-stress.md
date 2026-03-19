# Nested Inference Stress

## tuple and callback precision

### nested generic callback keeps const tuple element precision

> Nested callback inference should preserve const tuple literal element precision.

```ts
declare function withReader<T>(
    value: T,
    callback: (read: () => T) => void,
): void;

const tuple = [1, 2] as const;

withReader(tuple, read => {
    const current = read();
    current[0] satisfies 1;
    current[1] satisfies 2;
});
```

### nested generic callback does not keep mutable tuple element literals

> Nested callback inference should not preserve mutable tuple element literal precision.

```ts
declare function withReader<T>(
    value: T,
    callback: (read: () => T) => void,
): void;

let tuple = [1, 2];

withReader(tuple, read => {
    const current = read();
    current[0] satisfies 1;
});
```

- contains: not assignable

## overload and contextual routing

### contextual callback routing picks compatible overload signatures

> Contextual callback routing should pick compatible overload signatures in nested call paths.

```ts
declare function choose(input: string): string;
declare function choose(input: number): number;

declare function run(callback: (input: string) => string): string;

const selected = run((input: string) => choose(input));
selected satisfies string;
```

### contextual callback routing rejects incompatible overload expectations

> Contextual callback routing should reject incompatible overload expectations in nested call paths.

```ts
declare function choose(input: string): string;
declare function choose(input: number): number;

declare function run(callback: (input: string) => string): string;

const selected = run((input: string) => choose(input));
selected satisfies number;
```

- contains: not assignable

## module routing

### nested callback inference remains precise through renamed re-exports

> Renamed re-export routes should preserve nested callback tuple precision.

```ts:api.ts
export declare function withReader<T>(
    value: T,
    callback: (read: () => T) => void,
): void;
```

```ts:index.ts
export { withReader as useReader } from "./api";
```

```ts:main.ts
import { useReader } from "./index";

const tuple = [1, 2] as const;

useReader(tuple, read => {
    const current = read();
    current[0] satisfies 1;
    current[1] satisfies 2;
});
```

### nested callback inference through renamed re-exports does not keep mutable literals

> Renamed re-export routes should still widen mutable tuple element types.

```ts:api.ts
export declare function withReader<T>(
    value: T,
    callback: (read: () => T) => void,
): void;
```

```ts:index.ts
export { withReader as useReader } from "./api";
```

```ts:main.ts
import { useReader } from "./index";

let tuple = [1, 2];

useReader(tuple, read => {
    const current = read();
    current[0] satisfies 1;
});
```

- contains: not assignable

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

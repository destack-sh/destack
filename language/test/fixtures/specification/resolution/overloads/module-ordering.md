# Overload Module Ordering

## function routing through barrels

### renamed barrel exports keep first applicable overload

> Overload choice through renamed barrel exports should still obey declaration order and pick the first match.

```ts:helper.ts
export declare function choose(value: string): "string";
export declare function choose(value: number): "number";
```

```ts:barrel.ts
export * from "./helper";
```

```ts:index.ts
export { choose as routedChoose } from "./barrel";
```

```ts:main.ts
import { routedChoose } from "./index";

const value = routedChoose("ok");
value satisfies "string";
```

### renamed barrel exports do not select later overload results

> A later overload routed through the same barrel must not override a successful earlier candidate.

```ts:helper.ts
export declare function choose(value: string): "string";
export declare function choose(value: number): "number";
```

```ts:barrel.ts
export * from "./helper";
```

```ts:index.ts
export { choose as routedChoose } from "./barrel";
```

```ts:main.ts
import { routedChoose } from "./index";

const value = routedChoose("ok");
value satisfies "number";
```

- contains: not assignable

## namespace routing

### namespace imports preserve declaration order from re-exports

> Namespace imports from re-export chains should preserve original declaration ordering for overload resolution.

```ts:helper.ts
export declare function choose(value: string): "string";
export declare function choose(value: number): "number";
```

```ts:index.ts
export * from "./helper";
```

```ts:main.ts
import * as api from "./index";

const value = api.choose("ok");
value satisfies "string";
```

### contextual callback overloads remain declaration ordered through barrels

> Contextual callback overload resolution should keep declaration ordering even when symbols come through barrels.

```ts:helper.ts
export declare function drive(callback: (value: string) => "string"): "ok";
export declare function drive(callback: (value: number) => "number"): "num";
```

```ts:barrel.ts
export * from "./helper";
```

```ts:main.ts
import { drive } from "./barrel";

const value = drive(current => current);
value satisfies "ok";
```

```json:destack.json
{ "compiler": { "allowTs": true, "checkTs": true } }
```

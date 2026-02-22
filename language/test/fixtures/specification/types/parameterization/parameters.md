# Type Parameter Modifiers

## const type parameters

### const modifiers are not allowed on type aliases

> Type alias parameters cannot use the const modifier.

```ts
type Bad<const T> = T;
```

- contains: invalid type parameter modifier

### in modifiers are allowed on type aliases

> Type alias parameters can use the `in` variance modifier.

```ts
type Sink<in T> = (value: T) => void;

declare const sink: Sink<string>;
sink satisfies (value: string) => void;
```

### out modifiers are allowed on type aliases

> Type alias parameters can use the `out` variance modifier.

```ts
type Source<out T> = () => T;

declare const source: Source<string>;
source() satisfies string;
```

### const modifiers are allowed on functions

> Function type parameters can use the const modifier.

```ts
declare function id<const T>(value: T): T;

const value = id("ready");
value satisfies "ready";
```

### const function parameters preserve tuple literal precision

> Const generic function parameters preserve tuple literal precision.

```ts
declare function id<const T>(value: T): T;

const tuple = id([1, 2]);
tuple[0] satisfies 1;
```

### non-const function parameters widen tuple literal arguments

> Non-const generic parameters should not preserve tuple literal element precision.

```ts
declare function id<T>(value: T): T;

const tuple = id([1, 2]);
tuple[0] satisfies 1;
```

- contains: not assignable

### const function parameters preserve tuple literal precision through renamed re-exports

> Renamed re-exports should preserve const generic tuple literal precision.

```ts:helper.ts
export declare function id<const T>(value: T): T;
```

```ts:index.ts
export { id as stableId } from "./helper";
```

```ts:main.ts
import { stableId } from "./index";

const tuple = stableId([1, 2]);
tuple[0] satisfies 1;
```

### const function parameters preserve tuple literal precision through export-star barrels

> Export-star barrels should preserve const generic tuple literal precision.

```ts:helper.ts
export declare function id<const T>(value: T): T;
```

```ts:index.ts
export * from "./helper";
```

```ts:main.ts
import { id } from "./index";

const tuple = id([1, 2]);
tuple[0] satisfies 1;
```

### const function parameters preserve object literal property precision

> Const generic function parameters should preserve object literal property precision.

```ts
declare function id<const T>(value: T): T;

const value = id({ kind: "ready", level: 1 });
value.kind satisfies "ready";
value.level satisfies 1;
```

### non-const function parameters widen object literal property precision

> Non-const generic function parameters should widen object literal property precision.

```ts
declare function id<T>(value: T): T;

const value = id({ kind: "ready", level: 1 });
value.kind satisfies "ready";
```

- contains: not assignable

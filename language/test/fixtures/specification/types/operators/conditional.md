# Conditional Types

## tests

### conditional types pick true branch

> Conditional types select the matching branch.

```ds
type Select<T> = T extends string ? string : int32;

let ok: Select<string> = "ok";
```

### conditional types reject the opposite branch

> Conditional types reject values from the other branch.

```ds
type Select<T> = T extends string ? string : int32;

let bad: Select<string> = 1;
```

- contains: type 1 is not assignable to type select<<type>>

### conditional types pick false branch

> Conditional types select the else branch when the match fails.

```ds
type Select<T> = T extends string ? string : int32;

let ok: Select<int32> = 1;
```

### conditional types reject the true branch for non matches

> Non matching inputs reject the true branch.

```ds
type Select<T> = T extends string ? string : int32;

let bad: Select<int32> = "no";
```

- contains: type "no" is not assignable to type select<<type>>

### conditional types distribute over unions

> Conditional types distribute over union inputs.

```ds
type OnlyStrings<T> = T extends string ? T : never;

let ok: OnlyStrings<string | int32> = "ok";
```

### conditional types reject non matching union members

> Conditional types filter out non matching union members.

```ds
type OnlyStrings<T> = T extends string ? T : never;

let bad: OnlyStrings<string | int32> = 1;
```

- contains: not assignable to type OnlyStrings<<type>>

### conditional types with any yield union branches

> `any` produces the union of both branches.

```ds
type Select<T> = T extends string ? "yes" : "no";

type AnySelect = Select<any>;

const ok: AnySelect = "yes";
const ok2: AnySelect = "no";
```

```json:dsconfig.json
{ "compilerOptions": { "noAny": false } }
```

### conditional types with any reject non members

> `any` results still require branch members.

```ds
type Select<T> = T extends string ? "yes" : "no";

type AnySelect = Select<any>;

const bad: AnySelect = 1;
```

```json:dsconfig.json
{ "compilerOptions": { "noAny": false } }
```

- contains: not assignable

### conditional types with unknown select else branch

> `unknown` selects the false branch.

```ds
type Select<T> = T extends string ? "yes" : "no";

let ok: Select<unknown> = "no";
```

### conditional types with unknown reject true branch

> `unknown` rejects the true branch.

```ds
type Select<T> = T extends string ? "yes" : "no";

let bad: Select<unknown> = "yes";
```

- contains: not assignable

### conditional types treat never as empty unions

> `never` yields `never` in conditional types.

```ds
type OnlyStrings<T> = T extends string ? T : never;
type Result = OnlyStrings<never>;

let bad: Result = "no";
```

- contains: not assignable

### conditional types disable distribution with tuples

> Wrapping types disables distributive behavior.

```ds
type Wrapped<T> = [T] extends [string] ? "yes" : "no";

let ok: Wrapped<string | int32> = "no";
```

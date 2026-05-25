# Static If Modules

Imports and re-exports can be gated with `@if` when the condition is load-static.

## imports

### when false, `@if` removes import edges

When false, an import statement does not add a dependency edge.

```ds:main.ds
@if(false)
import { missing } from "./missing.ds";

const value = 1;
```

### when false, `@if` removes import items

When every import item is removed, the import statement does not add a dependency edge.

```ds:main.ds
import { @if(false) missing } from "./missing.ds";

const value = 1;
```

### when true, `@if` keeps import items

When any import item remains, the import statement keeps its dependency edge.

```ds:dep.ds
export const visible = 1;
```

```ds:main.ds
import { @if(false) missing, @if(true) visible } from "./dep.ds";

visible satisfies number;
```

## re-exports

### when false, `@if` removes re-export edges

When false, a re-export statement does not add a dependency edge.

```ds:main.ds
@if(false)
export { missing } from "./missing.ds";

export const value = 1;
```

### when false, `@if` removes re-export items

When every re-export item is removed, the re-export statement does not add a dependency edge.

```ds:main.ds
export { @if(false) missing } from "./missing.ds";

export const value = 1;
```

### when true, `@if` keeps re-export items

When any re-export item remains, the re-export statement keeps its dependency edge.

```ds:dep.ds
export const visible = 1;
```

```ds:main.ds
export { @if(false) missing, @if(true) visible } from "./dep.ds";
```

```ds:user.ds
import { visible } from "./main.ds";

visible satisfies number;
```

## globals

### when false, `@if` removes imports inside global blocks

An import inside `global` follows the same load-static gating rules.

```ds:main.ds
global {
    @if(false)
    import { missing } from "./missing.ds";
}

const value = 1;
```

### when false, `@if` removes global re-exports

A re-export inside `global` follows the same load-static gating rules.

```ds:main.ds
global {
    @if(false)
    export { missing } from "./missing.ds";
}

const value = 1;
```

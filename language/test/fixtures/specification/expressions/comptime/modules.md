# Comptime Modules

## module-level comptime blocks

### module-level comptime block is allowed

> Module-level comptime blocks are valid declarations.

```ds
comptime {
    let size = 4;
    let _ = size + 1;
}

const value: int32 = 1;
value satisfies int32;
```

### module-level comptime block can appear after declarations

> Comptime blocks can appear after other top-level items.

```ds
const base: int32 = 2;

comptime {
    let value = base + 1;
    let _ = value;
}

const next = base + 1;
next satisfies int32;
```

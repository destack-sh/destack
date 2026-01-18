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

# Comptime Expressions

Compile time evaluation of expressions and blocks.

## Coverage

- **Expression form**: `comptime 1 + 2`
- **Block form**: `comptime { ... }`
- **Member blocks**: comptime blocks inside structs and classes
- **Module blocks**: module level comptime execution
- **Comptime conditions**: `if (comptime ...)` branches
- **Slots**: comptime values used as constants

## Example

```ds
const value: int32 = comptime 1 + 2;
const table: uint8[] = comptime {
    let t: uint8[] = [];
    t
};
```

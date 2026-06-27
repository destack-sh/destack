# Static If Statements

Statements can be gated with `@if`.

## gating

### when false, `@if` removes module statements

When false, `@if` removes the statement.

```ds
@if(false)
missingSymbol;

const value = 1;
```

### when true, `@if` includes module statements

When true, `@if` includes the statement.

```ds
@if(true)
const value = 1;

value satisfies number;
```

### when false, `@if` removes block statements

When false, `@if` removes the statement inside a block.

```ds
function demo(): number {
    @if(false)
    missingSymbol;

    return 1;
}
```

### when true, `@if` includes block statements

When true, `@if` includes the statement inside a block.

```ds
function demo(): number {
    let value: number = 0;

    @if(true)
    value = 1;

    return value;
}
```

### when false, `@if` removes block declarations

When false, `@if` removes a local declaration.

```ds
function compute(): number {
    @if(false)
    const hidden = missingSymbol;

    return 1;
}
```

## generic

### generic-dependent statements use comptime conditions

Generic-dependent branches are ordinary checked code and may be erased after specialization.

```ds
function size<comptime Wide: bool>(): Wide extends true ? 8 : 4 {
    if (comptime Wide) {
        return 8;
    } else {
        return 4;
    }
}

size<true>() satisfies 8;
size<false>() satisfies 4;
```

### generic-dependent @if is rejected

`@if` cannot remove statements from each generic instantiation.

```ds
function demo<comptime Enabled: bool>(): void {
    @if(Enabled)
    const value = 1;

    value;
}
```

- contains: static @if condition must be statically decidable

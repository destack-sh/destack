# Hover

## Basic

### Local function signature

Hover should show the exact signature for a local function use site.

```ds:main.ds
function /*def*/greet(name: string): string {
    return name;
}

const message = [|/*hover*/greet|]("World");
```

```lsp hover_signature
function greet(name: string): string
```

## Presence

### Quick info is present at the caret

Hover should return one payload when the caret sits on a value name.

```ds:main.ds
const /*hover_exists*/value = 1;
```

```lsp quick_info hover_exists [0]
```

```lsp hover_signature
let value: 1
```

## Repeated Lookups

### Multiple markers share one signature

Hover should return the same normalized quick info at each marked call site.

```ds:main.ds
function greet(name: string): string {
    return name;
}

const first = /*first*/greet("World");
const second = /*second*/greet("Destack");
```

```lsp quick_info first [0]
```

```lsp quick_info second [0]
```

```lsp hover_signature
function greet(name: string): string
```

## Quick info stability

### Keep quick info stable after inserting an unrelated helper
Quick info should stay the same after unrelated declarations are inserted above the use site.

```ds:main.ds
function greet(name: string): string {
   return name;
}
const message = /*hover*/greet("World");
```

```ds:main.ds[1]
function helper(): void {
}
function greet(name: string): string {
   return name;
}
const message = /*hover*/greet("World");
```

```lsp quick_info hover [0]
```

```lsp quick_info hover [1]
```

```lsp hover_signature
function greet(name: string): string
```

### Keep quick info present after moving a binding down
Quick info presence should survive line shifts when the binding stays semantically valid.

```ds:main.ds
const /*hover_exists*/value = 1;
```

```ds:main.ds[1]
function helper(): int32 {
   return 0;
}
const /*hover_exists*/value = 1;
```

```lsp quick_info hover_exists [0]
```

```lsp quick_info hover_exists [1]
```

```lsp hover_signature
let value: 1
```

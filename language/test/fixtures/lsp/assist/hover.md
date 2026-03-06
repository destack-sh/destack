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

```lsp scenario quick-info-exists
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

```lsp scenario quick-infos
```

```lsp hover_signature
function greet(name: string): string
```


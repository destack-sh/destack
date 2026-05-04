# Function Annotations

Function annotations are resolved expressions on function-like declarations and supported statement forms.

## metadata

### function metadata preserves the call signature

> Metadata on a function does not wrap or replace the function.

```ds
newtype Label = { value: string };

function label(value: string): Label {
    Label({ value })
}

@label("filesystem")
function openFile(): int32 {
    return 1;
}

openFile() satisfies int32;
openFile satisfies () => int32;
```

### unresolved function annotations are rejected

> The expression after `@` must resolve.

```ds
@missing
function demo(): void {}
```

- contains: missing

## rewrites

### capture annotates closure capture policy

> `@capture` is a compiler-known rewrite annotation for function-like targets.

```ds
@capture("byValue")
function make() {
    return () => 1;
}
```

### unroll annotates counted loops

> `@unroll` is a compiler-known rewrite annotation for loop targets.

```ds
@unroll
for (let i = 0; i < 4; i++) {
    let value = i;
}
```

## targets

### parameter annotations are rejected

> Portable `.ds` does not support parameter annotations.

```ds
newtype Label = { value: string };

function label(value: string): Label {
    Label({ value })
}

function greet(@label("input") name: string): string {
    return name;
}
```

- contains: parameter

# Function Annotations

## Functions

### parameter annotation stays attached

Parameter annotations stay attached to the parameter.

```ds
function process(@nonempty input: string) { return input }
```

```ds expected
function process(@nonempty input: string) {
    return input;
}
```

### return type decorator annotation stays inline

Decorator prefixed return types stay attached after `:`.

```ds
function build(value: Buffer): @addrspace("shared") &Buffer { return value }
```

```ds expected
function build(value: Buffer): @addrspace("shared") &Buffer {
    return value;
}
```

### TypeScript return type decorator annotation stays inline

Decorator prefixed return types stay attached after `:` under non-default formatter options.

```ts:main.ts indent-width=2 line-width=80 quote-style=double
{
    function build(value: Buffer): @addrspace("shared") &Buffer { return value; }
}
```

```ts expected
{
  function build(value: Buffer): @addrspace("shared") &Buffer {
    return value;
  }
}
```

### method body boundary comment

Comments between method signatures and bodies stay at the boundary.

```ts:main.ts
class Box {
  run(): number // method-body
  {
    return 1
  }
}
```

```ts expected
class Box {
    run(): number {
        // method-body
        return 1;
    }
}
```

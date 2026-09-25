# Function Annotations

## Functions

### parameter annotation stays attached

Parameter annotations stay attached to the parameter.

```tspp
function process(@nonempty input: string) { return input }
```

```tspp expected
function process(@nonempty input: string) {
    return input;
}
```

### return type decorator annotation stays inline

Decorator prefixed return types stay attached after `:`.

```tspp
function build(value: Buffer): @addrspace("shared") &Buffer { return value }
```

```tspp expected
function build(value: Buffer): @addrspace("shared") &Buffer {
    return value;
}
```

### return type decorator annotation stays inline under non-default options

Decorator prefixed return types stay attached after `:` under non-default formatter options.

```tspp:main.tspp indent-width=2 line-width=80
{
    function build(value: Buffer): @addrspace("shared") &Buffer { return value; }
}
```

```tspp expected
{
  function build(value: Buffer): @addrspace("shared") &Buffer {
    return value;
  }
}
```

### method body boundary comment

Comments between method signatures and bodies stay at the boundary.

```tspp:main.tspp
class Box {
  run(): number // method-body
  {
    return 1
  }
}
```

```tspp expected
class Box {
    run(): number {
        // method-body
        return 1;
    }
}
```

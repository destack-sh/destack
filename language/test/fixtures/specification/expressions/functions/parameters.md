# Dynamic Parameters

Dynamic parameter passing and defaults.

## dynamic parameters

### positional arguments map to parameters

Positional arguments flow into parameters by order.

```ds
function sum(a: number, b: number): number {
    return a + b;
}

const value = sum(1, 2);
value satisfies number;
```

### defaulted parameters are optional

Parameters with defaults can be omitted at call sites.

```ds
function greet(name: string = "hi"): string {
    return name;
}

greet() satisfies string;
greet("hello") satisfies string;
```

### defaulted parameter types are enforced

Arguments still satisfy the declared parameter type.

```ds
function repeat(value: string = "hi"): string {
    return value;
}

repeat("ok");
repeat(1);
```

- contains: is not assignable

### tuple rest parameters infer element types

Tuple rest parameters preserve element types from call sites.

```ds
function pair<T, U>(...values: (T, U)): (T, U) {
    return (values[0], values[1]);
}

const result = pair(1, "hi");
result satisfies (int, string);
```

## parameter property rejections

### function parameters cannot be parameter properties

Parameter property modifiers are only allowed in constructors.

```ds
function build(public value: number) {}
```

- contains: parameter property

## optional parameter rejections

### optional pattern parameters are rejected

Optional parameters must use identifiers, not binding patterns.

```ds:main.ds
interface Payload {
    value: string;
}

function handle({ value }?: Payload) {
    let _ = value;
}
```

- contains: optional parameters cannot use binding patterns

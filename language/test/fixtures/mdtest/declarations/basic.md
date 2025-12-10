# Basic Functions

Tests for function declarations and type checking.

## Function Declarations

### function with no parameters

> Functions can be declared with no parameters.

```ds
function f(): void {}
```

### function with parameters

> Functions can have typed parameters.

```ds
function add(a: number, b: number): number {
    return a + b;
}
```

### function with return type

> Functions specify their return type after the parameter list.

```ds
function greet(name: string): string {
    return "hello " + name;
}
```

## Arrow Functions

### arrow with no parameters

> Arrow functions can be declared with no parameters.

```ds
const f = (): void => {};
f satisfies () => void;
```

### arrow with parameters

> Arrow functions can have typed parameters.

```ds
const add = (a: number, b: number): number => a + b;
add satisfies (a: number, b: number) => number;
```

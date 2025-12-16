# Hover

## Functions

### Hover over function name

Hovering over a function reference should display its signature.

```ds
function greet(name: string): string {
//       ^^^^^ def:greet
    return "Hello, " + name;
}

const msg = greet("World");
//          ^^^^^ use:greet
```

Hovering over `greet` at the call site should show "function greet".

```query hover use:greet
function greet
```

## Structs

### Hover over struct name

Hovering over a struct type reference should display its kind.

```ds
struct Point {
//     ^^^^^ def:Point
    x: float32,
    y: float32,
}

const p: Point = Point { x: 1, y: 2 };
//       ^^^^^ use:Point
```

Hovering over `Point` in the type annotation should show "struct Point".

```query hover use:Point
struct Point
```

## Classes

### Hover over class name

Hovering over a class type reference should display its kind.

```ds
class Animal {
//    ^^^^^^ def:Animal
    name: string
}

const a: Animal = new Animal();
//       ^^^^^^ use:Animal
```

Hovering over `Animal` in the type annotation should show "class Animal".

```query hover use:Animal
class Animal
```

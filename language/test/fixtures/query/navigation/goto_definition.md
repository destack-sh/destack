# Goto Definition

## Local Variables

### Local variable definition

Goto definition on a variable reference should jump to its declaration.

```ds
const foo = 1;
//    ^^^ def:foo

const bar = foo;
//          ^^^ use:foo
```

The `foo` in `const bar = foo` references the `foo` defined above, so goto definition should navigate from `use:foo` to `def:foo`.

```query goto_definition use:foo
def:foo
```

### Function parameter definition

Goto definition on a parameter reference should jump to the parameter declaration.

```ds
function add(x: int32, y: int32): int32 {
//           ^ def:x
    return x + y;
//         ^ use:x
}
```

The `x` in `return x + y` references the parameter `x` in the function signature, so goto definition should navigate from `use:x` to `def:x`.

```query goto_definition use:x
def:x
```

## Functions

### Function definition

Goto definition on a function call should jump to the function declaration.

```ds
function greet(name: string): string {
//       ^^^^^ def:greet
    return "Hello, " + name;
}

const msg = greet("World");
//          ^^^^^ use:greet
```

`greet("World")` calls the `greet` function defined above, so goto definition should navigate from `use:greet` to `def:greet`.

```query goto_definition use:greet
def:greet
```

## Struct Fields

### Goto definition on struct field access

Goto definition on a field access should jump to the field declaration.

```ds
struct Point {
    x: int32
//  ^ def:point_x
    y: int32
}

function main() {
    const p = Point { x: 1, y: 2 };
    const value = p.x;
//                   ^ use:point_x
}
```

Goto definition from `p.x` should navigate to the `x` field in `Point`.

```query goto_definition use:point_x
def:point_x
```

## Class Methods

### Goto definition on class method call

Goto definition on a class method call should jump to the method declaration.

```ds
class Logger {
    log(message: string): void {
//  ^^^ def:logger_log
        print(message);
    }
}

function main() {
    const logger = new Logger();
    logger.log("hello");
//         ^^^ use:logger_log
}
```

Goto definition from `logger.log` should navigate to the method definition.

```query goto_definition use:logger_log
def:logger_log
```

## Extension Methods

### Goto definition on extension method call

Goto definition on an extension method call should resolve to the extension method definition.

```ds
struct Calculator {}

extension for Calculator {
    add(x: int32, y: int32): int32 {
//  ^^^ def:calc_add
        return x + y;
    }
}

function main() {
    const calc = Calculator {};
    const result = calc.add(1, 2);
//                        ^ use:calc_add
}
```

Goto definition from `calc.add` should navigate to the extension method definition.

```query goto_definition use:calc_add
def:calc_add
```

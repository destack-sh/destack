# Find References

## Local Variables

### Find references to local variable

Find references should return all occurrences of a symbol, including its definition.

```ds
const foo = 1;
//    ^^^ def:foo

const bar = foo;
//          ^^^ use:foo
const baz = foo + foo;
```

`foo` is defined once and used 3 times (in `bar`, and twice in `baz`), so the total is 4 references.

```query find_references def:foo
main.ds:1:7-1:10
main.ds:3:13-3:16
main.ds:4:13-4:16
main.ds:4:19-4:22
```

## Functions

### Find references to function

Find references on a function should return its definition and all call sites.

```ds
function greet(name: string): string {
//       ^^^^^ def:greet
    return "Hello, " + name;
}

const a = greet("World");
//        ^^^^^ use:greet
const b = greet("Test");
```

`greet` is defined once and called twice (in `a` and `b`), so the total is 3 references.

```query find_references def:greet
main.ds:1:10-1:15
main.ds:5:11-5:16
main.ds:6:11-6:16
```

## Methods

### Find references to class method

Find references should include the method definition and all call sites.

```ds
class Calculator {
    add(a: int32, b: int32): int32 {
//  ^^^ def:calc_add
        return a + b;
    }
}

function main() {
    const calc = new Calculator();
    const first = calc.add(1, 2);
//                     ^^^ use:calc_add_1
    const second = calc.add(3, 4);
//                      ^^^ use:calc_add_2
}
```

`add` appears 3 times: its definition and two calls.

```query find_references def:calc_add
main.ds:2:5-2:8
main.ds:9:24-9:27
main.ds:10:25-10:28
```

## Struct Fields

### Find references to a struct field

Find references should include the field definition and all field accesses.

```ds
struct Point {
    x: int32,
//  ^ def:field_x
    y: int32,
}

function main(p: Point) {
    const a = p.x;
//              ^ use:field_x_1
    const b = p.x + p.x;
//              ^ use:field_x_2
//                    ^ use:field_x_3
}
```

`x` appears 4 times for this symbol: its definition and three accesses.

```query find_references def:field_x
main.ds:2:5-2:6
main.ds:7:17-7:18
main.ds:8:17-8:18
main.ds:8:23-8:24
```

## Cross Module

### Find references across modules

Find references should include imports and call sites in other modules.

```ds:lib.ds
export function ping(): void {}
//              ^^^^ def:ping
```

```ds:main.ds
import { ping } from "./lib.ds";
//       ^^^^ use:ping_import

ping();
// ^^^^ use:ping_call
```

```query find_references def:ping
lib.ds:1:17-1:21
main.ds:1:10-1:14
main.ds:3:1-3:5
```

## Re-Exports

### Find references through re-exported aliases

Find references should include re-exported aliases and downstream imports.

```ds:alias_base.ds
export function ping(): void {}
//              ^^^^ def:ping
```

```ds:alias_barrel.ds
export { ping as pingAlias } from "./alias_base.ds";
```

```ds:alias_main.ds
import { pingAlias } from "./alias_barrel.ds";

pingAlias();
```

```query find_references def:ping
alias_base.ds:1:17-1:21
alias_barrel.ds:1:10-1:14
alias_main.ds:1:10-1:19
alias_main.ds:3:1-3:10
```

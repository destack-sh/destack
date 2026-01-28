# Prepare Rename

## Local Variables

### Prepare rename local variable

Prepare rename should return the current name as placeholder for local variables.

```ds
const foo = 1;
//    ^^^ def:foo

const bar = foo + 2;
//          ^^^ ref:foo
```

When preparing to rename `foo` at its definition, we should get `foo` as the placeholder.

```query prepare_rename def:foo
foo
```

### Prepare rename at reference

We can also prepare rename from a reference site.

```ds
const value = 42;
//    ^^^^^ def:value

const result = value * 2;
//             ^^^^^ ref:value
```

Preparing rename at a reference should also return the current name.

```query prepare_rename ref:value
value
```

## Non-Renameable

### Prepare rename on literal

Prepare rename should fail on literals.

```ds
const value = 42;
//            ^^ literal
```

Preparing rename on a literal should return no result.

```query prepare_rename literal
<none>
```

## Members

### Prepare rename on struct field access

Prepare rename should resolve struct field accesses.

```ds
struct Point {
    x: int32,
    y: int32,
}

function main() {
    const p = Point { x: 1, y: 2 };
    const value = p.x;
//                  ^ use:point_x
}
```

Preparing rename at `p.x` should return `x`.

```query prepare_rename use:point_x
x
```

### Prepare rename on method call

Prepare rename should resolve method calls.

```ds
class Counter {
    value: int32,
    inc(): int32 {
        return this.value + 1;
    }
}

function main() {
    const counter = new Counter();
    const next = counter.inc();
//                       ^^^ use:counter_inc
}
```

Preparing rename at `counter.inc` should return `inc`.

```query prepare_rename use:counter_inc
inc
```

### Prepare rename on enum member

Prepare rename should resolve enum member references.

```ds
enum Status {
    Pending,
    Active,
}

const state = Status.Pending;
//                     ^ use:pending
```

```query prepare_rename use:pending
Pending
```

## Imports

### Prepare rename on type-only import usage

Prepare rename should work for type-only imports.

```ds:types.ds
export type Options = {
//          ^^^^^^^ def:Options
    name: string,
};
```

```ds:main.ds
import type { Options } from "./types.ds";

function configure(options: Options): void {
//                          ^^^^^^^ use:Options
    console.log(options.name);
}
```

Preparing rename at a type-only import usage should return the imported type name.

```query prepare_rename use:Options
Options
```

### Prepare rename on default import usage

Prepare rename should work for default imports.

```ds:default_export.ds
export default function greetDefault(name: string): string {
    return "Hello, " + name;
}
```

```ds:default_main.ds
import greetDefault from "./default_export.ds";

const message = greetDefault("Destack");
//              ^^^^^^^^^^^^ use:default_greet
```

```query prepare_rename use:default_greet
greetDefault
```

## Keywords

### Prepare rename on keyword

Prepare rename should fail on keywords.

```ds
function main(): void {
    return 1;
//  ^^^^^^ keyword:return
}
```

Preparing rename on a keyword should return no result.

```query prepare_rename keyword:return
<none>
```

# Goto Definition

## Local Variables

### Local variable definition

```ds
const foo = 1;
//    ^^^ def:foo

const bar = foo;
//          ^^^ use:foo
```

```query goto_definition use:foo
def:foo
```

### Function parameter definition

```ds
function add(x: int32, y: int32): int32 {
//           ^ def:x
    return x + y;
//         ^ use:x
}
```

```query goto_definition use:x
def:x
```

## Functions

### Function definition

```ds
function greet(name: string): string {
//       ^^^^^ def:greet
    return "Hello, " + name;
}

const msg = greet("World");
//          ^^^^^ use:greet
```

```query goto_definition use:greet
def:greet
```

## Cross-file

### Imported function definition

```ds:lib.ds
export function helper(): void {}
//              ^^^^^^ def:helper
```

```ds:main.ds
import { helper } from "./lib";
//       ^^^^^^ use:helper

helper();
```

```query goto_definition use:helper
def:helper
```

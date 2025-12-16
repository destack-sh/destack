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

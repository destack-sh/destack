
## Local Variables

### Highlight local variable occurrences

Highlights distinguish writes from reads of one local symbol.

```tspp main.tspp
let foo = 1;
    ^^^ occurrence:definition

foo = 2;
^^^ occurrence:write
const bar = foo + foo;
            ^^^ occurrence:first
                  ^^^ occurrence:second
```

```query highlight main.tspp#occurrence:definition
@highlight.range range=main.tspp#occurrence:definition kind=write
@highlight.range range=main.tspp#occurrence:write kind=write
@highlight.range range=main.tspp#occurrence:first kind=read
@highlight.range range=main.tspp#occurrence:second kind=read
```

### Highlight current references

Highlights include the current declaration and references.

```tspp main.tspp
const value = 1;
      ^^^^^ occurrence:definition
const first = value;
              ^^^^^ occurrence:first
```

```query highlight main.tspp#occurrence:first
@highlight.range range=main.tspp#occurrence:definition kind=write
@highlight.range range=main.tspp#occurrence:first kind=read
```

```tspp main.tspp change
const value = 1;
      ^^^^^ occurrence:definition
const first = value;
              ^^^^^ occurrence:first
const second = value;
               ^^^^^ occurrence:second
```

```query highlight main.tspp#occurrence:first
@highlight.range range=main.tspp#occurrence:definition kind=write
@highlight.range range=main.tspp#occurrence:first kind=read
@highlight.range range=main.tspp#occurrence:second kind=read
```

## Functions

### Highlight function occurrences

A function includes its declaration and every call.

```tspp main.tspp
function add(x: int32, y: int32): int32 {
         ^^^ occurrence:definition
    return x + y;
}

const result = add(1, 2) + add(3, 4);
               ^^^ occurrence:first
                           ^^^ occurrence:second
```

```query highlight main.tspp#occurrence:first
@highlight.range range=main.tspp#occurrence:definition kind=text
@highlight.range range=main.tspp#occurrence:first kind=text
@highlight.range range=main.tspp#occurrence:second kind=text
```

## Parameters

### Highlight parameter occurrences

A parameter includes its declaration and references.

```tspp main.tspp
function greet(name: string): string {
               ^^^^ occurrence:definition
    return name;
           ^^^^ occurrence:reference
}
```

```query highlight main.tspp#occurrence:reference
@highlight.range range=main.tspp#occurrence:definition kind=write
@highlight.range range=main.tspp#occurrence:reference kind=read
```

## Struct Fields

### Highlight struct field occurrences

A field includes its declaration and every access.

```tspp main.tspp
struct Point {
    x: int32;
    ^ occurrence:definition
    y: int32;
}

function main(point: Point) {
    const first = point.x;
                        ^ occurrence:first
    const second = point.x + point.x;
                         ^ occurrence:second
                                   ^ occurrence:third
}
```

```query highlight main.tspp#occurrence:first
@highlight.range range=main.tspp#occurrence:definition kind=text
@highlight.range range=main.tspp#occurrence:first kind=read
@highlight.range range=main.tspp#occurrence:second kind=read
@highlight.range range=main.tspp#occurrence:third kind=read
```

## Shadowing

### Highlight only one shadowed binding

Shadowed bindings remain separate symbols.

```tspp main.tspp
const value = 1;
      ^^^^^ occurrence:outer

function read(): int32 {
    const value = 2;
          ^^^^^ occurrence:inner_definition
    return value;
           ^^^^^ occurrence:inner_reference
}
```

```query highlight main.tspp#occurrence:inner_definition
@highlight.range range=main.tspp#occurrence:inner_definition kind=write
@highlight.range range=main.tspp#occurrence:inner_reference kind=read
```

## Methods

### Highlight method occurrences

A method includes its declaration and calls.

```tspp main.tspp
class Service {
    run(): void {}
    ^^^ occurrence:definition
}

function start(service: Service): void {
    service.run();
            ^^^ occurrence:first
    service.run();
            ^^^ occurrence:second
}
```

```query highlight main.tspp#occurrence:first
@highlight.range range=main.tspp#occurrence:definition kind=text
@highlight.range range=main.tspp#occurrence:first kind=text
@highlight.range range=main.tspp#occurrence:second kind=text
```

### Highlight every method reached through a union

A union member access combines the local occurrences of every reachable method.

```tspp main.tspp
class Alpha {
    run(): void {}
    ^^^ occurrence:alpha_definition
}

class Beta {
    run(): void {}
    ^^^ occurrence:beta_definition
}

function start(service: Alpha | Beta): void {
    service.run();
            ^^^ occurrence:reference
}
```

```query highlight main.tspp#occurrence:reference
@highlight.range range=main.tspp#occurrence:alpha_definition kind=text
@highlight.range range=main.tspp#occurrence:beta_definition kind=text
@highlight.range range=main.tspp#occurrence:reference kind=text
```

## Enum Members

### Highlight enum member occurrences

An enum member includes its declaration and accesses.

```tspp main.tspp
enum Color {
    Red,
    ^^^ occurrence:definition
}

const first = Color.Red;
                    ^^^ occurrence:first
const second = Color.Red;
                     ^^^ occurrence:second
```

```query highlight main.tspp#occurrence:first
@highlight.range range=main.tspp#occurrence:definition kind=text
@highlight.range range=main.tspp#occurrence:first kind=text
@highlight.range range=main.tspp#occurrence:second kind=text
```

## Import Aliases

### Highlight a local import alias

An explicit local alias remains separate from its exported symbol.

```tspp library.tspp
export function greet(): void {}
```

```tspp main.tspp
import { greet as welcome } from "./library.tspp";
                  ^^^^^^^ occurrence:definition

welcome();
^^^^^^^ occurrence:first
welcome();
^^^^^^^ occurrence:second
```

```query highlight main.tspp#occurrence:first
@highlight.range range=main.tspp#occurrence:definition kind=write
@highlight.range range=main.tspp#occurrence:first kind=read
@highlight.range range=main.tspp#occurrence:second kind=read
```

## Nominal Types

### Highlight nominal type occurrences

Type declarations and references are textual occurrences rather than value reads or writes.

```tspp main.tspp
class User {}
      ^^^^ occurrence:definition

declare const existing: User;
                        ^^^^ occurrence:type_reference
const created = new User();
                    ^^^^ occurrence:construction_reference
```

```query highlight main.tspp#occurrence:type_reference
@highlight.range range=main.tspp#occurrence:definition kind=text
@highlight.range range=main.tspp#occurrence:type_reference kind=text
@highlight.range range=main.tspp#occurrence:construction_reference kind=text
```

## Labels

### Highlight control label occurrences

A label declaration and its targeted breaks share one identity.

```tspp main.tspp
function choose(): int32 {
    outer: loop {
    ^^^^^ occurrence:definition
        break outer: 1;
              ^^^^^ occurrence:reference
    }
}
```

```query highlight main.tspp#occurrence:reference
@highlight.range range=main.tspp#occurrence:definition kind=text
@highlight.range range=main.tspp#occurrence:reference kind=text
```

## Empty Results

### Return no highlight for a literal

A literal has no symbol occurrences.

```tspp main.tspp
const value = 42;
              ^^ literal
```

```query highlight main.tspp#literal
@highlight.none
```

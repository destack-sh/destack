# Highlight

## Local Variables

### Highlight local variable occurrences

Highlights distinguish writes from reads of the selected local symbol.

```ds main.ds
let foo = 1;
    ^^^ occurrence:definition

foo = 2;
^^^ occurrence:write
const bar = foo + foo;
            ^^^ occurrence:first
                  ^^^ occurrence:second
```

```query highlight main.ds#occurrence:definition
@highlight.range range=main.ds#occurrence:definition kind=write
@highlight.range range=main.ds#occurrence:write kind=write
@highlight.range range=main.ds#occurrence:first kind=read
@highlight.range range=main.ds#occurrence:second kind=read
```

## Functions

### Highlight function occurrences

A selected function includes its declaration and every call.

```ds main.ds
function add(x: int32, y: int32): int32 {
         ^^^ occurrence:definition
    return x + y;
}

const result = add(1, 2) + add(3, 4);
               ^^^ occurrence:first
                           ^^^ occurrence:second
```

```query highlight main.ds#occurrence:first
@highlight.range range=main.ds#occurrence:definition kind=text
@highlight.range range=main.ds#occurrence:first kind=text
@highlight.range range=main.ds#occurrence:second kind=text
```

## Parameters

### Highlight parameter occurrences

A selected parameter includes its declaration and references.

```ds main.ds
function greet(name: string): string {
               ^^^^ occurrence:definition
    return name;
           ^^^^ occurrence:reference
}
```

```query highlight main.ds#occurrence:reference
@highlight.range range=main.ds#occurrence:definition kind=write
@highlight.range range=main.ds#occurrence:reference kind=read
```

## Struct Fields

### Highlight struct field occurrences

A selected field includes its declaration and every access.

```ds main.ds
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

```query highlight main.ds#occurrence:first
@highlight.range range=main.ds#occurrence:definition kind=text
@highlight.range range=main.ds#occurrence:first kind=read
@highlight.range range=main.ds#occurrence:second kind=read
@highlight.range range=main.ds#occurrence:third kind=read
```

## Shadowing

### Highlight only the selected shadow

Shadowed bindings remain separate symbols.

```ds main.ds
const value = 1;
      ^^^^^ occurrence:outer

function read(): int32 {
    const value = 2;
          ^^^^^ occurrence:inner_definition
    return value;
           ^^^^^ occurrence:inner_reference
}
```

```query highlight main.ds#occurrence:inner_definition
@highlight.range range=main.ds#occurrence:inner_definition kind=write
@highlight.range range=main.ds#occurrence:inner_reference kind=read
```

## Methods

### Highlight method occurrences

A selected method includes its declaration and calls.

```ds main.ds
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

```query highlight main.ds#occurrence:first
@highlight.range range=main.ds#occurrence:definition kind=text
@highlight.range range=main.ds#occurrence:first kind=text
@highlight.range range=main.ds#occurrence:second kind=text
```

### Highlight every exact method selected through a union

A union member access combines the local occurrence sets of each selected method.

```ds main.ds
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

```query highlight main.ds#occurrence:reference
@highlight.range range=main.ds#occurrence:alpha_definition kind=text
@highlight.range range=main.ds#occurrence:beta_definition kind=text
@highlight.range range=main.ds#occurrence:reference kind=text
```

## Enum Members

### Highlight enum member occurrences

A selected enum member includes its declaration and accesses.

```ds main.ds
enum Color {
    Red,
    ^^^ occurrence:definition
}

const first = Color.Red;
                    ^^^ occurrence:first
const second = Color.Red;
                     ^^^ occurrence:second
```

```query highlight main.ds#occurrence:first
@highlight.range range=main.ds#occurrence:definition kind=text
@highlight.range range=main.ds#occurrence:first kind=text
@highlight.range range=main.ds#occurrence:second kind=text
```

## Import Aliases

### Highlight a local import alias

An explicit local alias remains separate from its exported symbol.

```ds library.ds
export function greet(): void {}
```

```ds main.ds
import { greet as welcome } from "./library.ds";
                  ^^^^^^^ occurrence:definition

welcome();
^^^^^^^ occurrence:first
welcome();
^^^^^^^ occurrence:second
```

```query highlight main.ds#occurrence:first
@highlight.range range=main.ds#occurrence:definition kind=write
@highlight.range range=main.ds#occurrence:first kind=read
@highlight.range range=main.ds#occurrence:second kind=read
```

## Nominal Types

### Highlight nominal type occurrences

Type declarations and references are textual occurrences rather than value reads or writes.

```ds main.ds
class User {}
      ^^^^ occurrence:definition

declare const existing: User;
                        ^^^^ occurrence:type_reference
const created = new User();
                    ^^^^ occurrence:construction_reference
```

```query highlight main.ds#occurrence:type_reference
@highlight.range range=main.ds#occurrence:definition kind=text
@highlight.range range=main.ds#occurrence:type_reference kind=text
@highlight.range range=main.ds#occurrence:construction_reference kind=text
```

## Labels

### [ignored] Highlight control label occurrences

Label declarations and targeted breaks are textual control-flow occurrences.

```ds main.ds
function choose(): int32 {
    outer: loop {
    ^^^^^ occurrence:definition
        break outer: 1;
              ^^^^^ occurrence:reference
    }
}
```

```query highlight main.ds#occurrence:reference
@highlight.range range=main.ds#occurrence:definition kind=text
@highlight.range range=main.ds#occurrence:reference kind=text
```

## Empty Results

### Return no highlight for a literal

A literal has no semantic occurrence set.

```ds main.ds
const value = 42;
              ^^ literal
```

```query highlight main.ds#literal
@highlight.none
```

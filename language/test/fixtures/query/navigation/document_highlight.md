# Document Highlight

## Local Variables

### Highlight local variable occurrences

Document highlight should mark all occurrences of a symbol within the same file.

```ds
const foo = 1;
//    ^^^ def:foo

const bar = foo + foo;
//          ^^^ use:foo
```

`foo` appears 3 times: its definition and two uses in `foo + foo`.

```query document_highlight def:foo
main.ds:1:7-1:10 kind=write
main.ds:3:13-3:16 kind=read
main.ds:3:19-3:22 kind=read
```

## Functions

### Highlight function occurrences

Document highlight on a function should mark its definition and all call sites.

```ds
function add(x: int32, y: int32): int32 {
//       ^^^ def:add
    return x + y;
}

const result = add(1, 2) + add(3, 4);
```

`add` appears 3 times: its definition and two calls in the expression.

```query document_highlight def:add
main.ds:1:10-1:13 kind=write
main.ds:5:16-5:19 kind=read
main.ds:5:28-5:31 kind=read
```

## Parameters

### Highlight parameter occurrences

Document highlight should include parameter definitions and references.

```ds
function greet(name: string): string {
//             ^^^^ def:name
    return "hi " + name;
//                 ^^^^ use:name
}
```

```query document_highlight use:name
main.ds:1:16-1:20 kind=write
main.ds:2:20-2:24 kind=read
```

## Struct Fields

### Highlight struct field occurrences

Document highlight should include the field definition and all field accesses.

```ds
struct Point {
    x: int32
//  ^ def:field_x
    y: int32
}

function main(p: Point) {
    const a = p.x;
//              ^ use:field_x_1
    const b = p.x + p.x;
//              ^ use:field_x_2
//                    ^ use:field_x_3
}
```

Highlighting `p.x` should include the definition and all accesses.

```query document_highlight use:field_x_1
def:field_x
use:field_x_1
use:field_x_2
use:field_x_3
```

## Shadowing

### Highlight inner variable only

Document highlight should respect shadowing and only highlight the selected symbol.

```ds
const value = 1;
//    ^^^^^ def:outer_value

function test() {
    const value = 2;
//        ^^^^^ def:inner_value
    return value;
//         ^^^^^ use:inner_value
}
```

When highlighting `value` inside `test`, only the inner definition and use should be highlighted.

```query document_highlight def:inner_value
def:inner_value
use:inner_value
```

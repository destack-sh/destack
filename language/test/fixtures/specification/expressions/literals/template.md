# Template Literals

Template literal expressions are strings.

## templates

### template literal yields string

> Template expressions evaluate to string values.

```ds
let name = "Ada";
let greeting = `hello ${name}`;
```

### template literal is assignable to string

> Template expressions are assignable to string.

```ds
let greeting: string = `hello`;
```

### template literal is not assignable to number

> Template expressions are not assignable to number.

```ds
let value: number = `hello`;
```

- type int32 is not assignable to type string

## tagged templates

### tagged template uses tag return type

> Tagged templates use the tag function return type.

```ds libs=es5,es2015
declare function tag(strings: TemplateStringsArray, value: int32): boolean;

let result: boolean = tag`value=${1}`;
```

### tagged template enforces argument types

> Tagged templates check argument types against the tag signature.

```ds libs=es5,es2015
declare function tag(strings: TemplateStringsArray, value: string): string;

let result = tag`value=${1}`;
```

- type int32 is not assignable to type string

### tagged template return type is not widened

> Tagged templates preserve the tag return type.

```ds libs=es5,es2015
declare function tag(strings: TemplateStringsArray): int32;

let result: string = tag`value`;
```

- type int32 is not assignable to type string

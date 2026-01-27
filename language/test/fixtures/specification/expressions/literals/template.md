# Template Literals

Template literal expressions are strings.

## basic templates

### template literal yields string

> Template expressions evaluate to string values.

```ds
let name = "Destack";
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

- contains: not assignable

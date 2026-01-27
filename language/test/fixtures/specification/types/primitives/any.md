# Any Type

Tests for the `any` type.

## Any Accepts Everything

### number to any

> Any type accepts number values.

```json:dsconfig.json
{ "compilerOptions": { "noAny": false } }
```

```ds
const value: any = 42;
value satisfies any;
```

### string to any

> Any type accepts string values.

```json:dsconfig.json
{ "compilerOptions": { "noAny": false } }
```

```ds
const value: any = "hello";
value satisfies any;
```

### object to any

> Any type accepts object values.

```json:dsconfig.json
{ "compilerOptions": { "noAny": false } }
```

```ds
const value: any = { a: 1 };
value satisfies any;
```

## Any is Assignable to Everything

### any to number

> Any is assignable to number (unsafe but allowed).

```json:dsconfig.json
{ "compilerOptions": { "noAny": false } }
```

```ds
const anyValue: any = 42;
const numberValue: number = anyValue;
numberValue satisfies number;
```

### any to string

> Any is assignable to string (unsafe but allowed).

```json:dsconfig.json
{ "compilerOptions": { "noAny": false } }
```

```ds
const anyValue: any = "hello";
const stringValue: string = anyValue;
stringValue satisfies string;
```

## Any Member Access

### member access yields any

> Accessing a member on `any` produces `any`.

```json:dsconfig.json
{ "compilerOptions": { "noAny": false } }
```

```ds
const value: any = { nested: { value: 1 } };
const result = value.nested.value;
result satisfies any;
```

### index access yields any

> Indexing into `any` produces `any`.

```json:dsconfig.json
{ "compilerOptions": { "noAny": false } }
```

```ds
const value: any = { a: 1 };
const result = value["missing"];
result satisfies any;
```

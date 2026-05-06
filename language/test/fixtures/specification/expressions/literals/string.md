# String Literals

String literal type inference and checking.

## strings

### string literal

String literals can be assigned to string type.

```ds
const x: string = "hello";
```

### empty string

Empty strings are accepted string literals.

```ds
const x: string = "";
```

### string with spaces

Strings can contain spaces.

```ds
const x: string = "hello world";
```

## inference

### string literals stay exact

String literals without annotation keep their literal type.

```ds
const x = "hello";
x satisfies "hello";
x satisfies string;
```

### empty string literals stay exact

Empty string literals keep their literal type.

```ds
const x = "";
x satisfies "";
x satisfies string;
```

### string does not satisfy number

String literal cannot satisfy number type.

```ds
const x = "hello";
x satisfies number;
```

- contains: not assignable

## string members

### string length resolves

String literals expose string members.

```ds
const value = "hello";
value.length satisfies int32;
```

### string toUpperCase resolves

String methods preserve their declared return types.

```ds
const value = "hello";
const text = value.toUpperCase();
text satisfies string;
```

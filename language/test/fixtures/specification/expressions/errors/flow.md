# Try Control

## expressions

### try expressions require catch or finally

A try expression requires a catch or finally block.

```ds
const value = try { 1 };
value satisfies int;
```

- contains: requires a catch or finally

### try finally returns the body type

Finally does not affect the try expression type.

```ds
const value = try {
    1
} finally {
    2;
};
value satisfies int;
```

## try and catch

### try catch returns union type

Catch contributes to the try expression type.

```ds
const value = try {
    1
} catch (e) {
    e satisfies unknown;
    "fallback"
};
value satisfies int | string;
```

## try catch finally

### try catch finally returns union type

Catch contributes to the try expression type.

```ds
const value = try {
    1
} catch (e) {
    e satisfies unknown;
    "fallback"
} finally {
    2;
};
value satisfies int | string;
```

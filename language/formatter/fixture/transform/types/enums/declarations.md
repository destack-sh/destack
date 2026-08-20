# Enum Declarations

## Enum Forms

### enum declaration

Enums expand to multiple lines with trailing commas.

```ds
enum Status { Active; Inactive }
```

```ds expected
enum Status {
    Active,
    Inactive,
}
```

### enum with values

Enum members with values keep spacing around `=`.

```ds
enum Color { Red = "red"; Green = "green" }
```

```ds expected
enum Color {
    Red = "red",
    Green = "green",
}
```

## Enum Members

### enum with method

Enums can include methods with block bodies.

```ds
enum Mode { Normal; Debug; toString(): string { return "mode" } }
```

```ds expected
enum Mode {
    Normal,
    Debug,

    toString(): string {
        return "mode";
    }
}
```

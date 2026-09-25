# Enum Declarations

## Enum Forms

### enum declaration

Enums expand to multiple lines with trailing commas.

```tspp
enum Status { Active; Inactive }
```

```tspp expected
enum Status {
    Active,
    Inactive,
}
```

### enum with values

Enum members with values keep spacing around `=`.

```tspp
enum Color { Red = "red"; Green = "green" }
```

```tspp expected
enum Color {
    Red = "red",
    Green = "green",
}
```

## Enum Members

### enum with method

Enums can include methods with block bodies.

```tspp
enum Mode { Normal; Debug; toString(): string { return "mode" } }
```

```tspp expected
enum Mode {
    Normal,
    Debug,

    toString(): string {
        return "mode";
    }
}
```

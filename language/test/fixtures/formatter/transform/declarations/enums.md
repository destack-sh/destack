# Enum Declarations

Enum fixtures cover enum heads, members, comments, and generic parameters.

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

## Decorators

### decorated enum

Enum decorators appear on their own line above the declaration.

```ds
@description("The status of a task.")
enum Status { Todo; Done }
```

```ds expected
@description("The status of a task.")
enum Status {
    Todo,
    Done,
}
```

### decorated enum member

Body level enum member annotations stay on their own line above the member.

```ds
enum Status { @default Todo; @description("Completed work.") Done }
```

```ds expected
enum Status {
    @default
    Todo,
    @description("Completed work.")
    Done,
}
```

### stacked enum member decorators

Multiple enum member annotations each get their own line above the same member.

```ds
enum Status {
  @default
  @description("The item is pending.")
  Pending
}
```

```ds expected
enum Status {
    @default
    @description("The item is pending.")
    Pending,
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

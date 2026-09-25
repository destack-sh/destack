# Enum Decorators

## Decorators

### decorated enum

Enum decorators appear on their own line above the declaration.

```tspp
@description("The status of a task.")
enum Status { Todo; Done }
```

```tspp expected
@description("The status of a task.")
enum Status {
    Todo,
    Done,
}
```

### decorated enum member

Body level enum member annotations stay on their own line above the member.

```tspp
enum Status { @default Todo; @description("Completed work.") Done }
```

```tspp expected
enum Status {
    @default
    Todo,
    @description("Completed work.")
    Done,
}
```

### stacked enum member decorators

Multiple enum member annotations each get their own line above the same member.

```tspp
enum Status {
  @default
  @description("The item is pending.")
  Pending
}
```

```tspp expected
enum Status {
    @default
    @description("The item is pending.")
    Pending,
}
```

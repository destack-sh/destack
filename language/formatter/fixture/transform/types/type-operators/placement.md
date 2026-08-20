# Placement Type Operators

Placement type operators format like other prefix type operators.

## local

### local placement type

Extra whitespace after `local` is normalized.

```ds
type LocalUser = local   User
```

```ds expected
type LocalUser = local User;
```

### local placement with union

Composite operands stay parenthesized.

```ds
type MaybeLocal = local (User | undefined)
```

```ds expected
type MaybeLocal = local (User | undefined);
```

### local placement with owned type

Ownership and placement compose without extra grouping.

```ds
type LocalBox = local ^User
```

```ds expected
type LocalBox = local ^User;
```

## shared

### shared placement type

Extra whitespace after `shared` is normalized.

```ds
type SharedUser = shared   User
```

```ds expected
type SharedUser = shared User;
```

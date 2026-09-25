# Conditional Comments

## Comments in Ternary Expressions

### comment before ternary branches

Comments before ternary branches are preserved.

```tspp line-width=60
const x = condition ? /* then */ valueA : /* else */ valueB
```

```tspp expected
const x = condition ? /* then */ valueA : /* else */ valueB;
```

### comment in breaking ternary

Comments preserved when ternary breaks across lines.

```tspp line-width=30
const x = condition ? /* yes */ valueA : /* no */ valueB
```

```tspp expected
const x = condition
    ? /* yes */ valueA
    : /* no */ valueB;
```

# Conditional Comments

## Comments in Ternary Expressions

### comment before ternary branches

Comments before ternary branches are preserved.

```ds line-width=60
const x = condition ? /* then */ valueA : /* else */ valueB
```

```ds expected
const x = condition ? /* then */ valueA : /* else */ valueB;
```

### comment in breaking ternary

Comments preserved when ternary breaks across lines.

```ds line-width=30
const x = condition ? /* yes */ valueA : /* no */ valueB
```

```ds expected
const x = condition
    ? /* yes */ valueA
    : /* no */ valueB;
```

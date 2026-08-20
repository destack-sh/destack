# Assignment Comments

## Comments in Assignments

### comment in chained assignment

Comments in chained assignments are preserved.

```ds
x = /* important */ y = /* also important */ z
```

```ds expected
x = /* important */ y = /* also important */ z;
```

### comment before assignment value

Comment between equals and value.

```ds
const result = /* computed */ calculate(a, b)
```

```ds expected
const result = /* computed */ calculate(a, b);
```

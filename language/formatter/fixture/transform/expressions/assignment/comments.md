# Assignment Comments

## Comments in Assignments

### comment in chained assignment

Comments in chained assignments are preserved.

```tspp
x = /* important */ y = /* also important */ z
```

```tspp expected
x = /* important */ y = /* also important */ z;
```

### comment before assignment value

Comment between equals and value.

```tspp
const result = /* computed */ calculate(a, b)
```

```tspp expected
const result = /* computed */ calculate(a, b);
```

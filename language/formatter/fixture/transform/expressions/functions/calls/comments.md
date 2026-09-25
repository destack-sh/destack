# Call Comments

## Inline Comments

### comment before argument

Comments before arguments stay inline when the call still fits.

```tspp
foo(/* first */ a, /* second */ b)
```

```tspp expected
foo(/* first */ a, /* second */ b);
```

# Call Comments

## Inline Comments

### comment before argument

Comments before arguments stay inline when the call still fits.

```ds
foo(/* first */ a, /* second */ b)
```

```ds expected
foo(/* first */ a, /* second */ b);
```

# Export Statements

Tests for export statement formatting.

## Named Exports

### export braces have internal spacing

Export braces get internal spacing, like imports.

```ds
export {foo,bar,baz}
```

Spaces are added after `{` and before `}`.

```ds expected
export { foo, bar, baz };
```

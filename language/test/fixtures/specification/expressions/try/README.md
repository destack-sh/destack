# Try Expressions

Try, catch, and finally expression behavior.

## Coverage

- **Try expressions**: require catch or finally
- **Catch typing**: unions and catch variable defaults
- **Finally**: does not affect expression type
- **Try propagation**: `?` handling inside try
- **Options**: noExceptions gating

## Example

```ds
const value = try {
    1
} catch e {
    "fallback"
};
value satisfies int | string;
```

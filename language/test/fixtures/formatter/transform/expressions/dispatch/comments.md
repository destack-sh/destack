# Dispatch Comments

## Comments in Member Access

### comment in method chain

Comments between method calls in chains are preserved.

```ds
obj.method() /* step 1 */ .transform() /* step 2 */ .result()
```

```ds expected
obj.method() /* step 1 */
    .transform() /* step 2 */
    .result();
```

### comment before method call

When chains break, comments stay with their associated element.

```ds line-width=50
data.filter(x => x.valid) /* now map */ .map(x => x.value)
```

```ds expected
data.filter((x) => x.valid) /* now map */
    .map((x) => x.value);
```

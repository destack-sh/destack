# Dispatch Comments

## Comments in Member Access

### comment in method chain

Comments between method calls in chains are preserved.

```tspp
obj.method() /* step 1 */ .transform() /* step 2 */ .result()
```

```tspp expected
obj.method() /* step 1 */
    .transform() /* step 2 */
    .result();
```

### comment before method call

When chains break, comments stay with their associated element.

```tspp line-width=50
data.filter(x => x.valid) /* now map */ .map(x => x.value)
```

```tspp expected
data.filter((x) => x.valid) /* now map */
    .map((x) => x.value);
```

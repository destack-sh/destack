# CI

Repository policy, workflow validation, release automation, and metadata validation.

## Testing

Run these from the repository root.

```sh
# focused local loop
just check-workflow-policy

# clean gate
just check-hygiene

# exhaustive gate
just quick
```

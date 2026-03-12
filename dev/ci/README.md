# CI

Repository policy, workflow validation, release automation, and metadata validation.

## Testing

Run these from the repository root.

```sh
# focused local loop
just check-workflow-policy
just check-projects
just check-release-drift

# clean gate
just check-hygiene

# exhaustive gate
just quick
```

## Local Development

### Pre-commit hooks

To ensure a consistent codebase, we use pre-commit hooks.
If pre-commit is not already available, install it:

```
pip install pre-commit
```

Activate the pre-commit hooks:

```
pre-commit install --install-hooks --hook-type commit-msg
pre-commit install --install-hooks --hook-type pre-commit
```

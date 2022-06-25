## Local Development

### Mypy

We use [Mypy](https://mypy.readthedocs.io/en/stable/) for static type checking the Python parts of this project. To speed up local mypy, configure your IDE to use [the Mypy daemon](https://mypy.readthedocs.io/en/stable/mypy_daemon.html#mypy-daemon) (via dmypy).

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

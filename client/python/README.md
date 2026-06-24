# destack (Python)

Python language client for Destack.
This package is published to PyPI as `destack`.

## Installation

```sh
pip install destack
```

## API

```python
from destack import open_workspace

workspace = open_workspace(workspace=".")
print(workspace.root())
```

## Testing

Run these from the repository root.

```sh
# focused local loop
just client/test

# clean check
just client/check-quick

# exhaustive check
just client/check-full
```

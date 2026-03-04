# destack-py

Destack compatibility package for Python.
This package forwards imports from the canonical `destack` package.

## Installation

```sh
pip install destack-py
```

## API

```python
from destack_py import create_client

client = create_client()
assert client.backend == "python"
```

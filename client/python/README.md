# destack (Python)

Python client for Destack.
This package is published to PyPI as `destack`.

## Installation

```sh
pip install destack
```

## API

```python
from destack import create_client

client = create_client()
assert client.backend == "python"
assert client.version() == "0.55.2"
```

# destack (Python)

Python client for Destack.
This package is published to PyPI as `destack`.
Compatibility package `destack-py` is in [`compat/destack-py`](compat/destack-py/README.md).

## Installation

```sh
pip install destack
```

## API

```python
from destack import create_client

client = create_client()
assert client.backend == "python"
assert client.version() == "0.55.3"
assert client.capi_abi_version() > 0
assert client.capi_is_available() is True
```

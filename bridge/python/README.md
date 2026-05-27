# destack (Python)

Python client for Destack.
This package is published to PyPI as `destack`.
Legacy alias package `destack-py` is in [`aliases/destack-py`](aliases/destack-py/README.md).

## Installation

```sh
pip install destack
```

## API

```python
from destack import create_client

client = create_client()
assert client.backend == "python"
assert client.version() == "0.55.4"
assert client.capi_abi_version() > 0
assert client.capi_is_available() is True
```

## Testing

Run these from the repository root.

```sh
# focused local loop
just bridge/test

# clean check
just bridge/check-quick

# exhaustive check
just bridge/check-full
```

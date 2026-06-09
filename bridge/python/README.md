# destack (Python)

Python language bridge for Destack.
This package is published to PyPI as `destack`.

## Installation

```sh
pip install destack
```

## API

```python
from destack import Session, Source

session = Session.open(Source.file_system("."))
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

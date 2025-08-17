from ..builtin import Encoding
from .encoder import Encoder

_ENCODERS: dict[Encoding, Encoder] = {}


def _generate_encoder(encoding: Encoding) -> Encoder:
    raise NotImplementedError


def get_encoder(encoding: Encoding) -> Encoder:
    """Get the Encoder for the given Encoding. Generates it if not already done."""
    if encoding not in _ENCODERS:
        encoder = _generate_encoder(encoding)
        _ENCODERS[encoding] = encoder
    return _ENCODERS[encoding]

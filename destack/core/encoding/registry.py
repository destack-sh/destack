from typing import assert_never

from destack.core import Encoder, Encoding

_ENCODERS: dict[Encoding, Encoder] = {}


def _generate_encoder(encoding: Encoding) -> Encoder:
    if encoding == Encoding.JSON:
        from .json.encoder import JsonEncoder

        encoder = JsonEncoder.generate()
    elif encoding == Encoding.JSONC:
        from .jsonc.encoder import JsoncEncoder

        encoder = JsoncEncoder.generate()
    elif encoding == Encoding.KOMPAKT:
        from .kompakt.encoder import KompaktEncoder

        encoder = KompaktEncoder.generate()
    else:
        assert_never(encoding)

    return encoder


def get_encoder(encoding: Encoding) -> Encoder:
    """Get the Encoder for the given Encoding. Generates it if not already done."""
    if encoding not in _ENCODERS:
        encoder = _generate_encoder(encoding)
        _ENCODERS[encoding] = encoder
    return _ENCODERS[encoding]

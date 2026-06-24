from __future__ import annotations

from destack._impl.model import ModelImpl

import struct


class FloatValueImpl(ModelImpl):
    """Methods for floating point bit patterns."""

    def to_float(self) -> float:
        """Return the f64 value represented by the stored bits."""

        return struct.unpack("<d", int(self.bits).to_bytes(8, "little"))[0]

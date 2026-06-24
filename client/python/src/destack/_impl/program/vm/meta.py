from __future__ import annotations

from destack._impl.model import ModelImpl

KIND_MASK = 0x7
ACCESS_SHIFT = 3
ACCESS_MASK = 0x3 << ACCESS_SHIFT
NULLABILITY_SHIFT = 5
NULLABILITY_MASK = 0x3 << NULLABILITY_SHIFT
SPACE_SHIFT = 7
SPACE_MASK = 0x7 << SPACE_SHIFT


class ReferenceMetaImpl(ModelImpl):
    """Methods for reference metadata."""

    def kind(self):
        """Return the encoded reference kind."""

        bits = self.bits & KIND_MASK
        # managed references use kind tag 1
        if bits == 1:
            return "managed"

        # unique references use kind tag 2
        elif bits == 2:
            return "unique"

        # borrowed references use kind tag 3
        elif bits == 3:
            return "borrowed"

        # raw references use kind tag 4
        elif bits == 4:
            return "raw"
        # invalid tags do not decode to reference kinds
        else:
            return None

    def access(self):
        """Return the encoded reference access."""

        # invalid reference kinds do not have access bits
        if self.kind() is None:
            return None

        bits = (self.bits & ACCESS_MASK) >> ACCESS_SHIFT
        # readonly access is encoded as 0
        if bits == 0:
            return "readonly"

        # mutable access is encoded as 1
        elif bits == 1:
            return "mutable"

        # exclusive access is encoded as 2
        elif bits == 2:
            return "exclusive"
        # invalid tags do not decode to access modes
        else:
            return None

    def nullability(self):
        """Return the encoded nullability."""

        bits = (self.bits & NULLABILITY_MASK) >> NULLABILITY_SHIFT
        # nullability supports null only
        if bits == 1:
            return "null"

        # nullability supports undefined only
        elif bits == 2:
            return "undefined"

        # nullability supports both null and undefined
        elif bits == 3:
            return "nullOrUndefined"
        # zero and invalid tags default to non-nullable
        else:
            return "none"

    def space(self):
        """Return the encoded memory space."""

        bits = (self.bits & SPACE_MASK) >> SPACE_SHIFT
        # frame space is encoded as 1
        if bits == 1:
            return "frame"

        # static space is encoded as 2
        elif bits == 2:
            return "static"

        # shared space is encoded as 3
        elif bits == 3:
            return "shared"
        # zero and invalid tags default to local space
        else:
            return "local"

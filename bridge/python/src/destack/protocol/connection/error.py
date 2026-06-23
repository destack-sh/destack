from __future__ import annotations


class ProtocolRequestError(Exception):
    """Error thrown while sending one workspace protocol request."""

    @classmethod
    def from_protocol(cls, error: object) -> ProtocolRequestError:
        """Return one request error from a protocol error payload."""

        code = getattr(error, "code", "internal")
        message = getattr(error, "message", str(error))

        return cls(f"{code}: {message}")

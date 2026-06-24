# generated client target, do not edit

from __future__ import annotations

from .handshake import ClientDescriptor, ProtocolLimits
from .version import ProtocolRange, ProtocolVersion

protocol_version = ProtocolVersion(
    major=1,
    minor=0,
    patch=0,
)

min_protocol_version = ProtocolVersion(
    major=1,
    minor=0,
    patch=0,
)

protocol_range = ProtocolRange(
    min=min_protocol_version,
    max=protocol_version,
)

protocol_limits = ProtocolLimits(
    max_frame_bytes=16777216,
    max_payload_bytes=1048576,
)

client_descriptor = ClientDescriptor(
    name="destack-python",
    version="0.55.4",
    build=None,
)

__all__ = [
    "protocol_version",
    "min_protocol_version",
    "protocol_range",
    "protocol_limits",
    "client_descriptor",
]

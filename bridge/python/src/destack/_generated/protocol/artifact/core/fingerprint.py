# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes


@dataclass(frozen=True, slots=True)
class ArtifactFingerprint:
    """Deterministic identity of one artifact's complete semantic dependencies."""

    field_0: int


def encode_artifact_fingerprint(writer: Writer, value: ArtifactFingerprint) -> None:
    writer.write_unsigned(value.field_0)


def decode_artifact_fingerprint(reader: Reader) -> ArtifactFingerprint:
    field_0 = reader.read_unsigned()

    return ArtifactFingerprint(
        field_0=field_0,
    )


__all__ = [
    "ArtifactFingerprint",
    "encode_artifact_fingerprint",
    "decode_artifact_fingerprint",
]

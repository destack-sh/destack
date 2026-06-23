# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.artifact.version
import destack._generated.source.module

if TYPE_CHECKING:
    from destack._generated.artifact.version import (
        ArtifactVersion,
    )

    from destack._generated.source.module import (
        ModuleId,
    )


@dataclass(frozen=True, slots=True)
class DirParsed:
    """Typed projection of one parsed DIR artifact."""

    """Exact parsed artifact version."""
    version: ArtifactVersion
    """Parsed module id."""
    module: ModuleId


def encode_dir_parsed(writer: Writer, value: DirParsed) -> None:
    destack._generated.artifact.version.encode_artifact_version(writer, value.version)
    destack._generated.source.module.encode_module_id(writer, value.module)


def decode_dir_parsed(reader: Reader) -> DirParsed:
    field_0 = destack._generated.artifact.version.decode_artifact_version(reader)
    field_1 = destack._generated.source.module.decode_module_id(reader)

    return DirParsed(
        version=field_0,
        module=field_1,
    )


__all__ = [
    "DirParsed",
    "encode_dir_parsed",
    "decode_dir_parsed",
]

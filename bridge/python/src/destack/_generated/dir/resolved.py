# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.artifact.version
import destack._generated.source.module
import destack._generated.source.profile

if TYPE_CHECKING:
    from destack._generated.artifact.version import (
        ArtifactVersion,
    )

    from destack._generated.source.module import (
        ModuleId,
    )

    from destack._generated.source.profile import (
        ProfileId,
    )


@dataclass(frozen=True, slots=True)
class DirResolved:
    """Typed projection of one resolved DIR artifact."""

    """Exact resolved artifact version."""
    version: ArtifactVersion
    """Resolved module id."""
    module: ModuleId
    """Resolved semantic profile."""
    profile: ProfileId


def encode_dir_resolved(writer: Writer, value: DirResolved) -> None:
    destack._generated.artifact.version.encode_artifact_version(writer, value.version)
    destack._generated.source.module.encode_module_id(writer, value.module)
    destack._generated.source.profile.encode_profile_id(writer, value.profile)


def decode_dir_resolved(reader: Reader) -> DirResolved:
    field_0 = destack._generated.artifact.version.decode_artifact_version(reader)
    field_1 = destack._generated.source.module.decode_module_id(reader)
    field_2 = destack._generated.source.profile.decode_profile_id(reader)

    return DirResolved(
        version=field_0,
        module=field_1,
        profile=field_2,
    )


__all__ = [
    "DirResolved",
    "encode_dir_resolved",
    "decode_dir_resolved",
]

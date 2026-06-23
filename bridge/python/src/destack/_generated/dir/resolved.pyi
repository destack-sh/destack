# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, Writer

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

def encode_dir_resolved(writer: Writer, value: DirResolved) -> None: ...
def decode_dir_resolved(reader: Reader) -> DirResolved: ...

__all__ = [
    "DirResolved",
    "encode_dir_resolved",
    "decode_dir_resolved",
]

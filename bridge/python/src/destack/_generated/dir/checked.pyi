# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, Writer

import destack._generated.artifact.version
import destack._generated.source.component
import destack._generated.source.module
import destack._generated.source.profile

if TYPE_CHECKING:
    from destack._generated.artifact.version import (
        ArtifactVersion,
    )

    from destack._generated.source.component import (
        ComponentId,
    )

    from destack._generated.source.module import (
        ModuleId,
    )

    from destack._generated.source.profile import (
        ProfileId,
    )

@dataclass(frozen=True, slots=True)
class DirChecked:
    """Typed projection of one checked DIR module artifact."""

    """Exact checked facade artifact version."""
    version: ArtifactVersion
    """Checked module id."""
    module: ModuleId
    """Checked semantic profile."""
    profile: ProfileId
    """Component that owns the checked module output."""
    component: ComponentId
    """Component entry module."""
    entry: ModuleId

def encode_dir_checked(writer: Writer, value: DirChecked) -> None: ...
def decode_dir_checked(reader: Reader) -> DirChecked: ...

__all__ = [
    "DirChecked",
    "encode_dir_checked",
    "decode_dir_checked",
]

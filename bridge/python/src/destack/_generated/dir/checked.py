# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

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


def encode_dir_checked(writer: Writer, value: DirChecked) -> None:
    destack._generated.artifact.version.encode_artifact_version(writer, value.version)
    destack._generated.source.module.encode_module_id(writer, value.module)
    destack._generated.source.profile.encode_profile_id(writer, value.profile)
    destack._generated.source.component.encode_component_id(writer, value.component)
    destack._generated.source.module.encode_module_id(writer, value.entry)


def decode_dir_checked(reader: Reader) -> DirChecked:
    field_0 = destack._generated.artifact.version.decode_artifact_version(reader)
    field_1 = destack._generated.source.module.decode_module_id(reader)
    field_2 = destack._generated.source.profile.decode_profile_id(reader)
    field_3 = destack._generated.source.component.decode_component_id(reader)
    field_4 = destack._generated.source.module.decode_module_id(reader)

    return DirChecked(
        version=field_0,
        module=field_1,
        profile=field_2,
        component=field_3,
        entry=field_4,
    )


__all__ = [
    "DirChecked",
    "encode_dir_checked",
    "decode_dir_checked",
]

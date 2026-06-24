# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.file.model.component
import destack._generated.source.file.model.module
import destack._generated.source.file.model.profile

@dataclass(frozen=True, slots=True)
class ComponentGraph:
    """Strongly connected component partition of one profile's module graph."""

    # the profile this partition belongs to
    profile: destack._generated.source.file.model.profile.ProfileId
    # imported modules per module, deduplicated in import order
    imports: Mapping[
        destack._generated.source.file.model.module.ModuleId,
        Sequence[destack._generated.source.file.model.module.ModuleId],
    ]
    # owning component per module
    component_of: Mapping[
        destack._generated.source.file.model.module.ModuleId,
        destack._generated.source.file.model.component.ComponentId,
    ]
    # member modules per component, sorted, with the entry first
    members: Mapping[
        destack._generated.source.file.model.component.ComponentId,
        Sequence[destack._generated.source.file.model.module.ModuleId],
    ]
    # external components each component depends on
    dependencies: Mapping[
        destack._generated.source.file.model.component.ComponentId,
        Sequence[destack._generated.source.file.model.component.ComponentId],
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ComponentGraph: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ComponentGraph: ...

def encode_component_graph(writer: BinaryWriter, value: ComponentGraph) -> None: ...
def decode_component_graph(reader: BinaryReader) -> ComponentGraph: ...
def to_json_component_graph(value: ComponentGraph) -> Json: ...
def from_json_component_graph(value: Json) -> ComponentGraph: ...

__all__ = [
    "ComponentGraph",
    "encode_component_graph",
    "decode_component_graph",
    "to_json_component_graph",
    "from_json_component_graph",
]

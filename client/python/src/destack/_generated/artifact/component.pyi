# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.file.model.component
import destack._generated.source.file.model.module
import destack._generated.source.file.model.profile

@dataclass(frozen=True, slots=True)
class ModuleGraph:
    """Dense module dependency graph for one profile."""

    # the profile this graph belongs to
    profile: destack._generated.source.file.model.profile.ProfileId
    # modules sorted by stable id
    modules: Sequence[destack._generated.source.file.model.module.ModuleId]
    # per-module edge targets as dense module indexes
    edges: Sequence[Sequence[int]]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ModuleGraph: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ModuleGraph: ...

def encode_module_graph(writer: BinaryWriter, value: ModuleGraph) -> None: ...
def decode_module_graph(reader: BinaryReader) -> ModuleGraph: ...
def to_json_module_graph(value: ModuleGraph) -> Json: ...
def from_json_module_graph(value: Json) -> ModuleGraph: ...

@dataclass(frozen=True, slots=True)
class ComponentGraph:
    """Strongly connected component partition of one profile's module graph."""

    # the dense module graph being partitioned
    module_graph: ModuleGraph
    # per-module owning component
    component_of: Sequence[destack._generated.source.file.model.component.ComponentId]
    # components sorted by stable id
    components: Sequence[destack._generated.source.file.model.component.ComponentId]
    # per-component member start offsets into `member_modules`
    member_offsets: Sequence[int]
    # component members as module ids
    member_modules: Sequence[destack._generated.source.file.model.module.ModuleId]
    # component members as dense module indexes
    member_indexes: Sequence[int]
    # per-module dense component index
    module_components: Sequence[int]
    # per-component topological rank in the condensation graph
    component_ranks: Sequence[int]
    # external components each component depends on
    dependencies: Sequence[
        Sequence[destack._generated.source.file.model.component.ComponentId]
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
    "ModuleGraph",
    "encode_module_graph",
    "decode_module_graph",
    "to_json_module_graph",
    "from_json_module_graph",
    "ComponentGraph",
    "encode_component_graph",
    "decode_component_graph",
    "to_json_component_graph",
    "from_json_component_graph",
]

# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.tree.node
import destack._generated.dir.type.resolution
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class ResolutionSegment:
    """Resolutions added by one DIR phase."""

    # the module id of the resolution segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # checked lexical or path resolutions keyed by DIR node
    names: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.NameResolution,
    ]
    # checked label resolutions keyed by DIR node
    labels: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.LabelResolution,
    ]
    # checked receiver resolutions keyed by DIR node
    receivers: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.ReceiverResolution,
    ]
    # checked member resolutions keyed by DIR node
    members: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.MemberResolution,
    ]
    # checked call resolutions keyed by DIR node
    calls: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.CallResolution,
    ]
    # checked paired read-write resolutions keyed by DIR node
    read_writes: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.ReadWriteResolution,
    ]
    # checked construct resolutions keyed by DIR node
    constructs: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.ConstructResolution,
    ]
    # checked pattern resolutions keyed by DIR node
    patterns: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.PatternResolution,
    ]
    # checked assignment pattern resolutions keyed by DIR node
    assign_patterns: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.AssignPatternResolution,
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ResolutionSegment: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ResolutionSegment: ...

def encode_resolution_segment(
    writer: BinaryWriter, value: ResolutionSegment
) -> None: ...
def decode_resolution_segment(reader: BinaryReader) -> ResolutionSegment: ...
def to_json_resolution_segment(value: ResolutionSegment) -> Json: ...
def from_json_resolution_segment(value: Json) -> ResolutionSegment: ...

__all__ = [
    "ResolutionSegment",
    "encode_resolution_segment",
    "decode_resolution_segment",
    "to_json_resolution_segment",
    "from_json_resolution_segment",
]

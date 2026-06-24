# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.type.generic
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class GenericSegment:
    """Generic templates and parameters added by one DIR phase."""

    # the module id of the generic segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # the first generic template id owned by this table segment
    first_template_id: int
    # the first generic parameter id owned by this table segment
    first_parameter_id: int
    # generic templates
    templates: Sequence[destack._generated.dir.type.generic.GenericTemplate]
    # generic parameters
    parameters: Sequence[destack._generated.dir.type.generic.GenericParameterBinding]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GenericSegment: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GenericSegment: ...

def encode_generic_segment(writer: BinaryWriter, value: GenericSegment) -> None: ...
def decode_generic_segment(reader: BinaryReader) -> GenericSegment: ...
def to_json_generic_segment(value: GenericSegment) -> Json: ...
def from_json_generic_segment(value: Json) -> GenericSegment: ...

__all__ = [
    "GenericSegment",
    "encode_generic_segment",
    "decode_generic_segment",
    "to_json_generic_segment",
    "from_json_generic_segment",
]

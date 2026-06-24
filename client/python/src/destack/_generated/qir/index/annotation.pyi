# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.tree.node
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class AnnotationIndex:
    """Searchable annotation and decorator index."""

    # the annotation entries in stable order
    entries: Sequence[AnnotationEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AnnotationIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AnnotationIndex: ...

def encode_annotation_index(writer: BinaryWriter, value: AnnotationIndex) -> None: ...
def decode_annotation_index(reader: BinaryReader) -> AnnotationIndex: ...
def to_json_annotation_index(value: AnnotationIndex) -> Json: ...
def from_json_annotation_index(value: Json) -> AnnotationIndex: ...

@dataclass(frozen=True, slots=True)
class AnnotationEntry:
    """Searchable annotation or decorator entry."""

    # the annotation name when syntactically known
    name: str | None
    # the module that owns the annotation
    module_id: destack._generated.source.file.model.module.ModuleId
    # the decorator node
    decorator_id: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the annotated target node
    target_id: destack._generated.dir.tree.node.GlobalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AnnotationEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AnnotationEntry: ...

def encode_annotation_entry(writer: BinaryWriter, value: AnnotationEntry) -> None: ...
def decode_annotation_entry(reader: BinaryReader) -> AnnotationEntry: ...
def to_json_annotation_entry(value: AnnotationEntry) -> Json: ...
def from_json_annotation_entry(value: Json) -> AnnotationEntry: ...

__all__ = [
    "AnnotationIndex",
    "encode_annotation_index",
    "decode_annotation_index",
    "to_json_annotation_index",
    "from_json_annotation_index",
    "AnnotationEntry",
    "encode_annotation_entry",
    "decode_annotation_entry",
    "to_json_annotation_entry",
    "from_json_annotation_entry",
]

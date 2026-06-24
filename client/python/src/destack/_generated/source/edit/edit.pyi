# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.file.model.file
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class Patch:
    """A single patch: replace a span with new text."""

    # the span to replace
    span: destack._generated.source.file.model.span.Span
    # the replacement text (empty for deletion)
    new_text: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Patch: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Patch: ...

def encode_patch(writer: BinaryWriter, value: Patch) -> None: ...
def decode_patch(reader: BinaryReader) -> Patch: ...
def to_json_patch(value: Patch) -> Json: ...
def from_json_patch(value: Json) -> Patch: ...

@dataclass(frozen=True, slots=True)
class FilePatch:
    """Patches for a single file."""

    # the file to patch
    file: destack._generated.source.file.model.file.FileId
    # the patches to apply (should be non-overlapping)
    patches: Sequence[Patch]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FilePatch: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FilePatch: ...

def encode_file_patch(writer: BinaryWriter, value: FilePatch) -> None: ...
def decode_file_patch(reader: BinaryReader) -> FilePatch: ...
def to_json_file_patch(value: FilePatch) -> Json: ...
def from_json_file_patch(value: Json) -> FilePatch: ...

@dataclass(frozen=True, slots=True)
class PatchSet:
    """Patches across multiple files."""

    # per-file patches
    files: Sequence[FilePatch]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatchSet: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatchSet: ...

def encode_patch_set(writer: BinaryWriter, value: PatchSet) -> None: ...
def decode_patch_set(reader: BinaryReader) -> PatchSet: ...
def to_json_patch_set(value: PatchSet) -> Json: ...
def from_json_patch_set(value: Json) -> PatchSet: ...

__all__ = [
    "Patch",
    "encode_patch",
    "decode_patch",
    "to_json_patch",
    "from_json_patch",
    "FilePatch",
    "encode_file_patch",
    "decode_file_patch",
    "to_json_file_patch",
    "from_json_file_patch",
    "PatchSet",
    "encode_patch_set",
    "decode_patch_set",
    "to_json_patch_set",
    "from_json_patch_set",
]

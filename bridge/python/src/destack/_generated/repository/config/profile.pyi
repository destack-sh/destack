# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.artifact.core.target
import destack._generated.repository.config.compiler
import destack._generated.repository.config.stage

@dataclass(frozen=True, slots=True)
class ProfileOptions:
    """Normalized profile options."""

    # release stage for this profile
    stage: destack._generated.repository.config.stage.Stage | None
    # target platform / operating system for this profile
    platform: destack._generated.artifact.core.target.Platform | None
    # target host environment for this profile
    host: destack._generated.artifact.core.target.Host | None
    # comptime environment whitelist
    comptime_env: Sequence[str] | None
    # active source graph modes for this profile
    modes: Sequence[str]
    # active source graph roles for this profile
    roles: Sequence[str]
    # active source graph features for this profile
    features: Sequence[str]
    # active source graph tags for this profile
    tags: Sequence[str]
    # default tree tag builder provider
    tree: str | None
    # global provider modules for this profile
    globals: Sequence[str]
    # well-known derives automatically considered in this profile
    derive: Sequence[destack._generated.repository.config.compiler.Derive]
    # static semantic restrictions for this profile
    restrictions: destack._generated.repository.config.compiler.CompilerRestrictions

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ProfileOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ProfileOptions: ...

def encode_profile_options(writer: BinaryWriter, value: ProfileOptions) -> None: ...
def decode_profile_options(reader: BinaryReader) -> ProfileOptions: ...
def to_json_profile_options(value: ProfileOptions) -> Json: ...
def from_json_profile_options(value: Json) -> ProfileOptions: ...

__all__ = [
    "ProfileOptions",
    "encode_profile_options",
    "decode_profile_options",
    "to_json_profile_options",
    "from_json_profile_options",
]

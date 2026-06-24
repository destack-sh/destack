# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.artifact.core.condition
import destack._generated.artifact.core.target
import destack._generated.repository.config.runtime.clock
import destack._generated.repository.config.runtime.diagnostic
import destack._generated.repository.config.runtime.execution
import destack._generated.repository.config.runtime.heap
import destack._generated.repository.config.runtime.host
import destack._generated.repository.config.runtime.random
import destack._generated.repository.config.runtime.trace
import destack._generated.repository.config.runtime.worker

@dataclass(frozen=True, slots=True)
class RuntimeOptions:
    """Runtime configuration."""

    # runtime topology and policy identity
    identity: RuntimeIdentityOptions
    # build distribution profile
    profile: destack._generated.artifact.core.target.BuildProfile
    # build payload linkage
    linkage: destack._generated.artifact.core.target.BuildLinkage
    # active source graph conditions for runtime policy selection
    conditions: destack._generated.artifact.core.condition.ConditionSet
    # execution mode for scheduling and effect handling
    mode: destack._generated.repository.config.runtime.execution.ExecutionMode
    # runtime worker configuration
    worker: destack._generated.repository.config.runtime.worker.WorkerOptions
    # runtime clock seed configuration
    clock: destack._generated.repository.config.runtime.clock.ClockOptions
    # runtime randomness source configuration
    random: destack._generated.repository.config.runtime.random.RandomOptions
    # runtime trace configuration
    trace: destack._generated.repository.config.runtime.trace.TraceOptions
    # runtime heap configuration
    heap: destack._generated.repository.config.runtime.heap.HeapOptions
    # runtime diagnostics configuration
    diagnostic: (
        destack._generated.repository.config.runtime.diagnostic.RuntimeDiagnosticOptions
    )
    # runtime host module defaults
    host: destack._generated.repository.config.runtime.host.HostOptions

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> RuntimeOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> RuntimeOptions: ...

def encode_runtime_options(writer: BinaryWriter, value: RuntimeOptions) -> None: ...
def decode_runtime_options(reader: BinaryReader) -> RuntimeOptions: ...
def to_json_runtime_options(value: RuntimeOptions) -> Json: ...
def from_json_runtime_options(value: Json) -> RuntimeOptions: ...

@dataclass(frozen=True, slots=True)
class RuntimeIdentityOptions:
    """Runtime identity used for topology and policy selection."""

    # stable runtime name for policy selection
    name: str | None
    # stable runtime labels for topology and policy selection
    labels: Mapping[str, str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> RuntimeIdentityOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> RuntimeIdentityOptions: ...

def encode_runtime_identity_options(
    writer: BinaryWriter, value: RuntimeIdentityOptions
) -> None: ...
def decode_runtime_identity_options(reader: BinaryReader) -> RuntimeIdentityOptions: ...
def to_json_runtime_identity_options(value: RuntimeIdentityOptions) -> Json: ...
def from_json_runtime_identity_options(value: Json) -> RuntimeIdentityOptions: ...

__all__ = [
    "RuntimeOptions",
    "encode_runtime_options",
    "decode_runtime_options",
    "to_json_runtime_options",
    "from_json_runtime_options",
    "RuntimeIdentityOptions",
    "encode_runtime_identity_options",
    "decode_runtime_identity_options",
    "to_json_runtime_identity_options",
    "from_json_runtime_identity_options",
]

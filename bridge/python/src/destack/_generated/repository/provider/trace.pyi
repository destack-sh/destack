# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class TraceSnapshot:
    """Serializable snapshot of one trace."""

    # the wall time of the traced operation in microseconds
    total_micros: int
    # the number of workers that recorded attempts
    workers: int
    # artifact stats
    stats: TraceStats
    # operation-level spans around artifact execution
    spans: Sequence[TraceSpanSnapshot]
    # operation-level counters
    counters: Sequence[TraceCounterSnapshot]
    # busy time per toolchain stage, ordered by stage
    stages: Sequence[TraceStageSnapshot]
    # summed time per named trace span
    times: Sequence[TraceTimeSnapshot]
    # the recorded artifact attempts, present only in detailed snapshots
    artifacts: Sequence[ArtifactAttemptSnapshot]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceSnapshot: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TraceSnapshot: ...

def encode_trace_snapshot(writer: BinaryWriter, value: TraceSnapshot) -> None: ...
def decode_trace_snapshot(reader: BinaryReader) -> TraceSnapshot: ...
def to_json_trace_snapshot(value: TraceSnapshot) -> Json: ...
def from_json_trace_snapshot(value: Json) -> TraceSnapshot: ...

"""Trace detail returned to a caller."""
TraceView: typing.TypeAlias = typing.Literal["summary"] | typing.Literal["detailed"]

def encode_trace_view(writer: BinaryWriter, value: TraceView) -> None: ...
def decode_trace_view(reader: BinaryReader) -> TraceView: ...
def to_json_trace_view(value: TraceView) -> Json: ...
def from_json_trace_view(value: Json) -> TraceView: ...

@dataclass(frozen=True, slots=True)
class TraceStats:
    """Artifact attempt outcome counts in one trace."""

    # attempts that produced an artifact
    built: int
    # attempts served from memory
    memory_cached: int
    # attempts restored from the persistent store
    store_cached: int
    # attempts parked on missing requirements
    parked: int
    # attempts that failed
    failed: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceStats: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TraceStats: ...

def encode_trace_stats(writer: BinaryWriter, value: TraceStats) -> None: ...
def decode_trace_stats(reader: BinaryReader) -> TraceStats: ...
def to_json_trace_stats(value: TraceStats) -> Json: ...
def from_json_trace_stats(value: Json) -> TraceStats: ...

@dataclass(frozen=True, slots=True)
class TraceStageSnapshot:
    """Busy time of one toolchain stage."""

    # the stage display name
    name: str
    # the summed attempt time in microseconds
    micros: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceStageSnapshot: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TraceStageSnapshot: ...

def encode_trace_stage_snapshot(
    writer: BinaryWriter, value: TraceStageSnapshot
) -> None: ...
def decode_trace_stage_snapshot(reader: BinaryReader) -> TraceStageSnapshot: ...
def to_json_trace_stage_snapshot(value: TraceStageSnapshot) -> Json: ...
def from_json_trace_stage_snapshot(value: Json) -> TraceStageSnapshot: ...

@dataclass(frozen=True, slots=True)
class TraceTimeSnapshot:
    """Summed time of one named trace span."""

    # the span name
    name: str
    # the summed span time in microseconds
    micros: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceTimeSnapshot: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TraceTimeSnapshot: ...

def encode_trace_time_snapshot(
    writer: BinaryWriter, value: TraceTimeSnapshot
) -> None: ...
def decode_trace_time_snapshot(reader: BinaryReader) -> TraceTimeSnapshot: ...
def to_json_trace_time_snapshot(value: TraceTimeSnapshot) -> Json: ...
def from_json_trace_time_snapshot(value: Json) -> TraceTimeSnapshot: ...

@dataclass(frozen=True, slots=True)
class TraceSpanSnapshot:
    """One span in a trace snapshot."""

    # the span name
    name: str
    # the offset from the trace start in microseconds
    start_micros: int
    # the span duration in microseconds
    micros: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceSpanSnapshot: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TraceSpanSnapshot: ...

def encode_trace_span_snapshot(
    writer: BinaryWriter, value: TraceSpanSnapshot
) -> None: ...
def decode_trace_span_snapshot(reader: BinaryReader) -> TraceSpanSnapshot: ...
def to_json_trace_span_snapshot(value: TraceSpanSnapshot) -> Json: ...
def from_json_trace_span_snapshot(value: Json) -> TraceSpanSnapshot: ...

@dataclass(frozen=True, slots=True)
class TraceCounterSnapshot:
    """One counter in a trace snapshot."""

    # the counter name
    name: str
    # the counter value
    value: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceCounterSnapshot: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TraceCounterSnapshot: ...

def encode_trace_counter_snapshot(
    writer: BinaryWriter, value: TraceCounterSnapshot
) -> None: ...
def decode_trace_counter_snapshot(reader: BinaryReader) -> TraceCounterSnapshot: ...
def to_json_trace_counter_snapshot(value: TraceCounterSnapshot) -> Json: ...
def from_json_trace_counter_snapshot(value: Json) -> TraceCounterSnapshot: ...

@dataclass(frozen=True, slots=True)
class ArtifactAttemptSnapshot:
    """One artifact attempt in a detailed trace snapshot."""

    # the artifact kind name
    name: str
    # the toolchain stage display name
    stage: str
    # the resolved artifact label, usually a module display name
    label: str | None
    # the resolved target name for emitted and linked artifacts
    target: str | None
    # the worker that executed the attempt
    worker: int
    # the offset from the trace start in microseconds
    start_micros: int
    # the attempt duration in microseconds
    micros: int
    # the attempt outcome name
    outcome: str
    # interior spans recorded by the executor or provider
    spans: Sequence[TraceSpanSnapshot]
    # counters recorded by the executor or provider
    counters: Sequence[TraceCounterSnapshot]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactAttemptSnapshot: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ArtifactAttemptSnapshot: ...

def encode_artifact_attempt_snapshot(
    writer: BinaryWriter, value: ArtifactAttemptSnapshot
) -> None: ...
def decode_artifact_attempt_snapshot(
    reader: BinaryReader,
) -> ArtifactAttemptSnapshot: ...
def to_json_artifact_attempt_snapshot(value: ArtifactAttemptSnapshot) -> Json: ...
def from_json_artifact_attempt_snapshot(value: Json) -> ArtifactAttemptSnapshot: ...

__all__ = [
    "TraceSnapshot",
    "encode_trace_snapshot",
    "decode_trace_snapshot",
    "to_json_trace_snapshot",
    "from_json_trace_snapshot",
    "TraceView",
    "encode_trace_view",
    "decode_trace_view",
    "to_json_trace_view",
    "from_json_trace_view",
    "TraceStats",
    "encode_trace_stats",
    "decode_trace_stats",
    "to_json_trace_stats",
    "from_json_trace_stats",
    "TraceStageSnapshot",
    "encode_trace_stage_snapshot",
    "decode_trace_stage_snapshot",
    "to_json_trace_stage_snapshot",
    "from_json_trace_stage_snapshot",
    "TraceTimeSnapshot",
    "encode_trace_time_snapshot",
    "decode_trace_time_snapshot",
    "to_json_trace_time_snapshot",
    "from_json_trace_time_snapshot",
    "TraceSpanSnapshot",
    "encode_trace_span_snapshot",
    "decode_trace_span_snapshot",
    "to_json_trace_span_snapshot",
    "from_json_trace_span_snapshot",
    "TraceCounterSnapshot",
    "encode_trace_counter_snapshot",
    "decode_trace_counter_snapshot",
    "to_json_trace_counter_snapshot",
    "from_json_trace_counter_snapshot",
    "ArtifactAttemptSnapshot",
    "encode_artifact_attempt_snapshot",
    "decode_artifact_attempt_snapshot",
    "to_json_artifact_attempt_snapshot",
    "from_json_artifact_attempt_snapshot",
]

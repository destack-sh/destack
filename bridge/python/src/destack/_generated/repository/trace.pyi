# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, Writer

@dataclass(frozen=True, slots=True)
class TraceReport:
    """One bridge trace report."""

    """The wall time of the traced operation in microseconds."""
    total_micros: int
    """The number of workers that recorded attempts."""
    workers: int
    """Operation-level spans around artifact execution."""
    spans: Sequence[TraceSpan]
    """Operation-level counters."""
    counters: Sequence[TraceCounter]
    """Busy time per toolchain stage."""
    stages: Sequence[TraceStage]
    """Summed time per named trace span."""
    times: Sequence[TraceTime]
    """Detailed artifact attempts."""
    artifacts: Sequence[TraceArtifact]

def encode_trace_report(writer: Writer, value: TraceReport) -> None: ...
def decode_trace_report(reader: Reader) -> TraceReport: ...

@dataclass(frozen=True, slots=True)
class TraceStage:
    """Busy time of one toolchain stage."""

    """The stage display name."""
    name: str
    """The summed attempt time in microseconds."""
    micros: int

def encode_trace_stage(writer: Writer, value: TraceStage) -> None: ...
def decode_trace_stage(reader: Reader) -> TraceStage: ...

@dataclass(frozen=True, slots=True)
class TraceTime:
    """Summed time of one named trace span."""

    """The span name."""
    name: str
    """The summed span time in microseconds."""
    micros: int

def encode_trace_time(writer: Writer, value: TraceTime) -> None: ...
def decode_trace_time(reader: Reader) -> TraceTime: ...

@dataclass(frozen=True, slots=True)
class TraceArtifact:
    """One artifact attempt in a detailed trace report."""

    """The artifact kind name."""
    name: str
    """The toolchain stage display name."""
    stage: str
    """The resolved artifact label."""
    label: str | None
    """The resolved target name."""
    target: str | None
    """The worker that executed the attempt."""
    worker: int
    """The offset from the run start in microseconds."""
    start_micros: int
    """The attempt duration in microseconds."""
    micros: int
    """The attempt outcome name."""
    outcome: str
    """Interior phases recorded by the executor or provider."""
    spans: Sequence[TraceSpan]
    """Counters recorded by the executor or provider."""
    counters: Sequence[TraceCounter]

def encode_trace_artifact(writer: Writer, value: TraceArtifact) -> None: ...
def decode_trace_artifact(reader: Reader) -> TraceArtifact: ...

@dataclass(frozen=True, slots=True)
class TraceSpan:
    """One detailed span in a trace report."""

    """The phase name."""
    name: str
    """The offset from the run start in microseconds."""
    start_micros: int
    """The phase duration in microseconds."""
    micros: int

def encode_trace_span(writer: Writer, value: TraceSpan) -> None: ...
def decode_trace_span(reader: Reader) -> TraceSpan: ...

@dataclass(frozen=True, slots=True)
class TraceCounter:
    """One detailed counter in a trace report."""

    """The counter name."""
    name: str
    """The counter value."""
    value: int

def encode_trace_counter(writer: Writer, value: TraceCounter) -> None: ...
def decode_trace_counter(reader: Reader) -> TraceCounter: ...

__all__ = [
    "TraceReport",
    "encode_trace_report",
    "decode_trace_report",
    "TraceStage",
    "encode_trace_stage",
    "decode_trace_stage",
    "TraceTime",
    "encode_trace_time",
    "decode_trace_time",
    "TraceArtifact",
    "encode_trace_artifact",
    "decode_trace_artifact",
    "TraceSpan",
    "encode_trace_span",
    "decode_trace_span",
    "TraceCounter",
    "encode_trace_counter",
    "decode_trace_counter",
]

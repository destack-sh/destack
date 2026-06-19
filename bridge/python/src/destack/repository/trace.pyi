# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

class TraceReport:
    """One bridge trace report."""

    """The wall time of the traced operation in microseconds."""
    @property
    def total_micros(self) -> int: ...

    """The number of workers that recorded attempts."""
    @property
    def workers(self) -> int: ...

    """Operation-level spans around artifact execution."""
    @property
    def spans(self) -> list[TraceSpan]: ...

    """Operation-level counters."""
    @property
    def counters(self) -> list[TraceCounter]: ...

    """Busy time per toolchain stage."""
    @property
    def stages(self) -> list[TraceStage]: ...

    """Time spent on attempts that blocked on requirements."""
    @property
    def blocked_micros(self) -> int: ...

    """Detailed artifact attempts."""
    @property
    def artifacts(self) -> list[TraceArtifact]: ...

class TraceStage:
    """Busy time of one toolchain stage."""

    """The stage display name."""
    @property
    def name(self) -> str: ...

    """The summed attempt time in microseconds."""
    @property
    def micros(self) -> int: ...

class TraceArtifact:
    """One artifact attempt in a detailed trace report."""

    """The artifact kind name."""
    @property
    def name(self) -> str: ...

    """The toolchain stage display name."""
    @property
    def stage(self) -> str: ...

    """The resolved artifact label."""
    @property
    def label(self) -> str | None: ...

    """The resolved target name."""
    @property
    def target(self) -> str | None: ...

    """The worker that executed the attempt."""
    @property
    def worker(self) -> int: ...

    """The offset from the run start in microseconds."""
    @property
    def start_micros(self) -> int: ...

    """The attempt duration in microseconds."""
    @property
    def micros(self) -> int: ...

    """The attempt outcome name."""
    @property
    def outcome(self) -> str: ...

    """Interior phases recorded by the executor or provider."""
    @property
    def spans(self) -> list[TraceSpan]: ...

    """Counters recorded by the executor or provider."""
    @property
    def counters(self) -> list[TraceCounter]: ...

class TraceSpan:
    """One detailed span in a trace report."""

    """The phase name."""
    @property
    def name(self) -> str: ...

    """The offset from the run start in microseconds."""
    @property
    def start_micros(self) -> int: ...

    """The phase duration in microseconds."""
    @property
    def micros(self) -> int: ...

class TraceCounter:
    """One detailed counter in a trace report."""

    """The counter name."""
    @property
    def name(self) -> str: ...

    """The counter value."""
    @property
    def value(self) -> int: ...

from __future__ import annotations

from destack._impl.model import ModelImpl


class MemoryEffectImpl(ModelImpl):
    """Methods for memory effects."""

    def is_none(self) -> bool:
        """Return whether this effect does not access memory."""

        return not self.reads and not self.writes

    def is_read_only(self) -> bool:
        """Return whether this effect only reads memory."""

        return self.reads and not self.writes

    def is_write_only(self) -> bool:
        """Return whether this effect only writes memory."""

        return not self.reads and self.writes

    def is_read_write(self) -> bool:
        """Return whether this effect may both read and write memory."""

        return self.reads and self.writes


class FunctionBehaviorImpl(ModelImpl):
    """Methods for function behavior metadata."""

    def may_unwind(self) -> bool:
        """Return whether the behavior may unwind execution."""

        return self.panic == "mayPanic" or self.suspend == "maySuspend"

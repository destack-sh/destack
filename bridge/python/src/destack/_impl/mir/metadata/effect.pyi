class MemoryEffectImpl:
    """Methods for memory effects."""

    def is_none(self) -> bool:
        """Return whether this effect does not access memory."""
        ...

    def is_read_only(self) -> bool:
        """Return whether this effect only reads memory."""
        ...

    def is_write_only(self) -> bool:
        """Return whether this effect only writes memory."""
        ...

    def is_read_write(self) -> bool:
        """Return whether this effect may both read and write memory."""
        ...

class FunctionBehaviorImpl:
    """Methods for function behavior metadata."""

    def may_unwind(self) -> bool:
        """Return whether the behavior may unwind execution."""
        ...

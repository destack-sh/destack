class TraceMapImpl:
    """Methods for heap trace maps."""

    def has_reference(self) -> bool:
        """Return whether this map can reach heap references."""
        ...

    def has_local_reference(self) -> bool:
        """Return whether this map can reach local heap references."""
        ...

    def has_shared_reference(self) -> bool:
        """Return whether this map can reach shared heap references."""
        ...

    def has_tagged_reference(self) -> bool:
        """Return whether this map requires reading payload tags while scanning."""
        ...

class TraceTableImpl:
    """Methods for trace tables."""

    def trace(self, id_):
        """Return one trace map by id."""
        ...

    def trace_maps(self):
        """Return all trace maps."""
        ...

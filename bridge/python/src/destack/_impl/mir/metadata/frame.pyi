class FrameTableImpl:
    """Methods for frame tables."""

    def layout(self, layout):
        """Return one frame layout by id."""
        ...

    def materialization(self, state):
        """Return one frame materialization by state id."""
        ...

class FrameMaterializationImpl:
    """Methods for frame materialization plans."""

    def slots(self):
        """Return each source frame slot once."""
        ...

class FrameLayoutImpl:
    """Methods for frame layouts."""

    def slot(self, id_):
        """Return one frame slot by id."""
        ...

    def value_slot_id(self, value: int):
        """Return the frame slot id for one SSA value."""
        ...

    def local_slot_id(self, local: int):
        """Return the frame slot id for one local."""
        ...

    def value(self, value: int):
        """Return the value slot at one SSA value index."""
        ...

    def local(self, local: int):
        """Return the local slot at one local index."""
        ...

    def value_for_slot(self, id_) -> int | None:
        """Return the SSA value addressed by one slot id."""
        ...

    def local_for_slot(self, id_) -> int | None:
        """Return the local addressed by one slot id."""
        ...

    def is_environment_slot(self, id_) -> bool:
        """Return whether one slot id addresses the callable environment."""
        ...

    def environment(self):
        """Return the callable environment slot when present."""
        ...

    def values(self):
        """Return all value slots."""
        ...

    def locals(self):
        """Return all local slots."""
        ...

    def slot_len(self) -> int:
        """Return the frame slot count."""
        ...

    def slot_ids(self):
        """Return all frame slot ids."""
        ...

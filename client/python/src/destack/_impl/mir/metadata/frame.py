from __future__ import annotations

from destack._impl.model import ModelImpl


class FrameTableImpl(ModelImpl):
    """Methods for frame tables."""

    def layout(self, layout):
        """Return one frame layout by id."""

        return sequence_get(self.layouts, layout)

    def materialization(self, state):
        """Return one frame materialization by state id."""

        return sequence_get(self.materializations, state)


class FrameMaterializationImpl(ModelImpl):
    """Methods for frame materialization plans."""

    def slots(self):
        """Return each source frame slot once."""

        return self.copied_slots


class FrameLayoutImpl(ModelImpl):
    """Methods for frame layouts."""

    def slot(self, id_):
        """Return one frame slot by id."""

        return sequence_get(self.slots, id_)

    def value_slot_id(self, value: int):
        """Return the frame slot id for one SSA value."""

        if value >= self.value_count:
            return None

        return value

    def local_slot_id(self, local: int):
        """Return the frame slot id for one local."""

        if local >= self.local_count:
            return None

        return self.value_count + local

    def value(self, value: int):
        """Return the value slot at one SSA value index."""

        if value >= self.value_count:
            return None

        return sequence_get(self.slots, value)

    def local(self, local: int):
        """Return the local slot at one local index."""

        if local >= self.local_count:
            return None

        index = self.value_count + local

        return sequence_get(self.slots, index)

    def value_for_slot(self, id_) -> int | None:
        """Return the SSA value addressed by one slot id."""

        if id_ < self.value_count:
            return id_

        return None

    def local_for_slot(self, id_) -> int | None:
        """Return the local addressed by one slot id."""

        if id_ < self.value_count:
            return None

        local = id_ - self.value_count

        return local if local < self.local_count else None

    def is_environment_slot(self, id_) -> bool:
        """Return whether one slot id addresses the callable environment."""

        return self.environment_slot is not None and self.environment_slot == id_

    def environment(self):
        """Return the callable environment slot when present."""

        if self.environment_slot is None:
            return None

        return self.slot(self.environment_slot)

    def values(self):
        """Return all value slots."""

        return self.slots[: self.value_count]

    def locals(self):
        """Return all local slots."""

        start = self.value_count
        end = start + self.local_count

        return self.slots[start:end]

    def slot_len(self) -> int:
        """Return the frame slot count."""

        return len(self.slots)

    def slot_ids(self):
        """Return all frame slot ids."""

        from destack._generated.mir.metadata.frame import FrameSlotId

        return tuple(range(len(self.slots)))


def sequence_get(sequence, index: int):
    """Return one sequence item when the index is present."""

    if 0 <= index < len(sequence):
        return sequence[index]

    return None

from __future__ import annotations

from destack._impl.model import ModelImpl


class TraceMapImpl(ModelImpl):
    """Methods for heap trace maps."""

    def has_reference(self) -> bool:
        """Return whether this map can reach heap references."""

        return self.has_local_reference() or self.has_shared_reference()

    def has_local_reference(self) -> bool:
        """Return whether this map can reach local heap references."""

        if self.kind == "empty":
            return False

        # fixed maps carry direct local offsets
        if self.kind == "fixed":
            return len(self.local_offsets) > 0

        # nested maps delegate to their child map
        if self.kind == "nested":
            return self.map.has_local_reference()

        # composite maps aggregate child maps
        if self.kind == "composite":
            return any(map_.has_local_reference() for map_ in self.maps)

        # repeated maps only count when at least one element exists
        if self.kind == "repeated":
            return self.count > 0 and self.element.has_local_reference()

        return any(variant.map.has_local_reference() for variant in self.variants)

    def has_shared_reference(self) -> bool:
        """Return whether this map can reach shared heap references."""

        if self.kind == "empty":
            return False

        # fixed maps carry direct shared offsets
        if self.kind == "fixed":
            return len(self.shared_offsets) > 0

        # nested maps delegate to their child map
        if self.kind == "nested":
            return self.map.has_shared_reference()

        # composite maps aggregate child maps
        if self.kind == "composite":
            return any(map_.has_shared_reference() for map_ in self.maps)

        # repeated maps only count when at least one element exists
        if self.kind == "repeated":
            return self.count > 0 and self.element.has_shared_reference()

        return any(variant.map.has_shared_reference() for variant in self.variants)

    def has_tagged_reference(self) -> bool:
        """Return whether this map requires reading payload tags while scanning."""

        if self.kind in {"empty", "fixed"}:
            return False

        # nested maps delegate to their child map
        if self.kind == "nested":
            return self.map.has_tagged_reference()

        # composite maps aggregate child maps
        if self.kind == "composite":
            return any(map_.has_tagged_reference() for map_ in self.maps)

        # repeated maps delegate to their element map
        if self.kind == "repeated":
            return self.element.has_tagged_reference()

        return any(variant.map.has_reference() for variant in self.variants)


class TraceTableImpl(ModelImpl):
    """Methods for trace tables."""

    def trace(self, id_):
        """Return one trace map by id."""

        return sequence_get(self.traces, id_ - 1)

    def trace_maps(self):
        """Return all trace maps."""

        return self.traces


def sequence_get(sequence, index: int):
    """Return one sequence item when the index is present."""

    if 0 <= index < len(sequence):
        return sequence[index]

    return None

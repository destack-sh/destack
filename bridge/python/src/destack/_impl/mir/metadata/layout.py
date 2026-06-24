from __future__ import annotations

from destack._impl.model import ModelImpl


class LayoutMetadataImpl(ModelImpl):
    """Methods for layout metadata."""

    def type_layout(self, ty):
        """Return the layout entry for a type id when available."""

        layout = self.layout_id(ty)

        return None if layout is None else self.layout_table.layout(layout)

    def layout_id(self, ty):
        """Return the layout id for a type when present."""

        return self.layout_by_type.get(ty)


class LayoutTableImpl(ModelImpl):
    """Methods for layout tables."""

    def layout(self, id_):
        """Return a layout entry for an id."""

        return sequence_get(self.layouts, id_ - 1)


class LayoutImpl(ModelImpl):
    """Methods for concrete layouts."""

    def dynamic_dispatch_offset(self) -> int | None:
        """Return the byte offset of a dynamic dispatch pointer."""

        if self.shape.kind == "dynamic":
            return self.alignment

        return None

    def byte_len(self) -> int:
        """Return the byte width of this layout."""

        return self.size

    def field_at(self, index: int):
        """Return one field by layout index."""

        return sequence_get(self.shape.fields(), index)

    def field_count(self) -> int | None:
        """Return the field count for field-addressable layouts."""

        if self.shape.kind == "struct":
            return len(self.shape.struct.fields)

        # tuple layouts address elements as fields
        if self.shape.kind == "tuple":
            return len(self.shape.tuple.elements)

        # object layouts address instance fields
        if self.shape.kind == "object":
            return len(self.shape.object.fields)

        return None


class LayoutShapeImpl(ModelImpl):
    """Methods for layout shapes."""

    def elements(self):
        """Return element layout when this shape stores indexed elements inline."""

        if self.kind == "array":
            return self.array

        # vectors share the same inline element layout form
        if self.kind == "vector":
            return self.vector

        return None

    def fields(self):
        """Return field layouts for field-addressable shapes."""

        if self.kind == "struct":
            return self.struct.fields

        # tuple layouts address elements as fields
        if self.kind == "tuple":
            return self.tuple.elements

        # object layouts address instance fields
        if self.kind == "object":
            return self.object.fields

        return ()


def sequence_get(sequence, index: int):
    """Return one sequence item when the index is present."""

    if 0 <= index < len(sequence):
        return sequence[index]

    return None

class LayoutMetadataImpl:
    """Methods for layout metadata."""

    def type_layout(self, ty):
        """Return the layout entry for a type id when available."""
        ...

    def layout_id(self, ty):
        """Return the layout id for a type when present."""
        ...

class LayoutTableImpl:
    """Methods for layout tables."""

    def layout(self, id_):
        """Return a layout entry for an id."""
        ...

class LayoutImpl:
    """Methods for concrete layouts."""

    def dynamic_dispatch_offset(self) -> int | None:
        """Return the byte offset of a dynamic dispatch pointer."""
        ...

    def byte_len(self) -> int:
        """Return the byte width of this layout."""
        ...

    def field_at(self, index: int):
        """Return one field by layout index."""
        ...

    def field_count(self) -> int | None:
        """Return the field count for field-addressable layouts."""
        ...

class LayoutShapeImpl:
    """Methods for layout shapes."""

    def elements(self):
        """Return element layout when this shape stores indexed elements inline."""
        ...

    def fields(self):
        """Return field layouts for field-addressable shapes."""
        ...

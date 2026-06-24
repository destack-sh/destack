from destack._generated.mir.tree.type import Access, Nullability, ReferenceKind, Space

class ReferenceMetaImpl:
    """Methods for reference metadata."""

    def kind(self) -> ReferenceKind | None:
        """Return the encoded reference kind."""
        ...

    def access(self) -> Access | None:
        """Return the encoded reference access."""
        ...

    def nullability(self) -> Nullability:
        """Return the encoded nullability."""
        ...

    def space(self) -> Space:
        """Return the encoded memory space."""
        ...

from __future__ import annotations

from destack._impl.model import ModelImpl


class DispatchMetadataImpl(ModelImpl):
    """Methods for dispatch metadata."""

    def vtable(self, ty):
        """Return the vtable for a type when present."""

        for table in self.vtables:
            # compare structural ids
            if table.ty == ty:
                return table

        return None

    def dynamic_table(self, concrete, constraint):
        """Return the dynamic table for a concrete type and constraint."""

        for table in self.dynamic_tables:
            is_concrete = table.concrete == concrete
            is_constraint = table.constraint == constraint

            # return the matching table
            if is_concrete and is_constraint:
                return table

        return None

    def dynamic_shape(self, constraint):
        """Return dynamic shape metadata for a constraint type id."""

        return self.dynamic_shapes.get(constraint)

from __future__ import annotations

from destack._impl.model import ModelImpl


class MemoryMetadataImpl(ModelImpl):
    """Methods for memory metadata."""

    def memory_accesses(self, instruction):
        """Return memory accesses for an instruction when available."""

        return self.memory_accesses_by_instruction_id.get(instruction, ())

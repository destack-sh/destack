from __future__ import annotations

from destack._impl.model import ModelImpl


class FunctionMetadataTableImpl(ModelImpl):
    """Methods for function metadata tables."""

    def function(self, function):
        """Return metadata for one function."""

        return self.functions.get(function)

    def call(self, callsite):
        """Return metadata for one callsite."""

        return self.calls.get(callsite)

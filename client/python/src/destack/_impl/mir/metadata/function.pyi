class FunctionMetadataTableImpl:
    """Methods for function metadata tables."""

    def function(self, function):
        """Return metadata for one function."""
        ...

    def call(self, callsite):
        """Return metadata for one callsite."""
        ...

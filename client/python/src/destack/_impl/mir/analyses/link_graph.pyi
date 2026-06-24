class CallComponentGraphImpl:
    """Methods for call component graphs."""

    def component_id(self, symbol: int) -> int:
        """Return the component id of a symbol by dense id."""
        ...

    def is_recursive(self, symbol: int) -> bool:
        """Return whether a symbol's component contains a cycle."""
        ...

    def len(self) -> int:
        """Return the number of components."""
        ...

    def is_empty(self) -> bool:
        """Return whether there are no components."""
        ...

class LinkGraphImpl:
    """Methods for MIR link graphs."""

    def node(self, symbol):
        """Return one symbol node."""
        ...

    def defined_symbols(self):
        """Return all defined symbols and their nodes."""
        ...

    def edges_for(self, source):
        """Return the outgoing references from one source symbol."""
        ...

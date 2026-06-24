from __future__ import annotations

from destack._impl.model import ModelImpl


class CallComponentGraphImpl(ModelImpl):
    """Methods for call component graphs."""

    def component_id(self, symbol: int) -> int:
        """Return the component id of a symbol by dense id."""

        return self.component[symbol]

    def is_recursive(self, symbol: int) -> bool:
        """Return whether a symbol's component contains a cycle."""

        return self.recursive.contains(self.component[symbol])

    def len(self) -> int:
        """Return the number of components."""

        return self.recursive.len()

    def is_empty(self) -> bool:
        """Return whether there are no components."""

        return self.recursive.is_empty()


class LinkGraphImpl(ModelImpl):
    """Methods for MIR link graphs."""

    def node(self, symbol):
        """Return one symbol node."""

        return self.nodes.get(symbol)

    def defined_symbols(self):
        """Return all defined symbols and their nodes."""

        return tuple(self.nodes.items())

    def edges_for(self, source):
        """Return the outgoing references from one source symbol."""

        return self.edges.get(source, ())

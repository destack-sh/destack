from typing import TYPE_CHECKING, Collection, cast
from uuid import UUID

from bench.language.const import StructType
from bench.language.graph import NodeList
from bench.language.node import Node, Struct, struct_

if TYPE_CHECKING:
    pass


class NodeVisitor:
    def __init__(self):
        self._reference_by_ck: dict[UUID, Node] = {}

    def __str__(self):
        return f"{len(self._reference_by_ck)} nodes"

    def __repr__(self):
        return f"<NodeVisitor {self!s}>"

    @property
    def references(self) -> Collection["Node"]:
        return self._reference_by_ck.values()

    def visit_reference(self, node: "Node"):
        self._reference_by_ck[node.ck] = node


@struct_(StructType.PROJECTION)
class Projection(Struct):
    """
    A projection into the graph.
    NOTE :Incomplete :Architecture: figure out projection
     - how do we filter and LoD this?
     - how do we represent unloaded nodes?
     - how do we make projections reproducible and inspectable in the editor?
    """

    ...


def project_node(node: Node) -> list[Node]:
    """
    Gather the references and descendants of the given node, recursively.
    See Projection for details.
    """
    seen_by_ck: dict[UUID, Node] = {}
    to_visit: list[Node] = [node]

    def _visit_node(node: Node):
        if node.ck in seen_by_ck:
            return
        seen_by_ck[node.ck] = node

        # visit children
        for prop in node.__node_child_properties__.values():
            prop_value = cast(NodeList, getattr(node, prop.name))
            for child in prop_value:
                _visit_node(child)
        # visit references in all contained structs
        for struc in node._walk_struct():
            for prop in struc.__node_reference_properties__.values():
                if prop.id is None or prop.id < 30:  # skip system properties (incl. parent)
                    continue
                elif prop.is_list:
                    prop_value = cast(NodeList | None, getattr(struc, prop.name))
                    if prop_value:
                        for child in prop_value:
                            _visit_node(child)
                else:
                    ref = cast(Node | None, getattr(struc, prop.name))
                    if ref:
                        _visit_node(ref)

    # traverse
    while to_visit:
        node = to_visit.pop()
        _visit_node(node)

    return list(seen_by_ck.values())

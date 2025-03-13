from typing import cast
from uuid import UUID, uuid5

from bench.builtin.resource import BrowserPage, ComputerPage
from bench.builtin.source import ActionPage
from bench.language import (
    BENCH_BUILTIN_PACKAGE_ID,
    BENCH_BUILTIN_PACKAGE_SLUG,
    UUID_NAMESPACE,
    IsInstantiable,
    IsTraceable,
    NodeGraph,
    NodeMode,
    NodeReference,
    Package,
    PackageType,
)


def assign_builtin_ids(graph: NodeGraph):
    """Assign deterministic ids to the Nodes in the graph."""
    assigned_ptrs_by_node: dict[UUID, NodeReference] = {}

    # set deterministic ids
    for node in graph.nodes:
        old_node_id = node.id
        node.id = uuid5(namespace=UUID_NAMESPACE, name=node.absolute_path)
        if isinstance(node, IsInstantiable):
            cast(IsInstantiable, node).ck = uuid5(namespace=UUID_NAMESPACE, name=node.absolute_path)
        assigned_ptrs_by_node[old_node_id] = node.to_ref()

    # update references & reindex
    for node in graph.nodes:
        for prop in node.__wired_properties__.values():
            if not prop.is_node_reference or prop.is_computed:
                continue
            prop_value = getattr(node, prop.name)
            if not prop_value:
                continue
            if prop.is_list:
                new_value = [assigned_ptrs_by_node.get(v.id, v) for v in prop_value]
            else:
                new_value = assigned_ptrs_by_node.get(prop_value.id, prop_value)
            setattr(node, prop.name, new_value)

    graph._reindex()


BuiltinPackage = Package(
    type=PackageType.OPEN, id=BENCH_BUILTIN_PACKAGE_ID, slug=BENCH_BUILTIN_PACKAGE_SLUG
)
BuiltinPackage.extend(ActionPage, BrowserPage, ComputerPage)

# finalize
for node in BuiltinPackage._graph.nodes:
    if isinstance(node, IsTraceable) and node.mode == NodeMode.PRODUCTION:
        node.mode = NodeMode.BUILTIN
assign_builtin_ids(BuiltinPackage._graph)

from typing import cast
from uuid import UUID, uuid5

from bench.language import (
    BENCH_BUILTIN_PACKAGE_ID,
    BENCH_BUILTIN_PACKAGE_SLUG,
    UUID_NAMESPACE,
    IsInstantiable,
    IsTraceable,
    Node,
    NodeGraph,
    NodeMode,
    NodeReference,
    Package,
    PackageType,
    Session,
)


def sync_node(*, parent: Node, old_root: Node | None, new_root: Node) -> None:
    """Patches the old node *in place* from the new node (recursively)."""

    def _copy(node: Node, *, detach: bool) -> Node:
        """Copy the node (exact non-recursive clone with preserved identity)."""
        return node.clone(recursive=False, reset=False, detach=detach)

    def _sync(old: Node, new: Node) -> None:
        """Sync the old node *in place* from the new node."""
        for prop in old.__wired_properties__.values():
            if prop.id < 30 or prop.is_computed or prop.name == "order_key":
                continue  # ignore internal properties
            old_value = getattr(old, prop.name)
            new_value = getattr(new, prop.name)
            if old_value != new_value:
                old._do_set(prop.name, new_value, track=True)

    if old_root is None:
        old_root = _copy(new_root, detach=False)
        parent.append(old_root)

    # create/update new nodes
    _sync(old_root, new_root)
    for new in new_root.iter_descendants(recursive=True):
        old = old_root._graph.get(new.id)
        if old is None:
            new_copy = _copy(new, detach=True)
            if new.parent_ptr is None:
                old_parent = new_copy
            else:
                old_parent = old_root._graph.get(new.parent_ptr.id)
                assert isinstance(old_parent, Node), f"unexpected {old_parent!r} for {new!r}"
            old_parent.append(new_copy)
        else:
            assert isinstance(old, Node), f"unexpected {old!r} for {new!r}"
            _sync(old, new)

    # remove old nodes
    for old in parent.iter_descendants(recursive=True):
        if old.id not in new_root._graph:
            old.delete()


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


def make_builtin_package(session: Session) -> Package:
    """
    Create the Builtins (detached).
    """
    pkg = Package(
        type=PackageType.OPEN, id=BENCH_BUILTIN_PACKAGE_ID, slug=BENCH_BUILTIN_PACKAGE_SLUG
    )

    from .computer import ComputerPage

    pkg.append(ComputerPage)

    #
    # Finalize
    #

    for node in pkg._graph.nodes:
        if isinstance(node, IsTraceable) and node.mode == NodeMode.PRODUCTION:
            node.mode = NodeMode.BUILTIN

    assign_builtin_ids(pkg._graph)

    return pkg

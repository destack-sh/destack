from typing import Collection, cast
from uuid import UUID, uuid5

from bench.language import UUID_NAMESPACE, IsInstantiable, Node, NodeGraph, NodeReference


def assign_builtin_ids(graph: NodeGraph, ignore: Collection[Node] = ()):
    """
    Assign deterministic ids/cks to the Nodes in the graph (derived from their absolute path).
    """
    assigned_ptrs_by_node: dict[UUID, NodeReference] = {}

    # set deterministic ids
    for node in graph.nodes:
        if node in ignore:
            continue
        old_node_id = node.id
        node.id = uuid5(namespace=UUID_NAMESPACE, name=node.absolute_path)
        if isinstance(node, IsInstantiable):
            cast(IsInstantiable, node).ck = node.id
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


def patch_node(target: Node, reference: Node) -> None:
    """Patch the target node *in place* from the reference node."""
    for prop in target.__wired_properties__.values():
        if prop.id < 30 or prop.is_computed:
            continue  # ignore internal properties
        target_value = getattr(target, prop.name)
        reference_value = getattr(reference, prop.name)
        if target_value != reference_value:
            target._do_set(prop.name, reference_value, track=True)


def sync_node(*, parent: Node, target_root: Node, reference_root: Node) -> None:
    """Patches the target node *in place* from the reference node (recursively)."""

    # create/update target nodes
    patch_node(target_root, reference_root)
    for reference in reference_root.iter_descendants(recursive=True):
        target = target_root._graph.get(reference.id)
        if target is None:
            target = reference.clone(
                recursive=False,
                reset=False,
                detach=True,
                map=False,
                _graph=target_root._graph,
            )
            if reference.parent_ptr is None:
                target_parent = target_root
            else:
                parent_id = reference.parent_ptr.id
                assert parent_id is not None, f"unexpected {reference!r} has no parent"
                target_parent = target_root._graph.get(parent_id)
                assert target_parent is not None, f"missing parent {parent_id!r} for {reference!r}"
            target_parent.append(target)
        else:
            patch_node(target, reference)

    # remove old nodes
    for reference in parent.iter_descendants(recursive=True):
        if reference.id not in target_root._graph:
            reference.delete()

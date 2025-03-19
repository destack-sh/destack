from typing import Collection, cast
from uuid import UUID, uuid5

from bench.language import (
    UUID_NAMESPACE,
    Block,
    IsInstantiable,
    IsTemplatable,
    Node,
    NodeGraph,
    NodeReference,
)


def get_stable_builtin_path(node: Node) -> str:
    """Get a deterministic absolute path for a Node."""

    # assemble path (like in Path.render)
    path_parts: list[str] = []
    current = node
    while current is not None:
        path_key = current._ident
        if path_key is None:
            if isinstance(current, Block):
                inline_node = current.node
                assert inline_node is not None, f"block {current!r} has no inline node"
                inline_ident = inline_node._ident
                assert inline_ident is not None, f"inline node {inline_node!r} has no ident"
                path_key = f"Block[{inline_ident}]"
            else:
                raise ValueError(f"node {current!r} has no ident")

        path_parts.append(path_key)
        next_parent = current.parent
        if next_parent is None and current.metatype in node.__roots__:
            break  # reached the root
        current = next_parent

    return "/".join(reversed(path_parts))


def assign_builtin_ids(graph: NodeGraph, ignore: Collection[Node] = ()) -> None:
    """
    Assign deterministic ids/cks to the Nodes in the graph (derived from their absolute path).
    """
    assigned_ptrs_by_node: dict[UUID, NodeReference] = {}

    # deduplicate paths
    path_by_node: dict[Node, str] = {}
    node_by_path: dict[str, Node] = {}
    for node in graph.nodes:
        if node in ignore:
            continue
        path = get_stable_builtin_path(node)
        path_by_node[node] = path
        if path in node_by_path:
            raise ValueError(f"duplicate path {path!r} for {node!r} and {node_by_path[path]!r}")
        node_by_path[path] = node

    # set deterministic ids
    for node in graph.nodes:
        if node in ignore:
            continue
        old_node_id = node.id
        node.id = uuid5(namespace=UUID_NAMESPACE, name=path_by_node[node])
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


def sync_node(
    *,
    parent: Node,
    target: Node | None,
    reference: Node,
    recursive: bool = True,
    _is_root: bool = True,
) -> None:
    """Patches the target node *in place* from the reference node (recursively)."""

    if target is None or target.metatype != reference.metatype:
        if target is not None:
            # replace target completely
            target.erase()
        # target doesn't have that node, create id
        target = reference.clone(
            recursive=recursive,
            reset=False,
            detach=True,
            map=False,
            _graph=parent._graph,
        )
        if reference.parent_ptr is None:
            target_parent = parent
        else:
            parent_id = reference.parent_ptr.id
            assert parent_id is not None, f"unexpected {reference!r} has no parent"
            target_parent = parent._graph.get(parent_id)
            assert target_parent is not None, f"missing parent {parent_id!r} for {reference!r}"
        target_parent.append(target)
    else:
        # target has that node, diff it
        patch_node(target, reference)
        if recursive:
            for reference_child in reference.iter_descendants(recursive=False):
                target_child = target._graph.get(reference_child.id)
                sync_node(
                    parent=target,
                    target=target_child,
                    reference=reference_child,
                    recursive=recursive,
                    _is_root=False,
                )

    # remove old nodes
    if _is_root:
        for target_child in target.iter_descendants(recursive=True):
            if target_child.id not in reference._graph and (
                not isinstance(target_child, IsTemplatable) or target_child.template_ptr is not None
            ):
                target_child.erase()

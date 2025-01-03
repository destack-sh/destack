from uuid import UUID, uuid5

from bench.language.core import (
    UUID_NAMESPACE,
    BlockType,
    NodeGraph,
    NodeMode,
    NodeReference,
    SourceNode,
)
from bench.language.registry import _complete_bench_setup
from bench.language.runtime.session import Session
from bench.language.source import Block

_complete_bench_setup()


def _assign_builtin_ids(graph: NodeGraph):
    """Assign deterministic ids to the Nodes in the graph."""
    assigned_ptrs_by_node: dict[UUID, NodeReference] = {}

    # set deterministic ids
    for node in graph.nodes:
        assert isinstance(node, SourceNode), f"unexpected {node!r}"
        old_node_id = node.id
        node.ck = uuid5(namespace=UUID_NAMESPACE, name=node.absolute_path)
        node.id = uuid5(namespace=node.ck, name="")
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


def make_builtins(session: Session) -> Block:
    """
    Create the Builtins (detached).
    NOTE :Incomplete: we can't really use Builtins (until we have :Dependencies)
    """
    Builtins = Block.new(BlockType.PAGE, "Builtins")

    ...

    #
    # Finalize
    #

    for node in Builtins._graph.nodes:
        assert isinstance(node, SourceNode), f"unexpected {node!r}"
        node.mode = NodeMode.BUILTIN
    _assign_builtin_ids(Builtins._graph)

    return Builtins

from typing import Mapping
from uuid import UUID, uuid5

from bench.language.action import Agency
from bench.language.block import Block
from bench.language.const import UUID_NAMESPACE, BlockType
from bench.language.field import Field
from bench.language.flow import StepType
from bench.language.graph import NodeGraph
from bench.language.node import Node, NodeReference, SourceNode
from bench.language.text import Text
from bench.language.validation import constraint

BUILTINS = Block.new(BlockType.PAGE, "Builtins")

#
# Implementations (stubs)
# (adding schemas and the like for every :RichBuiltin)
#

IMPLEMENTATIONS = Block.new(BlockType.PAGE, "Implementations")
BUILTINS.blocks.append(IMPLEMENTATIONS)

# step
STUB_BY_STEP_TYPE: Mapping[StepType, Block] = {
    StepType.CREATE: Block.new(
        BlockType.ACTION,
        "Create",
        agency=Agency.CODE,
        fields=(
            Field.input(
                "Input", Node, is_required=True, constraint=constraint(node_is_attached=False)
            ),
            Field.output(
                "Node", Node, is_required=True, constraint=constraint(node_is_attached=True)
            ),
        ),
    ),
    StepType.FAIL: Block.new(
        BlockType.ACTION,
        "Fail",
        agency=Agency.CODE,
        fields=(
            Field.input("title", str),
            Field.input("text", Text),
            Field.input("node", Node),
        ),
    ),
}
IMPLEMENTATIONS.blocks.extend(*STUB_BY_STEP_TYPE.values())

#
# Computer
#

COMPUTER = Block.new(BlockType.PAGE, "Computer")
BUILTINS.blocks.append(COMPUTER)


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
            if not prop.is_node_reference:
                continue
            prop_value = getattr(node, prop.name)
            if not prop_value:
                continue
            if prop.is_list:
                new_value = [assigned_ptrs_by_node[v.id] for v in prop_value]
            else:
                new_value = assigned_ptrs_by_node[prop_value.id]
            setattr(node, prop.name, new_value)

    graph._reindex()


_assign_builtin_ids(BUILTINS._graph)

from typing import Mapping
from uuid import UUID, uuid5

from bench.language.action import Agency
from bench.language.block import Block
from bench.language.connection import NullEngine
from bench.language.const import NODE_TYPES, UUID_NAMESPACE, BlockType, NodeMode, _active_session
from bench.language.field import Field
from bench.language.flow import StepType
from bench.language.graph import NodeGraph, NodeSuperGraph
from bench.language.node import EMPTY_SCOPE, Node, NodeReference, SourceNode
from bench.language.session import Session
from bench.language.setup import _complete_bench_setup
from bench.language.text import Text
from bench.language.validation import constraint
from bench.utils.oracle import REAL_ORACLE

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


_supergraph = NodeSuperGraph(root_ptr=None)
_session = Session(
    _engines=(NullEngine(scope=EMPTY_SCOPE._to_data(), node_types=NODE_TYPES),),
    _supergraph=_supergraph,
    _oracle=REAL_ORACLE,
)
_token = _active_session.set(_session)

Builtins = Block.new(BlockType.PAGE, "Builtins")

#
# Stubs (schemas and the like for :RichBuiltin implementations)
#

Stubs = Block.new(BlockType.PAGE, "Stubs")
Builtins.blocks.append(Stubs)

# step
Steps = Block.new(BlockType.PAGE, "Steps")
Stubs.blocks.append(Steps)
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
Steps.blocks.extend(*STUB_BY_STEP_TYPE.values())


#
# Computer
#

Computer = Block.new(BlockType.PAGE, "Computer")
Builtins.blocks.append(Computer)

# complete builtins
for node in Builtins._graph.nodes:
    assert isinstance(node, SourceNode), f"unexpected {node!r}"
    node.mode = NodeMode.BUILTIN
_assign_builtin_ids(Builtins._graph)

_active_session.reset(_token)

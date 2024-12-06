from typing import Mapping
from uuid import UUID, uuid5

from bench.language.action import Agency
from bench.language.block import Block
from bench.language.const import UUID_NAMESPACE, BlockType, NodeMode
from bench.language.field import Field
from bench.language.flow import StepType
from bench.language.graph import NodeGraph
from bench.language.node import Node, NodeReference, SourceNode
from bench.language.registry import _complete_bench_setup
from bench.language.session import Session

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


def make_builtins(session: Session) -> Block:
    """
    Create the Builtins (detached).
    NOTE :Incomplete: we can't really use Builtins (until we have :Dependencies)
    """
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
        # read
        StepType.GET: Block.new(
            BlockType.ACTION,
            "Get",
            agency=Agency.CODE,
            fields=(Field.output("Node", Node, is_required=True),),
        ),
        StepType.SEARCH: Block.new(
            BlockType.ACTION,
            "Search",
            agency=Agency.CODE,
            fields=(Field.output("Nodes", Node, is_list=True, is_required=True),),
        ),
        # write
        StepType.CREATE: Block.new(
            BlockType.ACTION,
            "Create",
            agency=Agency.CODE,
            fields=(Field.output("Node", Node, is_required=True),),
        ),
    }
    Steps.blocks.extend(*STUB_BY_STEP_TYPE.values())

    #
    # Computer
    #

    Computer = Block.new(BlockType.PAGE, "Computer")
    Builtins.blocks.append(Computer)

    #
    # Finalize
    #

    for node in Builtins._graph.nodes:
        assert isinstance(node, SourceNode), f"unexpected {node!r}"
        node.mode = NodeMode.BUILTIN
    _assign_builtin_ids(Builtins._graph)

    return Builtins

from typing import Type
from uuid import UUID

from bench.function.base import Function, get_function_cls
from bench.models import FlowArtifactEdge, FlowNode, FlowNodeEdge
from bench.utils.spec import FunctionType


def compute_flow_node_specs(
    nodes: list[FlowNode],
    node_edges: list[FlowNodeEdge],
    artifact_edges: list[FlowArtifactEdge],
) -> dict[UUID, FunctionType]:
    function_cls_by_node: dict[UUID, Type[Function]] = {}

    for node in nodes:
        function_cls_by_node[node.id] = get_function_cls(node.function_id)

    pass

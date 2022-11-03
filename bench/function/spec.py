from typing import Callable, Union
from uuid import UUID

import structlog

from bench.artifact.base import ArtifactHandler
from bench.dataset.base import DatasetHandler
from bench.executor.base import ArtifactConnection, prepare_function_arguments
from bench.function.base import RecordFunction, load_function
from bench.model.base import ModelHandler
from bench.models import (
    ArtifactVersion,
    DatasetVersion,
    FlowArtifactEdge,
    FlowNode,
    FlowNodeEdge,
    FlowVersion,
    ModelVersion,
)
from bench.models.execution import ExecutionArtifactConnection
from bench.models.flow import FlowNodeMetadata
from bench.models.utils import DATASET_TYPE, MODEL_TYPE
from bench.utils.func import terrible_cast
from bench.utils.record import Record, RecordBatch
from bench.utils.spec import FunctionType

logger = structlog.stdlib.get_logger()


class StubError(NotImplementedError):
    pass


class StubModelHandler(ModelHandler):
    pass


def get_flow_node_specs(
    nodes: list[FlowNode],
    node_edges: list[FlowNodeEdge],
    artifact_edges: list[FlowArtifactEdge],
) -> dict[UUID, FunctionType]:
    functions: dict[UUID, RecordFunction] = {}
    stub_artifact_loaders: dict[str, Callable[[ArtifactVersion], ArtifactHandler]] = {
        MODEL_TYPE: lambda model: StubModelHandler(
            artifact_id=model.artifact.id, spec=terrible_cast(ModelVersion, model).spec
        ),
        DATASET_TYPE: lambda dataset: StubDatasetHandler(
            artifact_id=dataset.artifact.id, spec=terrible_cast(DatasetVersion, dataset).spec
        ),
    }

    # load functions with all config arguments (incl. artifacts stubs)
    for node in nodes:
        node_artifact_arguments = [
            ArtifactConnection.from_edge(art)
            for art in artifact_edges
            if art.dependent == node
            and art.connection_type == ExecutionArtifactConnection.ConnectionType.Argument
        ]

        try:
            arguments = prepare_function_arguments(
                node, node_artifact_arguments, stub_artifact_loaders
            )
            function = load_function(node.function_id, arguments=arguments)
        except (TypeError, ValueError) as e:
            logger.warning(f"failed to load function while updating spec for node {node}: {e}")
            continue

        if not isinstance(function, RecordFunction):
            raise ValueError(f"function not yet supported: {function}")
        functions[node.id] = function

    # base function specs only dependent on artifacts
    function_specs: dict[UUID, FunctionType] = {}
    for node in nodes:
        if node.id not in functions:
            # couldn't load that function
            continue

        function = functions[node.id]
        function_specs[node.id] = FunctionType(
            input_spec=function.input_spec, output_spec=function.output_spec
        )

    return function_specs


def update_flow_spec(flow: FlowVersion) -> list[FlowNode]:
    flow_node_specs = get_flow_node_specs(
        nodes=flow.nodes.all(),
        node_edges=flow.node_edges.all(),
        artifact_edges=flow.artifact_edges.all(),
    )
    updated_nodes = []
    for node in flow.nodes.all():
        if node.id not in flow_node_specs:
            continue  # spec could not be computed
        spec = flow_node_specs[node.id]
        new_metadata = FlowNodeMetadata(spec.input_spec, spec.output_spec).to_dict()
        if new_metadata != node.metadata:
            node.metadata = new_metadata
            updated_nodes.append(node)
    return updated_nodes

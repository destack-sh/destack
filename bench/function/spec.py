from typing import Callable, Union
from uuid import UUID

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
    ModelVersion,
)
from bench.models.execution import ExecutionArtifactConnection
from bench.models.utils import DATASET_TYPE, MODEL_TYPE
from bench.utils.func import terrible_cast
from bench.utils.record import Record, RecordBatch
from bench.utils.spec import FunctionType


class StubError(NotImplementedError):
    pass


class StubModelHandler(ModelHandler):
    def predict(self, record: Record) -> Union[Record, RecordBatch]:
        raise StubError

    def predict_batch(self, records: RecordBatch) -> RecordBatch:
        raise StubError


class StubDatasetHandler(DatasetHandler):
    pass


def get_flow_node_specs(
    nodes: list[FlowNode],
    node_edges: list[FlowNodeEdge],
    artifact_edges: list[FlowArtifactEdge],
) -> dict[UUID, FunctionType]:
    functions: dict[UUID, RecordFunction] = {}
    stub_artifact_loaders: dict[str, Callable[[ArtifactVersion], ArtifactHandler]] = {
        MODEL_TYPE: lambda model: StubModelHandler(spec=terrible_cast(ModelVersion, model).spec),
        DATASET_TYPE: lambda model: StubDatasetHandler(
            spec=terrible_cast(DatasetVersion, model).spec
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

        arguments = prepare_function_arguments(node, node_artifact_arguments, stub_artifact_loaders)
        function = load_function(node.function_id, arguments=arguments)
        if not isinstance(function, RecordFunction):
            raise ValueError(f"function not yet supported: {function}")
        functions[node.id] = function

    # base function specs only dependent on artifacts
    function_specs: dict[UUID, FunctionType] = {}
    for node in nodes:
        function = functions[node.id]
        function_specs[node.id] = FunctionType(
            input_spec=function.input_spec, output_spec=function.output_spec
        )

    return function_specs

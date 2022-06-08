import dataclasses
from collections import defaultdict
from functools import cached_property
from itertools import chain
from typing import (
    Callable,
    Dict,
    Iterable,
    Mapping,
    Optional,
    Tuple,
    Union,
    cast,
)
from uuid import UUID

import structlog
from django.db.models import QuerySet

from bench.dataset.accessor import write_to_dataset
from bench.executor.base import (
    Executor,
    FlowExecutionOptions,
    FlowRawArgument,
    ResourceRequirements,
)
from bench.executor.utils import get_model_iid
from bench.model.base import ModelHandler, load_model
from bench.models import ArtifactVersion, FlowExecution, ModelExecution
from bench.models.dataset import Dataset, DatasetMetadata
from bench.models.execution import (
    DEFAULT_CONNECTION_NAME,
    MODEL_EXECUTION_TYPE,
    ExecutionArtifactConnection,
    FlowNodeExecution,
)
from bench.models.flow import FlowArtifactEdge, FlowNode, FlowNodeEdge, FlowVersion
from bench.models.model import ModelVersion
from bench.utils.record import Record, RecordBatch, is_record

logger = structlog.stdlib.get_logger()


@dataclasses.dataclass
class ArtifactConnection:
    type: ExecutionArtifactConnection.ConnectionType
    artifact: ArtifactVersion
    edge: Optional[FlowArtifactEdge] = None


@dataclasses.dataclass
class FlowNodeConnection:
    edge: FlowNodeEdge
    intermediate_artifact: Optional[ArtifactVersion] = None


@dataclasses.dataclass
class FlowExecutionPlan:
    nodes: Mapping[UUID, FlowNode]
    static_inputs: Mapping[UUID, Mapping[str, ArtifactConnection]]
    static_arguments: Mapping[UUID, Mapping[str, ArtifactConnection]]
    connected_inputs: Mapping[UUID, Mapping[str, FlowNodeConnection]]
    connected_arguments: Mapping[UUID, Mapping[str, FlowNodeConnection]]
    final_outputs: Mapping[UUID, Mapping[str, ArtifactVersion]]

    def static(self, node_id: UUID) -> Iterable[Tuple[str, ArtifactConnection]]:
        return chain(
            self.static_inputs.get(node_id, {}).items(),
            self.static_arguments.get(node_id, {}).items(),
        )

    def connected(self, node_id: UUID) -> Iterable[Tuple[str, FlowNodeConnection]]:
        return chain(
            self.connected_inputs.get(node_id, {}).items(),
            self.connected_arguments.get(node_id, {}).items(),
            self.connected_inverse.get(node_id, {}).items(),
        )

    @cached_property
    def connected_inverse(self) -> Mapping[UUID, Mapping[str, FlowNodeConnection]]:
        inverse: dict[UUID, dict[str, FlowNodeConnection]] = defaultdict(dict)
        for node_inputs in chain(self.connected_inputs.values(), self.connected_arguments.values()):
            for name, connection in node_inputs.items():
                inverse[connection.edge.dependency.id][name] = connection
        return inverse


@dataclasses.dataclass
class FlowExecutionManifest:
    execution: FlowExecution
    node_executions: Mapping[UUID, FlowNodeExecution]


def _convert_records_to_dataset(name: str, data: RecordBatch) -> ArtifactVersion:
    dataset = Dataset.objects.create_dataset_version_by_name(
        name=name, metadata=DatasetMetadata.default_db(), version=None
    )
    write_to_dataset(dataset, data)
    return dataset


def _convert_arguments_to_artifact_connections(
    arguments: Mapping[UUID, Mapping[str, FlowRawArgument]],
    argument_id_func: Callable[[UUID, str], str],
    connection_type: ExecutionArtifactConnection.ConnectionType,
) -> Mapping[UUID, Mapping[str, ArtifactConnection]]:
    converted_arguments: dict[UUID, Mapping[str, ArtifactConnection]] = {}
    for node_id, node_arguments in arguments.items():
        converted_node_arguments: dict[str, ArtifactConnection] = {}
        for name, artifact in node_arguments.items():
            node_argument_id = argument_id_func(node_id, name)
            if isinstance(artifact, RecordBatch):
                artifact = _convert_records_to_dataset(node_argument_id, artifact)
            elif not isinstance(artifact, ArtifactVersion):
                raise ValueError(
                    f"node argument {node_argument_id} has unexpected type: {artifact}"
                )
            converted_node_arguments[name] = ArtifactConnection(
                type=connection_type, edge=None, artifact=artifact
            )

        converted_arguments[node_id] = converted_node_arguments
    return converted_arguments


def _get_final_outputs(flow, nodes: Iterable[FlowNode]):
    final_outputs: dict[UUID, dict[str, ArtifactVersion]] = {}
    for node in nodes:
        is_intermediate = FlowNodeEdge.objects.filter(dependency=node).exists()
        if is_intermediate:
            continue

        # TODO @Feature: get actual output names for multi-output nodes
        output_names = [DEFAULT_CONNECTION_NAME]
        for output_name in output_names:
            output_id = f"{flow.name}/{node.name}/outputs/{output_name}"
            output_dataset = Dataset.objects.create_dataset_version_by_name(
                name=output_id, metadata=DatasetMetadata.default_db()
            )
            final_outputs[node.id][output_name] = output_dataset
    return final_outputs


def _get_node_connections(
    flow_name: str,
    nodes: Iterable[FlowNode],
    captured_connection_types: set[FlowNodeEdge.ConnectionType],
):
    connected_inputs: dict[UUID, dict[str, FlowNodeConnection]] = defaultdict(dict)
    connected_arguments: dict[UUID, dict[str, FlowNodeConnection]] = defaultdict(dict)
    for node in nodes:
        node_dependencies: QuerySet[FlowNodeEdge] = FlowNodeEdge.objects.filter(dependent=node)
        for edge in node_dependencies:
            if edge.connection_type in captured_connection_types:
                output_id = f"{flow_name}/{node.name}/outputs/{edge.connection_name}"
                output_dataset = Dataset.objects.create_dataset_version_by_name(
                    name=output_id, metadata=DatasetMetadata.default_db()
                )
                connection = FlowNodeConnection(edge=edge, intermediate_artifact=output_dataset)
            else:
                connection = FlowNodeConnection(edge=edge, intermediate_artifact=None)

            if edge.connection_type == FlowNodeEdge.ConnectionType.Input:
                connected_inputs[node.id][edge.connection_name] = connection
            elif edge.connection_type == FlowNodeEdge.ConnectionType.Argument:
                connected_arguments[node.id][edge.connection_name] = connection
            else:
                raise ValueError(f"unexpected connection type: {edge}")
    return connected_arguments, connected_inputs


def _get_static_connections(nodes: Iterable[FlowNode]):
    static_inputs: dict[UUID, dict[str, ArtifactConnection]] = defaultdict(dict)
    static_arguments: dict[UUID, dict[str, ArtifactConnection]] = defaultdict(dict)
    for node in nodes:
        artifact_dependencies: QuerySet[FlowArtifactEdge] = FlowArtifactEdge.objects.filter(
            dependent=node
        )
        for edge in artifact_dependencies:
            connection_type = ExecutionArtifactConnection.ConnectionType(edge.connection_type)
            connection = ArtifactConnection(
                type=connection_type, artifact=edge.dependency, edge=edge
            )
            if edge.connection_type == FlowArtifactEdge.ConnectionType.Input:
                static_inputs[node.id][edge.connection_name] = connection
            elif edge.connection_type == FlowArtifactEdge.ConnectionType.Argument:
                static_arguments[node.id][edge.connection_name] = connection
            else:
                raise ValueError(f"unexpected connection type: {edge}")

    return static_arguments, static_inputs


def _manifest_execution_plan(flow: FlowVersion, plan: FlowExecutionPlan):
    execution = FlowExecution.objects.create(flow=flow)
    node_executions: dict[UUID, FlowNodeExecution] = {}
    for node in plan.nodes.values():
        node_execution = FlowNodeExecution.objects.create(flow=flow, node=node, parent=execution)
        # static inputs
        for name, static_connection in plan.static(node_id=node.id):
            node_execution.connected_artifacts.create(
                connection_type=static_connection.type,
                connection_name=name,
                flow_artifact_edge=static_connection.edge,
                artifact=static_connection.artifact,
            )
        # inter-node connections
        for name, node_connection in plan.connected(node_id=node.id):
            if node_connection.intermediate_artifact is None:
                continue

            connection_type = (
                ExecutionArtifactConnection.ConnectionType.Output
                if node == node_connection.edge.dependency
                else node_connection.edge.connection_type
            )
            node_execution.connected_artifacts.create(
                connection_type=connection_type,
                connection_name=name,
                artifact=node_connection.intermediate_artifact,
            )
        # final outputs
        for name, artifact in plan.final_outputs.get(node.id, {}).items():
            node_execution.connected_artifacts.create(
                connection_type=ExecutionArtifactConnection.ConnectionType.Output,
                connection_name=name,
                artifact=artifact,
            )
        node_executions[node.id] = node_execution

    manifest = FlowExecutionManifest(execution=execution, node_executions=node_executions)
    return manifest


class LocalExecutor(Executor):
    """
    A locally executed implementation of Executor without coordination or parallelism.
    """

    def __init__(self):
        self._loaded_models_by_iid: Dict[str, ModelHandler] = {}

    def _get_loaded_model(self, model: ModelVersion, load_if_needed: bool) -> ModelHandler:
        model_iid = get_model_iid(model)
        if model_iid not in self._loaded_models_by_iid:
            if not load_if_needed:
                raise RuntimeError("model " + model_iid + " is not load")
            else:
                self.load_model(model)
        return self._loaded_models_by_iid[model_iid]

    def load_model(
        self,
        model: ModelVersion,
        requirements: Optional[ResourceRequirements] = None,
    ):
        model_iid = get_model_iid(model=model)
        if model_iid in self._loaded_models_by_iid:
            return

        log = logger.bind(
            model_id=model.id,
            model_iid=model_iid,
            arguments=model.config_arguments,
            requirements=requirements,
        )
        log.info("model_load")
        model_handler: ModelHandler = load_model(
            model.handler_id,
            version=model.version,
            storage_uri=model.storage_uri,
            arguments=model.config_arguments,
            spec=model.spec,
        )
        self._loaded_models_by_iid[model_iid] = model_handler
        log.info("model_loaded")

    def run_model(
        self,
        model: ModelVersion,
        record: Union[Record, RecordBatch],
        blocking: bool = True,
        load_if_needed: bool = False,
    ) -> Tuple[ModelExecution, Union[None, Record, RecordBatch]]:
        if not blocking:
            # TODO @Performance: run_model is always blocking
            raise NotImplementedError("running non-blocking is not supported")
        execution = ModelExecution.objects.create(type=MODEL_EXECUTION_TYPE, model=model)
        with execution.capture(start=False):
            model_handler = self._get_loaded_model(model, load_if_needed)
            execution.start()
            if is_record(record):
                output = model_handler.predict(cast(Record, record))
            else:
                output = model_handler.predict_batch(cast(RecordBatch, record))
        return execution, output

    def run_flow(
        self,
        flow: FlowVersion,
        inputs: Mapping[UUID, Mapping[str, FlowRawArgument]],
        arguments: Mapping[UUID, Mapping[str, FlowRawArgument]],
        options: FlowExecutionOptions,
    ) -> Tuple[FlowExecution, Mapping[UUID, Mapping[str, ArtifactVersion]]]:
        nodes: Mapping[UUID, FlowNode] = {node.id: node for node in flow.nodes}

        # convert given inputs/arguments to persisted artifacts as needed
        extra_inputs = _convert_arguments_to_artifact_connections(
            inputs,
            lambda node_id, name: f"{flow.name}/{nodes[node_id].name}/inputs/{name}",
            connection_type=ExecutionArtifactConnection.ConnectionType.Input,
        )
        extra_arguments = _convert_arguments_to_artifact_connections(
            arguments,
            lambda node_id, name: f"{flow.name}/{nodes[node_id].name}/arguments/{name}",
            connection_type=ExecutionArtifactConnection.ConnectionType.Argument,
        )

        # define node<->artifact connections (from given extra and defined in flow)
        static_arguments, static_inputs = _get_static_connections(nodes.values())
        static_inputs = {**static_inputs, **extra_inputs}
        static_arguments = {**static_inputs, **extra_arguments}
        # define node<->node connections (with corresponding artifacts as needed)
        connected_arguments, connected_inputs = _get_node_connections(
            flow_name=flow.name,
            nodes=nodes.values(),
            captured_connection_types=options.capture_intermediate,
        )
        # define final outputs
        final_outputs = _get_final_outputs(flow, nodes.values())

        # define execution plan for data flow
        plan = FlowExecutionPlan(
            nodes=nodes,
            static_inputs=static_inputs,
            static_arguments=static_arguments,
            connected_inputs=connected_inputs,
            connected_arguments=connected_arguments,
            final_outputs=final_outputs,
        )

        # create and link executions according to plan
        manifest = _manifest_execution_plan(flow, plan)

        # load functions and execute plan
        # functions: dict[UUID, Function] = {}
        # for node in plan.nodes.values():
        #     pass

        return manifest.execution, plan.final_outputs

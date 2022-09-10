from __future__ import annotations

import abc
import dataclasses
import enum
import time
import uuid
from collections import defaultdict
from functools import cached_property
from itertools import chain
from typing import Callable, Dict, Iterable, Mapping, Optional, Tuple, Union
from uuid import UUID

import structlog
from django.db.models import QuerySet

from bench.artifact.base import ArtifactHandler
from bench.dataset.accessor import write_to_dataset
from bench.models import (
    ArtifactVersion,
    Dataset,
    FlowArtifactEdge,
    FlowExecution,
    FlowNode,
    FlowNodeExecution,
    ModelExecution,
)
from bench.models.artifact import ArtifactView
from bench.models.dataset import DatasetMetadata, DatasetVersion, DatasetViewData
from bench.models.execution import (
    DEFAULT_CONNECTION_NAME,
    FLOW_EXECUTION_TYPE,
    FLOW_NODE_EXECUTION_TYPE,
    Execution,
    ExecutionArtifactConnection,
)
from bench.models.flow import FlowNodeEdge, FlowVersion
from bench.models.model import ModelVersion
from bench.models.tag import default_tag
from bench.models.utils import UUIDT
from bench.utils.record import Record, RecordBatch
from bench.utils.spec import ArtifactType

logger = structlog.stdlib.get_logger()
Resource = str
ResourceRequirements = Dict[Resource, Union[int, float]]
PerNodeResourceRequirements = Dict[UUIDT, ResourceRequirements]

FlowRawArgument = Union[RecordBatch, ArtifactVersion]
FlowArgument = Union[ArtifactVersion]


class FlowRuntimeValidation(enum.Enum):
    Off = "off"
    Lazy = "lazy"
    Full = "full"


@dataclasses.dataclass
class FlowExecutionOptions:
    blocking: bool
    validate: FlowRuntimeValidation
    captured_edges: list[UUID] = dataclasses.field(default_factory=list)
    captured_connection_types: list[FlowNodeEdge.ConnectionType] = dataclasses.field(
        default_factory=list
    )

    @staticmethod
    def default():
        return FlowExecutionOptions(validate=FlowRuntimeValidation.Lazy, blocking=False)

    @staticmethod
    def default_blocking():
        return FlowExecutionOptions(validate=FlowRuntimeValidation.Lazy, blocking=True)


class Executor(abc.ABC):
    """
    Base executor for orchestrating, routing and executing resources.
    """

    def load_artifact(
        self,
        model: ArtifactVersion,
        requirements: Optional[ResourceRequirements] = None,
    ) -> ArtifactHandler:
        """
        Make the artifact available in this executor with the given resources.
        """
        raise NotImplementedError

    def get_runtime_artifact_spec(self, artifact: ArtifactVersion) -> ArtifactType:
        """
        Gets the runtime/actual specification of the given artifact (instead of the configured).
        This may require loading the given artifact and performing other expensive operations.
        """
        raise NotImplementedError

    def run_model(
        self,
        model: ModelVersion,
        record: Union[Record, RecordBatch],
        blocking: bool = True,
        load_if_needed: bool = False,
    ) -> Tuple[ModelExecution, Union[None, Record, RecordBatch]]:
        """
        Runs the given model on the given records, loading it first if needed.
        The returned execution object encapsulates this run and can be used to retrieve results.

        If run in non-blocking mode, output cannot be returned and is None.
        """
        raise NotImplementedError

    def run_flow(
        self,
        flow: FlowVersion,
        inputs: Mapping[UUID, Mapping[str, FlowRawArgument]],
        arguments: Mapping[UUID, Mapping[str, FlowRawArgument]],
        options: FlowExecutionOptions,
    ) -> Tuple[FlowExecution, FlowExecutionPlan]:
        """
        Runs the given flow with the provided inputs and arguments to each node.
        This method is non-blocking and the returned execution object can be used to retrieve results.
        """
        raise NotImplementedError


@dataclasses.dataclass
class ArtifactConnection:
    type: ExecutionArtifactConnection.ConnectionType
    name: str
    artifact: ArtifactVersion
    view: Optional[ArtifactView] = None
    view_inline: Optional[dict] = None
    edge: Optional[FlowArtifactEdge] = None
    manifested_id: Optional[uuid.UUID] = None

    @property
    def view_data(self) -> DatasetViewData:
        if self.view is not None:
            return DatasetViewData(**self.view.data)
        else:
            return DatasetViewData(**(self.view_inline or {}))

    @property
    def artifact_type(self) -> str:
        return self.artifact.artifact.type

    @staticmethod
    def from_edge(edge: FlowArtifactEdge) -> ArtifactConnection:
        connection_type = ExecutionArtifactConnection.ConnectionType(edge.connection_type)
        return ArtifactConnection(
            type=connection_type,
            name=edge.connection_name,
            artifact=edge.dependency,
            edge=edge,
            view=edge.view,
            view_inline=edge.view_inline,
        )


@dataclasses.dataclass
class FlowNodeConnection:
    edge: FlowNodeEdge
    intermediate_artifact: Optional[DatasetVersion] = None
    view_inline: Optional[dict] = None
    manifested_id: Optional[uuid.UUID] = None

    @property
    def dependency_name(self) -> str:
        return self.edge.connection_name_dependency

    @property
    def dependent_name(self) -> str:
        return self.edge.connection_name_dependent


@dataclasses.dataclass
class FlowExecutionPlan:
    nodes: Mapping[UUID, FlowNode]
    artifact_inputs: Mapping[UUID, Mapping[str, ArtifactConnection]]
    artifact_arguments: Mapping[UUID, Mapping[str, ArtifactConnection]]
    node_inputs: Mapping[UUID, Mapping[str, FlowNodeConnection]]
    node_arguments: Mapping[UUID, Mapping[str, FlowNodeConnection]]
    final_outputs: Mapping[UUID, Mapping[str, ArtifactConnection]]
    options: FlowExecutionOptions

    def static(self, node_id: UUID) -> Iterable[Tuple[str, ArtifactConnection]]:
        return chain(
            self.artifact_inputs.get(node_id, {}).items(),
            self.artifact_arguments.get(node_id, {}).items(),
        )

    def connected(self, node_id: UUID) -> Iterable[Tuple[str, FlowNodeConnection]]:
        return chain(
            self.node_inputs.get(node_id, {}).items(), self.node_arguments.get(node_id, {}).items()
        )

    @cached_property
    def connected_inverse(self) -> Mapping[UUID, Mapping[str, list[FlowNodeConnection]]]:
        inverse: dict[UUID, dict[str, list[FlowNodeConnection]]] = defaultdict(
            lambda: defaultdict(list)
        )
        for node_inputs in chain(self.node_inputs.values(), self.node_arguments.values()):
            for name, connection in node_inputs.items():
                inverse[connection.edge.dependency.id][name].append(connection)
        return inverse


@dataclasses.dataclass
class FlowExecutionManifest:
    execution: FlowExecution
    node_executions: Mapping[UUID, FlowNodeExecution]  # by node id
    execution_connections: Mapping[UUID, ExecutionArtifactConnection]  # by execution id


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
            view_inline = None
            if isinstance(artifact, RecordBatch):
                artifact, view_slice = write_to_dataset(node_argument_id, "0", artifact)
                artifact.set_tag(default_tag("source:inputs"))
                view_inline = DatasetViewData.from_slice(view_slice).asdict
            elif not isinstance(artifact, ArtifactVersion):
                raise ValueError(
                    f"node argument {node_argument_id} has unexpected type: {artifact}"
                )
            converted_node_arguments[name] = ArtifactConnection(
                type=connection_type,
                name=name,
                edge=None,
                artifact=artifact,
                view=None,
                view_inline=view_inline,
            )
            logger.debug(
                "execute_converted_partial", type=connection_type, artifact=artifact, name=name
            )

        converted_arguments[node_id] = converted_node_arguments
    return converted_arguments


def _make_final_outputs(flow: FlowVersion, nodes: Iterable[FlowNode]):
    final_outputs: dict[UUID, dict[str, ArtifactConnection]] = defaultdict(dict)
    for node in nodes:
        is_intermediate = FlowNodeEdge.objects.filter(dependency=node).exists()
        if is_intermediate:
            continue

        # TODO @Feature: get actual output names for multi-output nodes
        output_names = [DEFAULT_CONNECTION_NAME]
        for output_name in output_names:
            if output_name == DEFAULT_CONNECTION_NAME:
                output_id = f"{flow.flow.name}.{node.name}.outputs"
            else:
                output_id = f"{flow.flow.name}.{node.name}.outputs.{output_name}"
            output_dataset = Dataset.objects.get_or_create_dataset_version(
                name=output_id, version="0", metadata=DatasetMetadata.default_db()
            )
            output_dataset.set_tag(default_tag("source:outputs"))
            final_outputs[node.id][output_name] = ArtifactConnection(
                type=ExecutionArtifactConnection.ConnectionType.Output,
                name=output_id,
                artifact=output_dataset,
                view_inline=DatasetViewData.empty().asdict,
            )
    return {k: v for k, v in final_outputs.items()}  # convert to regular dict


def _make_node_connections(
    flow_name: str,
    nodes: Iterable[FlowNode],
    captured_connection_types: list[FlowNodeEdge.ConnectionType],
    captured_edges: list[UUID],
):
    node_inputs: dict[UUID, dict[str, FlowNodeConnection]] = defaultdict(dict)
    node_arguments: dict[UUID, dict[str, FlowNodeConnection]] = defaultdict(dict)
    for node in nodes:
        node_dependencies: QuerySet[FlowNodeEdge] = FlowNodeEdge.objects.filter(dependent=node)
        for edge in node_dependencies:
            if edge.connection_type in captured_connection_types or edge.id in captured_edges:
                output_id = _port_id(
                    flow_name, edge.dependency.name, "outputs", edge.connection_name_dependency
                )
                output_dataset = Dataset.objects.get_or_create_dataset_version(
                    name=output_id, version="0", metadata=DatasetMetadata.default_db()
                )
                output_dataset.set_tag(default_tag("source:outputs"))
                connection = FlowNodeConnection(
                    edge=edge,
                    intermediate_artifact=output_dataset,
                    view_inline=DatasetViewData.empty().asdict,
                )
            else:
                connection = FlowNodeConnection(edge=edge, intermediate_artifact=None)

            if edge.connection_type == FlowNodeEdge.ConnectionType.Input:
                node_inputs[node.id][edge.connection_name_dependent] = connection
            elif edge.connection_type == FlowNodeEdge.ConnectionType.Argument:
                node_arguments[node.id][edge.connection_name_dependent] = connection
            else:
                raise ValueError(f"unexpected connection type: {edge}")
    return node_inputs, node_arguments


def _get_static_connections(nodes: Iterable[FlowNode]):
    static_inputs: dict[UUID, dict[str, ArtifactConnection]] = defaultdict(dict)
    static_arguments: dict[UUID, dict[str, ArtifactConnection]] = defaultdict(dict)
    artifacts_dependencies: QuerySet[FlowArtifactEdge] = FlowArtifactEdge.objects.filter(
        dependent__in=nodes
    ).all()
    for node in nodes:
        artifact_dependencies = [art for art in artifacts_dependencies if art.dependent == node]
        for edge in artifact_dependencies:
            connection = ArtifactConnection.from_edge(edge)
            if edge.connection_type == FlowArtifactEdge.ConnectionType.Input:
                static_inputs[node.id][edge.connection_name] = connection
            elif edge.connection_type == FlowArtifactEdge.ConnectionType.Argument:
                static_arguments[node.id][edge.connection_name] = connection
            else:
                raise ValueError(f"unexpected connection type: {edge}")

    return static_arguments, static_inputs


def _port_id(flow_name: str, node_name: str, argument_kind: str, argument_name: str) -> str:
    if argument_name == DEFAULT_CONNECTION_NAME:
        return f"{flow_name}.{node_name}.{argument_kind}"
    else:
        return f"{flow_name}.{node_name}.{argument_kind}.{argument_name}"


def make_execution_plan(
    flow: FlowVersion,
    inputs: Mapping[UUID, Mapping[str, FlowRawArgument]],
    arguments: Mapping[UUID, Mapping[str, FlowRawArgument]],
    options: FlowExecutionOptions,
) -> FlowExecutionPlan:
    """
    Makes an execution plan and creates the corresponding artifacts
    """

    nodes: Mapping[UUID, FlowNode] = {node.id: node for node in flow.nodes.all()}

    # convert given inputs/arguments to persisted artifacts as needed
    extra_inputs = _convert_arguments_to_artifact_connections(
        inputs,
        lambda nid, name: _port_id(flow.flow.name, nodes[nid].name, "inputs", name),
        connection_type=ExecutionArtifactConnection.ConnectionType.Input,
    )
    extra_arguments = _convert_arguments_to_artifact_connections(
        arguments,
        lambda nid, name: _port_id(flow.flow.name, nodes[nid].name, "arguments", name),
        connection_type=ExecutionArtifactConnection.ConnectionType.Argument,
    )
    logger.debug("execute_converted")

    # define node<->artifact connections (from given extra and defined in flow)
    static_arguments, static_inputs = _get_static_connections(nodes.values())
    static_inputs = {**static_inputs, **extra_inputs}
    static_arguments = {**static_arguments, **extra_arguments}

    # define node<->node connections (with corresponding artifacts as needed)
    node_inputs, node_arguments = _make_node_connections(
        flow_name=flow.flow.name,
        nodes=nodes.values(),
        captured_connection_types=options.captured_connection_types,
        captured_edges=options.captured_edges,
    )
    logger.debug("execute_nodes_connected")

    # define final outputs
    final_outputs = _make_final_outputs(flow, nodes.values())
    logger.debug("execute_final_outputs_created")

    # define execution plan for data flow
    plan = FlowExecutionPlan(
        nodes=nodes,
        artifact_inputs=static_inputs,
        artifact_arguments=static_arguments,
        node_inputs=node_inputs,
        node_arguments=node_arguments,
        final_outputs=final_outputs,
        options=options,
    )
    return plan


def make_execution_manifest(flow: FlowVersion, plan: FlowExecutionPlan) -> FlowExecutionManifest:
    manifest = prepare_execution_manifest(flow, plan)
    save_execution_manifest(manifest)
    return manifest


def prepare_execution_manifest(flow: FlowVersion, plan: FlowExecutionPlan) -> FlowExecutionManifest:
    """
    Makes the actual execution objects and links them together
    """

    execution = FlowExecution(type=FLOW_EXECUTION_TYPE, flow=flow)
    node_executions: dict[UUID, FlowNodeExecution] = {}
    execution_connections: dict[UUID, ExecutionArtifactConnection] = {}
    for node in plan.nodes.values():
        node_execution = FlowNodeExecution(
            type=FLOW_NODE_EXECUTION_TYPE, flow=flow, flow_node=node, parent=execution
        )

        # static inputs
        for name, artifact_connection in plan.static(node_id=node.id):
            connection = ExecutionArtifactConnection(
                execution=node_execution,
                connection_type=artifact_connection.type,
                connection_name=name,
                flow_artifact_edge=artifact_connection.edge,
                artifact=artifact_connection.artifact,
                view=artifact_connection.view,
                view_inline=artifact_connection.view_inline,
            )
            execution_connections[connection.id] = connection
            artifact_connection.manifested_id = connection.id

        # inter-node connections
        for name, node_connection in plan.connected(node_id=node.id):
            if node_connection.intermediate_artifact is None:
                continue

            connection_type = (
                ExecutionArtifactConnection.ConnectionType.Output
                if node.id == node_connection.edge.dependency_id
                else node_connection.edge.connection_type
            )
            connection = ExecutionArtifactConnection(
                execution=node_execution,
                connection_type=connection_type,
                connection_name=name,
                artifact=node_connection.intermediate_artifact,
                view_inline=node_connection.view_inline,
            )
            execution_connections[connection.id] = connection
            node_connection.manifested_id = connection.id

        # final outputs
        for name, artifact_connection in plan.final_outputs.get(node.id, {}).items():
            connection = ExecutionArtifactConnection(
                execution=node_execution,
                connection_type=ExecutionArtifactConnection.ConnectionType.Output,
                connection_name=name,
                artifact=artifact_connection.artifact,
                view=artifact_connection.view,
                view_inline=artifact_connection.view_inline,
            )
            execution_connections[connection.id] = connection
            artifact_connection.manifested_id = connection.id
        node_executions[node.id] = node_execution

    return FlowExecutionManifest(execution, node_executions, execution_connections)


def save_execution_manifest(manifest: FlowExecutionManifest):
    start = time.time()
    Execution.objects.bulk_create(chain(manifest.node_executions.values()))
    ExecutionArtifactConnection.objects.bulk_create(manifest.execution_connections.values())
    logger.debug("execute_manifest_bulk_create", took=time.time() - start)


def prepare_function_arguments(
    node: FlowNode,
    node_artifact_arguments: Iterable[ArtifactConnection],
    handler_loaders: Mapping[str, Callable[[ArtifactVersion], ArtifactHandler]],
) -> dict:
    config_arguments = node.config_arguments
    artifact_arguments: dict[str, Union[ArtifactHandler]] = {}
    for artifact_connection in node_artifact_arguments:
        artifact_loader = handler_loaders.get(artifact_connection.artifact_type)
        if artifact_loader is None:
            raise ValueError(f"unknown artifact type: {artifact_connection}")
        artifact_arguments[artifact_connection.name] = artifact_loader(artifact_connection.artifact)
    return {**config_arguments, **artifact_arguments}

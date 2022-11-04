from __future__ import annotations

import abc
import dataclasses
import enum
import time
import uuid
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

from bench.artifact.base import ArtifactHandler
from bench.dataset.accessor import DatasetHandler, DatasetRecord
from bench.dataset.base import DatasetHandler
from bench.model.base import ModelHandler
from bench.models import (
    ArtifactVersion,
    Dataset,
    FlowArtifactEdge,
    FlowExecution,
    FlowInstruction,
    FlowInstructionExecution,
    ModelExecution,
)
from bench.models.artifact import DatasetView
from bench.models.dataset import DatasetVersion, DatasetViewData
from bench.models.execution import (
    DEFAULT_CONNECTION_NAME,
    FLOW_EXECUTION_TYPE,
    Execution,
    ExecutionArtifactConnection,
    flow_instruction_EXECUTION_TYPE,
)
from bench.models.flow import FlowInstructionEdge, FlowVersion
from bench.models.model import ModelVersion
from bench.models.tag import default_tag
from bench.models.utils import DATASET_TYPE, UUIDT
from bench.utils.record import Record, RecordBatch

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
    captured_connection_types: list[FlowInstructionEdge.ConnectionType] = dataclasses.field(
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
    dependency_name: Optional[str] = None
    view: Optional[DatasetView] = None
    view_inline: Optional[dict] = None
    edge: Optional[FlowArtifactEdge] = None
    manifested_id: Optional[uuid.UUID] = None

    @property
    def dataset(self):
        if self.artifact.artifact.type != DATASET_TYPE:
            raise ValueError(f"expected artifact to be a dataset: {self.artifact}")
        return cast(DatasetVersion, self.artifact)

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
class FlowInstructionConnection:
    edge: FlowInstructionEdge
    dataset: Optional[DatasetVersion] = None
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
    nodes: Mapping[UUID, FlowInstruction]
    artifact_inputs: Mapping[UUID, Mapping[str, ArtifactConnection]]
    artifact_arguments: Mapping[UUID, Mapping[str, ArtifactConnection]]
    node_inputs: Mapping[UUID, Mapping[str, FlowInstructionConnection]]
    node_arguments: Mapping[UUID, Mapping[str, FlowInstructionConnection]]
    final_outputs: Mapping[UUID, Mapping[str, ArtifactConnection]]
    options: FlowExecutionOptions

    def static(self, node_id: UUID) -> Iterable[Tuple[str, ArtifactConnection]]:
        return chain(
            self.artifact_inputs.get(node_id, {}).items(),
            self.artifact_arguments.get(node_id, {}).items(),
        )

    def connected(self, node_id: UUID) -> Iterable[Tuple[str, FlowInstructionConnection]]:
        return chain(
            self.node_inputs.get(node_id, {}).items(), self.node_arguments.get(node_id, {}).items()
        )

    @cached_property
    def connected_inverse(self) -> Mapping[UUID, Mapping[str, list[FlowInstructionConnection]]]:
        inverse: dict[UUID, dict[str, list[FlowInstructionConnection]]] = defaultdict(
            lambda: defaultdict(list)
        )
        for node_inputs in chain(self.node_inputs.values(), self.node_arguments.values()):
            for name, connection in node_inputs.items():
                inverse[connection.edge.dependency.id][name].append(connection)
        return inverse


@dataclasses.dataclass
class FlowExecutionManifest:
    execution: FlowExecution
    node_executions: Mapping[UUID, FlowInstructionExecution]  # by node id
    execution_connections: Mapping[UUID, ExecutionArtifactConnection]  # by execution id


def _convert_parameters_to_artifact_connections(
    flow: FlowVersion,
    parameters: Mapping[UUID, Mapping[str, FlowRawArgument]],
    connection_type: ExecutionArtifactConnection.ConnectionType,
) -> Mapping[UUID, Mapping[str, ArtifactConnection]]:
    nodes: Mapping[UUID, FlowInstruction] = {node.id: node for node in flow.nodes.all()}
    converted_parameters: dict[UUID, Mapping[str, ArtifactConnection]] = {}
    for node_id, node_parameters in parameters.items():
        converted_node_parameters: dict[str, ArtifactConnection] = {}
        for name, parameter in node_parameters.items():
            node_parameter_id = _port_id(
                flow.flow.name, nodes[node_id].name, connection_type.value + "s", name
            )
            view_inline = None
            if isinstance(parameter, RecordBatch):
                artifact = Dataset.objects.get_or_create_dataset_version(
                    node_parameter_id, flow.organization
                )
                artifact.set_tag(default_tag("source:inputs", flow.organization))
                accessor = DatasetHandler(artifact)
                view_slice = accessor.extend([DatasetRecord.make(data=data) for data in parameter])
                view_inline = DatasetViewData.from_slice(view_slice).asdict
                connection = ArtifactConnection(
                    type=connection_type, name=name, artifact=artifact, view_inline=view_inline
                )
            elif isinstance(parameter, ArtifactVersion):
                connection = ArtifactConnection(
                    type=connection_type, name=name, artifact=parameter, view_inline=view_inline
                )
            else:
                raise ValueError(
                    f"node parameter {node_parameter_id} has unexpected type: {parameter}"
                )
            converted_node_parameters[name] = connection
            logger.debug(
                "execute_converted_partial", type=connection_type, artifact=parameter, name=name
            )

        converted_parameters[node_id] = converted_node_parameters
    return converted_parameters


def _make_final_outputs(flow: FlowVersion, nodes: Iterable[FlowInstruction]):
    final_outputs: dict[UUID, dict[str, ArtifactConnection]] = defaultdict(dict)
    for node in nodes:
        is_intermediate = FlowInstructionEdge.objects.filter(dependency=node).exists()
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
                name=output_id, version="0", organization=flow.organization
            )
            output_dataset.set_tag(default_tag("source:outputs", flow.organization))
            final_outputs[node.id][output_name] = ArtifactConnection(
                type=ExecutionArtifactConnection.ConnectionType.Output,
                name=output_id,
                dependency_name=output_name,
                artifact=output_dataset,
                view_inline=DatasetViewData.empty().asdict,
            )
    return {k: v for k, v in final_outputs.items()}  # convert to regular dict


def _make_node_connections(
    flow: FlowVersion,
    nodes: Iterable[FlowInstruction],
    captured_connection_types: list[FlowInstructionEdge.ConnectionType],
    captured_edges: list[UUID],
):
    node_inputs: dict[UUID, dict[str, FlowInstructionConnection]] = defaultdict(dict)
    node_arguments: dict[UUID, dict[str, FlowInstructionConnection]] = defaultdict(dict)
    for node in nodes:
        node_dependencies: QuerySet[FlowInstructionEdge] = FlowInstructionEdge.objects.filter(
            dependent=node
        )
        for edge in node_dependencies:
            if edge.connection_type in captured_connection_types or edge.id in captured_edges:
                output_id = _port_id(
                    flow.flow.name, edge.dependency.name, "outputs", edge.connection_name_dependency
                )
                output_dataset = Dataset.objects.get_or_create_dataset_version(
                    name=output_id, version="0", organization=flow.organization
                )
                output_dataset.set_tag(default_tag("source:outputs", flow.organization))
                connection = FlowInstructionConnection(edge=edge, dataset=output_dataset)
            else:
                connection = FlowInstructionConnection(edge=edge)

            if edge.connection_type == FlowInstructionEdge.ConnectionType.Input:
                node_inputs[node.id][edge.connection_name_dependent] = connection
            elif edge.connection_type == FlowInstructionEdge.ConnectionType.Argument:
                node_arguments[node.id][edge.connection_name_dependent] = connection
            else:
                raise ValueError(f"unexpected connection type: {edge}")
    return node_inputs, node_arguments


def _get_static_connections(nodes: Iterable[FlowInstruction]):
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

    nodes: Mapping[UUID, FlowInstruction] = {node.id: node for node in flow.nodes.all()}

    # convert given inputs/arguments to persisted artifacts as needed
    extra_inputs = _convert_parameters_to_artifact_connections(
        flow, inputs, ExecutionArtifactConnection.ConnectionType.Input
    )
    extra_arguments = _convert_parameters_to_artifact_connections(
        flow, arguments, ExecutionArtifactConnection.ConnectionType.Argument
    )
    logger.debug("execute_converted")

    # define node<->artifact connections (from given extra and defined in flow)
    static_arguments, static_inputs = _get_static_connections(nodes.values())
    static_inputs = {**static_inputs, **extra_inputs}
    static_arguments = {**static_arguments, **extra_arguments}

    # define node<->node connections (with corresponding artifacts as needed)
    node_inputs, node_arguments = _make_node_connections(
        flow=flow,
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

    execution = FlowExecution(type=FLOW_EXECUTION_TYPE, flow=flow, organization=flow.organization)
    node_executions: dict[UUID, FlowInstructionExecution] = {}
    execution_connections: dict[UUID, ExecutionArtifactConnection] = {}
    for node in plan.nodes.values():
        node_execution = FlowInstructionExecution(
            type=flow_instruction_EXECUTION_TYPE,
            flow=flow,
            flow_instruction=node,
            parent=execution,
            organization=flow.organization,
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
            if node_connection.dataset is None:
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
                artifact=node_connection.dataset,
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
    node: FlowInstruction,
    node_artifact_arguments: Iterable[ArtifactConnection],
    model_loaders: Mapping[str, Callable[[ModelVersion], ModelHandler]],
    dataset_loaders: Mapping[str, Callable[[DatasetVersion], DatasetHandler]],
) -> dict:
    config_arguments = node.config_arguments
    artifact_arguments: dict[str, Union[ArtifactHandler]] = {}
    for artifact_connection in node_artifact_arguments:
        artifact_loader = handler_loaders.get(artifact_connection.artifact_type)
        if artifact_loader is None:
            raise ValueError(f"unknown artifact type: {artifact_connection}")
        artifact_arguments[artifact_connection.name] = artifact_loader(artifact_connection.artifact)
    return {**config_arguments, **artifact_arguments}

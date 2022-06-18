import abc
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
    Set,
    Tuple,
    Union,
)
from uuid import UUID

from django.db.models import QuerySet

from bench.dataset.accessor import convert_records_to_dataset
from bench.models import (
    ArtifactVersion,
    Dataset,
    FlowArtifactEdge,
    FlowExecution,
    FlowNode,
    FlowNodeExecution,
    ModelExecution,
)
from bench.models.dataset import DatasetMetadata, DatasetVersion
from bench.models.execution import DEFAULT_CONNECTION_NAME, ExecutionArtifactConnection
from bench.models.flow import FlowNodeEdge, FlowVersion
from bench.models.model import ModelVersion
from bench.models.utils import UUIDT
from bench.utils.record import Record, RecordBatch

Resource = str
ResourceRequirements = Dict[Resource, Union[int, float]]
PerNodeResourceRequirements = Dict[UUIDT, ResourceRequirements]

FlowRawArgument = Union[RecordBatch, ArtifactVersion]
FlowArgument = Union[ArtifactVersion]


@dataclasses.dataclass
class FlowExecutionOptions:
    blocking: bool
    capture_intermediate: Set[FlowNodeEdge.ConnectionType]

    @staticmethod
    def default():
        return FlowExecutionOptions(blocking=False, capture_intermediate=set())

    @staticmethod
    def default_blocking():
        return FlowExecutionOptions(blocking=True, capture_intermediate=set())


class Executor(abc.ABC):
    """
    Base executor for orchestrating, routing and executing resources.
    """

    def load_artifact(
        self,
        model: ArtifactVersion,
        requirements: Optional[ResourceRequirements] = None,
    ):
        """
        Make the artifact available in this executor with the given resources.
        """
        raise NotImplementedError

    def load_flow(self, flow: FlowVersion, requirements: PerNodeResourceRequirements):
        """
        Prepare the flow in this executor with the given resources.
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
    ) -> Tuple[FlowExecution, Mapping[UUID, Mapping[str, ArtifactVersion]]]:
        """
        Runs the given flow with the provided inputs and arguments to each node.
        This method is non-blocking and the returned execution object can be used to retrieve results.
        """
        raise NotImplementedError


@dataclasses.dataclass
class ArtifactConnection:
    type: ExecutionArtifactConnection.ConnectionType
    artifact: ArtifactVersion
    edge: Optional[FlowArtifactEdge] = None

    @property
    def artifact_type(self) -> str:
        return self.artifact.artifact.type

    @property
    def name(self) -> str:
        return self.edge.connection_name


@dataclasses.dataclass
class FlowNodeConnection:
    edge: FlowNodeEdge
    intermediate_artifact: Optional[DatasetVersion] = None

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
    final_outputs: Mapping[UUID, Mapping[str, ArtifactVersion]]

    def static(self, node_id: UUID) -> Iterable[Tuple[str, ArtifactConnection]]:
        return chain(
            self.artifact_inputs.get(node_id, {}).items(),
            self.artifact_arguments.get(node_id, {}).items(),
        )

    def connected(self, node_id: UUID) -> Iterable[Tuple[str, FlowNodeConnection]]:
        return chain(
            self.node_inputs.get(node_id, {}).items(),
            self.node_arguments.get(node_id, {}).items(),
            self.connected_inverse.get(node_id, {}).items(),
        )

    @cached_property
    def connected_inverse(self) -> Mapping[UUID, Mapping[str, FlowNodeConnection]]:
        inverse: dict[UUID, dict[str, FlowNodeConnection]] = defaultdict(dict)
        for node_inputs in chain(self.node_inputs.values(), self.node_arguments.values()):
            for name, connection in node_inputs.items():
                inverse[connection.edge.dependency.id][name] = connection
        return inverse


@dataclasses.dataclass
class FlowExecutionManifest:
    execution: FlowExecution
    node_executions: Mapping[UUID, FlowNodeExecution]


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
                artifact = convert_records_to_dataset(node_argument_id, artifact)
            elif not isinstance(artifact, ArtifactVersion):
                raise ValueError(
                    f"node argument {node_argument_id} has unexpected type: {artifact}"
                )
            converted_node_arguments[name] = ArtifactConnection(
                type=connection_type, edge=None, artifact=artifact
            )

        converted_arguments[node_id] = converted_node_arguments
    return converted_arguments


def _make_final_outputs(flow: FlowVersion, nodes: Iterable[FlowNode]):
    final_outputs: dict[UUID, dict[str, ArtifactVersion]] = defaultdict(dict)
    for node in nodes:
        is_intermediate = FlowNodeEdge.objects.filter(dependency=node).exists()
        if is_intermediate:
            continue

        # TODO @Feature: get actual output names for multi-output nodes
        output_names = [DEFAULT_CONNECTION_NAME]
        for output_name in output_names:
            output_id = f"{flow.flow.name}/{node.name}/outputs/{output_name}"
            output_dataset = Dataset.objects.create_dataset_version_by_name(
                name=output_id, metadata=DatasetMetadata.default_db()
            )
            final_outputs[node.id][output_name] = output_dataset
    return {k: v for k, v in final_outputs.items()}  # convert to regular dict


def _make_node_connections(
    flow_name: str,
    nodes: Iterable[FlowNode],
    captured_connection_types: set[FlowNodeEdge.ConnectionType],
):
    node_inputs: dict[UUID, dict[str, FlowNodeConnection]] = defaultdict(dict)
    node_arguments: dict[UUID, dict[str, FlowNodeConnection]] = defaultdict(dict)
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
                node_inputs[node.id][edge.connection_name_dependent] = connection
            elif edge.connection_type == FlowNodeEdge.ConnectionType.Argument:
                node_arguments[node.id][edge.connection_name_dependent] = connection
            else:
                raise ValueError(f"unexpected connection type: {edge}")
    return node_inputs, node_arguments


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
        lambda node_id, name: f"{flow.flow.name}/{nodes[node_id].name}/inputs/{name}",
        connection_type=ExecutionArtifactConnection.ConnectionType.Input,
    )
    extra_arguments = _convert_arguments_to_artifact_connections(
        arguments,
        lambda node_id, name: f"{flow.flow.name}/{nodes[node_id].name}/arguments/{name}",
        connection_type=ExecutionArtifactConnection.ConnectionType.Argument,
    )

    # define node<->artifact connections (from given extra and defined in flow)
    static_arguments, static_inputs = _get_static_connections(nodes.values())
    static_inputs = {**static_inputs, **extra_inputs}
    static_arguments = {**static_arguments, **extra_arguments}

    # define node<->node connections (with corresponding artifacts as needed)
    node_inputs, node_arguments = _make_node_connections(
        flow_name=flow.flow.name,
        nodes=nodes.values(),
        captured_connection_types=options.capture_intermediate,
    )

    # define final outputs
    final_outputs = _make_final_outputs(flow, nodes.values())

    # define execution plan for data flow
    plan = FlowExecutionPlan(
        nodes=nodes,
        artifact_inputs=static_inputs,
        artifact_arguments=static_arguments,
        node_inputs=node_inputs,
        node_arguments=node_arguments,
        final_outputs=final_outputs,
    )
    return plan


def manifest_execution(flow: FlowVersion, plan: FlowExecutionPlan) -> FlowExecutionManifest:
    """
    Makes the actual execution objects and links them together
    """

    execution = FlowExecution.objects.create(flow=flow)
    node_executions: dict[UUID, FlowNodeExecution] = {}
    for node in plan.nodes.values():
        node_execution = FlowNodeExecution.objects.create(
            flow=flow, flow_node=node, parent=execution
        )

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

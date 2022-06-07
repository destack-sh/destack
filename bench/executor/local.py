import dataclasses
from typing import Callable, Dict, Mapping, Optional, Tuple, Union, cast
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
from bench.function.base import Function
from bench.model.base import ModelHandler, load_model
from bench.models import ArtifactVersion, FlowExecution, ModelExecution
from bench.models.dataset import Dataset, DatasetMetadata
from bench.models.execution import FLOW_EXECUTION_TYPE, MODEL_EXECUTION_TYPE
from bench.models.flow import FlowNode, FlowNodeEdge, FlowVersion
from bench.models.model import ModelVersion
from bench.utils.record import Record, RecordBatch, is_record

logger = structlog.stdlib.get_logger()


def _convert_records_to_dataset(name: str, data: RecordBatch) -> ArtifactVersion:
    dataset = Dataset.objects.create_dataset_version_by_name(
        name=name, version=None, metadata=DatasetMetadata.default_db()
    )
    write_to_dataset(dataset, data)
    return dataset


@dataclasses.dataclass
class FlowExecutionPlan:
    nodes: Mapping[UUID, FlowNode]
    static_inputs: Mapping[UUID, Mapping[str, ArtifactVersion]]
    static_arguments: Mapping[UUID, Mapping[str, ArtifactVersion]]
    connected_inputs: Mapping[UUID, Mapping[str, UUID]]
    connected_arguments: Mapping[UUID, Mapping[str, UUID]]
    intermediate_outputs: Mapping[UUID, ArtifactVersion]
    final_outputs: Mapping[UUID, ArtifactVersion]


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
        execution = FlowExecution.objects.create(type=FLOW_EXECUTION_TYPE, flow=flow)
        nodes: Mapping[UUID, FlowNode] = {node.id: node for node in flow.nodes.all()}

        # convert given inputs/arguments to persisted artifacts as needed
        static_inputs = self._convert_arguments_to_artifacts(
            inputs,
            lambda node_id, name: f"{flow.name}/{nodes[node_id].name}/inputs/{name}",
        )
        static_arguments = self._convert_arguments_to_artifacts(
            arguments,
            lambda node_id, name: f"{flow.name}/{nodes[node_id].name}/arguments/{name}",
        )

        # define which nodes need to be connected (and how)
        connected_inputs: dict[UUID, dict[str, UUID]] = {}
        connected_arguments: dict[UUID, dict[str, UUID]] = {}
        intermediate_outputs: dict[UUID, ArtifactVersion] = {}
        final_outputs: dict[UUID, ArtifactVersion] = {}
        for node in nodes.values():
            connected_inputs[node.id] = {}
            connected_arguments[node.id] = {}
            dependency_edges: QuerySet[FlowNodeEdge] = FlowNodeEdge.objects.filter(dependent=node)
            for edge in dependency_edges:
                if edge.connection_type == FlowNodeEdge.ConnectionType.Input:
                    connected_inputs[node.id][edge.connection_name] = edge.dependency.id
                elif edge.connection_type == FlowNodeEdge.ConnectionType.Argument:
                    connected_arguments[node.id][edge.connection_name] = edge.dependency.id
                else:
                    raise ValueError(f"unexpected connection type: {edge}")
            is_intermediate = len(dependency_edges) > 1
            output_id = f"{flow.name}/{node.name}/output"
            if is_intermediate and options.capture_intermediate_outputs or not is_intermediate:
                output_dataset = Dataset.objects.create_dataset_version_by_name(
                    name=output_id, version=None, metadata=DatasetMetadata.default_db()
                )
                if is_intermediate:
                    intermediate_outputs[node.id] = output_dataset
                else:
                    final_outputs[node.id] = output_dataset

        # define execution plan for data flow
        plan = FlowExecutionPlan(
            nodes=nodes,
            static_inputs=static_inputs,
            static_arguments=static_arguments,
            connected_inputs=connected_inputs,
            connected_arguments=connected_arguments,
            intermediate_outputs=intermediate_outputs,
            final_outputs=final_outputs,
        )

        # load functions and execute plan
        functions: dict[UUID, Function] = {}
        for node in plan.nodes.values():
            pass

        return execution, final_outputs

    def _convert_arguments_to_artifacts(
        self,
        arguments: Mapping[UUID, Mapping[str, FlowRawArgument]],
        argument_id_func: Callable[[UUID, str], str],
    ):
        converted_arguments: dict[UUID, Mapping[str, ArtifactVersion]] = {}
        for node_id, node_arguments in arguments.items():
            converted_node_arguments: dict[str, ArtifactVersion] = {}
            for name, data in node_arguments.items():
                node_argument_id = argument_id_func(node_id, name)
                if isinstance(data, RecordBatch):
                    data = _convert_records_to_dataset(node_argument_id, data)
                elif not isinstance(data, ArtifactVersion):
                    raise ValueError(
                        f"node argument {node_argument_id} has unexpected type: {data}"
                    )
                converted_node_arguments[name] = data
            converted_arguments[node_id] = converted_node_arguments
        return converted_arguments

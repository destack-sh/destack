import dataclasses
from typing import Dict, Mapping, Optional, Tuple, Union, cast
from uuid import UUID

import structlog

from bench.dataset.accessor import write_to_dataset
from bench.executor.base import (
    Executor,
    FlowArgument,
    FlowInput,
    FlowOutput,
    FlowRawArgument,
    FlowRawInput,
    ResourceRequirements,
)
from bench.executor.utils import get_model_iid
from bench.function.base import Function
from bench.model.base import ModelHandler, load_model
from bench.models import ArtifactVersion, FlowExecution, ModelExecution
from bench.models.dataset import Dataset, DatasetMetadata
from bench.models.execution import FLOW_EXECUTION_TYPE, MODEL_EXECUTION_TYPE
from bench.models.flow import FlowNode, FlowVersion
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
    inputs: Mapping[UUID, Mapping[str, FlowInput]]
    outputs: Mapping[UUID, Mapping[str, FlowOutput]]
    arguments: Mapping[UUID, Mapping[str, FlowArgument]]


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
        inputs: Mapping[UUID, Mapping[str, FlowRawInput]],
        arguments: Mapping[UUID, Mapping[str, FlowRawArgument]],
    ) -> Tuple[FlowExecution, Mapping[UUID, Mapping[str, ArtifactVersion]]]:
        execution = FlowExecution.objects.create(type=FLOW_EXECUTION_TYPE, flow=flow)
        nodes: Mapping[UUID, FlowNode] = {node.id: node for node in flow.nodes.all()}

        # convert given inputs to persisted artifacts as needed
        converted_inputs: dict[UUID, Mapping[str, FlowInput]] = {}
        for node_id, node_inputs in inputs.items():
            converted_node_inputs: dict[str, ArtifactVersion] = {}
            for name, data in node_inputs.items():
                node_input_id = f"{flow.name}/{nodes[node_id].name}/inputs/{name}"
                if isinstance(data, RecordBatch):
                    data = _convert_records_to_dataset(node_input_id, data)
                elif not isinstance(data, ArtifactVersion):
                    raise ValueError(f"node input {node_input_id} has unexpected type: {data}")
                converted_node_inputs[name] = data
            converted_inputs[node_id] = converted_node_inputs

        # convert given arguments to persisted artifacts as needed
        converted_arguments: dict[UUID, Mapping[str, FlowArgument]] = {}
        for node_id, node_arguments in arguments.items():
            converted_node_arguments: dict[str, Union[ArtifactVersion, FlowNode]] = {}
            for name, data in node_arguments.items():
                node_argument_id = f"{flow.name}/{nodes[node_id].name}/arguments/{name}"
                if isinstance(data, RecordBatch):
                    data = _convert_records_to_dataset(node_argument_id, data)
                elif not isinstance(data, (ArtifactVersion, FlowNode)):
                    raise ValueError(
                        f"node argument {node_argument_id} has unexpected type: {data}"
                    )
                converted_node_arguments[name] = data
            converted_arguments[node_id] = converted_node_arguments

        defined_arguments: dict[UUID, Mapping[str, FlowArgument]] = {}
        implied_outputs = {}
        # define execution plan for piping flow data
        plan = FlowExecutionPlan(
            nodes=nodes,
            inputs=converted_inputs,
            arguments={**defined_arguments, **converted_arguments},
            outputs=implied_outputs,
        )

        # load functions and execute plan
        functions: dict[UUID, Function] = {}
        for node in plan.nodes.values():
            pass

        return execution, plan.outputs

from typing import Dict, Mapping, Optional, Tuple, Union, cast
from uuid import UUID

import structlog

from bench.dataset.accessor import read_dataset
from bench.executor.base import (
    Executor,
    FlowExecutionOptions,
    FlowRawArgument,
    ResourceRequirements,
    make_execution_plan,
    manifest_execution_plan,
)
from bench.executor.utils import get_model_iid
from bench.function.base import Function, RecordTransform, load_function
from bench.model.base import ModelHandler, load_model
from bench.models import ArtifactVersion, DatasetVersion, FlowExecution, ModelExecution
from bench.models.execution import DEFAULT_CONNECTION_NAME, MODEL_EXECUTION_TYPE
from bench.models.flow import FlowVersion
from bench.models.model import ModelVersion
from bench.models.utils import DATASET_TYPE
from bench.utils.record import Record, RecordBatch, is_record

logger = structlog.stdlib.get_logger()


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
        plan = make_execution_plan(flow, inputs, arguments, options)
        manifest = manifest_execution_plan(flow, plan)

        # load functions, define execution process loop
        functions: dict[UUID, Function] = {}
        for node in plan.nodes.values():
            # config_spec = get_config_spec(node.function_id)
            config_arguments = node.config_arguments
            # TODO @Feature: pass artifact function arguments (model, dataset)
            function = load_function(node.function_id, arguments=config_arguments)
            functions[node.id] = function

        # start execution
        for node_id, func in functions.items():
            if not isinstance(func, RecordTransform):
                # TODO @Feature: support functions other than record transforms
                raise ValueError(f"function is not supported at {node_id}: {func}")
        record_transforms = cast(Mapping[UUID, RecordTransform], functions)

        # keep track of not yet processed data by input node in `pending_data`
        #  currently only supports one connection channel ("main")
        pending_data: dict[UUID, RecordBatch] = dict()
        for node_id, named_inputs in plan.artifact_inputs.items():
            for input_key, connection in named_inputs.items():
                if input_key != DEFAULT_CONNECTION_NAME:
                    raise ValueError(
                        f"function with non-default connection is not supported: {input_key}"
                    )
                if connection.artifact.artifact.type == DATASET_TYPE:
                    dataset = cast(DatasetVersion, connection.artifact)
                    pending_data[node_id] = read_dataset(dataset)

        # process all pending data until nothing is left
        while True:
            for node_id, input_batch in pending_data.items():
                output_batch = record_transforms[node_id].transform_batch(input_batch)

                # write to corresponding output artifacts
                for input_key, connection in plan.connected_inverse[node_id].items():
                    if input_key != DEFAULT_CONNECTION_NAME:
                        raise ValueError(
                            f"function with non-default connection is not supported: {input_key}"
                        )
                    pending_data[connection.edge.dependent.node_id] = output_batch

            pending_data = {
                node_id: data for node_id, data in pending_data.items() if len(data) == 0
            }
            if len(pending_data) == 0:
                break

        return manifest.execution, plan.final_outputs

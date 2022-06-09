from typing import Dict, Mapping, Optional, Tuple, Union, cast
from uuid import UUID

import structlog

from bench.executor.base import (
    Executor,
    FlowExecutionOptions,
    FlowRawArgument,
    ResourceRequirements,
    make_execution_plan,
    manifest_execution_plan,
)
from bench.executor.utils import get_model_iid
from bench.function.base import Function, get_config_spec, load_function
from bench.model.base import ModelHandler, load_model
from bench.models import ArtifactVersion, FlowExecution, ModelExecution
from bench.models.execution import MODEL_EXECUTION_TYPE
from bench.models.flow import FlowVersion
from bench.models.model import ModelVersion
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
            config_spec = get_config_spec(node.function_id)
            config_arguments = node.config_arguments
            function = load_function(node.function_id)

        # start execution

        return manifest.execution, plan.final_outputs

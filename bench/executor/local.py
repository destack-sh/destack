from typing import Dict, Mapping, Optional, Tuple, Union, cast
from uuid import UUID

import structlog

from bench.dataset.accessor import get_dataset_version_handler, read_dataset, write_to_dataset
from bench.dataset.base import DatasetHandler
from bench.executor.base import (
    Executor,
    FlowExecutionOptions,
    FlowExecutionPlan,
    FlowRawArgument,
    ResourceRequirements,
    make_execution_plan,
    manifest_execution,
)
from bench.executor.utils import get_model_iid
from bench.function.base import Function, MetricFunction, RecordTransform, load_function
from bench.model.base import ModelHandler, load_model
from bench.models import ArtifactVersion, DatasetVersion, FlowExecution, ModelExecution
from bench.models.execution import DEFAULT_CONNECTION_NAME, MODEL_EXECUTION_TYPE
from bench.models.flow import FlowVersion
from bench.models.model import ModelVersion
from bench.models.utils import DATASET_TYPE, MODEL_TYPE
from bench.utils.record import Record, RecordBatch, RecordList, is_record

logger = structlog.stdlib.get_logger()


class LocalExecutor(Executor):
    """
    A locally executed implementation of Executor without coordination or parallelism.
    """

    def __init__(self):
        self._loaded_models_by_iid: Dict[str, ModelHandler] = {}

    def _get_model_handler(self, model: ModelVersion, load_if_needed: bool = True) -> ModelHandler:
        model_iid = get_model_iid(model)
        if model_iid not in self._loaded_models_by_iid:
            if not load_if_needed:
                raise RuntimeError("model " + model_iid + " is not load")
            else:
                self.load_model(model)
        return self._loaded_models_by_iid[model_iid]

    def _get_dataset_handler(self, dataset: DatasetVersion) -> DatasetHandler:
        return get_dataset_version_handler(dataset)

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
            model_handler = self._get_model_handler(model, load_if_needed)
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
        manifest = manifest_execution(flow, plan)

        if options.blocking:
            with manifest.execution.capture():
                self._do_execute(plan)
        else:
            raise ValueError("non-blocking execution not supported")

        return manifest.execution, plan.final_outputs

    def _do_execute(self, plan: FlowExecutionPlan):
        # load functions with corresponding arguments
        functions: dict[UUID, Function] = {}
        for node in plan.nodes.values():
            config_arguments = node.config_arguments
            artifact_arguments: dict[str, Union[ModelHandler, DatasetHandler]] = {}
            for name, artifact_connection in plan.artifact_arguments.get(node.id, {}).items():
                if artifact_connection.artifact_type == MODEL_TYPE:
                    model = cast(ModelVersion, artifact_connection.artifact)
                    artifact_arguments[name] = self._get_model_handler(model)
                elif artifact_connection.artifact_type == DATASET_TYPE:
                    dataset = cast(DatasetVersion, artifact_connection.artifact)
                    artifact_arguments[name] = self._get_dataset_handler(dataset)
                else:
                    raise ValueError(f"unknown artifact type: {artifact_connection}")

            arguments = {**config_arguments, **artifact_arguments}
            function = load_function(node.function_id, arguments=arguments)
            functions[node.id] = function

        # organise functions
        record_transforms: dict[UUID, RecordTransform] = {}
        metric_functions: dict[UUID, MetricFunction] = {}
        for node_id, func in functions.items():
            if isinstance(func, RecordTransform):
                record_transforms[node_id] = func
            elif isinstance(func, MetricFunction):
                metric_functions[node_id] = func
            else:
                # TODO @Feature: support all function types
                raise ValueError(f"function is not supported at {node_id}: {func}")

        # start actual execution
        # keep track of not yet processed data by input node in `pending_data`
        #  currently only supports one connection channel ("*")
        pending_data: dict[UUID, dict[str, RecordBatch]] = dict()
        for node_id, named_inputs in plan.artifact_inputs.items():
            for input_key, artifact_connection in named_inputs.items():
                if artifact_connection.artifact.artifact.type == DATASET_TYPE:
                    pending_data[node_id][artifact_connection.name] = read_dataset(
                        cast(DatasetVersion, artifact_connection.artifact)
                    )
                else:
                    raise ValueError(f"non-dataset artifacts not supported: {artifact_connection}")

        # process all pending data until nothing is left
        visited_node_ids: set[UUID] = set()
        new_pending_data: dict[UUID, dict[str, RecordBatch]] = dict()
        while True:
            for node_id, input_batches in pending_data.items():
                visited_node_ids.add(node_id)

                function = functions[node_id]
                if isinstance(function, RecordTransform):
                    # assume record transforms have only one default connection in and out
                    input_batch = input_batches[DEFAULT_CONNECTION_NAME]
                    output_batch = record_transforms[node_id].transform_batch(input_batch)
                    output_batches = {DEFAULT_CONNECTION_NAME: output_batch}
                elif isinstance(function, MetricFunction):
                    output_record = metric_functions[node_id].compute(**input_batches)
                    output_batch = RecordList([output_record])
                    output_batches = {DEFAULT_CONNECTION_NAME: output_batch}
                else:
                    raise RuntimeError(f"unexpected function: {function}")

                # write to next input nodes and intermediate output artifacts (if any)
                for node_connection in plan.connected_inverse[node_id].values():
                    dependent_id = node_connection.edge.dependent.id
                    if dependent_id in visited_node_ids:
                        raise RuntimeError(f"cycle between {node_id} and {dependent_id}")

                    output_batch = output_batches[node_connection.dependent_name]
                    new_pending_data[dependent_id][node_connection.dependent_name] = output_batch
                    if node_connection.intermediate_artifact is not None:
                        write_to_dataset(node_connection.intermediate_artifact, output_batch)

                # write to final outputs (if any)
                for artifact in plan.final_outputs.get(node_id, {}).values():
                    if artifact.artifact.type == DATASET_TYPE:
                        write_to_dataset(cast(DatasetVersion, artifact), output_batch)
                    else:
                        raise ValueError(f"non-dataset artifacts not supported: {artifact}")

            # clear already processed nodes from pending and stop if everything is processed
            if len(new_pending_data) == 0:
                break
            pending_data = new_pending_data
            new_pending_data = dict()

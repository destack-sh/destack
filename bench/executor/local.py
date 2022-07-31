from __future__ import annotations

import threading
import time
import uuid
from collections import defaultdict
from itertools import chain
from queue import Empty, Queue
from typing import Dict, Mapping, Optional, Tuple, Union, cast
from uuid import UUID

import structlog
from django.db.models import Q
from more_itertools import flatten

from bench.artifact.base import ArtifactHandler
from bench.dataset.accessor import (
    get_dataset_version_handler,
    read_dataset_version,
    update_dataset_spec,
    write_to_dataset,
    write_to_dataset_version,
)
from bench.dataset.base import DatasetHandler
from bench.executor.base import (
    ArtifactConnection,
    Executor,
    FlowExecutionManifest,
    FlowExecutionOptions,
    FlowExecutionPlan,
    FlowNodeConnection,
    FlowRawArgument,
    FlowRuntimeValidation,
    ResourceRequirements,
    make_execution_plan,
    prepare_execution_manifest,
    save_execution_manifest,
    validate_record_batch_type,
)
from bench.executor.utils import get_model_iid
from bench.function.base import Metric, RecordFunction, RecordTransform, load_function
from bench.model.base import ModelHandler, load_model
from bench.models import ArtifactVersion, DatasetVersion, FlowExecution, ModelExecution
from bench.models.dataset import DatasetViewData
from bench.models.execution import DEFAULT_CONNECTION_NAME, Execution, ExecutionArtifactConnection
from bench.models.flow import FlowNodeEdge, FlowVersion
from bench.models.model import ModelVersion
from bench.models.utils import DATASET_TYPE, MODEL_TYPE
from bench.utils.func import dict_to_ordered, terrible_cast
from bench.utils.record import Record, RecordBatch, RecordList
from bench.utils.spec import ArtifactSpec, FieldTypePrimitive, FieldTypeSpec

logger = structlog.stdlib.get_logger()


class LocalExecutorThread(threading.Thread):
    def __init__(
        self,
        executor: LocalExecutor,
        executions_queue: Queue[Tuple[FlowExecutionPlan, FlowExecutionManifest]],
        daemon: bool,
        **kwargs,
    ):
        super().__init__(**kwargs, daemon=daemon)
        self._executions_queue = executions_queue
        self._executor = executor
        self._should_stop = False

    def run(self):
        while not self._should_stop:
            try:
                plan, manifest = self._executions_queue.get_nowait()
            except Empty:
                time.sleep(0.01)
                continue

            save_execution_manifest(manifest)
            try:
                with manifest.execution.capture():
                    logger.info("execute_started", execution=manifest.execution)
                    self._executor._do_execute(plan, manifest)
                logger.info("execute_terminated", execution=manifest.execution)
            except Exception as e:
                logger.error("execute_failed", execution=manifest.execution, error=e)

    def stop(self):
        self._should_stop = True


class LocalExecutor(Executor):
    """
    A locally executed implementation of Executor without coordination or parallelism.
    """

    def __init__(self):
        self.executor_id = uuid.uuid4().hex
        self._loaded_models_by_iid: Dict[str, ModelHandler] = {}
        self._executions_queue: Queue[Tuple[FlowExecutionPlan, FlowExecutionManifest]] = Queue()
        self._executions_thread = LocalExecutorThread(self, self._executions_queue, daemon=True)

    def start(self):
        self._executions_thread.start()
        self.mark_dead_executions_failed()

    def stop(self):
        if self._executions_thread.is_alive():
            self._executions_thread.stop()
            self._executions_thread.join()

    def mark_dead_executions_failed(self):
        dead_executions = Execution.objects.filter(
            Q(state__in=[state.value for state in Execution.PENDING_STATES])
            & Q(metadata__queued__executor_type="local")
            & ~Q(metadata__queued__executor_id=self.executor_id),
        )
        for execution in dead_executions:
            logger.warning("mark_dead_queued_execution_failed", execution=execution)
            execution.terminate(
                state=Execution.State.Failed, transition_metadata={"message": "dead"}
            )

    def _get_model_handler(self, model: ModelVersion, load_if_needed: bool = True) -> ModelHandler:
        model_iid = get_model_iid(model)
        if model_iid not in self._loaded_models_by_iid:
            if not load_if_needed:
                raise RuntimeError("model " + model_iid + " is not loaded")
            else:
                self.load_model(model, requirements=None)
        return self._loaded_models_by_iid[model_iid]

    def _get_dataset_handler(self, dataset: DatasetVersion) -> DatasetHandler:
        return get_dataset_version_handler(dataset)

    def load_model(
        self, model: ModelVersion, requirements: Optional[ResourceRequirements]
    ) -> ModelHandler:
        model_iid = get_model_iid(model=model)
        if model_iid in self._loaded_models_by_iid:
            return self._loaded_models_by_iid[model_iid]

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
        return model_handler

    def load_artifact(
        self,
        artifact: ArtifactVersion,
        requirements: Optional[ResourceRequirements] = None,
    ) -> ArtifactHandler:
        if artifact.artifact.type == MODEL_TYPE:
            return self.load_model(terrible_cast(ModelVersion, artifact), requirements)
        elif artifact.artifact.type == DATASET_TYPE:
            return get_dataset_version_handler(terrible_cast(DatasetVersion, artifact))
        else:
            raise NotImplementedError(f"cannot load: {artifact}")

    def get_runtime_artifact_spec(self, artifact: ArtifactVersion) -> ArtifactSpec:
        artifact_handler = self.load_artifact(artifact)
        return artifact_handler.runtime_spec

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
        execution = ModelExecution.objects.create(model=model)

        with execution.capture(start=False):
            # record inputs
            input_dataset, view = write_to_dataset(f"{model.artifact.name}.inputs", "0", record)
            execution.connected_artifacts.create(
                connection_type=ExecutionArtifactConnection.ConnectionType.Input,
                connection_name=DEFAULT_CONNECTION_NAME,
                artifact=input_dataset,
                view_inline=DatasetViewData.from_slice(view).asdict,
            )

            # run model
            model_handler = self._get_model_handler(model, load_if_needed)
            execution.start()
            output: Union[Record, RecordBatch]
            if isinstance(record, RecordBatch):
                output = model_handler.predict_batch(record)
            else:
                output = model_handler.predict(record)

            # record outputs
            output_dataset, view = write_to_dataset(f"{model.artifact.name}.outputs", "0", output)
            execution.connected_artifacts.create(
                connection_type=ExecutionArtifactConnection.ConnectionType.Output,
                connection_name=DEFAULT_CONNECTION_NAME,
                artifact=output_dataset,
                view_inline=DatasetViewData.from_slice(view).asdict,
            )

        return execution, output

    def run_flow(
        self,
        flow: FlowVersion,
        inputs: Mapping[UUID, Mapping[str, FlowRawArgument]],
        arguments: Mapping[UUID, Mapping[str, FlowRawArgument]],
        options: FlowExecutionOptions,
    ) -> Tuple[FlowExecution, FlowExecutionPlan]:
        logger.debug("execute_planning")
        plan = make_execution_plan(flow, inputs, arguments, options)
        logger.debug("execute_planned")
        manifest = prepare_execution_manifest(flow, plan)
        logger.debug("execute_manifested", execution=manifest.execution)

        executor_metadata = {"executor_id": self.executor_id, "executor_type": "local"}
        if options.blocking:
            manifest.execution.save()
            save_execution_manifest(manifest)
            with manifest.execution.capture(start_metadata=executor_metadata):
                logger.info("execute_started", execution=manifest.execution)
                self._do_execute(plan, manifest)
            logger.info("execute_terminated", execution=manifest.execution)
        else:
            manifest.execution.update_state(
                state=FlowExecution.State.Queued, transition_metadata=executor_metadata
            )
            self._executions_queue.put((plan, manifest))

        return manifest.execution, plan

    def _do_execute(self, plan: FlowExecutionPlan, manifest: FlowExecutionManifest):
        # load functions with corresponding arguments
        functions = self._load_functions(plan)

        # update specs for special functions (like identity) where input/output spec is dynamic
        # TODO @Cleanup @Architecture: don't update function specs at runtime
        #  Ideally we set this ahead of time, e.g. by the user configuring input specs and by
        #  setting specs for dynamic spec functions whenever the flow changes.
        self._patch_dynamic_function_specs(functions, plan)

        self._run_execution_loop(functions, plan)

        # update dynamic (node and final) connections with view data
        self._update_dynamic_connections(manifest, plan)

    def _run_execution_loop(self, functions: dict[UUID, RecordFunction], plan: FlowExecutionPlan):
        # organise functions by type
        record_transforms: dict[UUID, RecordTransform] = {}
        metric_functions: dict[UUID, Metric] = {}
        for node_id, function in functions.items():
            if isinstance(function, RecordTransform):
                record_transforms[node_id] = function
            elif isinstance(function, Metric):
                metric_functions[node_id] = function
            else:
                # TODO @Feature: support all function types
                raise ValueError(f"function is not supported at {node_id}: {function}")

        # start actual execution
        # keep track of not yet processed data by input node in `pending_data`
        pending_data: dict[UUID, dict[str, RecordBatch]] = defaultdict(dict)
        for node_id, named_inputs in plan.artifact_inputs.items():
            for input_key, artifact_connection in named_inputs.items():
                if artifact_connection.artifact.artifact.type == DATASET_TYPE:
                    dataset = cast(DatasetVersion, artifact_connection.artifact)
                    pending_data[node_id][artifact_connection.name] = read_dataset_version(
                        dataset, artifact_connection.view_data
                    )
                    input_spec = functions[node_id].input_spec[input_key]
                    update_dataset_spec(dataset, input_spec)
                else:
                    raise ValueError(f"non-dataset artifacts not supported: {artifact_connection}")

        # general settings
        validate = plan.options.validate in (FlowRuntimeValidation.Lazy, FlowRuntimeValidation.Full)
        validate_lazy = plan.options.validate == FlowRuntimeValidation.Lazy

        def _validate_records(
            source_node_id: UUID,
            records: RecordBatch,
            spec_type: Union[FieldTypeSpec, FieldTypePrimitive],
        ):
            if validate:
                validate_record_batch_type(
                    records, spec_type, ignore_extraneous=True, lazy=validate_lazy
                )

        # process all pending data until nothing is left
        visited_node_ids: set[UUID] = set()
        new_pending_data: dict[UUID, dict[str, RecordBatch]] = defaultdict(dict)
        max_iterations = len(plan.nodes) ** 2  # set arbitrarily high to catch loops
        iteration = 0
        while True:
            if iteration > max_iterations:
                # looks like we're stuck, abort
                raise RuntimeError(f"reached maximum iteration {iteration} in plan: {plan}")

            logger.debug("local_execute", iteration=iteration, pending_data=pending_data, plan=plan)
            for node_id, input_batches in pending_data.items():
                function = functions[node_id]
                missing_input_keys = function.input_spec.keys() - input_batches.keys()
                if missing_input_keys:
                    # wait until all inputs are provided, skip this node in the current iteration
                    new_pending_data[node_id].update(input_batches)
                    # if inputs are never provided, terminate via the catch at the top
                    continue

                visited_node_ids.add(node_id)
                if isinstance(function, RecordTransform):
                    # assume record transforms have only one default connection in and out
                    input_batch = input_batches[DEFAULT_CONNECTION_NAME]
                    output_batch = record_transforms[node_id].transform_batch(input_batch)
                    output_batches = {DEFAULT_CONNECTION_NAME: output_batch}
                elif isinstance(function, Metric):
                    # assume input batches contains all required inputs (for now)
                    output_record = metric_functions[node_id].compute(**input_batches)
                    output_batch = RecordList([output_record])
                    output_batches = {DEFAULT_CONNECTION_NAME: output_batch}
                else:
                    raise RuntimeError(f"unexpected function: {function}")

                # validate output against output spec
                output_spec = function.output_spec[DEFAULT_CONNECTION_NAME]
                _validate_records(node_id, output_batch, output_spec.type)

                # write to next input nodes and intermediate output artifacts (if any)
                for node_connection in flatten(plan.connected_inverse[node_id].values()):
                    dependent_id = node_connection.edge.dependent.id
                    if dependent_id in visited_node_ids:
                        raise RuntimeError(f"cycle between {node_id} and {dependent_id}")

                    output_batch = output_batches[node_connection.dependent_name]
                    new_pending_data[dependent_id][node_connection.dependency_name] = output_batch

                    # validate output against next input spec
                    input_spec = functions[dependent_id].input_spec[node_connection.dependency_name]
                    _validate_records(node_id, output_batch, input_spec.type)

                    if node_connection.intermediate_artifact is not None:
                        output_spec = function.output_spec[node_connection.dependent_name]
                        view = write_to_dataset_version(
                            node_connection.intermediate_artifact,
                            output_batch,
                            record_spec=output_spec,
                        )
                        node_connection.view_inline = DatasetViewData.from_slice(view).asdict

                # write to final outputs (if any)
                for artifact_connection in plan.final_outputs.get(node_id, {}).values():
                    if artifact_connection.artifact_type != DATASET_TYPE:
                        raise ValueError(
                            f"non-dataset artifacts not supported: {artifact_connection}"
                        )

                    output_spec = function.output_spec[DEFAULT_CONNECTION_NAME]
                    view = write_to_dataset_version(
                        cast(DatasetVersion, artifact_connection.artifact),
                        output_batch,
                        record_spec=output_spec,
                    )
                    artifact_connection.view_inline = DatasetViewData.from_slice(view).asdict

            # clear already processed nodes from pending and stop if everything is processed
            if len(new_pending_data) == 0:
                break
            pending_data = new_pending_data
            new_pending_data = defaultdict(dict)
            iteration += 1

    def _update_dynamic_connections(self, manifest: FlowExecutionManifest, plan: FlowExecutionPlan):
        dynamic_connections = flatten(
            connections.values()
            for connections in chain(
                plan.node_inputs.values(), plan.node_arguments.values(), plan.final_outputs.values()
            )
        )
        for connection in cast(
            list[Union[FlowNodeConnection, ArtifactConnection]], dynamic_connections
        ):
            connection_id = connection.manifested_id
            if connection_id is not None:  # only if dynamic connection is actually manifested
                execution_connection = manifest.execution_connections[connection_id]
                execution_connection.view_inline = connection.view_inline
        if dynamic_connections:
            ExecutionArtifactConnection.objects.bulk_update(
                manifest.execution_connections.values(), ["view_inline"]
            )

    def _load_functions(self, plan: FlowExecutionPlan) -> dict[UUID, RecordFunction]:
        functions: dict[UUID, RecordFunction] = {}
        for node in plan.nodes.values():
            config_arguments = node.config_arguments
            artifact_arguments: dict[str, Union[ModelHandler, DatasetHandler]] = {}
            for name, artifact_connection in plan.artifact_arguments.get(node.id, {}).items():
                if artifact_connection.artifact_type == MODEL_TYPE:
                    model = terrible_cast(ModelVersion, artifact_connection.artifact)
                    artifact_arguments[name] = self._get_model_handler(model)
                elif artifact_connection.artifact_type == DATASET_TYPE:
                    dataset = terrible_cast(DatasetVersion, artifact_connection.artifact)
                    artifact_arguments[name] = self._get_dataset_handler(dataset)
                else:
                    raise ValueError(f"unknown artifact type: {artifact_connection}")

            arguments = {**config_arguments, **artifact_arguments}
            function = load_function(node.function_id, arguments=arguments)
            if not isinstance(function, RecordFunction):
                raise ValueError(f"function not yet supported: {function}")
            functions[node.id] = function
        return functions

    def _patch_dynamic_function_specs(
        self, functions: dict[UUID, RecordFunction], plan: FlowExecutionPlan
    ):
        for node_id, function in functions.items():
            # only bench.identity is "dynamic"(ally) dependent on other functions right now
            if plan.nodes[node_id].function_id != "bench.identity":
                continue

            input_node_connection = plan.node_inputs.get(node_id, {}).get("*")
            if input_node_connection is not None:
                input_function = functions[input_node_connection.edge.dependency_id]
                function.input_spec = input_function.output_spec
                function.output_spec = function.input_spec
                continue
            output_node_connections = plan.connected_inverse[node_id].get("*")
            for output_node_connection in output_node_connections or []:
                if output_node_connection.edge.connection_type != FlowNodeEdge.ConnectionType.Input:
                    continue
                output_function = functions[output_node_connection.edge.dependent_id]
                function.output_spec = dict_to_ordered(output_function.input_spec)
                function.input_spec = function.output_spec
                break

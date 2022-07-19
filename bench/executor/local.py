from __future__ import annotations

import threading
import uuid
from collections import defaultdict
from itertools import chain
from queue import Queue
from typing import Dict, Iterable, Mapping, Optional, Tuple, Union, cast
from uuid import UUID

import structlog
from django.db.models import Q
from more_itertools import flatten

from bench.dataset.accessor import (
    get_dataset_version_handler,
    read_dataset_version,
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
    ResourceRequirements,
    make_execution_manifest,
    make_execution_plan,
)
from bench.executor.utils import get_model_iid
from bench.function.base import MetricFunction, RecordFunction, RecordTransform, load_function
from bench.model.base import ModelHandler, load_model
from bench.models import DatasetVersion, FlowExecution, ModelExecution
from bench.models.dataset import DatasetViewData
from bench.models.execution import DEFAULT_CONNECTION_NAME, Execution, ExecutionArtifactConnection
from bench.models.flow import FlowVersion
from bench.models.model import ModelVersion
from bench.models.utils import DATASET_TYPE, MODEL_TYPE
from bench.utils.func import terrible_cast
from bench.utils.record import Record, RecordBatch, RecordList

logger = structlog.stdlib.get_logger()


class LocalExecutorThread(threading.Thread):
    def __init__(
        self,
        executor: LocalExecutor,
        executions_queue: Queue[Tuple[FlowExecutionPlan, FlowExecutionManifest]],
        **kwargs,
    ):
        super().__init__(**kwargs)
        self._executions_queue = executions_queue
        self._executor = executor

    def run(self):
        while True:
            plan, manifest = self._executions_queue.get()
            with manifest.execution.capture():
                logger.info("execute_started", execution=manifest.execution)
                self._executor._do_execute(plan, manifest)
            logger.info("execute_terminated", execution=manifest.execution)


class LocalExecutor(Executor):
    """
    A locally executed implementation of Executor without coordination or parallelism.
    """

    def __init__(self):
        self.executor_id = uuid.uuid4().hex
        self._loaded_models_by_iid: Dict[str, ModelHandler] = {}
        self._executions_queue: Queue[Tuple[FlowExecutionPlan, FlowExecutionManifest]] = Queue()
        self._executions_thread = LocalExecutorThread(self, self._executions_queue)

    def start(self):
        self._executions_thread.start()
        self.mark_dead_executions_failed()

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
        manifest = make_execution_manifest(flow, plan)
        logger.debug("execute_manifested", execution=manifest.execution)

        if options.blocking:
            with manifest.execution.capture():
                logger.info("execute_started", execution=manifest.execution)
                self._do_execute(plan, manifest)
            logger.info("execute_terminated", execution=manifest.execution)
        else:
            manifest.execution.update_state(
                state=FlowExecution.State.Queued,
                transition_metadata={"executor_id": self.executor_id, "executor_type": "local"},
            )
            self._executions_queue.put((plan, manifest))

        return manifest.execution, plan

    def _do_execute(self, plan: FlowExecutionPlan, manifest: FlowExecutionManifest):
        # load functions with corresponding arguments
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

        # organise functions
        record_transforms: dict[UUID, RecordTransform] = {}
        metric_functions: dict[UUID, MetricFunction] = {}
        for node_id, function in functions.items():
            if isinstance(function, RecordTransform):
                record_transforms[node_id] = function
            elif isinstance(function, MetricFunction):
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
                    pending_data[node_id][artifact_connection.name] = read_dataset_version(
                        cast(DatasetVersion, artifact_connection.artifact),
                        artifact_connection.view_data,
                    )
                else:
                    raise ValueError(f"non-dataset artifacts not supported: {artifact_connection}")

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
                elif isinstance(function, MetricFunction):
                    # assume input batches contains all required inputs (for now)
                    output_record = metric_functions[node_id].compute(**input_batches)
                    output_batch = RecordList([output_record])
                    output_batches = {DEFAULT_CONNECTION_NAME: output_batch}
                else:
                    raise RuntimeError(f"unexpected function: {function}")

                # write to next input nodes and intermediate output artifacts (if any)
                for node_connection in flatten(plan.connected_inverse[node_id].values()):
                    dependent_id = node_connection.edge.dependent.id
                    if dependent_id in visited_node_ids:
                        raise RuntimeError(f"cycle between {node_id} and {dependent_id}")

                    output_batch = output_batches[node_connection.dependent_name]
                    new_pending_data[dependent_id][node_connection.dependency_name] = output_batch
                    if node_connection.intermediate_artifact is not None:
                        view = write_to_dataset_version(
                            node_connection.intermediate_artifact, output_batch
                        )
                        node_connection.view_inline = DatasetViewData.from_slice(view).asdict

                # write to final outputs (if any)
                for artifact_connection in plan.final_outputs.get(node_id, {}).values():
                    if artifact_connection.artifact_type != DATASET_TYPE:
                        raise ValueError(
                            f"non-dataset artifacts not supported: {artifact_connection}"
                        )

                    view = write_to_dataset_version(
                        cast(DatasetVersion, artifact_connection.artifact), output_batch
                    )
                    artifact_connection.view_inline = DatasetViewData.from_slice(view).asdict

            # clear already processed nodes from pending and stop if everything is processed
            if len(new_pending_data) == 0:
                break
            pending_data = new_pending_data
            new_pending_data = defaultdict(dict)
            iteration += 1

        # update dynamic (node and final) connections with view data
        dynamic_connections: Iterable[Union[FlowNodeConnection, ArtifactConnection]] = flatten(
            connections.values()
            for connections in chain(
                plan.node_inputs.values(), plan.node_arguments.values(), plan.final_outputs.values()
            )
        )
        for connection in dynamic_connections:
            connection_id = cast(UUID, connection.manifested_id)
            if connection_id is not None:  # only if dynamic connection is actually manifested
                execution_connection = manifest.execution_connections[connection_id]
                execution_connection.view_inline = connection.view_inline
        if dynamic_connections:
            ExecutionArtifactConnection.objects.bulk_update(
                manifest.execution_connections.values(), ["view_inline"]
            )

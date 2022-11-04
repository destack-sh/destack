from __future__ import annotations

import threading
import time
import traceback
import uuid
from collections import defaultdict
from functools import cached_property
from itertools import chain
from queue import Empty, Queue
from typing import Dict, Mapping, Optional, Tuple, Union, cast
from uuid import UUID

import structlog
from more_itertools import flatten

from bench.executor.base import (
    ArtifactConnection,
    Executor,
    FlowExecutionManifest,
    FlowExecutionOptions,
    FlowExecutionPlan,
    FlowInstructionConnection,
    FlowRawArgument,
    FlowRuntimeValidation,
    ResourceRequirements,
    make_execution_plan,
    prepare_execution_manifest,
    prepare_function_arguments,
    save_execution_manifest,
)
from bench.executor.utils import get_model_iid
from bench.function.base import Metric, RecordFunction, RecordTransform, load_function
from bench.function.utils import ModelRecordTransform
from bench.model.base import ModelHandler
from bench.models import DatasetRecord, DatasetVersion, FlowExecution, ModelExecution
from bench.models.dataset import Dataset, DatasetViewData
from bench.models.execution import DEFAULT_CONNECTION_NAME, Execution, ExecutionArtifactConnection
from bench.models.flow import FlowArtifactEdge, FlowInstruction, FlowVersion
from bench.models.tag import default_tag
from bench.models.utils import DATASET_TYPE, MODEL_TYPE
from bench.utils.func import terrible_cast
from bench.utils.record import Record, RecordBatch, RecordList
from bench.utils.spec import FieldTypePrimitive, FieldTypeSpec, RecordSpec
from bench.utils.validate import validate_record_batch_type

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
                # print stacktrace for e to terminal
                traceback.print_exception(type(e), e, e.__traceback__)
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
            status__in=[status.value for status in Execution.PENDING_STATUSES],
            metadata__queued__executor_type="local",
        )
        for execution in dead_executions:
            logger.warning("mark_dead_queued_execution_failed", execution=execution)
            execution.terminate(
                status=Execution.Status.Failed, transition_metadata={"message": "dead"}
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

    @cached_property
    def _artifact_handler_loaders(self):
        return {
            MODEL_TYPE: lambda model: self._get_model_handler(terrible_cast(ModelVersion, model)),
            DATASET_TYPE: lambda dataset: self._get_dataset_handler(
                terrible_cast(DatasetVersion, dataset)
            ),
        }

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
        model_handler: ModelHandler = get_model_version_handler(model)
        self._loaded_models_by_iid[model_iid] = model_handler
        log.info("model_loaded")
        return model_handler

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
        execution = ModelExecution.objects.create(model=model, organization=model.organization)

        with execution.capture(start=False):
            model_handler = self._get_model_handler(model, load_if_needed)

            # wrap the model as a function to pipe into the same execution and caching system
            model_function = ModelRecordTransform(model_handler)
            model_function_content = FlowInstruction.to_content(
                "bench.model",
                config_arguments={},
                connected_artifacts={(FlowArtifactEdge.ConnectionType.Argument, "model"): model},
            )
            model_function_hash = FlowInstruction.hash_content(model_function_content).hex()

            # record inputs
            input_dataset = Dataset.objects.get_or_create_dataset_version(
                f"{model.artifact.name}.inputs", model.organization
            )
            view = write_dataset(input_dataset, record)
            input_dataset.set_tag(default_tag("source:inputs", model.organization))
            execution.connected_artifacts.create(
                connection_type=ExecutionArtifactConnection.ConnectionType.Input,
                connection_name=DEFAULT_CONNECTION_NAME,
                artifact=input_dataset,
                view_inline=DatasetViewData.from_slice(view).asdict,
            )

            # run model
            execution.start()
            outputs = self._run_function_record_transform(
                model_function, model_function_hash, [DatasetRecord.make(record)]
            )

            # record outputs
            output_dataset = Dataset.objects.get_or_create_dataset_version(
                f"{model.artifact.name}.outputs", model.organization
            )
            view = DatasetHandler(output_dataset).extend(outputs)
            output_dataset.set_tag(default_tag("source:outputs", model.organization))
            execution.connected_artifacts.create(
                connection_type=ExecutionArtifactConnection.ConnectionType.Output,
                connection_name=DEFAULT_CONNECTION_NAME,
                artifact=output_dataset,
                view_inline=DatasetViewData.from_slice(view).asdict,
            )

        return execution, outputs

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
            manifest.execution.update_status(
                status=FlowExecution.Status.Queued, transition_metadata=executor_metadata
            )
            self._executions_queue.put((plan, manifest))

        return manifest.execution, plan

    def _do_execute(self, plan: FlowExecutionPlan, manifest: FlowExecutionManifest):
        # load functions with corresponding arguments
        functions = self._load_functions(plan)

        # run actual execution
        self._run_execution_loop(functions, plan)

        # update dynamic (node and final) connections with view data
        self._update_dynamic_connections(manifest, plan)

    def _run_execution_loop(self, functions: dict[UUID, RecordFunction], plan: FlowExecutionPlan):
        # start actual execution
        # keep track of not yet processed data by input node in `pending_data`
        pending_data = self._collect_initial_data(functions, plan)

        # general settings
        validate = plan.options.validate in (FlowRuntimeValidation.Lazy, FlowRuntimeValidation.Full)
        validate_lazy = plan.options.validate == FlowRuntimeValidation.Lazy

        def _validate_records(
            source_node_id: UUID,
            records: list[DatasetRecord],
            spec_type: Union[FieldTypeSpec, FieldTypePrimitive],
        ):
            if not validate:
                return

            try:
                if validate_lazy and len(records) > 2:
                    # we know that lazy validation only runs on first and last items
                    records_data = [records[0].data, records[-1].data]
                else:
                    records_data = [r.data for r in records]

                validate_record_batch_type(
                    records_data, spec_type, ignore_extraneous=True, lazy=validate_lazy
                )
            except ValueError as e:
                raise RuntimeError(
                    f"node {plan.nodes[source_node_id]} failed validation: {e}", e
                ) from e

        def _validate_batches(
            batches: dict[str, list[DatasetRecord]], specs: Mapping[str, RecordSpec]
        ):
            for name, batch in batches.items():
                spec = specs[name]
                _validate_records(node_id, batch, spec.type)

        # process all pending data until nothing is left
        visited_node_ids: set[UUID] = set()
        new_pending_data: dict[UUID, dict[str, list[DatasetRecord]]] = defaultdict(dict)
        max_iterations = len(plan.nodes) ** 2  # set arbitrarily high to catch loops
        iteration = 0
        while True:
            if iteration > max_iterations:
                # looks like we're stuck, abort
                raise RuntimeError(f"reached maximum iteration {iteration} in plan: {plan}")

            logger.debug("local_execute", iteration=iteration, pending_data=pending_data, plan=plan)
            for node_id, input_batches in pending_data.items():
                function = functions[node_id]

                # wait until all inputs are provided, skip this node in the current iteration
                # if inputs are never provided, we terminate via the catch at the top
                missing_input_keys = function.input_spec.keys() - input_batches.keys()
                if missing_input_keys:
                    new_pending_data[node_id].update(input_batches)
                    continue

                # crudely break cycles by observing visited nodes
                if node_id in visited_node_ids:
                    raise RuntimeError(f"cycle in plan {plan} at {node_id}")
                visited_node_ids.add(node_id)

                _validate_batches(input_batches, function.input_spec)
                output_batches = self._run_function(function, input_batches, node_id, plan)
                _validate_batches(output_batches, function.output_spec)

                # write to next nodes and intermediate datasets
                for node_connection in flatten(plan.connected_inverse[node_id].values()):
                    dependent_id = node_connection.edge.dependent.id
                    output_batch = output_batches[node_connection.dependency_name]
                    new_pending_data[dependent_id][node_connection.dependent_name] = output_batch

                    if node_connection.dataset is not None:
                        output_spec = function.output_spec[node_connection.dependency_name]
                        update_dataset_spec(node_connection.dataset, output_spec)
                        write_view = DatasetHandler(node_connection.dataset).extend(output_batch)
                        node_connection.view_inline = DatasetViewData.from_slice(write_view).asdict

                # write to final outputs (if any)
                for artifact_connection in plan.final_outputs.get(node_id, {}).values():
                    if artifact_connection.dependency_name is None:
                        raise ValueError(
                            f"final output must have dependency name: {artifact_connection}"
                        )

                    output_batch = output_batches[artifact_connection.dependency_name]
                    output_spec = function.output_spec[artifact_connection.dependency_name]
                    update_dataset_spec(artifact_connection.dataset, output_spec)
                    write_view = DatasetHandler(artifact_connection.dataset).extend(output_batch)
                    artifact_connection.view_inline = DatasetViewData.from_slice(write_view).asdict

            # clear already processed nodes from pending and stop if everything is processed
            if len(new_pending_data) == 0:
                break
            pending_data = new_pending_data
            new_pending_data = defaultdict(dict)
            iteration += 1

    def _collect_initial_data(self, functions: dict[UUID, RecordFunction], plan: FlowExecutionPlan):
        pending_data: dict[UUID, dict[str, list[AnyDatasetRecord]]] = defaultdict(dict)
        for node_id, named_inputs in plan.artifact_inputs.items():
            for input_key, artifact_connection in named_inputs.items():
                if artifact_connection.artifact.artifact.type == DATASET_TYPE:
                    dataset = cast(DatasetVersion, artifact_connection.artifact)
                    inputs = DatasetHandler(dataset).get_records_view(artifact_connection.view_data)
                    pending_data[node_id][artifact_connection.name] = inputs

                    # update input spec given function it is assigned to (not great)
                    # TODO @Cleanup: dataset spec should not derive from input spec at execution time
                    input_spec = functions[node_id].input_spec[input_key]
                    update_dataset_spec(dataset, input_spec)
                else:
                    raise ValueError(f"non-dataset artifacts not supported: {artifact_connection}")
        return pending_data

    def _run_function(
        self,
        function: RecordFunction,
        input_batches: dict[str, list[DatasetRecord]],
        node_id: UUID,
        plan: FlowExecutionPlan,
    ) -> dict[str, list[DatasetRecord]]:
        # actually run node
        if isinstance(function, RecordTransform):
            # assume record transforms have only one default connection in and out
            input_batch = input_batches[DEFAULT_CONNECTION_NAME]
            function_hash = plan.nodes[node_id].content_hash.hex()
            output_batch = self._run_function_record_transform(function, function_hash, input_batch)

            return {DEFAULT_CONNECTION_NAME: output_batch}
        elif isinstance(function, Metric):
            # assume input batches contains all required inputs (as checked above)
            output_data = function.compute(**input_batches)
            output_batch = [DatasetRecord.make(output_data)]
            return {DEFAULT_CONNECTION_NAME: output_batch}
        else:
            raise RuntimeError(f"unexpected function: {function}")

    def _run_function_record_transform(
        self,
        function: RecordTransform,
        function_hash: str,
        input_batch: list[DatasetRecord],
    ):
        """
        Runs a given loaded function on the given inputs, automatically handling caching.
        """

        # get cached outputs with the same input + function hash
        input_hashes = [DatasetRecord.hash_content_hex(r.data) for r in input_batch]
        cached_outputs = DatasetRecord.objects.filter(
            metadata__source__function_hash=function_hash,
            metadata__source__input_hash__in=input_hashes,
            metadata__source__cached=False,  # get original outputs only
        )
        cached_outputs_by_input_hashes = defaultdict(list)
        for output_db_record in cached_outputs:
            input_hash = output_db_record.metadata["source"]["input_hash"]
            cached_outputs_by_input_hashes[input_hash].append(output_db_record)

        # compute outputs for uncached inputs
        missing_inputs = [
            record.data
            for i, record in enumerate(input_batch)
            if input_hashes[i] not in cached_outputs_by_input_hashes
        ]
        cache_hits = len(input_batch) - len(missing_inputs)
        logger.debug(
            "compute_record_transform",
            missing_inputs=len(missing_inputs),
            cached_inputs=cache_hits,
            total_inputs=len(input_batch),
            function=function,
        )
        computed_outputs = function.transform_batch(RecordList(missing_inputs))

        # assumes consistent input -> output stride
        if missing_inputs:
            output_stride = len(computed_outputs) // len(missing_inputs)
        else:
            output_stride = 1

        # re-assemble output batch from cached + computed
        output_batch: list[DatasetRecord] = []
        for i, input_hash in enumerate(input_hashes):
            cached_outputs = cached_outputs_by_input_hashes.get(input_hash)
            output_metadata = {
                "source": {
                    "input_hash": input_hash,
                    "function_hash": function_hash,
                    "cached": cached_outputs is not None,
                }
            }
            if cached_outputs:
                # cache hit
                output_batch.extend(
                    DatasetRecord.make(output.data, output_metadata) for output in cached_outputs
                )
            else:
                # cache miss, get from computed outputs
                outputs = computed_outputs[i * output_stride : (i + 1) * output_stride]
                output_batch.extend(
                    DatasetRecord.make(output, output_metadata) for output in outputs
                )
        return output_batch

    def _update_dynamic_connections(self, manifest: FlowExecutionManifest, plan: FlowExecutionPlan):
        dynamic_connections = flatten(
            connections.values()
            for connections in chain(
                plan.node_inputs.values(), plan.node_arguments.values(), plan.final_outputs.values()
            )
        )
        for connection in cast(
            list[Union[FlowInstructionConnection, ArtifactConnection]], dynamic_connections
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
            # prepare arguments
            arguments = prepare_function_arguments(
                node,
                node_artifact_arguments=plan.artifact_arguments.get(node.id, {}).values(),
                handler_loaders=self._artifact_handler_loaders,
            )
            function = load_function(node.function_id, arguments=arguments)
            if not isinstance(function, RecordFunction):
                raise ValueError(f"function not yet supported: {function}")
            functions[node.id] = function
        return functions

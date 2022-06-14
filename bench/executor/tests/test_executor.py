from typing import cast

import pytest

from bench.dataset.accessor import convert_records_to_dataset, read_dataset
from bench.executor import LocalExecutor
from bench.executor.base import FlowExecutionOptions, make_execution_plan
from bench.models import DatasetVersion, Flow, FlowNode, FlowNodeEdge, Model
from bench.models.execution import DEFAULT_CONNECTION_NAME, Execution
from bench.models.model import ModelMetadata
from bench.utils.record import RecordList


@pytest.mark.django_db
def test_plan_empty_flow():
    flow = Flow.objects.create_flow_version_by_name("empty")
    plan = make_execution_plan(
        flow, inputs={}, arguments={}, options=FlowExecutionOptions.default()
    )

    assert plan.artifact_inputs == {}
    assert plan.artifact_arguments == {}
    assert plan.node_inputs == {}
    assert plan.node_arguments == {}
    assert plan.final_outputs == {}


@pytest.mark.django_db
def test_plan_identity_flow():
    flow = Flow.objects.create_flow_version_by_name("identity")
    identity_node: FlowNode = flow.nodes.create(function_id="bench.identity", config_arguments={})

    plan = make_execution_plan(
        flow, inputs={}, arguments={}, options=FlowExecutionOptions.default()
    )

    assert plan.artifact_inputs == {}
    assert plan.artifact_arguments == {}
    assert plan.node_inputs == {}
    assert plan.node_arguments == {}
    assert identity_node.id in plan.final_outputs
    assert DEFAULT_CONNECTION_NAME in plan.final_outputs[identity_node.id]


@pytest.fixture()
def local_executor() -> LocalExecutor:
    return LocalExecutor()


@pytest.mark.django_db
def test_local_execute_empty_flow(local_executor: LocalExecutor):
    flow = Flow.objects.create_flow_version_by_name("identity")
    execution, outputs = local_executor.run_flow(
        flow, inputs={}, arguments={}, options=FlowExecutionOptions.default_blocking()
    )
    assert execution.state == Execution.State.Completed
    assert len(outputs) == 0, "no outputs"


@pytest.mark.django_db
def test_local_execute_one_node_identity_flow(local_executor: LocalExecutor):
    flow = Flow.objects.create_flow_version_by_name("identity")
    identity_node: FlowNode = flow.nodes.create(
        function_id="bench.identity", name="identity_1", config_arguments={}
    )

    input_records = RecordList([{"text": "test"}, {"abc": 123}, {"1": "bananas"}])
    inputs = {identity_node.id: {"main": input_records}}
    execution, outputs = local_executor.run_flow(
        flow, inputs=inputs, arguments={}, options=FlowExecutionOptions.default_blocking()
    )

    assert execution.state == Execution.State.Completed
    assert len(outputs) == 1, "one node has final outputs"
    assert len(outputs[identity_node.id]) == 1, "node has one output key"
    output_records = read_dataset(cast(DatasetVersion, outputs[identity_node.id]["main"]))
    assert output_records == inputs[identity_node.id]["main"], "outputs match inputs"


@pytest.mark.django_db
def test_local_execute_two_node_identity_flow(local_executor: LocalExecutor):
    flow = Flow.objects.create_flow_version_by_name("identity")
    identity_node_1: FlowNode = flow.nodes.create(
        function_id="bench.identity", name="identity_1", config_arguments={}
    )
    identity_node_2: FlowNode = flow.nodes.create(
        function_id="bench.identity", name="identity_2", config_arguments={}
    )
    identity_node_2.depends_on_nodes.add(
        identity_node_1,
        through_defaults=dict(
            connection_type=FlowNodeEdge.ConnectionType.Input, connection_name="main"
        ),
    )

    input_records = RecordList([{"text": "test"}, {"abc": 123}, {"1": "bananas"}])
    inputs = {identity_node_1.id: {"main": input_records}}
    execution, outputs = local_executor.run_flow(
        flow, inputs=inputs, arguments={}, options=FlowExecutionOptions.default_blocking()
    )

    assert execution.state == Execution.State.Completed
    assert len(outputs) == 1, "one node has final outputs"
    assert len(outputs[identity_node_2.id]) == 1, "node has one output key"
    output_records = read_dataset(cast(DatasetVersion, outputs[identity_node_2.id]["main"]))
    assert output_records == inputs[identity_node_1.id]["main"], "outputs match inputs"


@pytest.mark.django_db
def test_local_execute_model_flow(local_executor: LocalExecutor):
    flow = Flow.objects.create_flow_version_by_name("model")
    model_node_1: FlowNode = flow.nodes.create(
        function_id="bench.model", name="model_1", config_arguments={}
    )
    model = Model.objects.create_model_version_by_name(
        "spacy_en_core_web_sm",
        metadata=ModelMetadata(
            handler_id="bench.spacy.bundled", config_arguments={"model_name": "en_core_web_sm"}
        ),
    )

    input_records = RecordList([{"text": "test"}])
    execution, outputs = local_executor.run_flow(
        flow,
        inputs={model_node_1.id: {"main": input_records}},
        arguments={model_node_1.id: {"model": model}},
        options=FlowExecutionOptions.default_blocking(),
    )

    assert execution.state == Execution.State.Completed


@pytest.mark.django_db
def test_local_execute_dataset_flow(local_executor: LocalExecutor):
    flow = Flow.objects.create_flow_version_by_name("dataset")
    swap_node_1: FlowNode = flow.nodes.create(
        function_id="bench.text.swap", name="swap_1", config_arguments={}
    )
    dataset = convert_records_to_dataset(
        "badword_replacements",
        RecordList(
            [
                {"pattern": "fizz", "replacement": "buzz"},
                {"pattern": "buzz", "replacement": "lightyear"},
            ]
        ),
    )

    input_records = RecordList([{"text": "1 2 fizz 4 buzz"}, {"text": "4 buzz 6"}])
    execution, outputs = local_executor.run_flow(
        flow,
        inputs={swap_node_1.id: {"main": input_records}},
        arguments={swap_node_1.id: {"swaps_dataset": dataset}},
        options=FlowExecutionOptions.default_blocking(),
    )

    assert execution.state == Execution.State.Completed
    output_records = read_dataset(outputs[swap_node_1.id]["main"])
    assert output_records == [{"text": "1 2 buzz 4 buzz"}, {"text": "4 lightyear 6"}]

from typing import cast

import pytest

from bench.dataset.accessor import read_dataset
from bench.executor import LocalExecutor
from bench.executor.base import FlowExecutionOptions, make_execution_plan
from bench.models import DatasetVersion, Flow, FlowNode
from bench.models.execution import DEFAULT_CONNECTION_NAME, Execution
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
def test_local_execute_identity_flow(local_executor: LocalExecutor):
    flow = Flow.objects.create_flow_version_by_name("identity")
    identity_node: FlowNode = flow.nodes.create(
        function_id="bench.identity", name="identity_1", config_arguments={}
    )

    input_records = RecordList([{"text": "test"}, {"abc": 123}, {"1": "bananas"}])
    connection = DEFAULT_CONNECTION_NAME
    inputs = {identity_node.id: {connection: input_records}}
    execution, outputs = local_executor.run_flow(
        flow, inputs=inputs, arguments={}, options=FlowExecutionOptions.default_blocking()
    )

    assert execution.state == Execution.State.Completed
    assert len(outputs) == 1, "one node has outputs"
    assert len(outputs[identity_node.id]) == 1, "node has one output key"
    output_records = read_dataset(cast(DatasetVersion, outputs[identity_node.id][connection]))
    assert output_records == inputs[identity_node.id][connection], "outputs match inputs"

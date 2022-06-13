import pytest

from bench.executor import LocalExecutor
from bench.executor.base import FlowExecutionOptions, make_execution_plan
from bench.models import Flow, FlowNode
from bench.models.execution import DEFAULT_CONNECTION_NAME


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
    identity_node: FlowNode = flow.nodes.create(function_id="bench.identity", config_arguments={})

    local_executor.run_flow(flow, inputs={}, arguments={}, options=FlowExecutionOptions.default())

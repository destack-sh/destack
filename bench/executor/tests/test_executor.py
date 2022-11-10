from typing import Union

import pytest

from bench.dataset.accessor import read_dataset, write_dataset
from bench.executor import LocalExecutor
from bench.executor.base import FlowExecutionOptions, make_execution_plan
from bench.model.base import UnbatchedModelHandler, models
from bench.models import Dataset, Flow, Instruction, InstructionEdge, Model, Organization
from bench.models.execution import DEFAULT_CONNECTION_NAME, Execution
from bench.models.model import ModelMetadata
from bench.utils.record import Record, RecordBatch, RecordList


@pytest.fixture()
def local_executor() -> LocalExecutor:
    return LocalExecutor()


@pytest.fixture()
def test_organization() -> Organization:
    return Organization.objects.create(name="test")


@pytest.mark.django_db
def test_plan_empty_flow(test_organization: Organization):
    flow = Flow.objects.create_flow_version_by_name("empty", test_organization)
    plan = make_execution_plan(
        flow, inputs={}, arguments={}, options=FlowExecutionOptions.default()
    )

    assert plan.artifact_inputs == {}
    assert plan.artifact_arguments == {}
    assert plan.node_inputs == {}
    assert plan.node_arguments == {}
    assert plan.final_outputs == {}


@pytest.mark.django_db
def test_plan_identity_flow(test_organization: Organization):
    flow = Flow.objects.create_flow_version_by_name("identity", test_organization)
    identity_node: Instruction = flow.nodes.create(
        function_id="bench.identity", config_arguments={}
    )

    plan = make_execution_plan(
        flow, inputs={}, arguments={}, options=FlowExecutionOptions.default()
    )

    assert plan.artifact_inputs == {}
    assert plan.artifact_arguments == {}
    assert plan.node_inputs == {}
    assert plan.node_arguments == {}
    assert identity_node.id in plan.final_outputs
    assert DEFAULT_CONNECTION_NAME in plan.final_outputs[identity_node.id]


@pytest.mark.django_db
def test_local_execute_empty_flow(local_executor: LocalExecutor, test_organization: Organization):
    flow = Flow.objects.create_flow_version_by_name("identity", organization=test_organization)
    execution, plan = local_executor.run_flow(
        flow, inputs={}, arguments={}, options=FlowExecutionOptions.default_blocking()
    )
    assert execution.status == Execution.Status.Completed
    assert len(plan.final_outputs) == 0, "no outputs"


@pytest.mark.django_db
def test_local_execute_one_node_identity_flow(
    local_executor: LocalExecutor, test_organization: Organization
):
    flow = Flow.objects.create_flow_version_by_name("identity", organization=test_organization)
    identity_node: Instruction = flow.nodes.create(
        function_id="bench.identity", name="identity_1", config_arguments={}
    )

    input_records = RecordList([{"text": "test"}, {"abc": 123}, {"1": "bananas"}])
    inputs = {identity_node.id: {"*": input_records}}
    execution, plan = local_executor.run_flow(
        flow, inputs=inputs, arguments={}, options=FlowExecutionOptions.default_blocking()
    )

    assert execution.status == Execution.Status.Completed
    assert len(plan.final_outputs) == 1, "one node has final outputs"
    assert len(plan.final_outputs[identity_node.id]) == 1, "node has one output key"
    output_records = read_dataset(plan.final_outputs[identity_node.id]["*"].artifact)
    assert output_records == inputs[identity_node.id]["*"], "outputs match inputs"


@pytest.mark.django_db
def test_local_execute_two_node_identity_flow(
    local_executor: LocalExecutor, test_organization: Organization
):
    flow = Flow.objects.create_flow_version_by_name("identity", organization=test_organization)
    identity_node_1: Instruction = flow.nodes.create(
        function_id="bench.identity", name="identity_1", config_arguments={}
    )
    identity_node_2: Instruction = flow.nodes.create(
        function_id="bench.identity", name="identity_2", config_arguments={}
    )
    identity_node_2.depends_on_nodes.add(
        identity_node_1,
        through_defaults=dict(
            flow=flow,
            connection_type=InstructionEdge.ConnectionType.Input,
            connection_name_dependency="*",
            connection_name_dependent="*",
        ),
    )

    input_records = RecordList([{"text": "test"}, {"abc": 123}, {"1": "bananas"}])
    inputs = {identity_node_1.id: {"*": input_records}}
    execution, plan = local_executor.run_flow(
        flow, inputs=inputs, arguments={}, options=FlowExecutionOptions.default_blocking()
    )

    assert execution.status == Execution.Status.Completed
    assert len(plan.final_outputs) == 1, "one node has final outputs"
    assert len(plan.final_outputs[identity_node_2.id]) == 1, "node has one output key"
    output_records = read_dataset(plan.final_outputs[identity_node_2.id]["*"].artifact)
    assert output_records == inputs[identity_node_1.id]["*"], "outputs match inputs"


@pytest.mark.django_db
def test_local_execute_two_node_augmented_flow(
    local_executor: LocalExecutor, test_organization: Organization
):
    flow = Flow.objects.create_flow_version_by_name("augment", organization=test_organization)
    identity_node_1: Instruction = flow.nodes.create(
        function_id="bench.identity", name="identity_1", config_arguments={}
    )
    augment_node_2: Instruction = flow.nodes.create(
        function_id="bench.text.case.upper", name="upper_case_2", config_arguments={}
    )
    augment_node_2.depends_on_nodes.add(
        identity_node_1,
        through_defaults=dict(
            flow=flow,
            connection_type=InstructionEdge.ConnectionType.Input,
        ),
    )

    input_records = RecordList([{"text": "test"}])
    inputs = {identity_node_1.id: {"*": input_records}}
    execution, plan = local_executor.run_flow(
        flow, inputs=inputs, arguments={}, options=FlowExecutionOptions.default_blocking()
    )

    assert execution.status == Execution.Status.Completed
    assert len(plan.final_outputs) == 1, "one node has final outputs"
    assert len(plan.final_outputs[augment_node_2.id]) == 1, "node has one output key"
    output_records = read_dataset(plan.final_outputs[augment_node_2.id]["*"].artifact)
    assert output_records[0] == {"text": "TEST"}, "outputs match augmented inputs"


@pytest.mark.django_db
def test_local_execute_model_flow(local_executor: LocalExecutor, test_organization: Organization):
    flow = Flow.objects.create_flow_version_by_name("model", organization=test_organization)
    model_node_1: Instruction = flow.nodes.create(
        function_id="bench.model", name="model_1", config_arguments={}
    )
    model = Model.objects.create_model_version(
        "spacy_en_core_web_sm",
        test_organization,
        metadata=ModelMetadata(
            handler_id="bench.spacy.bundled", config_arguments={"model_name": "en_core_web_sm"}
        ),
    )

    input_records = RecordList([{"text": "test"}])
    execution, plan = local_executor.run_flow(
        flow,
        inputs={model_node_1.id: {"*": input_records}},
        arguments={model_node_1.id: {"model": model}},
        options=FlowExecutionOptions.default_blocking(),
    )

    assert execution.status == Execution.Status.Completed


@pytest.mark.django_db
def test_local_execute_dataset_flow(local_executor: LocalExecutor, test_organization: Organization):
    flow = Flow.objects.create_flow_version_by_name("dataset", test_organization)
    swap_node_1: Instruction = flow.nodes.create(
        function_id="bench.text.substitute", name="swap_1", config_arguments={}
    )
    dataset = Dataset.objects.create_dataset_version("badword_replacements", test_organization)
    write_dataset(
        dataset,
        [
            {"pattern": "fizz", "replacement": "buzz"},
            {"pattern": "buzz", "replacement": "lightyear"},
        ],
    )

    input_records = RecordList([{"text": "1 2 fizz 4 buzz"}, {"text": "4 buzz 6"}])
    execution, plan = local_executor.run_flow(
        flow,
        inputs={swap_node_1.id: {"*": input_records}},
        arguments={swap_node_1.id: {"swaps_dataset": dataset}},
        options=FlowExecutionOptions.default_blocking(),
    )

    assert execution.status == Execution.Status.Completed
    output_records = read_dataset(plan.final_outputs[swap_node_1.id]["*"].artifact)
    assert output_records == [{"text": "1 2 buzz 4 buzz"}, {"text": "4 lightyear 6"}]


@pytest.mark.django_db
def test_local_execute_test_flow(local_executor: LocalExecutor, test_organization: Organization):
    flow = Flow.objects.create_flow_version_by_name("test", test_organization)
    model_node_1: Instruction = flow.nodes.create(
        function_id="bench.model", name="model_1", config_arguments={}
    )
    metric_node_2: Instruction = flow.nodes.create(
        function_id="bench.metric.accuracy",
        name="metric_accuracy_2",
        config_arguments={"prediction_key": "score", "reference_key": "score"},
    )
    test_node_3: Instruction = flow.nodes.create(
        function_id="bench.test.compare_constant",
        name="test_3",
        config_arguments={"operator": "Gte", "value": 0.5, "key": "accuracy"},
    )
    metric_node_2.depends_on_nodes.add(
        model_node_1,
        through_defaults=dict(
            flow=flow,
            connection_type=InstructionEdge.ConnectionType.Input,
            connection_name_dependent="predictions",
            connection_name_dependency="*",
        ),
    )
    test_node_3.depends_on_nodes.add(
        metric_node_2,
        through_defaults=dict(
            flow=flow,
            connection_type=InstructionEdge.ConnectionType.Input,
            connection_name_dependent="*",
            connection_name_dependency="*",
        ),
    )

    @models.register("test.stub")
    class StubModel(UnbatchedModelHandler):
        config_static_keys: set[str] = set()

        def run(self, record: Record) -> Union[Record, RecordBatch]:
            return {**record, "score": 1}

    model = Model.objects.create_model_version(
        "stub_model", test_organization, metadata=ModelMetadata(handler_id="test.stub")
    )
    execution, plan = local_executor.run_flow(
        flow,
        inputs={
            model_node_1.id: {"*": RecordList([{"text": "test"}, {"text": "test"}])},
            metric_node_2.id: {"references": RecordList([{"score": 0}, {"score": 1}])},
        },
        arguments={model_node_1.id: {"model": model}},
        options=FlowExecutionOptions.default_blocking(),
    )
    models._unregister("test.stub")

    assert execution.status == Execution.Status.Completed
    output_records = read_dataset(plan.final_outputs[test_node_3.id]["*"].artifact)
    assert output_records == [{"result": True}]

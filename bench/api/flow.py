from itertools import chain
from typing import Mapping
from uuid import UUID

from rest_framework import serializers, validators, viewsets
from rest_framework.decorators import action
from rest_framework.request import Request
from rest_framework.response import Response

from bench.api.execution import ExecutionSerializer
from bench.executor import executor
from bench.executor.base import FlowRawArgument, FlowRawInput
from bench.models import ArtifactVersion, Flow, FlowNode
from bench.models.flow import FlowVersion
from bench.models.utils import MAX_NAME_LENGTH


class FlowSerializer(serializers.ModelSerializer):
    name = serializers.CharField(
        max_length=MAX_NAME_LENGTH,
        validators=[
            validators.UniqueValidator(
                queryset=Flow.objects.all(),
                message="There is already a flow with the given name",
            )
        ],
    )

    class Meta:
        model = Flow
        fields = ["id", "name", "description", "created_at"]
        read_only_fields = ["id", "created_at"]


class FlowNodeInputDataSerializer(serializers.Serializer):
    name = serializers.CharField()
    # plain records or existing artifact (version) reference
    records = serializers.JSONField(required=False)
    artifact = serializers.PrimaryKeyRelatedField(
        queryset=ArtifactVersion.objects.all(), required=False
    )

    def validate(self, data):
        data_keys = ["records", "artifact"]
        num_specified = sum(1 if key in data else 0 for key in data_keys)
        if num_specified != 1:
            raise serializers.ValidationError(f"must specify exactly one of data keys: {data_keys}")


class FlowNodeInputsSerializer(serializers.Serializer):
    node = serializers.PrimaryKeyRelatedField(queryset=FlowNode.objects.all())
    inputs = FlowNodeInputDataSerializer(many=True)


class FlowNodeArgumentDataSerializer(serializers.Serializer):
    name = serializers.CharField()
    # plain records or existing artifact (version) reference
    other_node = serializers.PrimaryKeyRelatedField(queryset=FlowNode.objects.all(), required=False)
    records = serializers.JSONField(required=False)
    artifact = serializers.PrimaryKeyRelatedField(
        queryset=ArtifactVersion.objects.all(), required=False
    )

    def validate(self, data):
        data_keys = ["other_node", "records", "artifact"]
        num_specified = sum(1 if key in data else 0 for key in data_keys)
        if num_specified != 1:
            raise serializers.ValidationError(f"must specify exactly one of data keys: {data_keys}")


class FlowNodeArgumentsSerializer(serializers.Serializer):
    node = serializers.PrimaryKeyRelatedField(queryset=FlowNode.objects.all())
    arguments = FlowNodeArgumentDataSerializer(many=True)


class FlowExecutionPlanSerializer(serializers.Serializer):
    flow = serializers.PrimaryKeyRelatedField(queryset=FlowVersion.objects.all())
    inputs = FlowNodeInputsSerializer(many=True)
    arguments = FlowNodeArgumentsSerializer(many=True)

    def validate(self, data):
        bad_arguments = filter(
            lambda argument: argument.node.flow_id != data["flow"].id,
            chain(data["inputs"], data["arguments"]),
        )
        if bad_arguments:
            raise serializers.ValidationError("argument flow nodes refer to different flow")


class FlowViewSet(viewsets.ModelViewSet):
    @action(methods=["POST"], detail=True)
    def execute(self, request: Request, *args, **kwargs) -> Response:
        serializer = FlowExecutionPlanSerializer(request.data)
        serializer.is_valid(raise_exception=True)

        # assemble inputs into dict form
        inputs: dict[UUID, Mapping[str, FlowRawInput]] = {}
        for node_input in serializer.validated_data["inputs"]:
            node_inputs: dict[str, FlowRawInput] = {}
            for input_data in node_input["inputs"]:
                # use first non-null data given
                node_inputs[input_data["name"]] = input_data.get(
                    "records", input_data.get("artifact")
                )
            inputs[node_input["node"].id] = node_inputs

        # assemble arguments into dict form
        arguments: dict[UUID, Mapping[str, FlowRawArgument]] = {}
        for node_argument in serializer.validated_data["arguments"]:
            node_arguments: dict[str, FlowRawArgument] = {}
            for argument_data in node_argument["arguments"]:
                # use first non-null data given
                node_arguments[argument_data["name"]] = argument_data.get(
                    "other_node", argument_data.get("records", argument_data.get("artifact"))
                )
            arguments[node_argument["node"].id] = node_arguments

        flow: FlowVersion = serializer.validated_data["flow"]
        execution, outputs = executor.run_flow(flow, inputs, arguments)
        serialized_execution = ExecutionSerializer(execution).data
        serialized_outputs: Mapping[UUID, Mapping[str, UUID]] = {
            node_id: {name: artifact.id for name, artifact in artifacts.items()}
            for node_id, artifacts in outputs.items()
        }
        return Response({"execution": serialized_execution, "outputs": serialized_outputs})

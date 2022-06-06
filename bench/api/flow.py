from itertools import chain

from rest_framework import serializers, validators, viewsets
from rest_framework.decorators import action
from rest_framework.request import Request
from rest_framework.response import Response

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
            raise serializers.ValidationError(f"argument flow nodes refer to different flow")


class FlowViewSet(viewsets.ModelViewSet):
    @action(methods=["POST"], detail=True)
    def execute(self, request: Request, *args, **kwargs) -> Response:
        pass

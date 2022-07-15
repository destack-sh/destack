from rest_framework import serializers
from rest_framework.decorators import action
from rest_framework.request import Request
from rest_framework.response import Response

from bench.api.artifact import (
    ArtifactSerializer,
    ArtifactVersionSerializer,
    ArtifactVersionViewSet,
    ArtifactViewSet,
)
from bench.api.execution import ExecutionSerializer
from bench.executor import executor
from bench.models import Model, ModelVersion
from bench.models.utils import MODEL_TYPE


class ModelSerializer(ArtifactSerializer):
    type = serializers.CharField(max_length=64, default=MODEL_TYPE)

    class Meta(ArtifactSerializer.Meta):
        model = Model


class ModelVersionSerializer(ArtifactVersionSerializer):
    artifact: serializers.SlugRelatedField = serializers.SlugRelatedField(
        queryset=Model.objects.all(), slug_field="name"
    )
    parents: serializers.SlugRelatedField = serializers.SlugRelatedField(
        queryset=ModelVersion.objects.all(), slug_field="version", many=True
    )

    class Meta(ArtifactVersionSerializer.Meta):
        model = ModelVersion
        # fields/read_only_fields same as super


class ModelViewSet(ArtifactViewSet):
    queryset = Model.objects.all()
    serializer_class = ModelSerializer


class ModelVersionViewSet(ArtifactVersionViewSet):
    queryset = ModelVersion.objects.filter(artifact__type=MODEL_TYPE).all()
    serializer_class = ModelVersionSerializer

    @action(methods=["POST"], detail=True)
    def predict(self, request: Request, *args, **kwargs) -> Response:
        model = self.get_object()
        input_record = {"text": request.data.get("text")}
        execution, prediction = executor.run_model(
            model,
            record=input_record,
            blocking=True,
            load_if_needed=True,
        )
        serialized_execution = ExecutionSerializer(execution).data
        return Response({"execution": serialized_execution, "result": prediction})

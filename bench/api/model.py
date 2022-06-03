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
from bench.executor import executor
from bench.models import Model, ModelVersion
from bench.models.utils import MODEL_TYPE


class ModelSerializer(ArtifactSerializer):
    type = serializers.CharField(max_length=64, default=MODEL_TYPE)

    class Meta(ArtifactSerializer.Meta):
        model = Model


class ModelVersionSerializer(ArtifactVersionSerializer):
    artifact = serializers.SlugRelatedField(queryset=Model.objects.all(), slug_field="name")
    parents = serializers.SlugRelatedField(
        queryset=ModelVersion.objects.all(), slug_field="version", many=True
    )

    class Meta(ArtifactVersionSerializer.Meta):
        model = ModelVersion


class ModelViewSet(ArtifactViewSet):
    queryset = Model.objects.all()
    serializer_class = ModelSerializer


class ModelVersionViewSet(ArtifactVersionViewSet):
    queryset = ModelVersion.objects.filter(artifact__type=MODEL_TYPE).all()
    serializer_class = ModelVersionSerializer

    @action(methods=["POST"], detail=True)
    def predict(self, request: Request, artifact_name: str, version: str) -> Response:
        model = self.get_object()
        execution, prediction = executor.run_model(
            model,
            record={"text": request.GET["text"]},
            blocking=True,
            load_if_needed=True,
        )
        return Response(prediction)

from django.core.validators import RegexValidator
from rest_framework import serializers, validators, viewsets
from rest_framework.decorators import action
from rest_framework.request import Request
from rest_framework.response import Response

from bench.api.execution import ExecutionSerializer
from bench.executor import executor
from bench.models import Model, ModelVersion, Organization
from bench.models.utils import MAX_NAME_LENGTH
from bench.models.versioning import get_head
from bench.utils.func import terrible_cast


class ModelSerializer(serializers.ModelSerializer):
    name = serializers.CharField(
        max_length=MAX_NAME_LENGTH,
        validators=[
            validators.UniqueValidator(
                queryset=Model.objects.all(),
                message="There is already an artifact with the given name in this organization",
            ),
            RegexValidator(
                regex=r"^[\w.\-]+$", message="Artifact names must follow pattern [\\w.\\-]+"
            ),
        ],
    )
    organization: serializers.SlugRelatedField = serializers.SlugRelatedField(
        slug_field="slug", queryset=Organization.objects.all()
    )

    class Meta:
        model = Model
        fields = [
            "id",
            "type",
            "created_at",
            "organization",
            "name",
            "versions",
            "description",
            "head",
            "tags",
        ]
        read_only_fields = ["id", "created_at", "head", "versions"]


class ModelViewSet(viewsets.ModelViewSet):
    queryset = Model.objects.all()
    serializer_class = ModelSerializer

    @action(methods=["POST"], detail=True)
    def predict(self, request: Request, *args, **kwargs):
        model = terrible_cast(ModelVersion, get_head(self.get_object()))
        return _predict_response(model, request)


def _predict_response(model: ModelVersion, request: Request):
    input_record = {"text": request.data.get("text")}
    execution, prediction = executor.run_model(
        model,
        record=input_record,
        blocking=True,
        load_if_needed=True,
    )
    serialized_execution = ExecutionSerializer(execution).data
    return Response(serialized_execution)

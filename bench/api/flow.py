from rest_framework import serializers, validators, viewsets
from rest_framework.decorators import action
from rest_framework.request import Request
from rest_framework.response import Response

from bench.models import Flow
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


class FlowViewSet(viewsets.ModelViewSet):
    @action()
    def run(self, request: Request, *args, **kwargs) -> Response:
        pass

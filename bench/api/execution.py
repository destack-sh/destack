from django_filters.rest_framework import DjangoFilterBackend
from rest_framework import serializers, viewsets
from rest_framework.pagination import LimitOffsetPagination

from bench.api.utils import ArtifactVersionListingField, FlowVersionListingField
from bench.models import Execution
from bench.models.execution import ExecutionArtifactConnection


class ExecutionArtifactConnectionSerializer(serializers.ModelSerializer):
    artifact = ArtifactVersionListingField(read_only=True)

    class Meta:
        model = ExecutionArtifactConnection
        fields = ["connection_type", "connection_name", "artifact"]


class ExecutionSerializer(serializers.ModelSerializer):
    flow = FlowVersionListingField(read_only=True)
    flow_node: serializers.PrimaryKeyRelatedField = serializers.PrimaryKeyRelatedField(
        read_only=True
    )
    model = ArtifactVersionListingField(read_only=True)
    connected_artifacts = ExecutionArtifactConnectionSerializer(many=True)

    class Meta:
        model = Execution
        fields = "__all__"
        read_only_fields = [
            "type",
            "created_at",
            "updated_at",
            "started_at",
            "terminated_at",
            "state",
            "metadata",
            "parent",
            "flow",
            "flow_node",
            "model",
            "connected_artifacts",
        ]
        depth = 1


class ExecutionViewSet(viewsets.ReadOnlyModelViewSet):
    queryset = Execution.objects.order_by("-created_at").all()
    serializer_class = ExecutionSerializer
    filter_backends = [DjangoFilterBackend]
    pagination_class = LimitOffsetPagination
    filterset_fields = [
        "type",
        "flow",
        "flow_node",
        "model",
        "created_at",
        "updated_at",
        "started_at",
        "terminated_at",
        "state",
        "parent",
    ]

from django_filters.rest_framework import DjangoFilterBackend
from rest_framework import serializers, viewsets

from bench.models import Execution
from bench.models.execution import ExecutionArtifactConnection


class ExecutionArtifactConnectionSerializer(serializers.ModelSerializer):
    artifact = serializers.PrimaryKeyRelatedField(read_only=True)

    class Meta:
        model = ExecutionArtifactConnection
        fields = ["connection_type", "connection_name", "artifact"]


class ExecutionSerializer(serializers.ModelSerializer):
    flow = serializers.PrimaryKeyRelatedField(read_only=True)
    flow_node = serializers.PrimaryKeyRelatedField(read_only=True)
    model = serializers.PrimaryKeyRelatedField(read_only=True)
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

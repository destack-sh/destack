from django_filters.rest_framework import DjangoFilterBackend
from rest_framework import serializers, viewsets
from rest_framework.pagination import LimitOffsetPagination

from bench.api.utils import ArtifactVersionListingField, FlowVersionListingField, terrible_cast
from bench.dataset.accessor import get_dataset_version_reader
from bench.models import DatasetVersion, Execution
from bench.models.execution import ExecutionArtifactConnection


class ExecutionArtifactConnectionSerializer(serializers.ModelSerializer):
    artifact = ArtifactVersionListingField(read_only=True)
    dataset_preview = serializers.SerializerMethodField(read_only=True)

    def get_dataset_preview(self, obj: ExecutionArtifactConnection):
        if obj.artifact.artifact.type != "dataset":
            return None

        dataset = terrible_cast(DatasetVersion, obj.artifact)

        # TODO @Performance: configure execution connection records preview in api
        offset = 0
        limit = 3
        reader = get_dataset_version_reader(dataset)
        records = list(reader[offset : (offset + limit)])
        return {
            "limit": limit,
            "count": len(reader),
            "offset": offset,
            "results": records,
        }

    class Meta:
        model = ExecutionArtifactConnection
        fields = ["connection_type", "connection_name", "artifact", "dataset_preview"]


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
    pagination_class = LimitOffsetPagination
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

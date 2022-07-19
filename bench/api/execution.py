from django_filters import rest_framework as filters
from django_filters.rest_framework import DjangoFilterBackend
from rest_framework import serializers, viewsets
from rest_framework.pagination import LimitOffsetPagination

from bench.api.artifact import ArtifactViewSerializer
from bench.api.utils import ArtifactVersionListingField, FlowVersionListingField
from bench.dataset.accessor import get_dataset_version_reader
from bench.models import DatasetVersion, Execution
from bench.models.dataset import DatasetViewData
from bench.models.execution import ExecutionArtifactConnection
from bench.utils.func import terrible_cast


class ExecutionArtifactConnectionSerializer(serializers.ModelSerializer):
    artifact = ArtifactVersionListingField(read_only=True)
    dataset_preview = serializers.SerializerMethodField(read_only=True)
    view = ArtifactViewSerializer()

    def get_dataset_preview(self, obj: ExecutionArtifactConnection):
        if obj.artifact.artifact.type != "dataset":
            return None

        dataset = terrible_cast(DatasetVersion, obj.artifact)
        view = DatasetViewData(**(obj.view_inline or {}))

        # TODO @Performance: enable configuring execution connection records preview via api
        offset = 0
        limit = 3
        reader = get_dataset_version_reader(dataset)
        start = view.apply(offset)
        end = view.apply(offset + limit)
        records = list(reader[start:end])
        return {
            "limit": limit,
            "count": len(reader),
            "offset": offset,
            "results": records,
        }

    class Meta:
        model = ExecutionArtifactConnection
        fields = [
            "connection_type",
            "connection_name",
            "artifact",
            "dataset_preview",
            "view",
            "view_inline",
        ]
        read_only_fields = fields


class ExecutionSerializer(serializers.ModelSerializer):
    flow = FlowVersionListingField(read_only=True)
    flow_node: serializers.PrimaryKeyRelatedField = serializers.PrimaryKeyRelatedField(
        read_only=True
    )
    model = ArtifactVersionListingField(read_only=True)
    children = serializers.SerializerMethodField(read_only=True)
    connected_artifacts = ExecutionArtifactConnectionSerializer(many=True)

    def get_children(self, data: Execution):
        # specify children as method field because we need ExecutionSerializer (recursive)
        return ExecutionSerializer(data.children, many=True).data  # type: ignore

    class Meta:
        model = Execution
        fields = [
            "id",
            "type",
            "created_at",
            "updated_at",
            "started_at",
            "terminated_at",
            "state",
            "metadata",
            "parent",
            "children",
            "flow",
            "flow_node",
            "model",
            "connected_artifacts",
        ]
        read_only_fields = fields
        depth = 1


class UUIDArrayFilter(filters.UUIDFilter):
    def filter(self, qs, value):
        if value is not None and not isinstance(value, list):
            value = [value]
        return super().filter(qs, value)


class ExecutionFilter(filters.FilterSet):
    id__in = UUIDArrayFilter(field_name="id", lookup_expr="in")
    updated_at__gt = filters.DateTimeFilter(field_name="updated_at", lookup_expr="gt")

    class Meta:
        model = Execution
        fields = ["id", "type", "flow", "flow_node", "model", "state", "parent"]


class ExecutionViewSet(viewsets.ReadOnlyModelViewSet):
    queryset = Execution.objects.order_by("-created_at").all()
    serializer_class = ExecutionSerializer
    pagination_class = LimitOffsetPagination
    filter_backends = [DjangoFilterBackend]
    filterset_class = ExecutionFilter

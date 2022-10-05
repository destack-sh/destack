from django_filters import rest_framework as filters
from django_filters.rest_framework import DjangoFilterBackend
from rest_framework import serializers, viewsets
from rest_framework.pagination import LimitOffsetPagination

from bench.api.artifact import ArtifactViewSerializer
from bench.api.dataset import DatasetRecordSerializer
from bench.api.utils import ArtifactVersionListingField, FlowVersionListingField
from bench.dataset.accessor import DatasetAccessor
from bench.models import DatasetVersion, Execution, Organization, Project
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
        accessor = DatasetAccessor(dataset)
        start = view.apply(offset)
        end = view.apply(offset + limit)
        records = list(accessor.get_records_slice(start, end))
        count = view.apply(len(accessor)) - view.apply(0)
        return {
            "limit": limit,
            "count": count,
            "offset": offset,
            "results": DatasetRecordSerializer(records, many=True).data,
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
    organization: serializers.SlugRelatedField = serializers.SlugRelatedField(
        slug_field="slug", queryset=Organization.objects.all()
    )
    project: serializers.SlugRelatedField = serializers.SlugRelatedField(
        slug_field="slug", allow_empty=True, queryset=Project.objects.all()
    )

    def get_children(self, data: Execution):
        # specify children as method field because we need ExecutionSerializer (recursive)
        return ExecutionSerializer(data.children, many=True).data

    class Meta:
        model = Execution
        fields = [
            "id",
            "type",
            "created_at",
            "updated_at",
            "started_at",
            "terminated_at",
            "status",
            "metadata",
            "parent",
            "children",
            "flow",
            "flow_node",
            "model",
            "connected_artifacts",
            "organization",
            "project",
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
        fields = ["id", "type", "flow", "flow_node", "model", "status", "parent"]


class ExecutionViewSet(viewsets.ReadOnlyModelViewSet):
    queryset = Execution.objects.order_by("-created_at").all()
    serializer_class = ExecutionSerializer
    pagination_class = LimitOffsetPagination
    filter_backends = [DjangoFilterBackend]
    filterset_class = ExecutionFilter

    def get_queryset(self):
        queryset = super().get_queryset()
        queryset = queryset.filter(organization__slug=self.kwargs.get("organization"))
        return queryset

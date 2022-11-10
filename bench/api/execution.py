from django_filters import rest_framework as filters
from django_filters.rest_framework import DjangoFilterBackend
from rest_framework import serializers, viewsets
from rest_framework.pagination import LimitOffsetPagination

from bench.models import Execution, Organization, Project


class ExecutionSerializer(serializers.ModelSerializer):
    flow = serializers.PrimaryKeyRelatedField(read_only=True)
    instruction: serializers.PrimaryKeyRelatedField = serializers.PrimaryKeyRelatedField(
        read_only=True
    )
    model = serializers.PrimaryKeyRelatedField(read_only=True)
    children = serializers.SerializerMethodField(read_only=True)
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
            "instruction",
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
        fields = ["id", "type", "flow", "instruction", "model", "status", "parent"]


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

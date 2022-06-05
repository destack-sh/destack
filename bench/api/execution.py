from rest_framework import serializers, viewsets

from bench.models import Execution


class ExecutionSerializer(serializers.ModelSerializer):
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
        ]
        depth = 1


class ExecutionViewSet(viewsets.ReadOnlyModelViewSet):
    queryset = Execution.objects.order_by("-created_at").all()
    serializer_class = ExecutionSerializer

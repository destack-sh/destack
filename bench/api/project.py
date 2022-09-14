from rest_framework import serializers, viewsets

from bench.models import Organization, Project


class ProjectSerializer(serializers.ModelSerializer):
    organization: serializers.SlugRelatedField = serializers.SlugRelatedField(
        slug_field="slug", queryset=Organization.objects.all()
    )
    artifacts: serializers.SlugRelatedField = serializers.SlugRelatedField(
        slug_field="name", read_only=True
    )

    class Meta:
        fields = [
            "id",
            "name",
            "description",
            "created_at",
            "updated_at",
            "organization",
            "artifacts",
        ]
        read_only_fields = ["id", "created_at", "updated_at"]
        model = Project


class ProjectViewSet(viewsets.ModelViewSet):
    queryset = Project.objects.all()
    serializer_class = ProjectSerializer
    lookup_field = "id"

    def get_queryset(self):
        queryset = super().get_queryset()
        queryset = queryset.filter(organization__slug=self.kwargs.get("organization"))
        return queryset

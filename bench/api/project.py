from rest_framework import serializers, viewsets

from bench.models import Organization, Project


class ProjectSerializer(serializers.ModelSerializer):
    organization = serializers.SlugRelatedField(
        slug_field="slug", queryset=Organization.objects.all()
    )

    class Meta:
        model = Project


class ProjectViewSet(viewsets.ModelViewSet):
    queryset = Project.objects.all()
    serializer_class = ProjectSerializer
    lookup_field = "id"

    def get_queryset(self):
        queryset = super().get_queryset()
        queryset = queryset.filter(organization__slug=self.kwargs.get("organization"))
        return queryset

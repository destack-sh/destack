from rest_framework import serializers, viewsets

from bench.models.team import Team


class TeamSerializer(serializers.ModelSerializer):
    organization: serializers.SlugRelatedField = serializers.SlugRelatedField(
        slug_field="slug", read_only=True
    )

    class Meta:
        fields = ["id", "name", "created_at", "updated_at", "organization"]
        read_only_fields = ["id", "created_at", "updated_at"]
        model = Team


class TeamViewSet(viewsets.ModelViewSet):
    queryset = Team.objects.all()
    serializer_class = TeamSerializer
    lookup_field = "id"

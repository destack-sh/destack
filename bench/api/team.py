from rest_framework import serializers, viewsets

from bench.models.team import Team


class TeamSerializer(serializers.ModelSerializer):
    class Meta:
        model = Team


class TeamViewSet(viewsets.ModelViewSet):
    queryset = Team.objects.all()
    serializer_class = TeamSerializer
    lookup_field = "id"

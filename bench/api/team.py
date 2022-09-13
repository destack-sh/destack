from rest_framework import serializers, viewsets

from bench.models.team import Team


class TeamSerializer(serializers.ModelSerializer):
    class Meta:
        model = Team


class TeamViewSet(viewsets.ModelViewSet):
    serializer_class = TeamSerializer

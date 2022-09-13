from rest_framework import serializers, viewsets

from bench.models.organization import Organization


class OrganizationSerializer(serializers.ModelSerializer):
    class Meta:
        model = Organization


class OrganizationViewSet(viewsets.ModelViewSet):
    serializer_class = OrganizationSerializer

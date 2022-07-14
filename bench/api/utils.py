from rest_framework import serializers

from bench.models import ArtifactVersion
from bench.models.flow import FlowVersion


class FlowVersionListingField(serializers.RelatedField):
    def to_representation(self, value: FlowVersion):
        return f"{value.flow.name}@{value.version}"


class ArtifactVersionListingField(serializers.RelatedField):
    def to_representation(self, value: ArtifactVersion):
        return f"{value.artifact.name}@{value.version}"

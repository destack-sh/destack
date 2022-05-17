from rest_framework import serializers, validators, viewsets

from bench.models import Artifact, Dataset, DatasetVersion, Record
from bench.models.dataset import DatasetMetadata
from bench.models.utils import DATASET_TYPE, MAX_NAME_LENGTH


class DatasetSerializer(serializers.ModelSerializer):
    name = serializers.CharField(
        max_length=MAX_NAME_LENGTH,
        validators=[
            validators.UniqueValidator(
                queryset=Artifact.objects.all(),
                message="There is already an artifact with the given name",
            )
        ],
    )

    class Meta:
        model = Dataset
        fields = ["id", "type", "created_at", "name", "description"]
        read_only_fields = ["id", "type", "created_at"]


class DatasetVersionSerializer(serializers.HyperlinkedModelSerializer):
    class Meta:
        model = DatasetVersion
        fields = [
            "id",
            "parents",
            "version",
            "artifact",
            "storage_uri",
            "metadata",
            "content_hash",
            "immutable",
        ]
        read_only_fields = ["id", "parents", "version", "content_hash", "immutable"]

    def validate_metadata(self, value: dict):
        try:
            DatasetMetadata.from_dict(value)
        except ValueError as e:
            raise serializers.ValidationError(f"metadata is invalid: {e}")


class RecordSerializer(serializers.ModelSerializer):
    class Meta:
        model = Record
        fields = ["data", "metadata"]
        read_only_fields = ["id", "content_hash"]


class DatasetViewSet(viewsets.ModelViewSet):
    queryset = Dataset.datasets.all()
    serializer_class = DatasetSerializer


class DatasetVersionViewSet(viewsets.ModelViewSet):
    queryset = DatasetVersion.objects.filter(artifact__type=DATASET_TYPE).all()
    serializer_class = DatasetVersionSerializer


class RecordViewSet(viewsets.ModelViewSet):
    queryset = Record.objects.all()
    serializer_class = RecordSerializer

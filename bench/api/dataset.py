from rest_framework import serializers

from bench.models import Dataset, DatasetVersion, Record


class DatasetSerializer(serializers.ModelSerializer):
    class Meta:
        model = Dataset
        fields = ["name", "description"]
        read_only_fields = ["id", "type", "created_at"]


class DatasetVersionSerializer(serializers.HyperlinkedModelSerializer):
    class Meta:
        model = DatasetVersion
        fields = [
            "artifact",
            "version",
            "parents",
            "storage_uri",
            "metadata",
        ]
        read_only_fields = ["id"]


class RecordSerializer(serializers.ModelSerializer):
    class Meta:
        model = Record
        fields = ["data", "metadata"]
        read_only_fields = ["id"]

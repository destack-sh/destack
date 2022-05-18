import json

from django.shortcuts import get_object_or_404
from rest_framework import serializers, status, validators, viewsets
from rest_framework.pagination import LimitOffsetPagination
from rest_framework.request import Request
from rest_framework.response import Response
from rest_framework_extensions.mixins import NestedViewSetMixin

from bench.models import Artifact, Dataset, DatasetVersion, Record
from bench.models.dataset import DatasetMetadata
from bench.models.utils import DATASET_TYPE, MAX_NAME_LENGTH
from dataset.accessor import get_dataset_version_reader, get_dataset_version_writer


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
    type = serializers.CharField(max_length=64, default=DATASET_TYPE)

    class Meta:
        model = Dataset
        fields = ["id", "type", "created_at", "name", "description"]
        read_only_fields = ["id", "type", "created_at"]


class DatasetVersionSerializer(serializers.HyperlinkedModelSerializer):
    artifact = serializers.SlugRelatedField(queryset=Dataset.objects.all(), slug_field="name")

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

    def validate_metadata(self, value: str):
        # guaranteed to be valid JSON because metadata is a JSONField
        value = json.loads(value)
        try:
            DatasetMetadata.from_dict(value)
        except (ValueError, KeyError) as e:
            raise serializers.ValidationError(f"metadata is invalid: {e}")
        return value


class RecordSerializer(serializers.ModelSerializer):
    class Meta:
        model = Record
        fields = ["id", "content_hash", "data", "metadata"]
        read_only_fields = ["id", "content_hash"]


class DatasetViewSet(viewsets.ModelViewSet):
    queryset = Dataset.datasets.all()
    serializer_class = DatasetSerializer
    lookup_field = "name"


class DatasetVersionViewSet(NestedViewSetMixin, viewsets.ModelViewSet):
    queryset = DatasetVersion.objects.filter(artifact__type=DATASET_TYPE).all()
    serializer_class = DatasetVersionSerializer
    lookup_field = "version"


class RecordPagination(LimitOffsetPagination):
    default_limit = 100
    max_limit = 1000


class RecordViewSet(viewsets.GenericViewSet):
    lookup_field = "index"
    serializer_class = RecordSerializer
    pagination_class = RecordPagination

    def create(
        self, request: Request, parent_lookup_artifact_name: str, parent_lookup_version: str
    ) -> Response:
        dataset_version = get_object_or_404(
            DatasetVersion,
            artifact__name=parent_lookup_artifact_name,
            version=parent_lookup_version,
        )
        writer = get_dataset_version_writer(dataset_version)

        serializer: serializers.BaseSerializer = self.get_serializer(data=request.data)
        serializer.is_valid(raise_exception=True)
        record = serializer.save()
        # TODO @Feature: use DatasetAccessor or similar to support reading/writing record metadata
        #  This includes RecordViewSet.create/list and any other record access.
        writer.append(record.data)

        return Response(serializer.data, status=status.HTTP_201_CREATED)

    def list(
        self, request: Request, parent_lookup_artifact_name: str, parent_lookup_version: str
    ) -> Response:
        dataset_version = get_object_or_404(
            DatasetVersion,
            artifact__name=parent_lookup_artifact_name,
            version=parent_lookup_version,
        )
        reader = get_dataset_version_reader(dataset_version)

        limit = request.data.get("limit", self.pagination_class.default_limit)
        limit = min(limit, self.pagination_class.max_limit)
        offset = request.data.get("limit", 0)
        records = list(reader[offset : offset + limit])
        return Response({"limit": limit, "offset": offset, "results": records})

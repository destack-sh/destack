import json
from typing import cast

from django.db import models
from django.http import Http404
from django.shortcuts import get_object_or_404
from rest_framework import serializers, status, validators, viewsets
from rest_framework.pagination import LimitOffsetPagination
from rest_framework.request import Request
from rest_framework.response import Response

from bench.dataset.accessor import get_dataset_version_handler
from bench.dataset.base import DatasetHandler, DatasetReader, DatasetWriter
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


class DatasetVersionViewSet(viewsets.ModelViewSet):
    queryset = DatasetVersion.objects.filter(artifact__type=DATASET_TYPE).all()
    serializer_class = DatasetVersionSerializer
    lookup_field = "version"

    def get_queryset(self) -> models.QuerySet[DatasetVersion]:
        return self.queryset.filter(artifact__name=self.kwargs.get("artifact_name"))

    def destroy(self, request: Request, *args, **kwargs) -> Response:
        instance: DatasetVersion = self.get_object()
        writer = _to_dataset_writer(_get_dataset_handler_by_instance(instance))
        writer.clear()
        self.perform_destroy(instance)
        return Response(status=status.HTTP_204_NO_CONTENT)


class RecordPagination(LimitOffsetPagination):
    default_limit = 100
    max_limit = 1000


class RecordViewSet(viewsets.GenericViewSet):
    lookup_field = "index"
    serializer_class = RecordSerializer
    pagination_class = RecordPagination

    def create(self, request: Request, artifact_name: str, version_version: str) -> Response:
        writer = _get_dataset_writer(artifact_name, version_version)
        serializer: serializers.BaseSerializer = self.get_serializer(data=request.data)
        serializer.is_valid(raise_exception=True)
        record = serializer.save()
        # TODO @Feature: use DatasetAccessor or similar to support reading/writing record metadata
        #  This includes RecordViewSet.create/list and any other record access.
        writer.append(record.data)

        return Response(serializer.data, status=status.HTTP_201_CREATED)

    def list(self, request: Request, artifact_name: str, version_version: str) -> Response:
        reader = _get_dataset_reader(artifact_name, version_version)
        limit = cast(RecordPagination, self.paginator).get_limit(request)
        offset = cast(RecordPagination, self.paginator).get_offset(request)
        records = list(reader[offset : offset + limit])
        return Response({"limit": limit, "offset": offset, "results": records})

    def retrieve(
        self, request: Request, index: str, artifact_name: str, version_version: str
    ) -> Response:
        index = int(index)
        reader = _get_dataset_reader(artifact_name, version_version)
        try:
            record = reader[index]
        except LookupError:
            raise Http404("No record matches the given query.")
        return Response(record)


def _get_dataset_reader(artifact_name: str, version: str) -> DatasetReader:
    dataset_version = get_object_or_404(
        DatasetVersion,
        artifact__name=artifact_name,
        version=version,
    )
    handler = _get_dataset_handler_by_instance(dataset_version)
    return _to_dataset_reader(handler)


def _to_dataset_reader(handler: DatasetHandler) -> DatasetReader:
    if not isinstance(handler, DatasetReader):
        raise ValueError("dataset version is not readable")
    return cast(DatasetReader, handler)


def _get_dataset_writer(artifact_name: str, version: str) -> DatasetWriter:
    dataset_version = get_object_or_404(
        DatasetVersion,
        artifact__name=artifact_name,
        version=version,
    )
    handler = _get_dataset_handler_by_instance(dataset_version)
    return _to_dataset_writer(handler)


def _to_dataset_writer(handler: DatasetHandler) -> DatasetWriter:
    if not isinstance(handler, DatasetWriter):
        raise ValueError("dataset version is not writable")
    return cast(DatasetWriter, handler)


def _get_dataset_handler_by_instance(version: DatasetVersion) -> DatasetHandler:
    return get_dataset_version_handler(version)

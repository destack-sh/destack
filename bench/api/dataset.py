import json
from typing import Union, cast

import structlog
from django.db import models
from django.http import Http404
from django.shortcuts import get_object_or_404
from rest_framework import serializers, status, viewsets
from rest_framework.decorators import action
from rest_framework.pagination import LimitOffsetPagination, _positive_int
from rest_framework.request import Request
from rest_framework.response import Response

from bench.api.artifact import (
    ArtifactSerializer,
    ArtifactVersionSerializer,
    ArtifactVersionViewSet,
    ArtifactViewSet,
)
from bench.artifact.base import ArtifactVersionHandler
from bench.dataset.accessor import get_dataset_version_handler
from bench.dataset.base import DatasetHandler, DatasetReader, DatasetWriter
from bench.models import Dataset, DatasetVersion, Record
from bench.models.dataset import DatasetMetadata
from bench.models.utils import DATASET_TYPE, proxies
from tasks.synchronize import copy_dataset_version

logger = structlog.stdlib.get_logger()


class DatasetSerializer(ArtifactSerializer):
    type = serializers.CharField(max_length=64, default=DATASET_TYPE)

    class Meta(ArtifactSerializer.Meta):
        model = Dataset


class DatasetVersionSerializer(ArtifactVersionSerializer):
    artifact: serializers.SlugRelatedField = serializers.SlugRelatedField(
        queryset=Dataset.objects.all(), slug_field="name"
    )
    parents: serializers.SlugRelatedField = serializers.SlugRelatedField(
        queryset=DatasetVersion.objects.all(), slug_field="version", many=True
    )

    class Meta(ArtifactVersionSerializer.Meta):
        model = DatasetVersion
        # fields/read_only_fields same as super

    def validate(self, data):
        if len(data["parents"]) > 1:
            # TODO @Feature: merge dataset versions with multiple parents
            raise serializers.ValidationError(
                "creating versions with multiple parents is not supported yet"
            )
        if any(not parent.committed for parent in data["parents"]):
            raise serializers.ValidationError("all parent versions must be committed")

        return data

    def validate_metadata(self, value: Union[str, dict]):
        try:
            # should be valid JSON because metadata is a JSONField
            if isinstance(value, str):
                value = json.loads(value)
            DatasetMetadata.from_dict(value)
        except (ValueError, KeyError) as e:
            raise serializers.ValidationError(f"metadata is invalid: {e}")
        return value


class RecordSerializer(serializers.ModelSerializer):
    class Meta:
        model = Record
        fields = ["id", "content_hash", "data", "metadata"]
        read_only_fields = ["id", "content_hash"]


class DatasetViewSet(ArtifactViewSet):
    queryset = Dataset.objects.all()
    serializer_class = DatasetSerializer


class DatasetVersionViewSet(ArtifactVersionViewSet):
    queryset = DatasetVersion.objects.filter(artifact__type=DATASET_TYPE).all()
    serializer_class = DatasetVersionSerializer

    def perform_create(self, serializer: serializers.BaseSerializer) -> None:
        instance: DatasetVersion = serializer.save(committed=False)
        parents: models.QuerySet[DatasetVersion] = proxies(instance.parents.all(), DatasetVersion)
        if parents:
            # this should be caught in DatasetVersionSerializer validation
            if len(parents) != 1:
                raise RuntimeError("creating versions with multiple parents is not supported yet")

            # note: parent metadata is copied in ArtifactVersionViewSet.create prior to actual create
            # create a new version of the dataset state based on the parent
            # TODO @Architecture: creating new version logic should be elsewhere (DatasetAccessor?)
            #  because it is a shared concern and needs to drill down into (partial) sub-datasets.
            parent = parents[0]
            parent_handler = _get_dataset_handler_by_instance(parent)
            if not isinstance(parent_handler, ArtifactVersionHandler):
                logger.info("create_version_fallback_copy", parent=parent, instance=instance)
                # TODO @Performance: copy dataset version as background job
                copy_dataset_version(parent, instance)
            else:
                parent_handler.checkout(instance.version)

    @action(methods=["POST"], detail=True)
    def commit(self, request: Request, *args, **kwargs) -> Response:
        instance: DatasetVersion = self.get_object()
        if instance.committed:
            raise serializers.ValidationError("already committed")
        writer = _to_dataset_writer(_get_dataset_handler_by_instance(instance))
        if isinstance(writer, ArtifactVersionHandler):
            writer.commit()
        instance.committed = True
        instance.save()
        return Response(status=status.HTTP_200_OK)

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

    def list(self, request: Request, artifact: str, version: str) -> Response:
        reader = _get_dataset_reader(artifact, version)
        paginator = cast(RecordPagination, self.paginator)
        offset: int = paginator.get_offset(request)
        limit: int = cast(int, paginator.get_limit(request))
        records = list(reader[offset : offset + limit])
        return Response({"limit": limit, "offset": offset, "results": records})

    def create(self, request: Request, artifact: str, version: str) -> Response:
        writer = _get_dataset_writer(artifact, version)
        serializer: serializers.BaseSerializer = self.get_serializer(data=request.data)
        serializer.is_valid(raise_exception=True)
        record = serializer.save()
        # TODO @Feature: use DatasetAccessor or similar to support reading/writing record metadata
        writer.append(record.data)

        return Response(record.data, status=status.HTTP_201_CREATED)

    def update(self, request: Request, index: str, artifact: str, version: str) -> Response:
        index_int: int = _positive_int(index)
        writer = _get_dataset_writer(artifact, version)
        serializer: serializers.BaseSerializer = self.get_serializer(data=request.data)
        serializer.is_valid(raise_exception=True)
        # TODO @Feature: use DatasetAccessor or similar to support reading/writing record metadata
        writer.update(index_int, serializer.validated_data["data"])

        reader = _get_dataset_reader(artifact, version)
        record = reader[index_int]
        return Response(record)

    def retrieve(self, request: Request, index: str, artifact: str, version: str) -> Response:
        index_int: int = _positive_int(index)
        reader = _get_dataset_reader(artifact, version)
        try:
            record = reader[index_int]
        except LookupError:
            raise Http404("No record matches the given query.")
        return Response(record)


def _get_dataset_reader(artifact: str, version: str) -> DatasetReader:
    dataset_version = get_object_or_404(DatasetVersion, artifact__name=artifact, version=version)
    handler = _get_dataset_handler_by_instance(dataset_version)
    return _to_dataset_reader(handler)


def _to_dataset_reader(handler: DatasetHandler) -> DatasetReader:
    if not isinstance(handler, DatasetReader):
        raise ValueError("dataset version is not readable")
    return handler


def _get_dataset_writer(artifact: str, version: str) -> DatasetWriter:
    dataset_version = get_object_or_404(DatasetVersion, artifact__name=artifact, version=version)
    handler = _get_dataset_handler_by_instance(dataset_version)
    return _to_dataset_writer(handler)


def _to_dataset_writer(handler: DatasetHandler) -> DatasetWriter:
    if not isinstance(handler, DatasetWriter):
        raise ValueError("dataset version is not writable")
    return handler


def _get_dataset_handler_by_instance(version: DatasetVersion) -> DatasetHandler:
    return get_dataset_version_handler(version)

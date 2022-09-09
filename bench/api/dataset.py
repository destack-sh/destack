import json
from typing import Union, cast

import structlog
from django.http import Http404
from django.shortcuts import get_object_or_404
from rest_framework import serializers, status, viewsets
from rest_framework.decorators import action
from rest_framework.exceptions import ValidationError
from rest_framework.pagination import LimitOffsetPagination, _positive_int
from rest_framework.request import Request
from rest_framework.response import Response

from bench.api.artifact import (
    ArtifactSerializer,
    ArtifactVersionSerializer,
    ArtifactVersionViewSet,
    ArtifactViewSet,
)
from bench.dataset.accessor import DatasetAccessor, DatasetRecord
from bench.models import Dataset, DatasetVersion
from bench.models.dataset import DatasetMetadata, DatasetViewData
from bench.models.utils import DATASET_TYPE

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


class DatasetRecordSerializer(serializers.ModelSerializer):
    id = serializers.UUIDField(required=False)
    data_ = serializers.JSONField(source="data")
    metadata = serializers.JSONField()

    class Meta:
        fields = ["id", "data", "metadata"]
        read_only_fields = ["id"]


class DatasetViewSet(ArtifactViewSet):
    queryset = Dataset.objects.all()
    serializer_class = DatasetSerializer


class DatasetVersionViewSet(ArtifactVersionViewSet):
    queryset = DatasetVersion.objects.filter(artifact__type=DATASET_TYPE).all()
    serializer_class = DatasetVersionSerializer

    def perform_create(self, serializer: serializers.BaseSerializer) -> None:
        instance: DatasetVersion = serializer.save(committed=False)
        DatasetAccessor.checkout_from_parents(instance)

    @action(methods=["POST"], detail=True)
    def commit(self, request: Request, *args, **kwargs) -> Response:
        instance: DatasetVersion = self.get_object()
        DatasetAccessor(instance).commit()
        return Response(status=status.HTTP_200_OK)

    def destroy(self, request: Request, *args, **kwargs) -> Response:
        # instance: DatasetVersion = self.get_object()
        # TODO @Feature: delete dataset versions
        return Response(status=status.HTTP_501_NOT_IMPLEMENTED)


class DatasetRecordPagination(LimitOffsetPagination):
    default_limit = 100
    max_limit = 1000


class DatasetRecordViewSet(viewsets.GenericViewSet):
    lookup_field = "index"
    serializer_class = DatasetRecordSerializer
    pagination_class = DatasetRecordPagination

    @property
    def view(self) -> DatasetViewData:
        try:
            return DatasetViewData(**self.request.data)
        except ValueError as e:
            raise ValidationError(f"invalid view: {e}")

    def view_apply(self, index: int) -> int:
        index = self.view.apply(index)
        if index is None:
            raise Http404()
        return index

    def get_dataset_instance(self, artifact: str, version: str) -> DatasetVersion:
        return get_object_or_404(DatasetVersion, artifact__name=artifact, version=version)

    def list(self, request: Request, artifact: str, version: str) -> Response:
        accessor = DatasetAccessor(self.get_dataset_instance(artifact, version))
        paginator = cast(DatasetRecordPagination, self.paginator)
        offset: int = paginator.get_offset(request)
        limit: int = cast(int, paginator.get_limit(request))
        records = list(
            accessor.get_records_slice(self.view_apply(offset), self.view_apply(offset + limit))
        )
        return Response({"limit": limit, "offset": offset, "results": records})

    def create(self, request: Request, artifact: str, version: str) -> Response:
        accessor = DatasetAccessor(self.get_dataset_instance(artifact, version))
        serializer: serializers.BaseSerializer = self.get_serializer(data=request.data)
        serializer.is_valid(raise_exception=True)
        ds_record: DatasetRecord = serializer.save()
        accessor.append(ds_record)

        return Response(DatasetRecordSerializer(ds_record).data, status=status.HTTP_201_CREATED)

    def update(self, request: Request, index: str, artifact: str, version: str) -> Response:
        accessor = DatasetAccessor(self.get_dataset_instance(artifact, version))
        index_int: int = self.view_apply(_positive_int(index))

        serializer: serializers.BaseSerializer = self.get_serializer(data=request.data)
        serializer.is_valid(raise_exception=True)
        ds_record: DatasetRecord = serializer.save()
        accessor.update(index_int, ds_record)

        ds_record = accessor.get_record(index_int)
        return Response(DatasetRecordSerializer(ds_record).data)

    def retrieve(self, request: Request, index: str, artifact: str, version: str) -> Response:
        accessor = DatasetAccessor(self.get_dataset_instance(artifact, version))
        index_int: int = self.view_apply(_positive_int(index))
        try:
            ds_record = accessor.get_record(index_int)
        except LookupError:
            raise Http404("No record matches the given query.")
        return Response(DatasetRecordSerializer(ds_record).data)

import json
from typing import Optional, Union, cast

import structlog
from django.http import Http404
from django.shortcuts import get_object_or_404
from drf_spectacular.utils import extend_schema
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
from bench.dataset.accessor import DatasetAccessor, DatasetRecord, DatasetSearch
from bench.models import Dataset, DatasetVersion
from bench.models.dataset import DatasetMetadata, DatasetViewData
from bench.models.utils import DATASET_TYPE
from bench.models.versioning import get_head
from bench.utils.func import terrible_cast

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
    metadata = serializers.JSONField(default=DatasetMetadata.default_db().to_dict())

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


class DatasetRecordSerializer(serializers.Serializer):
    id = serializers.UUIDField(required=False)
    index = serializers.IntegerField(required=False)
    data = serializers.JSONField()
    metadata = serializers.JSONField(required=False, default=None)

    def create(self, validated_data):
        return DatasetRecord(**validated_data)

    class Meta:
        fields = ["id", "data", "metadata", "index"]
        read_only_fields = ["id", "index"]


class DatasetSearchSerializer(serializers.Serializer):
    text_like = serializers.CharField(required=False)

    def create(self, validated_data):
        return DatasetSearch(**validated_data)


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
    default_limit = 20
    max_limit = 100


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

    @extend_schema(request=DatasetSearchSerializer)
    def list(
        self, request: Request, organization: str, artifact: str, version: str = None
    ) -> Response:
        accessor = get_dataset_accessor(organization, artifact, version)
        paginator = cast(DatasetRecordPagination, self.paginator)
        offset: int = paginator.get_offset(request)
        limit: int = cast(int, paginator.get_limit(request))

        if request.query_params:
            # search was specified
            search_serializer = DatasetSearchSerializer(data=request.query_params)
            search_serializer.is_valid(raise_exception=True)
            search: DatasetSearch = search_serializer.save()
            records = accessor.search_records(limit=limit, offset=offset, search=search)
        else:
            records = list(
                accessor.get_records_slice(self.view_apply(offset), self.view_apply(offset + limit))
            )
        records_data = DatasetRecordSerializer(records, many=True).data
        return Response({"limit": limit, "offset": offset, "results": records_data})

    def create(
        self, request: Request, organization: str, artifact: str, version: str = None
    ) -> Response:
        accessor = get_dataset_accessor(organization, artifact, version)
        serializer: serializers.BaseSerializer = self.get_serializer(data=request.data)
        serializer.is_valid(raise_exception=True)
        ds_record: DatasetRecord = serializer.save()
        accessor.append(ds_record)

        return Response(DatasetRecordSerializer(ds_record).data, status=status.HTTP_201_CREATED)

    def update(
        self, request: Request, index: str, organization: str, artifact: str, version: str = None
    ) -> Response:
        accessor = get_dataset_accessor(organization, artifact, version)
        index_int: int = self.view_apply(_positive_int(index))

        serializer: serializers.BaseSerializer = self.get_serializer(data=request.data)
        serializer.is_valid(raise_exception=True)
        ds_record: DatasetRecord = serializer.save()
        accessor.update(index_int, ds_record)

        # get actually written record
        return Response(DatasetRecordSerializer(accessor.get_record(index_int)).data)

    def retrieve(
        self, index: str, organization: str, artifact: str, version: str = None
    ) -> Response:
        accessor = get_dataset_accessor(organization, artifact, version)
        index_int: int = self.view_apply(_positive_int(index))
        try:
            ds_record = accessor.get_record(index_int)
        except LookupError:
            raise Http404("No record matches the given query.")
        return Response(DatasetRecordSerializer(ds_record).data)


def get_dataset_accessor(
    organization: str, artifact: str, version: Optional[str]
) -> DatasetAccessor:
    return DatasetAccessor(get_dataset_version(organization, artifact, version))


def get_dataset_version(organization: str, artifact: str, version: Optional[str]) -> DatasetVersion:
    if version is not None:
        return get_object_or_404(
            DatasetVersion,
            artifact__name=artifact,
            organization__slug=organization,
            version=version,
        )
    else:
        return terrible_cast(DatasetVersion, get_head(get_object_or_404(Dataset, name=artifact)))

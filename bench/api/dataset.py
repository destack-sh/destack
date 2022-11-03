import json
from typing import Optional, Union, cast

import structlog
from django.core.validators import RegexValidator
from django.db import models
from django.http import Http404
from django.shortcuts import get_object_or_404
from drf_spectacular.utils import extend_schema
from rest_framework import serializers, status, validators, viewsets
from rest_framework.decorators import action
from rest_framework.exceptions import ValidationError
from rest_framework.pagination import LimitOffsetPagination, _positive_int
from rest_framework.request import Request
from rest_framework.response import Response

from bench.api.dataset import (
    DatasetSerializer,
    DatasetVersionSerializer,
    DatasetVersionViewSet,
    DatasetViewSet,
)
from bench.api.tag import TaggedItemSerializerMixin
from bench.models import Dataset, DatasetVersion, Organization
from bench.models.dataset import DatasetRecord, DatasetSearch, DatasetViewData
from bench.models.utils import DATASET_TYPE, MAX_NAME_LENGTH
from bench.models.versioning import get_head
from bench.utils.func import terrible_cast

logger = structlog.stdlib.get_logger()


class DatasetSerializer(DatasetSerializer):
    type = serializers.CharField(max_length=64, default=DATASET_TYPE)
    name = serializers.CharField(
        max_length=MAX_NAME_LENGTH,
        validators=[
            validators.UniqueValidator(
                queryset=Dataset.objects.all(),
                message="There is already an dataset with the given name in this organization",
            ),
            RegexValidator(
                regex=r"^[\w.\-]+$", message="Dataset names must follow pattern [\\w.\\-]+"
            ),
        ],
    )
    organization: serializers.SlugRelatedField = serializers.SlugRelatedField(
        slug_field="slug", queryset=Organization.objects.all()
    )
    versions: serializers.SlugRelatedField = serializers.SlugRelatedField(
        many=True, read_only=True, slug_field="version"
    )
    head = serializers.SerializerMethodField(required=False, read_only=True)

    class Meta:
        model = Dataset
        fields = [
            "id",
            "type",
            "created_at",
            "organization",
            "name",
            "versions",
            "description",
            "head",
            "tags",
        ]
        read_only_fields = ["id", "created_at", "head", "versions"]

    def get_head(self, obj: Dataset):
        head = get_head(obj)
        return DatasetVersionSerializer(head).data if head is not None else None


class DatasetVersionSerializer(TaggedItemSerializerMixin, serializers.ModelSerializer):
    dataset: serializers.SlugRelatedField = serializers.SlugRelatedField(
        queryset=Dataset.objects.all(), slug_field="name"
    )
    parents: serializers.SlugRelatedField = serializers.SlugRelatedField(
        queryset=DatasetVersion.objects.all(), slug_field="version", many=True
    )

    class Meta(DatasetVersionSerializer.Meta):
        model = DatasetVersion
        fields = [
            "id",
            "created_at",
            "parents",
            "version",
            "artifact",
            "committed",
            "tags",
        ]
        read_only_fields = ["id", "created_at", "parents", "version", "content_hash", "committed"]

    def validate(self, data):
        if len(data["parents"]) > 1:
            # TODO @Feature: merge dataset versions with multiple parents
            raise serializers.ValidationError(
                "creating versions with multiple parents is not supported yet"
            )
        if any(not parent.committed for parent in data["parents"]):
            raise serializers.ValidationError("all parent versions must be committed")

        return data


class DatasetRecordSerializer(serializers.Serializer):
    index = serializers.IntegerField(required=False)
    data = serializers.JSONField()
    metadata = serializers.JSONField(required=False, default=None)

    def create(self, validated_data):
        return DatasetRecord(**validated_data)

    class Meta:
        fields = ["data", "metadata", "index"]
        read_only_fields = ["index"]


class DatasetSearchSerializer(serializers.Serializer):
    text_like = serializers.CharField(required=False)

    def create(self, validated_data):
        return DatasetSearch(**validated_data)


class DatasetViewSet(viewsets.ModelViewSet):
    queryset = Dataset.objects.all()
    serializer_class = DatasetSerializer

    lookup_field = "name"
    lookup_value_regex = r"[\w.\-]+"

    def create(self, request: Request, *args, **kwargs):
        # add organization from path argument
        request.data["organization"] = kwargs.pop("organization")
        return super().create(request, *args, **kwargs)

    def get_queryset(self):
        queryset = super().get_queryset()
        queryset = queryset.filter(organization__slug=self.kwargs.get("organization"))
        return queryset


class DatasetVersionViewSet(viewsets.ModelViewSet):
    queryset = DatasetVersion.objects.filter(dataset__type=DATASET_TYPE).all()
    serializer_class = DatasetVersionSerializer

    def get_queryset(self) -> models.QuerySet[DatasetVersion]:
        return self.queryset.filter(
            artifact__organization__slug=self.kwargs.get("organization"),
            artifact__name=self.kwargs.get("artifact"),
        )

    def create(self, request: Request, *args, **kwargs) -> Response:
        request.data["artifact"] = kwargs.pop("artifact")
        # auto-insert parent metadata if not explicitly given and there is only one parent
        if request.data.get("parents"):
            parents_field_serializer = self.get_serializer().fields["parents"]  # type: ignore
            parents = parents_field_serializer.to_internal_value(request.data.get("parents"))
            if len(parents) == 1 and "metadata" not in request.data:
                request.data["metadata"] = parents[0].metadata.copy()

        return super().create(request, *args, **kwargs)

    def perform_create(self, serializer: serializers.BaseSerializer) -> None:
        instance: DatasetVersion = serializer.save(committed=False)
        instance.checkout_from_parents()

    @action(methods=["POST"], detail=True)
    def commit(self, request: Request, *args, **kwargs) -> Response:
        instance: DatasetVersion = self.get_object()
        instance.commit()
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
        self, request: Request, organization: str, dataset: str, version: str = None
    ) -> Response:
        dataset = get_dataset_version(organization, dataset, version)
        paginator = cast(DatasetRecordPagination, self.paginator)
        offset: int = paginator.get_offset(request)
        limit: int = cast(int, paginator.get_limit(request))

        if request.query_params:
            # search was specified
            search_serializer = DatasetSearchSerializer(data=request.query_params)
            search_serializer.is_valid(raise_exception=True)
            search: DatasetSearch = search_serializer.save()
            records = dataset.search_records(limit=limit, offset=offset, search=search)
        else:
            records = list(
                dataset.get_records_slice(self.view_apply(offset), self.view_apply(offset + limit))
            )
        records_data = DatasetRecordSerializer(records, many=True).data
        return Response({"limit": limit, "offset": offset, "results": records_data})

    def create(
        self, request: Request, organization: str, dataset: str, version: str = None
    ) -> Response:
        dataset = get_dataset_version(organization, dataset, version)
        serializer: serializers.BaseSerializer = self.get_serializer(data=request.data)
        serializer.is_valid(raise_exception=True)
        ds_record: DatasetRecord = serializer.save()
        dataset.append(ds_record)

        return Response(DatasetRecordSerializer(ds_record).data, status=status.HTTP_201_CREATED)

    def update(
        self, request: Request, index: str, organization: str, dataset: str, version: str = None
    ) -> Response:
        dataset = get_dataset_version(organization, dataset, version)
        index_int: int = self.view_apply(_positive_int(index))

        serializer: serializers.BaseSerializer = self.get_serializer(data=request.data)
        serializer.is_valid(raise_exception=True)
        ds_record: DatasetRecord = serializer.save()
        dataset.update(index_int, ds_record)

        # get actually written record
        return Response(DatasetRecordSerializer(dataset.get_record(index_int)).data)

    def retrieve(
        self, index: str, organization: str, dataset: str, version: str = None
    ) -> Response:
        dataset = get_dataset_version(organization, dataset, version)
        index_int: int = self.view_apply(_positive_int(index))
        try:
            ds_record = dataset.get_record(index_int)
        except LookupError:
            raise Http404("No record matches the given query.")
        return Response(DatasetRecordSerializer(ds_record).data)


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

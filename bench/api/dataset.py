from typing import cast

import structlog
from django.core.validators import RegexValidator
from django.http import Http404
from drf_spectacular.utils import extend_schema
from rest_framework import serializers, status, validators, viewsets
from rest_framework.exceptions import ValidationError
from rest_framework.pagination import LimitOffsetPagination, _positive_int
from rest_framework.request import Request
from rest_framework.response import Response

from bench.api.tag import TaggedItemSerializerMixin
from bench.models import Dataset, Organization
from bench.models.dataset import DatasetRecord, DatasetSearch, DatasetViewData
from bench.models.utils import MAX_NAME_LENGTH

logger = structlog.stdlib.get_logger()


class DatasetSerializer(serializers.ModelSerializer):
    type = serializers.CharField(max_length=64)
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


class DatasetVersionSerializer(TaggedItemSerializerMixin, serializers.ModelSerializer):
    dataset: serializers.SlugRelatedField = serializers.SlugRelatedField(
        queryset=Dataset.objects.all(), slug_field="name"
    )

    class Meta:
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

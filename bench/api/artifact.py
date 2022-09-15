from __future__ import annotations

import structlog
from django.core.validators import RegexValidator
from django.db import models
from rest_framework import mixins, serializers, validators, viewsets
from rest_framework.decorators import action
from rest_framework.generics import get_object_or_404
from rest_framework.pagination import LimitOffsetPagination
from rest_framework.request import Request
from rest_framework.response import Response

from bench.api.tag import TaggedItemSerializerMixin
from bench.executor import executor
from bench.models import Artifact, ArtifactVersion, Organization
from bench.models.artifact import ArtifactView
from bench.models.utils import MAX_NAME_LENGTH
from bench.models.versioning import get_head
from bench.utils.serializer import DatasetTypeSerializer, ModelTypeSerializer
from bench.utils.spec import DatasetType, ModelType

logger = structlog.stdlib.get_logger()


class ArtifactSerializer(TaggedItemSerializerMixin, serializers.HyperlinkedModelSerializer):
    name = serializers.CharField(
        max_length=MAX_NAME_LENGTH,
        validators=[
            validators.UniqueValidator(
                queryset=Artifact.objects.all(),
                message="There is already an artifact with the given name in this organization",
            ),
            RegexValidator(
                regex=r"^[\w.\-]+$", message="Artifact names must follow pattern [\\w.\\-]+"
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
        model = Artifact
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

    def get_head(self, obj: Artifact):
        head = get_head(obj)
        return ArtifactVersionSerializer(head).data if head is not None else None


class ArtifactVersionSerializer(TaggedItemSerializerMixin, serializers.ModelSerializer):
    artifact: serializers.SlugRelatedField = serializers.SlugRelatedField(
        queryset=Artifact.objects.all(), slug_field="name"
    )
    parents: serializers.SlugRelatedField = serializers.SlugRelatedField(
        queryset=ArtifactVersion.objects.all(), slug_field="version", many=True
    )

    class Meta:
        model = ArtifactVersion
        fields = [
            "id",
            "created_at",
            "parents",
            "version",
            "artifact",
            "storage_uri",
            "metadata",
            "committed",
            "tags",
        ]
        read_only_fields = ["id", "created_at", "parents", "version", "content_hash", "committed"]


class ArtifactViewSerializer(TaggedItemSerializerMixin, serializers.ModelSerializer):
    artifact: serializers.SlugRelatedField = serializers.SlugRelatedField(
        queryset=ArtifactView.objects.all(), slug_field="name"
    )
    compatible_versions: serializers.SlugRelatedField = serializers.SlugRelatedField(
        queryset=ArtifactVersion.objects.all(), slug_field="version", many=True
    )

    class Meta:
        model = ArtifactView
        fields = [
            "id",
            "created_at",
            "type",
            "artifact",
            "compatible_versions",
            "name",
            "description",
            "data",
            "tags",
        ]
        read_only_fields = ["id", "created_at"]


def _get_serialized_runtime_spec(artifact: ArtifactVersion) -> dict:
    """Gets the serialized runtime spec (as opposed to configured spec) for an artifact"""
    spec = executor.get_runtime_artifact_spec(artifact)
    if isinstance(spec, ModelType):
        serialized_spec = ModelTypeSerializer(spec).data
    elif isinstance(spec, DatasetType):
        serialized_spec = DatasetTypeSerializer(spec).data
    else:
        raise serializers.ValidationError(f"invalid artifact: {artifact}")
    return serialized_spec


class ArtifactViewSet(viewsets.ModelViewSet):
    queryset = Artifact.objects.all()
    serializer_class = ArtifactSerializer
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


class ArtifactVersionViewSet(viewsets.ModelViewSet):
    queryset = ArtifactVersion.objects.all()
    serializer_class = ArtifactVersionSerializer
    lookup_field = "version"
    lookup_value_regex = r"[\w.]+"
    # TODO @Feature: paginate artifact versions with branches correctly
    pagination_class = LimitOffsetPagination

    def get_queryset(self) -> models.QuerySet[ArtifactVersion]:
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

    # TODO add get runtime spec for temporary and live artifacts
    #  temporary for creating new models, other for existing
    @action(url_path="spec", methods=["GET"], detail=True)
    def get_runtime_spec(self, request: Request, artifact: str, version: str):
        artifact_instance: ArtifactVersion = get_object_or_404(
            ArtifactVersion, artifact__name=artifact, version=version
        )
        serialized_spec = _get_serialized_runtime_spec(artifact_instance)
        return Response(serialized_spec)


class ArtifactTagsViewSet(viewsets.GenericViewSet, mixins.RetrieveModelMixin):
    queryset = ArtifactVersion.objects.all()
    serializer_class = ArtifactVersionSerializer

    def get_object(self):
        # overwrites GenericViewSet.get_object to remove filtering by pk (which is used as tag here)
        queryset = self.filter_queryset(self.get_queryset())
        obj = get_object_or_404(queryset)
        self.check_object_permissions(self.request, obj)
        return obj

    def get_queryset(self) -> models.QuerySet[ArtifactVersion]:
        # TODO @Feature: support tags for artifacts & flows
        #  Need to figure out where to put tags first - on the object/in the project/workspace?
        if self.kwargs.get("pk") != "HEAD":
            raise serializers.ValidationError("tag must be HEAD")
        return self.queryset.filter(
            artifact__organization__slug=self.kwargs.get("organization"),
            artifact__name=(self.kwargs.get("artifact")),
        ).order_by("-created_at")[:1]

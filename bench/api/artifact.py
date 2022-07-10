import structlog
from django.db import models
from rest_framework import mixins, serializers, validators, viewsets
from rest_framework.generics import get_object_or_404
from rest_framework.request import Request
from rest_framework.response import Response

from bench.models import Artifact, ArtifactVersion
from bench.models.utils import MAX_NAME_LENGTH

logger = structlog.stdlib.get_logger()


class ArtifactSerializer(serializers.HyperlinkedModelSerializer):
    name = serializers.CharField(
        max_length=MAX_NAME_LENGTH,
        validators=[
            validators.UniqueValidator(
                queryset=Artifact.objects.all(),
                message="There is already an artifact with the given name",
            )
        ],
    )
    versions: serializers.SlugRelatedField = serializers.SlugRelatedField(
        many=True, read_only=True, slug_field="version"
    )

    class Meta:
        model = Artifact
        fields = ["id", "type", "created_at", "name", "versions", "description"]
        read_only_fields = ["id", "created_at"]


class ArtifactVersionSerializer(serializers.ModelSerializer):
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
            "parents",
            "version",
            "artifact",
            "storage_uri",
            "metadata",
            "content_hash",
            "committed",
        ]
        read_only_fields = ["id", "parents", "version", "content_hash", "committed"]


class ArtifactViewSet(viewsets.ModelViewSet):
    queryset = Artifact.objects.all()
    serializer_class = ArtifactSerializer
    lookup_field = "name"
    lookup_value_regex = r"[\w.]+"


class ArtifactVersionViewSet(viewsets.ModelViewSet):
    queryset = ArtifactVersion.objects.all()
    serializer_class = ArtifactVersionSerializer
    lookup_field = "version"
    lookup_value_regex = r"[\w.]+"

    def get_queryset(self) -> models.QuerySet[ArtifactVersion]:
        return self.queryset.filter(artifact__name=self.kwargs.get("artifact_name"))

    def create(self, request: Request, *args, **kwargs) -> Response:
        # auto-insert artifact_name provided by nested dataset versions route
        if "artifact_name" in kwargs:
            request.data["artifact"] = kwargs.pop("artifact_name")

        # auto-insert parent metadata if not explicitly given and there is only one parent
        if request.data.get("parents"):
            parents_field_serializer = self.get_serializer().fields["parents"]  # type: ignore
            parents = parents_field_serializer.to_internal_value(request.data.get("parents"))
            if len(parents) == 1 and "metadata" not in request.data:
                request.data["metadata"] = parents[0].metadata.copy()

        return super().create(request, *args, **kwargs)


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
        return self.queryset.filter(artifact__name=self.kwargs.get("artifact_name")).order_by(
            "created_at"
        )[:1]

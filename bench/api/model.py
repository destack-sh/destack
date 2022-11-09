from django.core.validators import RegexValidator
from rest_framework import serializers, validators, viewsets

from bench.models import Model, Organization
from bench.models.utils import MAX_NAME_LENGTH


class ModelSerializer(serializers.ModelSerializer):
    name = serializers.CharField(
        max_length=MAX_NAME_LENGTH,
        validators=[
            validators.UniqueValidator(
                queryset=Model.objects.all(),
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

    class Meta:
        model = Model
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


class ModelViewSet(viewsets.ModelViewSet):
    queryset = Model.objects.all()
    serializer_class = ModelSerializer

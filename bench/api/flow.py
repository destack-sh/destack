import structlog
from django.core.validators import RegexValidator
from django.db import models
from drf_spectacular.utils import extend_schema
from rest_framework import serializers, validators, viewsets
from rest_framework.decorators import action
from rest_framework.generics import get_object_or_404
from rest_framework.request import Request
from rest_framework.response import Response

from bench.api.execution import ExecutionSerializer
from bench.api.tag import TaggedItemSerializerMixin
from bench.models import Organization
from bench.models.flow import Flow, Instruction
from bench.models.utils import MAX_NAME_LENGTH

# ========================
# General Flow serializers
# ========================

logger = structlog.stdlib.get_logger()


class FlowSerializer(TaggedItemSerializerMixin, serializers.ModelSerializer):
    name = serializers.CharField(
        max_length=MAX_NAME_LENGTH,
        validators=[
            validators.UniqueValidator(
                queryset=Flow.objects.all(),
                message="There is already a flow with the given name in this organization",
            ),
        ],
    )
    head = serializers.SerializerMethodField(required=False, read_only=True)
    organization: serializers.SlugRelatedField = serializers.SlugRelatedField(
        slug_field="slug", queryset=Organization.objects.all()
    )

    class Meta:
        model = Flow
        fields = ["id", "name", "description", "created_at", "organization", "head", "tags"]
        read_only_fields = ["id", "created_at", "head", "versions"]


class InstructionSerializer(serializers.ModelSerializer):
    name = serializers.CharField(
        max_length=MAX_NAME_LENGTH,
        validators=[
            RegexValidator(
                regex=r"^[\w.\-]+$", message="Flow node names must follow pattern [\\w.\\-]+"
            )
        ],
    )

    class Meta:
        model = Instruction
        fields = ["id", "flow", "name", "created_at", "function_id", "config_arguments", "metadata"]
        read_only_fields = ["id", "created_at", "committed"]
        validators = [
            validators.UniqueTogetherValidator(
                queryset=Instruction.objects.all(), fields=["flow", "name"]
            )
        ]


class FlowVersionSerializer(TaggedItemSerializerMixin, serializers.ModelSerializer):
    flow: serializers.SlugRelatedField = serializers.SlugRelatedField(
        queryset=Flow.objects.all(), slug_field="name"
    )
    # nodes, node_edges and artifact_edges are read only duplicates of the respective nested viewsets
    root_instruction = InstructionSerializer(many=False, read_only=True)

    class Meta:
        fields = [
            "id",
            "created_at",
            "parents",
            "version",
            "flow",
            "nodes",
            "node_edges",
            "artifact_edges",
            "committed",
            "tags",
        ]
        read_only_fields = ["id", "created_at", "parents", "flow", "version", "committed"]

    def validate(self, data):
        if len(data["parents"]) > 1:
            # TODO @Feature: merge flow versions with multiple parents
            raise serializers.ValidationError(
                "creating versions with multiple parents is not supported yet"
            )
        if any(not parent.committed for parent in data["parents"]):
            raise serializers.ValidationError("all parent versions must be committed")

        return data


# ===================================
# Execution-specific Flow serializers
# ===================================

instruction_ARGUMENT_KEYS: set[str] = {"other_node", "records", "artifact"}
instruction_INPUT_KEYS: set[str] = {"records", "artifacts"}


# ==========
# Flow views
# ==========


class FlowViewSet(viewsets.ModelViewSet):
    queryset = Flow.objects.order_by("-created_at").all()
    serializer_class = FlowSerializer
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

    @action(methods=["POST"], detail=True)
    @extend_schema(responses=ExecutionSerializer())
    def execute(self, request: Request, organization: str, name: str) -> Response:
        flow: Flow = get_object_or_404(Flow, organization__slug=organization, name=name)
        return _execute_response(get_head(flow), request)


class InstructionViewSet(viewsets.ModelViewSet):
    queryset = Instruction.objects.all()
    serializer_class = InstructionSerializer

    # TODO @Cleanup: repetition of get_queryset/create mapping among Instruction&Edges
    def get_queryset(self) -> models.QuerySet[Instruction]:
        return self.queryset.filter(
            flow__flow__name=self.kwargs.get("flow"), flow__version=self.kwargs.get("version")
        )

    def create(self, request: Request, *args, **kwargs) -> Response:
        # TODO @Cleanup: argument mapping could be in nested router?
        # map nested arguments to Instruction.flow representation
        if "flow" in kwargs and "version" in kwargs:
            request.data["flow"] = f"{kwargs['flow']}@{kwargs['version']}"
        return super().create(request, *args, **kwargs)

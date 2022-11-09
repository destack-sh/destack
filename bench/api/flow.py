import structlog
from django.core.validators import RegexValidator
from django.db import models
from drf_spectacular.utils import extend_schema
from rest_framework import serializers, validators, viewsets
from rest_framework.decorators import action
from rest_framework.generics import get_object_or_404
from rest_framework.pagination import LimitOffsetPagination
from rest_framework.request import Request
from rest_framework.response import Response

from bench.api.execution import ExecutionSerializer
from bench.api.tag import TaggedItemSerializerMixin
from bench.api.utils import FlowVersionListingField
from bench.models import Organization
from bench.models.flow import Flow, FlowInstruction, FlowVersion
from bench.models.utils import MAX_NAME_LENGTH

# ========================
# General Flow serializers
# ========================
from bench.models.versioning import get_head

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

    def get_head(self, obj: Flow):
        head = get_head(obj)
        return FlowVersionSerializer(head).data if head is not None else None


class FlowInstructionSerializer(serializers.ModelSerializer):
    flow = FlowVersionListingField(queryset=FlowVersion.objects.all())
    name = serializers.CharField(
        max_length=MAX_NAME_LENGTH,
        validators=[
            RegexValidator(
                regex=r"^[\w.\-]+$", message="Flow node names must follow pattern [\\w.\\-]+"
            )
        ],
    )

    class Meta:
        model = FlowInstruction
        fields = ["id", "flow", "name", "created_at", "function_id", "config_arguments", "metadata"]
        read_only_fields = ["id", "created_at", "committed"]
        validators = [
            validators.UniqueTogetherValidator(
                queryset=FlowInstruction.objects.all(), fields=["flow", "name"]
            )
        ]


class FlowVersionSerializer(TaggedItemSerializerMixin, serializers.ModelSerializer):
    flow: serializers.SlugRelatedField = serializers.SlugRelatedField(
        queryset=Flow.objects.all(), slug_field="name"
    )
    parents: serializers.SlugRelatedField = serializers.SlugRelatedField(
        queryset=FlowVersion.objects.all(), slug_field="version", many=True
    )
    # nodes, node_edges and artifact_edges are read only duplicates of the respective nested viewsets
    root_instruction = FlowInstructionSerializer(many=False, read_only=True)

    class Meta:
        model = FlowVersion
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

flow_instruction_ARGUMENT_KEYS: set[str] = {"other_node", "records", "artifact"}
flow_instruction_INPUT_KEYS: set[str] = {"records", "artifacts"}


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


class FlowVersionViewSet(viewsets.ModelViewSet):
    queryset = FlowVersion.objects.all()
    serializer_class = FlowVersionSerializer
    lookup_field = "version"
    lookup_value_regex = r"[\w.]+"
    # TODO @Feature: paginate flow versions with branches correctly
    pagination_class = LimitOffsetPagination

    def get_queryset(self) -> models.QuerySet[FlowVersion]:
        return self.queryset.filter(
            flow__organization__slug=self.kwargs.get("organization"),
            flow__name=self.kwargs.get("flow"),
        )

    def create(self, request: Request, *args, **kwargs) -> Response:
        # add flow from path arguments
        request.data["flow"] = kwargs.pop("flow")
        return super().create(request, *args, **kwargs)

    def perform_create(self, serializer: serializers.BaseSerializer) -> None:
        # TODO @Cleanup: why force committed=False in FlowVersion create? (also see DatasetVersion)
        instance: FlowVersion = serializer.save(committed=False)
        parents: models.QuerySet[FlowVersion] = instance.parents.all()

        if parents:
            # this should be caught in FlowVersionSerializer validation
            if len(parents) != 1:
                raise RuntimeError("creating versions with multiple parents is not supported yet")

            # copy flow nodes and edges from parent
            instance.copy_from(parents[0])

    @action(methods=["POST"], detail=True)
    @extend_schema(responses=ExecutionSerializer())
    def execute(self, request: Request, organization: str, flow: str, version: str) -> Response:
        flow_instance = get_object_or_404(
            FlowVersion, flow__organization__slug=organization, flow__name=flow, version=version
        )
        return _execute_response(flow_instance, request)


class FlowInstructionViewSet(viewsets.ModelViewSet):
    queryset = FlowInstruction.objects.all()
    serializer_class = FlowInstructionSerializer

    # TODO @Cleanup: repetition of get_queryset/create mapping among FlowInstruction&Edges
    def get_queryset(self) -> models.QuerySet[FlowInstruction]:
        return self.queryset.filter(
            flow__flow__name=self.kwargs.get("flow"), flow__version=self.kwargs.get("version")
        )

    def create(self, request: Request, *args, **kwargs) -> Response:
        # TODO @Cleanup: argument mapping could be in nested router?
        # map nested arguments to FlowInstruction.flow representation
        if "flow" in kwargs and "version" in kwargs:
            request.data["flow"] = f"{kwargs['flow']}@{kwargs['version']}"
        return super().create(request, *args, **kwargs)

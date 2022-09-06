from collections import defaultdict
from typing import NamedTuple
from uuid import UUID

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
from rest_framework_dataclasses.serializers import DataclassSerializer, _strip_empty_sentinels

from bench.api.execution import ExecutionSerializer
from bench.api.tag import TaggedItemSerializerMixin
from bench.api.utils import ArtifactVersionListingField, FlowVersionListingField
from bench.executor import executor
from bench.executor.base import FlowExecutionOptions, FlowRawArgument
from bench.function.spec import update_flow_spec
from bench.models import ArtifactVersion, Flow, FlowNode
from bench.models.flow import FlowArtifactEdge, FlowNodeEdge, FlowVersion
from bench.models.utils import MAX_NAME_LENGTH

# ========================
# General Flow serializers
# ========================
from bench.utils.func import get_first
from bench.utils.record import RecordList

logger = structlog.stdlib.get_logger()


class FlowSerializer(TaggedItemSerializerMixin, serializers.ModelSerializer):
    name = serializers.CharField(
        max_length=MAX_NAME_LENGTH,
        validators=[
            validators.UniqueValidator(
                queryset=Flow.objects.all(),
                message="There is already a flow with the given name",
            ),
            RegexValidator(
                regex=r"^[\w.\-]+$", message="Flow names must follow pattern [\\w.\\-]+"
            ),
        ],
    )
    latest_version = serializers.SerializerMethodField(required=False, read_only=True)

    class Meta:
        model = Flow
        fields = ["id", "name", "description", "created_at", "latest_version", "tags"]
        read_only_fields = ["id", "created_at", "latest_version"]

    def get_latest_version(self, obj: Flow):
        latest_version = obj.versions.all().order_by("-created_at").first()
        if latest_version is not None:
            return FlowVersionSerializer(latest_version).data
        else:
            return None


class FlowNodeSerializer(serializers.ModelSerializer):
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
        model = FlowNode
        fields = ["id", "flow", "name", "created_at", "function_id", "config_arguments", "metadata"]
        read_only_fields = ["id", "created_at", "committed"]
        validators = [
            validators.UniqueTogetherValidator(
                queryset=FlowNode.objects.all(), fields=["flow", "name"]
            )
        ]


class FlowNodeEdgeSerializer(serializers.ModelSerializer):
    flow = FlowVersionListingField(queryset=FlowVersion.objects.all())
    dependent: serializers.PrimaryKeyRelatedField = serializers.PrimaryKeyRelatedField(
        queryset=FlowNode.objects.all()
    )
    dependency: serializers.PrimaryKeyRelatedField = serializers.PrimaryKeyRelatedField(
        queryset=FlowNode.objects.all()
    )

    class Meta:
        model = FlowNodeEdge
        fields = "__all__"
        read_only_fields = ["id"]


class FlowArtifactEdgeSerializer(serializers.ModelSerializer):
    flow = FlowVersionListingField(queryset=FlowVersion.objects.all())
    dependent: serializers.PrimaryKeyRelatedField = serializers.PrimaryKeyRelatedField(
        queryset=FlowNode.objects.all()
    )
    dependency = ArtifactVersionListingField(queryset=ArtifactVersion.objects.all())

    class Meta:
        model = FlowArtifactEdge
        fields = "__all__"
        read_only_fields = ["id"]


class FlowVersionSerializer(TaggedItemSerializerMixin, serializers.ModelSerializer):
    flow: serializers.SlugRelatedField = serializers.SlugRelatedField(
        queryset=Flow.objects.all(), slug_field="name"
    )
    parents: serializers.SlugRelatedField = serializers.SlugRelatedField(
        queryset=FlowVersion.objects.all(), slug_field="version", many=True
    )
    # nodes, node_edges and artifact_edges are read only duplicates of the respective nested viewsets
    nodes = FlowNodeSerializer(many=True, read_only=True)
    node_edges = FlowNodeEdgeSerializer(many=True, read_only=True)
    artifact_edges = FlowArtifactEdgeSerializer(many=True, read_only=True)

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

FLOW_NODE_ARGUMENT_KEYS: set[str] = {"other_node", "records", "artifact"}
FLOW_NODE_INPUT_KEYS: set[str] = {"records", "artifacts"}


class FlowNodeArgumentDataSerializer(serializers.Serializer):
    type = serializers.ChoiceField(choices=["input", "argument"])
    node: serializers.PrimaryKeyRelatedField = serializers.PrimaryKeyRelatedField(
        queryset=FlowNode.objects.all()
    )
    name = serializers.CharField()
    # actual argument is union of either another node, an existing artifact or ad-hoc records
    artifact = ArtifactVersionListingField(queryset=ArtifactVersion.objects.all(), required=False)
    records = serializers.JSONField(required=False)

    def validate(self, data):
        allowed_keys = ["artifact", "records"]
        num_specified = sum(1 if key in data else 0 for key in allowed_keys)
        if num_specified != 1:
            raise serializers.ValidationError(
                f"must specify exactly one of data keys: {allowed_keys}"
            )
        return data


class FlowExecutionOptionsSerializer(DataclassSerializer):
    class Meta:
        dataclass = FlowExecutionOptions


class FlowExecutionRequest(NamedTuple):
    arguments: dict[str, dict]
    options: FlowExecutionOptions


class FlowExecutionRequestSerializer(serializers.Serializer):
    arguments = serializers.DictField(child=FlowNodeArgumentDataSerializer(many=True))
    options = FlowExecutionOptionsSerializer(required=False, default=FlowExecutionOptions.default)

    def create(self, validated_data) -> FlowExecutionRequest:
        # TODO @Cleanup: don't call private _strip_empty_sentinels on FlowExecutionOptionsSerializer
        # For some reason this doesn't happen automatically, though it should according to
        # DataclassSerializer.validated_data.. I'm probably missing something here, fix later.
        return FlowExecutionRequest(
            arguments=validated_data["arguments"],
            options=_strip_empty_sentinels(validated_data["options"]),
        )


# ==========
# Flow views
# ==========


class FlowViewSet(viewsets.ModelViewSet):
    queryset = Flow.objects.order_by("-created_at").all()
    serializer_class = FlowSerializer
    lookup_field = "name"
    lookup_value_regex = r"[\w.\-]+"


class FlowVersionViewSet(viewsets.ModelViewSet):
    queryset = FlowVersion.objects.order_by("-created_at").all()
    serializer_class = FlowVersionSerializer
    lookup_field = "version"
    lookup_value_regex = r"[\w.]+"
    # TODO @Feature: paginate flow versions with branches correctly
    pagination_class = LimitOffsetPagination

    def get_queryset(self) -> models.QuerySet[FlowVersion]:
        return self.queryset.filter(flow__name=self.kwargs.get("flow"))

    def create(self, request: Request, *args, **kwargs) -> Response:
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

    # TODO @Performance: update flow spec directly on edit rather than "manually" post-edit
    @action(methods=["POST"], detail=True)
    @extend_schema(responses=FlowNodeSerializer(many=True))
    def update_spec(self, request: Request, flow: str, version: str) -> Response:
        flow_instance = get_object_or_404(FlowVersion, flow__name=flow, version=version)
        updated_nodes = update_flow_spec(flow_instance)

        # TODO @Performance: bulk update nodes in flow spec update
        for node in updated_nodes:
            node.save()

        serialized_updated_nodes = FlowNodeSerializer(updated_nodes, many=True).data
        return Response(serialized_updated_nodes)

    @action(methods=["POST"], detail=True)
    @extend_schema(responses=ExecutionSerializer())
    def execute(self, request: Request, flow: str, version: str) -> Response:
        logger.debug("execute_attempt", flow=flow, version=version)
        flow_instance = get_object_or_404(FlowVersion, flow__name=flow, version=version)

        request_serializer = FlowExecutionRequestSerializer(data=request.data)
        request_serializer.is_valid(raise_exception=True)
        exec_request: FlowExecutionRequest = request_serializer.save()

        # assemble plan arguments into inputs/arguments dicts
        inputs: dict[UUID, dict[str, FlowRawArgument]] = defaultdict(dict)
        arguments: dict[UUID, dict[str, FlowRawArgument]] = defaultdict(dict)
        for named_arguments in exec_request.arguments.values():
            for named_argument in named_arguments:
                value = get_first(named_argument, ("artifact", "records"))
                # convert record RecordBatch
                if not isinstance(value, ArtifactVersion):
                    value = RecordList(value)

                node: FlowNode = named_argument["node"]
                name: str = named_argument["name"]
                if named_argument["type"] == "input":
                    inputs[node.id][name] = value
                elif named_argument["type"] == "argument":
                    arguments[node.id][name] = value

        execution, _ = executor.run_flow(flow_instance, inputs, arguments, exec_request.options)
        serialized_execution = ExecutionSerializer(execution).data
        logger.info("execute_serialized")
        return Response(serialized_execution)


class FlowNodeViewSet(viewsets.ModelViewSet):
    queryset = FlowNode.objects.all()
    serializer_class = FlowNodeSerializer

    # TODO @Cleanup: repetition of get_queryset/create mapping among FlowNode&Edges
    def get_queryset(self) -> models.QuerySet[FlowNode]:
        return self.queryset.filter(
            flow__flow__name=self.kwargs.get("flow"), flow__version=self.kwargs.get("version")
        )

    def create(self, request: Request, *args, **kwargs) -> Response:
        # TODO @Cleanup: argument mapping could be in nested router?
        # map nested arguments to FlowNode.flow representation
        if "flow" in kwargs and "version" in kwargs:
            request.data["flow"] = f"{kwargs['flow']}@{kwargs['version']}"
        return super().create(request, *args, **kwargs)


class FlowNodeEdgeViewSet(viewsets.ModelViewSet):
    queryset = FlowNodeEdge.objects.all()
    serializer_class = FlowNodeEdgeSerializer

    def get_queryset(self) -> models.QuerySet[FlowNode]:
        return self.queryset.filter(
            flow__flow__name=self.kwargs.get("flow"), flow__version=self.kwargs.get("version")
        )

    def create(self, request: Request, *args, **kwargs) -> Response:
        # map nested arguments to FlowNode.flow representation
        if "flow" in kwargs and "version" in kwargs:
            request.data["flow"] = f"{kwargs['flow']}@{kwargs['version']}"
        return super().create(request, *args, **kwargs)


class FlowArtifactEdgeViewSet(viewsets.ModelViewSet):
    queryset = FlowArtifactEdge.objects.all()
    serializer_class = FlowArtifactEdgeSerializer

    def get_queryset(self) -> models.QuerySet[FlowNode]:
        return self.queryset.filter(
            flow__flow__name=self.kwargs.get("flow"), flow__version=self.kwargs.get("version")
        )

    def create(self, request: Request, *args, **kwargs) -> Response:
        # map nested arguments to FlowNode.flow representation
        if "flow" in kwargs and "version" in kwargs:
            request.data["flow"] = f"{kwargs['flow']}@{kwargs['version']}"
        return super().create(request, *args, **kwargs)

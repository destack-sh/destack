from itertools import chain
from typing import Mapping
from uuid import UUID

from django.core.validators import RegexValidator
from django.db import models
from rest_framework import serializers, validators, viewsets
from rest_framework.decorators import action
from rest_framework.pagination import LimitOffsetPagination
from rest_framework.request import Request
from rest_framework.response import Response

from bench.api.execution import ExecutionSerializer
from bench.api.utils import ArtifactVersionListingField, FlowVersionListingField
from bench.executor import executor
from bench.executor.base import FlowExecutionOptions, FlowRawArgument
from bench.models import ArtifactVersion, Flow, FlowNode
from bench.models.flow import FlowArtifactEdge, FlowNodeEdge, FlowVersion
from bench.models.utils import MAX_NAME_LENGTH

# ========================
# General Flow serializers
# ========================


class FlowSerializer(serializers.ModelSerializer):
    name = serializers.CharField(
        max_length=MAX_NAME_LENGTH,
        validators=[
            validators.UniqueValidator(
                queryset=Flow.objects.all(),
                message="There is already a flow with the given name",
            ),
            RegexValidator(regex=r"[\w.\-]+", message="Flow names must follow pattern [\\w.\\-]+"),
        ],
    )

    class Meta:
        model = Flow
        fields = ["id", "name", "description", "created_at"]
        read_only_fields = ["id", "created_at"]


class FlowNodeSerializer(serializers.ModelSerializer):
    flow = FlowVersionListingField(queryset=FlowVersion.objects.all())

    class Meta:
        model = FlowNode
        fields = ["id", "flow", "name", "created_at", "function_id", "config_arguments"]
        read_only_fields = ["id", "created_at", "committed"]


class FlowNodeEdgeSerializer(serializers.ModelSerializer):
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
    dependent: serializers.PrimaryKeyRelatedField = serializers.PrimaryKeyRelatedField(
        queryset=FlowNode.objects.all()
    )
    dependency = ArtifactVersionListingField(queryset=ArtifactVersion.objects.all())

    class Meta:
        model = FlowNodeEdge
        fields = "__all__"
        read_only_fields = ["id"]


class FlowVersionSerializer(serializers.ModelSerializer):
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
    def __init__(self, allowed_keys: set[str], **kwargs):
        self.allowed_keys = allowed_keys
        super().__init__(**kwargs)

    name = serializers.CharField()
    # plain records or existing artifact (version) reference
    other_node: serializers.PrimaryKeyRelatedField = serializers.PrimaryKeyRelatedField(
        queryset=FlowNode.objects.all(), required=False
    )
    records = serializers.JSONField(required=False)
    artifact: serializers.PrimaryKeyRelatedField = serializers.PrimaryKeyRelatedField(
        queryset=ArtifactVersion.objects.all(), required=False
    )

    def validate(self, data):
        num_specified = sum(1 if key in data else 0 for key in self.allowed_keys)
        if num_specified != 1:
            raise serializers.ValidationError(
                f"must specify exactly one of data keys: {self.allowed_keys}"
            )


class FlowNodeInputsSerializer(serializers.Serializer):
    node: serializers.PrimaryKeyRelatedField = serializers.PrimaryKeyRelatedField(
        queryset=FlowNode.objects.all()
    )
    inputs = FlowNodeArgumentDataSerializer(many=True, allowed_keys=FLOW_NODE_INPUT_KEYS)


class FlowNodeArgumentsSerializer(serializers.Serializer):
    node: serializers.PrimaryKeyRelatedField = serializers.PrimaryKeyRelatedField(
        queryset=FlowNode.objects.all()
    )
    arguments = FlowNodeArgumentDataSerializer(many=True, allowed_keys=FLOW_NODE_ARGUMENT_KEYS)


class FlowExecutionPlanSerializer(serializers.Serializer):
    flow: serializers.PrimaryKeyRelatedField = serializers.PrimaryKeyRelatedField(
        queryset=FlowVersion.objects.all()
    )
    inputs = FlowNodeArgumentsSerializer(many=True)
    arguments = FlowNodeArgumentsSerializer(many=True)

    def validate(self, data):
        bad_arguments = filter(
            lambda argument: argument.node.flow_id != data["flow"].id,
            chain(data["inputs"], data["arguments"]),
        )
        if bad_arguments:
            raise serializers.ValidationError("argument flow nodes refer to different flow")


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
        return super().create(*args, **kwargs)

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
    def execute(self, request: Request, *args, **kwargs) -> Response:
        serializer = FlowExecutionPlanSerializer(request.data)
        serializer.is_valid(raise_exception=True)

        # assemble inputs into dict form
        inputs: dict[UUID, Mapping[str, FlowRawArgument]] = {}
        for node_input in serializer.validated_data["inputs"]:
            node_inputs: dict[str, FlowRawArgument] = {}
            for input_data in node_input["inputs"]:
                # use first non-null data given
                node_inputs[input_data["name"]] = input_data.get(
                    "records", input_data.get("artifact")
                )
            inputs[node_input["node"].id] = node_inputs

        # assemble arguments into dict form
        arguments: dict[UUID, Mapping[str, FlowRawArgument]] = {}
        for node_argument in serializer.validated_data["arguments"]:
            node_arguments: dict[str, FlowRawArgument] = {}
            for argument_data in node_argument["arguments"]:
                # use first non-null data given
                node_arguments[argument_data["name"]] = argument_data.get(
                    "other_node", argument_data.get("records", argument_data.get("artifact"))
                )
            arguments[node_argument["node"].id] = node_arguments

        flow: FlowVersion = serializer.validated_data["flow"]
        execution, outputs = executor.run_flow(
            flow, inputs, arguments, options=FlowExecutionOptions.default()
        )
        serialized_execution = ExecutionSerializer(execution).data
        serialized_outputs: Mapping[UUID, Mapping[str, UUID]] = {
            node_id: {name: artifact.id for name, artifact in artifacts.items()}
            for node_id, artifacts in outputs.items()
        }
        return Response({"execution": serialized_execution, "outputs": serialized_outputs})


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

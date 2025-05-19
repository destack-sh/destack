import dataclasses
from datetime import datetime
from typing import TYPE_CHECKING, Any, Collection, Optional, cast

import pytz
import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench import pb2
from bench.language.registry import NODE_CLASS_BY_TYPE
from bench.pb2 import (
    AnyNodeData,
    ChangeVignetteData,
    ClientOriginData,
    EditData,
    EditOperationData,
    GraphScopeData,
)

from .const import EditOperationType, EditType, NodeType, PrimitiveType, StructType
from .graph import Graph, GraphData
from .node import Node, NodeReference, Subject
from .object import Scope
from .property import p_regular, p_system
from .struct import Struct, struct_
from .value import pack_value_data

if TYPE_CHECKING:
    from bench.language import ChangeCategory, ClientOrigin, Run, Supergraph

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@struct_(StructType.EDIT_OPERATION)
class EditOperation(Struct):
    """An edit operation."""

    type: EditOperationType = p_system(30)
    key: str = p_system(31)

    new_value_packed: Any | None = p_regular(40, primitive_type=PrimitiveType.JSON)


@struct_(StructType.EDIT)
class Edit(Struct):
    """
    An Edit to a Node.
    """

    # core
    id: str = p_system(
        2,
        description="Unique identifier for the Edit within a Session.",
    )
    type: EditType = p_system(30, description="Type of Edit.")
    node: Node = p_system(31, description="Which Node.")
    edited_at: datetime = p_system(33, description="When the Edit was made.")
    old_edited_at: Optional[datetime] = p_system(
        34,
        default=None,
        description="The timestamp of the Edit being undone with this Edit.",
    )

    # content
    node_data: AnyNodeData | None = p_system(40, primitive_type=None, is_node_data=True)
    operations: list[EditOperation] = p_system(
        41,
        description="The operations to perform on the Node",
    )

    # meta
    scope: Scope = p_system(60, description="Enclosing scope of the Edit.")
    change_key: UUID | None = p_system(61, description="The Change that this Edit is part of.")
    category: "ChangeCategory | None" = p_system(
        62, description="Optional classification for the Edit."
    )
    subject: Optional[Subject] = p_system(63, description="Who made the Edit.")
    origin: "ClientOrigin | None" = p_system(64, description="Where the Edit came from.")


@dataclasses.dataclass(slots=True)
class EditEvent:
    """
    A compact representation of an Edit that we summarize into actual Edits on flush.
    """

    node: Node
    type: EditType
    subject_ptr: NodeReference | None
    origin: ClientOriginData | None
    run: "Run | None"
    now: datetime
    scope: GraphScopeData
    node_data: AnyNodeData | None = None
    operations: list[EditOperationData] | None = None

    def __str__(self):
        return f"{self.type.bench_name} {self.node!r}"

    def __repr__(self):
        return f"<EditEvent {self}>"


def apply_edit_operation(node: Node, op: EditOperationData, validate: bool) -> None:
    raise NotImplementedError


@tracer.start_as_current_span("graph.edit_graph")
def edit_graph(
    graph: Graph,
    supergraph: "Supergraph",
    edits: Collection[EditData],
    *,
    include_removed: bool,
    validate: bool,
    ignore_missing: bool = False,
) -> None:
    """Applies the edits to the graph (in place!)."""
    trace.get_current_span().set_attribute("edits", len(edits))

    from bench.proto import wiring

    for edit in edits:
        edit_type = cast(EditType, edit.type)
        node_id = UUID(edit.node_ptr.id)

        if edit_type in (EditType.CREATE, EditType.UPSERT) or (
            not include_removed
            and (edit_type == EditType.UNARCHIVE or edit_type == EditType.RESTORE)
        ):
            assert edit.HasField("node_data"), f"missing node_data for {edit!r}"
            node_data = wiring.unwrap_some_node(edit.node_data)
            # inline implicit metadata
            if edit_type == EditType.CREATE or edit_type == EditType.UPSERT:
                node_data.created_at.CopyFrom(edit.edited_at)
            node_data.updated_at.CopyFrom(edit.edited_at)
            node_data.ClearField("deleted_at")
            if edit.subject_ptr.metatype != 0:
                if edit_type == EditType.CREATE or edit_type == EditType.UPSERT:
                    node_data.created_by_ptr.CopyFrom(edit.subject_ptr)
                node_data.updated_by_ptr.CopyFrom(edit.subject_ptr)
            # unpack
            node = wiring.unpack_builtin_object(
                node_data, graph=graph, supergraph=supergraph, expect=Node
            )
            # NOTE :Robustness: should edit_graph ignore new nodes without parents? :RichGraph (like in bench-web)
            if edit_type == EditType.CREATE or node.id not in graph:
                graph.add(node)
            else:
                graph.update(node)
        elif edit_type == EditType.ERASE or (
            not include_removed and (edit_type == EditType.ARCHIVE or edit_type == EditType.DELETE)
        ):
            node = graph.get(node_id)
            if node is None:
                if not ignore_missing:
                    raise LookupError(
                        f"missing node {wiring.describe_node_ptr(edit.node_ptr)} for remove: {wiring.describe_edit(edit)}"
                    )
                continue
            graph.remove(node)
        else:
            # some update
            node = graph.get(node_id)
            if node is None:
                if not ignore_missing:
                    raise LookupError(
                        f"missing node {wiring.describe_node_ptr(edit.node_ptr)} for update: {wiring.describe_edit(edit)}"
                    )
                continue

            # apply edit operations
            for op in edit.operations:
                apply_edit_operation(node, op, validate=validate)

            # implicit metadata
            node.updated_at = edit.edited_at.ToDatetime(tzinfo=pytz.utc)
            updated_by_ptr = (
                wiring.unpack_builtin_object_prop(
                    Node.get_property("updated_by"), edit.subject_ptr, supergraph=supergraph
                )
                if edit.subject_ptr.metatype != 0
                else None
            )
            node._do_set("updated_by_ptr", updated_by_ptr, track=False)
            if edit_type == EditType.ARCHIVE:
                node._do_set("archived_at", edit.edited_at, track=False)
            elif edit_type == EditType.UNARCHIVE:
                node._do_set("archived_at", None, track=False)
            elif edit_type == EditType.DELETE:
                node._do_set("deleted_at", edit.edited_at, track=False)
            elif edit_type == EditType.RESTORE:
                node._do_set("deleted_at", None, track=False)
            graph.update(node)


def apply_edit_operation_data(node: AnyNodeData, op: EditOperationData, is_prepass: bool) -> None:
    raise NotImplementedError


@tracer.start_as_current_span("graph.edit_data_graph")
def edit_data_graph(
    graph: GraphData,
    edits: Collection[EditData],
    *,
    include_removed: bool,
    is_prepass: bool = False,
    ignore_missing: bool = False,
) -> list[EditData] | None:
    """
    Applies the edits to the data graph (edited nodes are copied before update).
    During prepass, we do some extra work (before applying the edits within in the system):
      1. Update edits (and edit operations) with the ground truth state.
      2. Simplify hierarchical edits into flat set/clear edits (for storage).
    """
    trace.get_current_span().set_attribute("edits", len(edits))

    from bench.proto import wiring

    def _make_vignette(node: AnyNodeData) -> ChangeVignetteData:
        vignette = ChangeVignetteData(metatype=pb2.OBJECT_TYPE_CHANGE_VIGNETTE)
        if name := getattr(node, "name", None):
            vignette.name = name
        if (subtype := getattr(node, "type", None)) is not None:
            vignette.subtype = subtype
        if (icon := getattr(node, "icon", None)) is not None:
            vignette.icon.CopyFrom(icon)
        return vignette

    flat_edits: list[EditData] = []  # for prepass
    for edit in edits:
        edit_type = cast(EditType, edit.type)
        node_type = NodeType(edit.node_ptr.node_type)
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        node_id = edit.node_ptr.id
        assert node_id is not None, f"missing node id for {edit!r}"
        subject_ptr = edit.subject_ptr if edit.subject_ptr.metatype != 0 else None

        if edit_type in (EditType.CREATE, EditType.UPSERT) or (
            not include_removed
            and (edit_type == EditType.UNARCHIVE or edit_type == EditType.RESTORE)
            and not is_prepass
        ):
            # add
            assert edit.HasField("node_data"), f"missing node_data for {edit!r}"
            node = wiring.unwrap_some_node(edit.node_data)
            # inline implicit metadata
            if edit_type == EditType.CREATE or edit_type == EditType.UPSERT:
                node.created_at.CopyFrom(edit.edited_at)
            node.updated_at.CopyFrom(edit.edited_at)
            node.ClearField("deleted_at")
            if subject_ptr is not None:
                if edit_type == EditType.CREATE or edit_type == EditType.UPSERT:
                    node.created_by_ptr.CopyFrom(subject_ptr)
                node.updated_by_ptr.CopyFrom(subject_ptr)
            else:
                node.ClearField("created_by_ptr")
                node.ClearField("updated_by_ptr")
            # NOTE :Robustness: should edit_graph ignore new nodes without parents? :RichGraph (like in bench-web)
            if edit_type == EditType.CREATE or node.id not in graph:
                graph.add(node)
            else:
                graph.update(node)

            # prepass: update new_node and make vignette with new data
            if is_prepass:
                edit.ClearField("node_data")
                edit.node_data.CopyFrom(wiring.wrap_some_node(node))  # :EditData
                edit.vignette.CopyFrom(_make_vignette(node))
                flat_edits.append(edit)
        elif (
            edit_type == EditType.ERASE
            or (
                not include_removed
                and (edit_type == EditType.ARCHIVE or edit_type == EditType.DELETE)
            )
        ) and not is_prepass:
            # remove
            node = graph.get(node_id)
            if node is None:
                if not ignore_missing:
                    raise LookupError(
                        f"missing node {wiring.describe_node_ptr(edit.node_ptr)} for {wiring.describe_edit(edit)}"
                    )
                continue
            graph.remove(node)
        else:
            # update
            node = graph.get(node_id)
            if node is None:
                if not ignore_missing:
                    raise LookupError(
                        f"missing node {wiring.describe_node_ptr(edit.node_ptr)} for {wiring.describe_edit(edit)}"
                    )
                continue
            node = wiring.copy_struct(node)

            # prepass: make vignette with old data
            if is_prepass:
                edit.vignette.CopyFrom(_make_vignette(node))
                if edit.type not in (EditType.UPDATE, EditType.MOVE):
                    edit.node_data.CopyFrom(wiring.wrap_some_node(node))

            # apply edit operations
            if edit_type in (EditType.UPDATE, EditType.MOVE):
                for op in edit.operations:
                    apply_edit_operation_data(node, op, is_prepass=is_prepass)

                # prepass: convert hierarchical edits into flat set/clear edits
                if is_prepass:
                    properties = {int(op.path[0]) for op in edit.operations}
                    flat_operations: list[EditOperationData] = []
                    for prop_id in properties:
                        prop = node_cls.__properties_by_id__.get(prop_id)
                        if prop is None:
                            continue
                        if prop.ptr_prop:
                            prop = prop.ptr_prop
                        if prop.is_optional_scalar and not node.HasField(prop.name):
                            new_value_packed = None
                            op_type = pb2.EDIT_OPERATION_TYPE_CLEAR
                        else:
                            new_value = getattr(node, prop.name)
                            new_value_packed = pack_value_data(new_value, prop.type_info)
                            op_type = pb2.EDIT_OPERATION_TYPE_SET
                        flat_op = EditOperationData(
                            metatype=pb2.OBJECT_TYPE_EDIT_OPERATION,
                            path=[str(prop_id)],
                            type=op_type,
                            new_value_packed=wiring.pack_proto_json(new_value_packed),
                        )
                        flat_operations.append(flat_op)
                    flat_edit = wiring.copy_struct(edit)
                    flat_edit.ClearField("operations")
                    flat_edit.operations.extend(flat_operations)
                    flat_edits.append(flat_edit)
            elif is_prepass:
                flat_edits.append(edit)

            # implicit metadata
            node.updated_at.CopyFrom(edit.edited_at)
            if edit.subject_ptr.metatype != 0:
                node.updated_by_ptr.CopyFrom(edit.subject_ptr)
            else:
                node.ClearField("updated_by_ptr")
            if edit_type == EditType.ARCHIVE:
                node.archived_at.CopyFrom(edit.edited_at)
            elif edit_type == EditType.UNARCHIVE:
                node.ClearField("archived_at")
            elif edit_type == EditType.DELETE:
                node.deleted_at.CopyFrom(edit.edited_at)
            elif edit_type == EditType.RESTORE:
                node.ClearField("deleted_at")

            graph.update(node)

    return flat_edits if is_prepass else None

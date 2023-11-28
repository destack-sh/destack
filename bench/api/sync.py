import functools
import inspect
from inspect import Signature
from typing import Any, Callable, Optional, Sequence
from uuid import UUID

import strawberry
import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import PermissionDenied
from strawberry.types import Info

from bench import models
from bench.api.auth import has_module_node_access
from bench.api.type import PMT, EditType
from bench.api.utils import get_client_origin_from_info, wrap_exceptions
from bench.language import wire
from bench.language.edit import EditData, EditKind, NodeType
from bench.models import ModuleAccessLevel, packer
from bench.msg import NMessageType
from bench.msg.core import NMessage, request
from bench.msg.messages import RepWriteEditsPayload, ReqWriteEditsPayload
from bench.worker.edit import input_to_gql_jsonable

logger = structlog.get_logger(__name__)

INPUT_CLASS_BY_TYPE = {}


#
#  Note that this is all annoying and terrible and will be ripped out with :BE-114.
#


class BatchEditInput:
    def unbatch(self) -> list:
        raise NotImplementedError


def bench_edit(
    type: EditType | PMT,
    *,
    batch: bool = False,
    skip_auth_check: bool = False,
    register: bool = True,
    extensions: Optional[Sequence[object]] = None,
    return_transform: Callable[[Any], Any] = None,
):
    """
    A wrapper for a Bench module edit, applied via the runtime host. This will be ripped out soon.
    """

    extensions = extensions or []

    def make_resolver(func):
        needs_info = "info" in func.__annotations__
        if register:
            _register_edit(type, func)

        @functools.wraps(func)
        async def wrapped_edit(self, info: Info, *args, **kwargs):
            if needs_info:
                kwargs["info"] = info
            ret = await sync_to_async(func)(self, *args, **kwargs)
            if batch:
                # assumes things property on any returned batches (see ThingBatch)
                things = ret.things
                if len(things) == 0:
                    raise RuntimeError("batch edit returned empty batch")
                thing = things[0]
            else:
                thing = ret
                things = [thing]

            # validate (ignoring constraints; 'revision' field which may be an F expression)
            access = await sync_to_async(has_module_node_access)(
                info, thing, ModuleAccessLevel.Edit
            )
            if access and access.project_version.committed or not skip_auth_check and not access:
                raise PermissionDenied("User cannot do this.")
            # (not doing actual validation here anymore since it can't run async, fine until :BE-114)
            # thing.full_clean(
            #     validate_unique=False, validate_constraints=False, exclude=["revision"]
            # )

            # map api edit to actual edit
            origin = get_client_origin_from_info(info)
            api_input = kwargs.get("input")
            if batch:
                inputs = api_input.unbatch()
            else:
                inputs = [api_input]
            edits = []
            for input, thing in zip(inputs, things):
                edit = map_edit_from_api(type, input, thing, access.project_version)
                edits.append(edit)

            # edit through runtime
            rep: NMessage[RepWriteEditsPayload] = await request(
                NMessageType.WRITE_EDITS,
                ReqWriteEditsPayload(
                    module_id=access.project_version.id, client=origin, edits=edits
                ),
            )
            if not rep.p.success:
                raise RuntimeError(f"failed to write edits: {rep.p.error}")

            # use returned nodes as return value (assuming their values, ignore any other new nodes)
            edited_nodes_by_id: dict[UUID, wire.NodeData] = {n.id: n for n in rep.p.nodes}
            for thing in things:
                updated_node = edited_nodes_by_id[thing.id]
                for key in updated_node.__dict__.keys():
                    if hasattr(thing, key) and getattr(thing, key) != getattr(updated_node, key):
                        thing.__dict__[key] = getattr(updated_node, key)
            if batch:  # restore batch wrapper (convert to kwargs)
                things_key = "statements" if type.node_type == NodeType.STATEMENT else "records"
                ret = ret.__class__(**{things_key: things})
            else:
                ret = things[0]
            if return_transform:
                ret = return_transform(ret)
            return ret

        # add info to wrapped_edit function signature if missing
        _add_info_parameter(func, wrapped_edit)

        return strawberry.mutation(wrap_exceptions(wrapped_edit), extensions=extensions)

    return make_resolver


def _register_edit(type, func):
    if type in INPUT_CLASS_BY_TYPE:
        raise RuntimeError(f"type {type} is registered to {INPUT_CLASS_BY_TYPE[type]}")
    input_class = func.__annotations__["input"]
    INPUT_CLASS_BY_TYPE[type] = input_class


def _add_info_parameter(original: callable, wrapped: callable):
    """Adds an 'info' Parameter to a wrapped function signature if missing."""
    if "info" not in wrapped.__annotations__:
        wrapped.__annotations__["info"] = Info
        original_signature = Signature.from_callable(original)
        original_parameters = list(original_signature.parameters.values())
        info_arg = inspect.Parameter(
            "info", inspect.Parameter.POSITIONAL_OR_KEYWORD, annotation=Info
        )
        wrapped.__signature__ = original_signature.replace(
            parameters=original_parameters + [info_arg]
        )


def map_edit_from_api(
    type: EditType,
    input: Any,
    thing: models.CrudNode | wire.RecordData,
    project_v: models.ProjectVersion,
) -> EditData:
    """
    Remap/create API multiplayer edit for other clients and internals.
    Returns both the internal and API edits to publish.
    """
    edit = EditData(
        type=EditType(type.kind + "_" + type.node_type.caps_name),
        project_version_id=project_v.id,
        revision=thing.revision,
        input=input_to_gql_jsonable(input),
        thing=thing,
    )
    edit.node = packer.pack_node_flat(thing) if not isinstance(thing, wire.NodeData) else thing
    # guesstimate changed properties
    edit.properties = [
        k for k in input.__dict__.keys() if k in edit._node.__dict__ and k not in ("id", "ck")
    ]
    if edit.kind in (EditKind.SOFT_DELETE, EditKind.RESTORE):
        edit.properties.append("deleted_at")  # not part of input
    # map source file/statement
    if isinstance(thing, models.File):
        edit.file_id = thing.id
        edit.statement_id = None
    elif isinstance(thing, models.Statement):
        edit.file_id = thing.file_id
        edit.statement_id = thing.id
    elif isinstance(thing, (models.Field, models.Tagging, models.Trigger)):
        edit.file_id = thing.statement.file_id
        edit.statement_id = thing.statement_id
    elif isinstance(thing, wire.RecordData):
        edit.file_id = None  # ?
        edit.statement_id = thing.parent_id
    else:
        raise TypeError(f"thing is not a project thing: {thing}")
    return edit

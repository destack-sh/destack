import functools
import inspect
from inspect import Signature
from typing import Any, Optional, Sequence

import strawberry
import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import PermissionDenied
from strawberry.types import Info

from bench import models
from bench.api.auth import has_module_node_access
from bench.api.type import MET, PMT
from bench.api.utils import get_client_origin_from_info, wrap_exceptions
from bench.language.edit import EditData
from bench.models import ModuleAccessLevel, packer
from bench.msg import NMessageType
from bench.msg.core import NMessage, request
from bench.msg.messages import RepWriteEditsPayload, ReqWriteEditsPayload
from bench.search import mirror
from bench.worker.edit import MutableThing, input_to_gql_jsonable

logger = structlog.get_logger(__name__)

INPUT_CLASS_BY_TYPE = {}


class BatchEditInput:
    def unbatch(self) -> list:
        raise NotImplementedError


def db_edit(
    type: MET | PMT,
    *,
    batch: bool = False,
    skip_auth_check: bool = False,
    register: bool = True,
    extensions: Optional[Sequence[object]] = None,
):
    """
    A module in-DB edit of a specific type
    Handles auth, revision bumping and edit pub. To be used as a decorator.

    For batch edits this does not handle revision bumping,
     and assumes that all things belong to the same project (only checks committed for one).

    Assumes that your wrapped func is either marked atomic or does not save changes itself.
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
            # nocheckin: 2. reroute db_edits to runtime host
            origin = get_client_origin_from_info(info)
            api_input = kwargs.get("input")
            if batch:
                inputs = api_input.unbatch()
            else:
                inputs = [api_input]
            edits = []
            for input, thing in zip(inputs, things):
                input = input_to_gql_jsonable(input)
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

            # use returned nodes as return value
            # nocheckin: this mapping is almost definitely wrong
            if batch:
                ret = ret.__class__(rep.p.nodes)
            else:
                ret = rep.p.nodes[0]
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
    """Adds an 'info' parameter to a wrapped function signature if missing."""
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
    type: MET, input: Any, thing: MutableThing, project_v: models.ProjectVersion
) -> EditData:
    """
    Remap/create API multiplayer edit for other clients and internals.
    Returns both the internal and API edits to publish.
    """
    edit = EditData(
        type=MET(type.kind + "_" + type.mnt.caps_name),
        project_version_id=project_v.id,
        revision=thing.revision,
        input=input,
        thing=thing,
    )
    if isinstance(thing, models.File):
        edit.file_id = thing.id
        edit.statement_id = None
    elif isinstance(thing, models.Statement):
        edit.file_id = thing.file_id
        edit.statement_id = thing.id
    elif isinstance(thing, (models.Field, models.Record, models.Tagging, models.Trigger)):
        edit.file_id = thing.statement.file_id
        edit.statement_id = thing.statement_id
    else:
        raise TypeError(f"thing is not a project thing: {thing}")
    if isinstance(thing, mirror.Document):  # os indexed Document
        edit.node = mirror.pack_node_flat(thing)
    else:
        edit.node = packer.pack_node_flat(thing)
    return edit

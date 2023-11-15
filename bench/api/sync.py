import functools
import inspect
from inspect import Signature
from typing import Any, Optional, Sequence

import strawberry_django
import structlog
from django.core.exceptions import PermissionDenied
from django.db import transaction
from strawberry.types import Info

from bench.api.auth import has_module_node_access
from bench.api.type import MET, PMT
from bench.api.utils import get_client_origin_from_info, wrap_exceptions
from bench.language import Statement
from bench.language.edit import EditData
from bench.models import ModuleAccessLevel, ProjectVersion
from bench.msg.core import publish_soon
from bench.msg.messages import (
    ClientOrigin,
    ModuleChangedPayload,
    NMessageType,
)
from bench.server.search import write_edits_to_os
from bench.worker.edit import MutableThing, input_to_gql_jsonable, map_edit_from_api

logger = structlog.get_logger(__name__)

INPUT_CLASS_BY_TYPE = {}


class BatchEditInput:
    def unbatch(self) -> list:
        raise NotImplementedError


def db_edit(
    type: MET | PMT,
    *,
    atomic: bool = False,
    batch: bool = False,
    skip_save: bool = False,
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
        def wrapped_edit(self, info: Info, *args, **kwargs):
            if needs_info:
                kwargs["info"] = info
            ret = func(self, *args, **kwargs)
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
            access = has_module_node_access(info, thing, ModuleAccessLevel.Edit)
            if access and access.project_version.committed or not skip_auth_check and not access:
                raise PermissionDenied("User cannot do this.")
            thing.full_clean(
                validate_unique=False, validate_constraints=False, exclude=["revision"]
            )

            # dual write, publish and track edit
            # nocheckin: 2. reroute db_edits to runtime host
            origin = get_client_origin_from_info(info)
            # api_edits, internal_edits = publish_tracked_edit(
            #     access.project_version, origin, type, kwargs.get("input"), things, batch
            # )
            # write_edits_to_os(access.project_version, internal_edits, refresh=False)

            return ret

        # add info to wrapped_edit function signature if missing
        _add_info_parameter(func, wrapped_edit)

        # wrap in atomic if needed
        if atomic:

            @functools.wraps(wrapped_edit)
            def wrapped_atomic(*args, **kwargs):
                with transaction.atomic():
                    return wrapped_edit(*args, **kwargs)

            rewrapped = wrapped_atomic
        else:
            rewrapped = wrapped_edit

        return strawberry_django.mutation((wrap_exceptions(rewrapped)), extensions=extensions)

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


def publish_tracked_edit(
    project_v: ProjectVersion,
    origin: ClientOrigin,
    type: MET,
    original_input: Any,
    things: list[MutableThing],
    batch: bool,
    statement: Optional[Statement] = None,
) -> tuple[list[EditData], list[EditData]]:
    """Publish edits."""
    if type in (MET.PASTE_FILE, MET.PASTE_STATEMENT):
        inputs = [original_input] * len(things)  # not directly unbatchable
    elif batch:
        inputs = original_input.unbatch()
    else:
        inputs = [original_input]
    api_edits = []
    internal_edits = []
    for input, thing in zip(inputs, things):
        input = input_to_gql_jsonable(input)
        internal, public = map_edit_from_api(type, input, thing, project_v, statement)
        api_edits.extend(public)
        internal_edits.extend(internal)

    publish_soon(
        NMessageType.MODULE_CHANGED,
        ModuleChangedPayload(
            project_id=project_v.project_id,
            module_id=project_v.id,
            origins=[origin],
            edits=api_edits,
        ),
    )
    return api_edits, internal_edits

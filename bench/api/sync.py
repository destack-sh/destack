import functools
import inspect
import typing
from inspect import Signature
from typing import Any, Optional, Sequence

import posthog
import strawberry_django
import structlog
from django.db import transaction
from django.db.models import F
from strawberry.types import Info

from bench.api.auth import check_module_node_access
from bench.api.type import MMT, PMT
from bench.api.utils import get_client_origin_from_info, get_user_from_info, wrap_exceptions
from bench.language import Statement
from bench.language.mutate import ModuleMutation, ModuleMutationKind
from bench.models import ProjectVersion
from bench.msg.core import publish_soon
from bench.msg.messages import (
    ClientOrigin,
    ModuleChangedPayload,
    ModuleInternalChangedPayload,
    NMessageType,
)
from bench.opensearch.index import write_mutations_to_os
from bench.worker.mutate import MutableThing, input_to_gql_jsonable, map_mutation_from_api

logger = structlog.get_logger(__name__)

INPUT_CLASS_BY_TYPE = {}


class BatchMutationInput:
    def unbatch(self) -> list:
        raise NotImplementedError


def tracked_db_mutation(
    type: MMT | PMT,
    *,
    atomic: bool = False,
    batch: bool = False,
    skip_save: bool = False,
    skip_auth_check: bool = False,
    register: bool = True,
    extensions: Optional[Sequence[object]] = None,
):
    """
    A module in-DB mutation of a specific type
    Handles auth, revision bumping and mutation pub. To be used as a decorator.

    For batch mutations this does not handle revision bumping,
     and assumes that all things belong to the same project (only checks committed for one).

    Assumes that your wrapped func is either marked atomic or does not save changes itself.
    """

    extensions = extensions or []

    def make_resolver(func):
        needs_info = "info" in func.__annotations__
        if register:
            _register_mutation(type, func)

        @functools.wraps(func)
        def wrapped_mutation(self, info: Info, *args, **kwargs):
            if needs_info:
                kwargs["info"] = info
            ret = func(self, *args, **kwargs)
            if batch:
                # assumes things property on any returned batches (see ThingBatch)
                things = ret.things
                if len(things) == 0:
                    raise RuntimeError("batch mutation returned empty batch")
                thing = things[0]
            else:
                thing = ret
                things = [thing]

            # validate (ignoring constraints; 'revision' field which may be an F expression)
            access = check_module_node_access(info, thing, check_auth=not skip_auth_check)
            thing.full_clean(
                validate_unique=False, validate_constraints=False, exclude=["revision"]
            )

            # save and bump revision (if not new or batched)
            if not batch and not skip_save:
                is_new = thing._state.adding or type.kind == ModuleMutationKind.CREATE
                if not is_new:
                    thing.revision = F("revision") + 1
                thing.save()
                if not is_new:
                    thing.refresh_from_db(fields=["revision"])  # @Performance: inefficient?

            # dual write, publish and track mutation
            origin = get_client_origin_from_info(info)
            api_mutations, internal_mutations = publish_tracked_mutation(
                access.project_version, origin, type, kwargs.get("input"), things, batch
            )
            write_mutations_to_os(access.project_version, api_mutations, wait=False)
            track_mutation_for_analytics(type, access.project_version, things, batch, info)

            return ret

        # add info to wrapped_mutation function signature if missing
        _add_info_parameter(func, wrapped_mutation)

        # wrap in atomic if needed
        if atomic:

            @functools.wraps(wrapped_mutation)
            def wrapped_atomic(*args, **kwargs):
                with transaction.atomic():
                    return wrapped_mutation(*args, **kwargs)

            rewrapped = wrapped_atomic
        else:
            rewrapped = wrapped_mutation

        return strawberry_django.mutation((wrap_exceptions(rewrapped)), extensions=extensions)

    return make_resolver


def tracked_os_mutation(
    type: MMT,
    *,
    batch: bool = False,
    register: bool = True,
):
    """
    A module out-of-DB mutations that uses OpenSearch as the source of truth.
    Does NOT handle auth, revision bumping or mutation pub. To be used as a decorator.
    Currently only used for Record mutations, will need to consolidate/remove this
    when we put records into the DB (again) for :DbRecord
    """

    def make_resolver(func):
        needs_info = "info" in func.__annotations__
        if register:
            _register_mutation(type, func)

        return_type = func.__annotations__["return"]
        return_type_args = typing.get_args(return_type)
        return_type = return_type_args[0]

        @functools.wraps(func)
        def wrapped_mutation(self, info: Info, *args, **kwargs):
            if needs_info:
                kwargs["info"] = info
            project_v, statement, ret = func(self, *args, **kwargs)
            if batch:
                # assumes things property on any returned batches (see ThingBatch)
                things = ret.things
            else:
                things = [ret]

            # dual write, publish and track mutation
            origin = get_client_origin_from_info(info)
            publish_tracked_mutation(
                project_v, origin, type, kwargs.get("input"), things, batch, statement=statement
            )
            track_mutation_for_analytics(type, project_v, things, batch, info)

            ret = return_type.from_os(ret)
            return ret

        return strawberry_django.mutation(wrap_exceptions(wrapped_mutation))

    return make_resolver


def _register_mutation(type, func):
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


def publish_tracked_mutation(
    project_v: ProjectVersion,
    origin: ClientOrigin,
    type: MMT,
    original_input: Any,
    things: list[MutableThing],
    batch: bool,
    statement: Optional[Statement] = None,
) -> tuple[list[ModuleMutation], list[ModuleMutation]]:
    """Publish mutations."""
    if type in (MMT.PASTE_FILE, MMT.PASTE_STATEMENT):
        inputs = [original_input] * len(things)  # not directly unbatchable
    elif batch:
        inputs = original_input.unbatch()
    else:
        inputs = [original_input]
    api_mutations = []
    internal_mutations = []
    for input, thing in zip(inputs, things):
        input = input_to_gql_jsonable(input)
        internal, public = map_mutation_from_api(type, input, thing, project_v, statement)
        api_mutations.extend(public)
        internal_mutations.extend(internal)

    publish_soon(
        NMessageType.MODULE_CHANGED,
        ModuleChangedPayload(
            project_id=project_v.project_id,
            module_id=project_v.id,
            origins=[origin],
            mutations=api_mutations,
        ),
    )
    if internal_mutations:
        publish_soon(
            NMessageType.MODULE_INTERNAL_CHANGED,
            ModuleInternalChangedPayload(
                project_id=project_v.project_id,
                module_id=project_v.id,
                origins=[origin],
                mutations=internal_mutations,
            ),
        )
    return api_mutations, internal_mutations


def track_mutation_for_analytics(
    type: MMT, project_version: ProjectVersion, things, batch: bool, info: Info
):
    """Tracks a project mutation for Posthog analytics."""
    user = get_user_from_info(info)
    if user.is_anonymous:
        return

    if "FILE" in type.value:
        properties = {"file_id": things[0].id, "name": things[0].name, "path": things[0].path}
    elif "STATEMENT" in type.value or "SYMBOL" in type.value:
        properties = {
            "statement_id": things[0].id,
            "name": things[0].name,
            "order_key": things[0].order_key,
            "file_id": things[0].file_id,
        }
    elif "FIELD" in type.value:
        properties = {
            "field_id": things[0].id,
            "name": things[0].name,
            "order_key": things[0].order_key,
        }
    elif "RECORD" in type.value:
        properties = {"record_id": things[0].id, "order_key": things[0].order_key}
    else:
        return  # just ignore for now

    project_properties = {
        "project_id": project_version.project_id,
        "project_version_id": project_version.id,
        "project_path": project_version.project.path,
    }
    normalized_type = type.name.lower().replace("_", " ")
    posthog.capture(
        str(user.id), normalized_type, properties={batch: batch, **properties, **project_properties}
    )

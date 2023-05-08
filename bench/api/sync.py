import functools
import inspect
from inspect import Signature
from typing import Any, Optional, Sequence, cast

import posthog
import structlog
from django.core.exceptions import PermissionDenied
from django.db import transaction
from django.db.models import F
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.utils.resolvers import async_safe

from bench import models
from bench.api.auth import check_can_write_project
from bench.api.util import wrap_exceptions
from bench.language.mutate import MMT
from bench.models import ProjectVersion
from bench.msg import NMessageType
from bench.msg.core import publish_soon
from bench.msg.messages import ClientOrigin, ModuleChangedPayload, ModuleInternalChangedPayload
from bench.runtime.mutate import MutableThing, input_to_gql_jsonable, map_mutation_from_public
from bench.settings import SEND_API_PUB_MSG

logger = structlog.get_logger(__name__)

INPUT_CLASS_BY_MMT = {}


class BatchMutationInput:
    def unbatch(self) -> list:
        raise NotImplementedError


def project_mutation(
    type: MMT,
    *,
    atomic: bool = False,
    batch: bool = False,
    skip_auth_check: bool = False,
    register: bool = True,
    directives: Optional[Sequence[object]] = None,
):
    """
    A project content mutation (CUD) of a specific type.
    Handles auth, revision bumping and mutation pub. Must be used as a decorator.

    For batch mutations does not handle revision bumping,
     and assumes that all things belong to the same project (only checks committed for one).

    :ProjectContentSync
    Assumes that your wrapped func is either marked atomic or does not save changes itself.
    """

    directives = directives or []

    def make_resolver(func):
        needs_info = "info" in func.__annotations__

        # register mutation type to input class
        if register:
            if type in INPUT_CLASS_BY_MMT:
                raise RuntimeError(f"type {type} is registered to {INPUT_CLASS_BY_MMT[type]}")
            input_class = func.__annotations__["input"]
            INPUT_CLASS_BY_MMT[type] = input_class

        @functools.wraps(func)
        def wrapped_mutation(self, info: Info, *args, **kwargs):
            if needs_info:
                kwargs["info"] = info
            ret = func(self, *args, **kwargs)
            if batch:
                # assumes things property on any returned batches (see StatementBatch)
                things = ret.things
                if len(things) == 0:
                    raise RuntimeError("batch mutation returned empty batch")
                thing = things[0]
            else:
                thing = ret
                things = [thing]

            if isinstance(thing, (models.File, models.Statement)):
                project_version = models.ProjectVersion.objects.only("committed_at").get(
                    id=thing.project_version_id
                )
            elif isinstance(thing, (models.SimpleTypeNode, models.DatasetRecord)):
                # TODO @Performance: fetching project_version for statement mutation is inefficient
                project_version = models.ProjectVersion.objects.only("committed_at").get(
                    id=thing.statement.project_version_id
                )
            else:
                raise TypeError(f"thing is not a project thing: {thing}")

            # check that containing project is not committed
            if project_version.committed:
                raise PermissionDenied("cannot mutate committed project version")

            # check auth
            if not skip_auth_check:
                check_can_write_project(info, thing)

            # validate (ignoring uniqueness, constraints, and 'revision' field which may be an F expression)
            thing.full_clean(
                validate_unique=False, validate_constraints=False, exclude=["revision"]
            )

            # save and bump revision (if not new or batched)
            if not batch:
                is_new = thing._state.adding
                if not is_new:
                    thing.revision = F("revision") + 1
                thing.save()
                if not is_new:
                    thing.refresh_from_db(fields=["revision"])  # @Performance: inefficient?

            # publish and track mutation
            client_id = info.context.request.scope["session"]["client_id"]
            client_nonce = info.context.request.headers.get("x-client-nonce")
            origin = ClientOrigin("user", client_id, client_nonce)
            pub_project_mutation(origin, type, kwargs.get("input", None), things, batch=batch)
            track_project_mutation(type, project_version, things, batch, info)

            return ret

        # add info to wrapped_mutation function signature if missing
        if "info" not in wrapped_mutation.__annotations__:
            wrapped_mutation.__annotations__["info"] = Info
            original_signature = Signature.from_callable(func)
            original_parameters = list(original_signature.parameters.values())
            info_arg = inspect.Parameter(
                "info", inspect.Parameter.POSITIONAL_OR_KEYWORD, annotation=Info
            )
            wrapped_mutation.__signature__ = original_signature.replace(
                parameters=original_parameters + [info_arg]
            )

        # wrap in atomic if needed
        if atomic:

            @functools.wraps(wrapped_mutation)
            def wrapped_atomic(*args, **kwargs):
                with transaction.atomic():
                    return wrapped_mutation(*args, **kwargs)

            rewrapped = wrapped_atomic
        else:
            rewrapped = wrapped_mutation

        return gql.mutation(async_safe(wrap_exceptions(rewrapped)), directives=directives)

    return make_resolver


def pub_project_mutation(
    origin: ClientOrigin, type: MMT, original_input: Any, things: list[MutableThing], batch: bool
):
    """Publish mutations."""
    if not things or not SEND_API_PUB_MSG:
        return

    if type in (MMT.PASTE_FILE, MMT.PASTE_STATEMENT):
        inputs = [original_input] * len(things)  # not directly unbatchable
    elif batch:
        inputs = original_input.unbatch()
    else:
        inputs = [original_input]
    mutations = []
    internal_mutations = []
    for input, thing in zip(inputs, things):
        input = input_to_gql_jsonable(input)
        internal, public = map_mutation_from_public(type, input, thing)
        mutations.extend(public)
        internal_mutations.extend(internal)

    project_version_id = mutations[0].project_version_id
    publish_soon(
        NMessageType.MODULE_CHANGED,
        ModuleChangedPayload(module_id=project_version_id, client=origin, mutations=mutations),
    )
    if internal_mutations:
        publish_soon(
            NMessageType.MODULE_INTERNAL_CHANGED,
            ModuleInternalChangedPayload(
                module_id=project_version_id, client=origin, mutations=internal_mutations
            ),
        )


def track_project_mutation(
    type: MMT, project_version: ProjectVersion, things, batch: bool, info: Info
):
    """Tracks a project mutation for Posthog analytics."""
    user = cast(models.User, info.context.request.scope["user"]._wrapped)
    if user.is_anonymous:
        return

    if "FILE" in type.value:
        properties = {"file_id": things[0].id, "name": things[0].name, "path": things[0].path}
    elif "STATEMENT" in type.value:
        properties = {
            "statement_id": things[0].id,
            "name": things[0].name,
            "order_key": things[0].order_key,
            "file_id": things[0].file_id,
        }
    elif "TYPE_NODE" in type.value:
        properties = {
            "simple_type_node_id": things[0].id,
            "name": things[0].name,
            "order_key": things[0].order_key,
        }
    elif "RECORD" in type.value:
        properties = {"dataset_record_id": things[0].id, "order_key": things[0].order_key}
    elif type == MMT.COMMIT:
        properties = {}
    else:
        properties = {}
        logger.warning("unknown_project_mutation", type=type)

    project_properties = {
        "project_id": project_version.project_id,
        "project_version_id": project_version.id,
        "project_path": project_version.project.path,
    }
    normalized_type = type.name.lower().replace("_", " ")
    posthog.capture(
        str(user.id), normalized_type, properties={batch: batch, **properties, **project_properties}
    )

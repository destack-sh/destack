import functools
import inspect
from collections import OrderedDict
from dataclasses import fields, is_dataclass
from datetime import datetime
from inspect import Signature
from typing import Any, Optional, Sequence, Union, cast
from uuid import UUID

import posthog
import structlog
from asgiref.sync import async_to_sync
from django.core.exceptions import PermissionDenied
from django.db import transaction
from django.db.models import F
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.utils.resolvers import async_safe

from bench import models
from bench.api.auth import CanWriteProject
from bench.api.util import wrap_exceptions
from bench.models import ProjectVersion, mapper
from bench.msg import NMessageType
from bench.msg.core import publish
from bench.msg.messages import ClientOrigin, ModuleChangedPayload
from bench.language.mutate import ModuleMutation, ModuleMutationType
from bench.settings import SEND_API_PUB_MSG

logger = structlog.get_logger(__name__)

MMT = ModuleMutationType


def project_mutation(
    type: MMT,
    *,
    atomic: bool = False,
    batch: bool = False,
    skip_auth_check: bool = False,
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
    if not skip_auth_check:
        directives.append(CanWriteProject())

    def make_resolver(func):
        needs_info = "info" in func.__annotations__

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

            # validate (ignoring uniqueness, constraints, and 'revision' field which may be an F expression)
            thing.full_clean(
                validate_unique=False, validate_constraints=False, exclude=["revision"]
            )

            if not batch:
                # save and bump revision (if not new or batched)
                is_new = thing._state.adding
                if not is_new:
                    thing.revision = F("revision") + 1
                thing.save()
                if not is_new:
                    thing.refresh_from_db(fields=["revision"])  # @Performance: inefficient?

            # TODO @Robustness @Broken: trigger pub_project_mutation after resolver
            #  Currently this is also triggered even if permission check (on ret) fails,
            #  because the permission check runs after the return value is computed.
            #  This also creates a race condition where the mutation may be published before it's written.
            client_id = info.context.request.scope["session"]["client_id"]
            # publish change
            pub_project_mutation(
                client_id=client_id, type=type, input=kwargs.get("input", None), things=things
            )
            # analytics
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
    client_id: UUID,
    type: MMT,
    input: Any,
    things: list[Union[models.File, models.Statement, models.SimpleTypeNode, models.DatasetRecord]],
):
    """Publish a project mutation to the project change pub socket."""
    if not things or not SEND_API_PUB_MSG:
        return
    mutations = []
    input = input_to_jsonable(input)  # original input is some dataclass
    for thing in things:
        if isinstance(thing, models.File):
            statement_id = None
            project_version_id = thing.project_version_id
            file_id = thing.id
        elif isinstance(thing, models.Statement):
            statement_id = thing.id
            project_version_id = thing.project_version_id
            file_id = thing.file_id
        elif isinstance(thing, (models.SimpleTypeNode, models.DatasetRecord)):
            project_version_id = thing.statement.project_version_id
            file_id = thing.statement.file_id
            statement_id = thing.statement_id
        else:
            raise TypeError(f"thing is not a project thing: {thing}")
        data = mapper.rmap_flat(thing)
        mutation = ModuleMutation(
            type=type,
            project_version_id=project_version_id,
            file_id=file_id,
            statement_id=statement_id,
            type_node_id=thing.id if isinstance(thing, models.SimpleTypeNode) else None,
            record_id=thing.id if isinstance(thing, models.DatasetRecord) else None,
            revision=thing.revision,
            input=input,
            data=data,
        )
        mutations.append(mutation)
    # TODO @Performance: using async_to_sync to publish mutation is inefficient
    async_to_sync(publish)(
        NMessageType.MODULE_CHANGED,
        ModuleChangedPayload(
            module_id=project_version_id,
            client=ClientOrigin("user", client_id),
            mutations=mutations,
        ),
    )


def track_project_mutation(
    type: MMT, project_version: ProjectVersion, things, batch: bool, info: Info
):
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


def input_to_jsonable(value: Any) -> Any:
    """Walk and transform a GraphQL input into a JSON object that could be used as an input."""
    if isinstance(value, GlobalID):
        return str(value)
    elif is_dataclass(value):
        data = OrderedDict()
        for field in fields(value):
            key = field.name
            data[key] = input_to_jsonable(getattr(value, key))
        return data
    elif isinstance(value, (list, tuple)):
        return [input_to_jsonable(item) for item in value]
    elif isinstance(value, dict):  # JSON
        return value
    elif isinstance(value, (int, float, str, bool, type(None))):
        return value
    elif isinstance(value, (UUID, datetime)):
        return str(value)
    else:
        raise TypeError(f"unexpected value: {value}")

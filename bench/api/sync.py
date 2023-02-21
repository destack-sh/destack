import functools
import typing
from typing import AsyncGenerator, Optional, Sequence, Union

import structlog
import zmq
from django.core.exceptions import PermissionDenied, ValidationError
from django.db import IntegrityError, transaction
from django.db.models import F
from strawberry_django_plus import gql
from strawberry_django_plus.mutations.fields import _map_exception
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo
from strawberry_django_plus.utils.resolvers import async_safe

from bench import models
from bench.msg import ZMessageType, send_message, zmq_ctx_sync
from bench.msg.messages import ProjectVersionChangedPayload
from bench.msg.sync import ProjectMutation, ProjectMutationType
from bench.settings import SEND_API_PUB_MSG, ZMQ_API_PUB_ADDR

logger = structlog.get_logger(__name__)

# Bind pub addr to localhost if it's a wildcard.
ZMQ_API_PUB_ADDR = ZMQ_API_PUB_ADDR.replace("*", "localhost")

# sync because it's used in the synchronous API
project_change_pub_sync = zmq_ctx_sync.socket(zmq.PUB)
if SEND_API_PUB_MSG:
    logger.info("bind_sync", addr=ZMQ_API_PUB_ADDR)
    project_change_pub_sync.bind(ZMQ_API_PUB_ADDR)


PMT = ProjectMutationType


def map_exception(e: Exception) -> Union[OperationInfo, Exception]:
    # extend strawberry's _map_exception
    if isinstance(e, IntegrityError):
        e = ValidationError(e.args[0])
    return _map_exception(e)  # borrowed from strawberry_django_plus


def _wrap_exceptions(func):
    @functools.wraps(func)
    def wrapped(*args, **kwargs):
        try:
            return func(*args, **kwargs)
        except Exception as e:
            e = map_exception(e)
            if isinstance(e, OperationInfo):
                return e
            raise e

    return wrapped


def project_mutation(
    type: PMT, *, atomic: bool = False, directives: Optional[Sequence[object]] = ()
):
    """
    A project content mutation (CUD) of a specific type.
    Handles revision bumping and mutation publishing. Must be used as a decorator.
    :ProjectContentSync

    Assumes that your wrapped func is either marked atomic or does not save changes itself.
    """

    def make_resolver(func):
        # check that func returns a union with OperationInfo
        return_type = func.__annotations__.get("return")
        if return_type is None:
            raise TypeError(f"return type annotation required: {func}")
        return_type_args = typing.get_args(return_type)
        if OperationInfo not in return_type_args:
            raise TypeError(f"return must union with OperationInfo: {func}")

        @functools.wraps(func)
        def wrapped_mutation(*args, **kwargs):
            thing = func(*args, **kwargs)
            if not isinstance(thing, (models.File, models.Statement)):
                raise TypeError(f"thing must be File or Statement: {thing}")

            # check that containing project is not committed
            project_version = models.ProjectVersion.objects.only("committed_at").get(
                id=thing.project_version_id
            )
            if project_version.committed:
                raise PermissionDenied("cannot mutate committed project version")

            # validate
            thing.full_clean(validate_unique=False, validate_constraints=False)

            # save and bump revision (if not new)
            is_new = thing._state.adding
            if not is_new:
                thing.revision = F("revision") + 1
            thing.save()
            if not is_new:
                thing.refresh_from_db(fields=["revision"])  # @Performance: inefficient?

            # publish change
            pub_project_mutation(type, thing)

            return thing

        # wrap in atomic if needed
        if atomic:

            @functools.wraps(wrapped_mutation)
            def wrapped_atomic(*args, **kwargs):
                with transaction.atomic():
                    return wrapped_mutation(*args, **kwargs)

            rewrapped = wrapped_atomic
        else:
            rewrapped = wrapped_mutation

        return gql.mutation(async_safe(_wrap_exceptions(rewrapped)), directives=directives)

    return make_resolver


def pub_project_mutation(
    type: PMT,
    thing: Union[models.File, models.Statement],
    revision: Optional[int] = None,
):
    """Publish a project mutation to the project change pub socket."""
    if isinstance(thing, models.File):
        file_id = thing.id
        statement_id = None
    elif isinstance(thing, models.Statement):
        file_id = None
        statement_id = thing.id
    else:
        raise TypeError(f"thing must be File or Statement: {thing}")

    if not SEND_API_PUB_MSG:
        return
    mutation = ProjectMutation(
        type,
        project_version_id=thing.project_version_id,
        file_id=file_id,
        statement_id=statement_id,
        revision=revision,
    )
    send_message(
        project_change_pub_sync,
        ZMessageType.PROJECT_VERSION_CHANGED,
        ProjectVersionChangedPayload(thing.project_version_id, mutations=[mutation]),
    )


@gql.type
class ProjectSubscription:
    @gql.subscription
    async def project_changed(self, project_id: GlobalID) -> AsyncGenerator[ProjectMutation, None]:
        raise NotImplementedError

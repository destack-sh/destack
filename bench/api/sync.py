import functools
from typing import Optional, Sequence, Union

import structlog
from asgiref.sync import async_to_sync
from django.core.exceptions import PermissionDenied
from django.db import transaction
from django.db.models import F
from strawberry_django_plus import gql
from strawberry_django_plus.utils.resolvers import async_safe

from bench import models
from bench.api.auth import CanWriteProject
from bench.api.util import wrap_exceptions
from bench.msg import NMessageType
from bench.msg.core import publish
from bench.msg.messages import ProjectVersionChangedPayload
from bench.msg.sync import ProjectMutation, ProjectMutationType
from bench.settings import SEND_API_PUB_MSG

logger = structlog.get_logger(__name__)

PMT = ProjectMutationType


def project_mutation(
    type: PMT,
    *,
    atomic: bool = False,
    batch: bool = False,
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

    directives = directives or [CanWriteProject()]

    def make_resolver(func):
        @functools.wraps(func)
        def wrapped_mutation(*args, **kwargs):
            ret = func(*args, **kwargs)
            if batch:
                # assumes things property on any returned batches (see StatementBatch)
                things = ret.things
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

            # validate
            thing.full_clean(validate_unique=False, validate_constraints=False)

            if not batch:
                # save and bump revision (if not new or batched)
                is_new = thing._state.adding
                if not is_new:
                    thing.revision = F("revision") + 1
                thing.save()
                if not is_new:
                    thing.refresh_from_db(fields=["revision"])  # @Performance: inefficient?

            # TODO @Robustness @Performance: trigger pub_project_mutation after resolver
            #  Currently this is also triggered even if permission check (on ret) fails,
            #  because the permission check runs after the return value is computed.
            # publish change
            pub_project_mutation(type, things)

            return ret

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
    type: PMT,
    things: list[Union[models.File, models.Statement, models.SimpleTypeNode, models.DatasetRecord]],
    revision: Optional[int] = None,
):
    """Publish a project mutation to the project change pub socket."""
    if not things:
        return
    mutations = []
    for thing in things:
        if isinstance(thing, models.File):
            file_id = thing.id
            statement_id = None
            project_version_id = thing.project_version_id
        elif isinstance(thing, models.Statement):
            file_id = None
            statement_id = thing.id
            project_version_id = thing.project_version_id
        elif isinstance(thing, (models.SimpleTypeNode, models.DatasetRecord)):
            file_id = None
            statement_id = thing.statement_id
            project_version_id = thing.statement.project_version_id
        else:
            raise TypeError(f"thing is not a project thing: {thing}")

        if not SEND_API_PUB_MSG:
            return
        mutation = ProjectMutation(
            type,
            project_version_id=project_version_id,
            file_id=file_id,
            statement_id=statement_id,
            revision=revision,
        )
        mutations.append(mutation)
    # TODO @Performance: using async_to_sync to publish mutation is inefficient
    async_to_sync(publish)(
        NMessageType.PROJECT_VERSION_CHANGED,
        ProjectVersionChangedPayload(project_version_id, mutations=mutations),
    )

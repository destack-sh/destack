from typing import AsyncGenerator, Optional, Union, cast
from uuid import UUID

import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import PermissionDenied
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api import sync
from bench.api.auth import check_can_view_project_by_id
from bench.api.interp import Issue, ResolvedField
from bench.api.type import ProjectMutationType
from bench.api.utils import asafe_subscription, to_global_id, to_uuid
from bench.bench import mutate
from bench.bench.mutate import ModuleMutator
from bench.bench.wire import ModuleTree
from bench.models import packer
from bench.msg.core import NMessage, subscribe
from bench.msg.messages import ModuleChangedPayload, NMessageType, ProjectChangedPayload

logger = structlog.get_logger(__name__)


@gql.interface
class Change:
    id: UUID
    client_id: Optional[GlobalID]


@gql.type
class ProjectMutation:
    type: ProjectMutationType
    project_version_id: GlobalID


@gql.type
class ProjectChange(Change):
    id: UUID
    client_id: Optional[GlobalID]
    # individual mutations are not needed for now


ModuleMutationType = gql.enum(sync.MMT)


@gql.type
class ModuleMutation:
    type: ModuleMutationType
    project_version_id: GlobalID
    file_id: GlobalID
    statement_id: Optional[GlobalID]
    revision: Optional[int]
    input: Optional[JSON]
    data: Union[Issue, ResolvedField, None]


@gql.type
class ModuleChange(Change):
    id: UUID
    client_id: Optional[GlobalID]
    mutations: list[ModuleMutation]


# some (yet unused) scaffolding to understand project change sync interface


@gql.type
class CommentMutation:
    project_version_id: GlobalID
    comment_id: GlobalID
    file_id: Optional[GlobalID]
    statement_id: Optional[GlobalID]
    screen_id: Optional[GlobalID]
    tile_id: Optional[GlobalID]
    revision: Optional[int]
    input: Optional[JSON]


@gql.type
class CommentChange(Change):
    id: UUID
    client_id: Optional[GlobalID]
    mutations: list[CommentMutation]


@gql.type
class ScreenMutation:
    project_version_id: GlobalID
    screen_id: GlobalID
    tile_id: Optional[GlobalID]
    revision: Optional[int]
    input: Optional[JSON]


@gql.type
class ScreenChange(Change):
    id: UUID
    client_id: Optional[GlobalID]
    mutations: list[ScreenMutation]


def unpack_project_mutation(mutation: ProjectMutation) -> ProjectMutation:
    return ProjectMutation(
        type=mutation.type,
        project_version_id=to_global_id("ProjectVersion", mutation.project_version_id),
    )


def unpack_module_mutations(
    mutations: list[mutate.ModuleMutation], module: ModuleTree, project_v: models.ProjectVersion
) -> list[ModuleMutation]:
    unpacked_mutations = []
    unpacked_data = {}
    data_to_unpack = [m.data for m in mutations if m.data is not None]
    if data_to_unpack:
        for node in packer.unpack_nodes(project_v, module, data_to_unpack):
            unpacked_data[node.id] = node
    for m in mutations:
        unpacked_mutations = ModuleMutation(
            type=m.type,
            project_version_id=to_global_id("ProjectVersion", m.project_version_id),
            file_id=to_global_id("File", m.file_id),
            statement_id=to_global_id("Statement", m.statement_id),
            revision=m.revision,
            input=m.input,
            data=unpacked_data.get(m.data.id) if m.data is not None else None,
        )
    return unpacked_mutations


@gql.type
class MultiplayerSubscription:
    @asafe_subscription
    async def project_changed(
        self, info: Info, project_id: GlobalID
    ) -> AsyncGenerator[ProjectChange, None]:
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        project_id = UUID(project_id.node_id)
        log = logger.bind(project_id=project_id, user=user)
        client_id = to_uuid(info.context.request.scope["session"].get("client_id"))
        client_nonce = to_uuid(info.context.connection_params.get("X-Client-Nonce"))
        try:
            await sync_to_async(check_can_view_project_by_id)(user, project_id=project_id)
        except PermissionDenied:
            log.warning("project_changed.subscribe_denied", exc_info=True)
            return

        project_sub = await subscribe(
            f"{NMessageType.PROJECT_CHANGED}.{project_id}", payload_t=ProjectChangedPayload
        )
        log.info("project.listen")
        while True:
            change: NMessage[ProjectChangedPayload] = await project_sub.next_msg()
            if client_id is not None and change.p.has_origin(client_id, client_nonce):
                continue  # skip self
            log.debug(
                "project.update",
                id=change.id,
                origin_type=change.p.origin.type,
                origin_id=change.p.origin.id,
            )
            origin_id = (
                to_global_id("Client", change.p.origin.id)
                if change.p.origin.type == "user"
                else None
            )
            # individual mutations aren't needed yet
            yield ProjectChange(id=change.id, client_id=origin_id)

    @asafe_subscription
    async def module_changed(
        self, info: Info, project_version_id: GlobalID
    ) -> AsyncGenerator[ModuleChange, None]:
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        project_version_id = UUID(project_version_id.node_id)
        log = logger.bind(project_version_id=project_version_id, user=user)
        client_id = to_uuid(info.context.request.scope["session"].get("client_id"))
        client_nonce = to_uuid(info.context.connection_params.get("X-Client-Nonce"))
        try:
            await sync_to_async(check_can_view_project_by_id)(user, project_version_id)
        except PermissionDenied:
            log.warning("module_changed.subscribe_denied", exc_info=True)
            return

        # keep synced module state in memory for efficient updates
        # this may be inefficient/too much, we'll see
        project_version = await models.ProjectVersion.objects.aget(id=project_version_id)
        mutator = ModuleMutator(await sync_to_async(packer.pack_module)(project_version))
        module_sub = await subscribe(
            f"{NMessageType.MODULE_CHANGED}.{project_version_id}", payload_t=ModuleChangedPayload
        )
        log.info("module.listen")
        while True:
            change: NMessage[ModuleChangedPayload] = await module_sub.next_msg()
            for mutation in change.p.mutations:
                mutator.apply(mutation)  # keep local module data in sync
            if client_id is not None and change.p.has_origin(client_id, client_nonce):
                continue  # skip self

            log.debug(
                "module.update",
                id=change.id,
                origin_type=change.p.origin.type,
                origin_id=change.p.origin.id,
            )
            origin_id = (
                to_global_id("Client", change.p.origin.id)
                if change.p.origin.type == "user"
                else None
            )
            mutations = unpack_module_mutations(change.p.mutations, mutator.module, project_version)
            yield ModuleChange(id=change.id, client_id=origin_id, mutations=mutations)

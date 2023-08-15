from typing import AsyncGenerator, Optional, Union
from uuid import UUID

import strawberry
import strawberry_django
import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import PermissionDenied
from strawberry.relay import GlobalID
from strawberry.scalars import JSON
from strawberry.types import Info

from bench import models
from bench.api import sync
from bench.api.auth import check_can_view_project_by_id
from bench.api.interp import Issue, ResolvedField
from bench.api.type import ProjectMutationType
from bench.api.utils import asafe_subscription, get_user_from_info, to_global_id, to_uuid
from bench.language import mutate, wire
from bench.models import packer
from bench.msg.core import NMessage, subscribe
from bench.msg.messages import ModuleChangedPayload, NMessageType, ProjectChangedPayload

logger = structlog.get_logger(__name__)


@strawberry.interface
class Change:
    id: UUID
    client_id: Optional[GlobalID]


@strawberry.type
class ProjectMutation:
    type: ProjectMutationType
    project_version_id: GlobalID


@strawberry.type
class ProjectChange(Change):
    id: UUID
    client_id: Optional[GlobalID]
    # individual mutations are not needed for now


ModuleMutationType = strawberry.enum(sync.MMT)


@strawberry.type
class ModuleMutation:
    type: ModuleMutationType
    project_version_id: GlobalID
    file_id: Optional[GlobalID]
    statement_id: Optional[GlobalID]
    revision: Optional[int]
    input: Optional[JSON]
    data: Union[Issue, ResolvedField, None]
    properties: Optional[list[str]]


@strawberry.type
class ModuleChange(Change):
    id: UUID
    client_id: Optional[GlobalID]
    mutations: list[ModuleMutation]


async def unpack_module_mutations(
    mutations: list[mutate.ModuleMutation], project_v: models.ProjectVersion
) -> list[ModuleMutation]:
    unpacked_mutations = []
    for m in mutations:
        # :RawMutations
        if isinstance(m.data, (wire.IssueData, wire.ResolvedFieldData)):
            # unpack data (somewhat inefficiently)
            if m.statement_id is not None:
                parent = await project_v.statements.aget(id=m.statement_id)
            elif m.file_id is not None:
                parent = await project_v.files.aget(id=m.file_id)
            else:
                parent = project_v
            data = packer.unpack_node_flat(m.data, parent)[0]
        else:  # ignore other data types
            data = None

        unpacked_mutation = ModuleMutation(
            type=m.type,
            project_version_id=to_global_id("ProjectVersion", m.project_version_id),
            file_id=to_global_id("File", m.file_id),
            statement_id=to_global_id("Statement", m.statement_id),
            revision=m.revision,
            input=m.input,
            data=data,
            properties=m.properties,
        )
        unpacked_mutations.append(unpacked_mutation)
    return unpacked_mutations


@strawberry.type
class MultiplayerSubscription:
    @asafe_subscription
    async def project_changed(
        self, info: Info, project_id: GlobalID
    ) -> AsyncGenerator[ProjectChange, None]:
        user = get_user_from_info(info)
        project_id = UUID(project_id.node_id)
        log = logger.bind(project_id=project_id, user=user)
        client_id = to_uuid(info.context["request"].scope["session"].get("client_id"))
        client_nonce = to_uuid(info.context["connection_params"].get("X-Client-Nonce"))
        try:
            await sync_to_async(check_can_view_project_by_id)(user, project_id=project_id)
        except PermissionDenied:
            log.warning("project_changed.subscribe_denied", exc_info=True)
            return

        project_sub = await subscribe(
            f"{NMessageType.PROJECT_CHANGED}.{project_id}", payload_t=ProjectChangedPayload
        )
        log.info("project.subscribe")
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
        user = get_user_from_info(info)
        project_version_id = UUID(project_version_id.node_id)
        log = logger.bind(project_version_id=project_version_id, user=user)
        client_id = to_uuid(info.context["request"].scope["session"].get("client_id"))
        client_nonce = to_uuid(info.context["connection_params"].get("X-Client-Nonce"))
        try:
            await sync_to_async(check_can_view_project_by_id)(user, project_version_id)
        except PermissionDenied:
            log.warning("module_changed.subscribe_denied", exc_info=True)
            return

        project_version = await models.ProjectVersion.objects.aget(id=project_version_id)
        routing_key = f"{project_version.project_id}.{project_version_id}"
        module_sub = await subscribe(
            f"{NMessageType.MODULE_CHANGED}.{routing_key}", payload_t=ModuleChangedPayload
        )
        log.info("module.subscribe")
        while True:
            change: NMessage[ModuleChangedPayload] = await module_sub.next_msg()
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
            mutations = await unpack_module_mutations(change.p.mutations, project_version)
            yield ModuleChange(id=change.id, client_id=origin_id, mutations=mutations)

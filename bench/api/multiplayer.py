from typing import AsyncGenerator, Optional, Union
from uuid import UUID

import strawberry
import structlog
from asgiref.sync import sync_to_async
from django.core.exceptions import PermissionDenied
from strawberry.relay import GlobalID
from strawberry.scalars import JSON
from strawberry.types import Info

from bench import models
from bench.api import sync
from bench.api.auth import check_module_access
from bench.api.interp import Issue, ResolvedField
from bench.api.type import ProjectMutationType
from bench.api.utils import asafe_subscription, to_global_id, to_uuid
from bench.language import edit, wire
from bench.models import ModuleAccessLevel, packer
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
    # individual edits are not needed for now


EditType = strawberry.enum(sync.MET)


@strawberry.type
class Edit:
    type: EditType
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
    edits: list[Edit]


async def unpack_module_edits(
    edits: list[edit.EditData], project_v: models.ProjectVersion
) -> list[Edit]:
    unpacked_edits = []
    for e in edits:
        # :RawMutations
        if isinstance(e.node, (wire.IssueData, wire.ResolvedFieldData)):
            # unpack data (somewhat inefficiently)
            if e.statement_id is not None:
                parent = await project_v.statements.aget(id=e.statement_id)
            elif e.file_id is not None:
                parent = await project_v.files.aget(id=e.file_id)
            else:
                parent = project_v
            data = packer.unpack_node_flat(e.node, parent)[0]
        else:  # ignore other data types
            data = None

        unpacked_edit = Edit(
            type=e.type,
            project_version_id=to_global_id("ProjectVersion", e.project_version_id),
            file_id=to_global_id("File", e.file_id),
            statement_id=to_global_id("Statement", e.statement_id),
            revision=e.revision,
            input=e.input,
            data=data,
            properties=e.properties,
        )
        unpacked_edits.append(unpacked_edit)
    return unpacked_edits


@strawberry.type
class MultiplayerSubscription:
    @asafe_subscription
    async def project_changed(
        self, info: Info, project_id: GlobalID
    ) -> AsyncGenerator[ProjectChange, None]:
        project_id = UUID(project_id.node_id)
        try:
            access = await sync_to_async(check_module_access)(
                info, project_id, ModuleAccessLevel.Read
            )
        except PermissionDenied:
            logger.warning("project_changed.subscribe_denied", exc_info=True)
            return

        log = logger.bind(project_id=project_id, user=access.user)
        client_id = to_uuid(info.context["request"].scope["session"].get("client_id"))
        client_nonce = to_uuid(info.context["connection_params"].get("X-Client-Nonce"))
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
            # individual edits aren't needed yet
            yield ProjectChange(id=change.id, client_id=origin_id)

    @asafe_subscription
    async def module_changed(
        self, info: Info, project_id: GlobalID, project_version_id: GlobalID
    ) -> AsyncGenerator[ModuleChange, None]:
        project_id = UUID(project_id.node_id)
        project_version_id = UUID(project_version_id.node_id)
        try:
            access = await sync_to_async(check_module_access)(
                info, project_id, ModuleAccessLevel.Read
            )
        except PermissionDenied:
            logger.warning("module_changed.subscribe_denied", exc_info=True)
            return

        log = logger.bind(project_version_id=project_version_id, user=access.user)
        client_id = to_uuid(info.context["request"].scope["session"].get("client_id"))
        client_nonce = to_uuid(info.context["connection_params"].get("X-Client-Nonce"))
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
            edits = await unpack_module_edits(change.p.edits, project_version)
            yield ModuleChange(id=change.id, client_id=origin_id, edits=edits)

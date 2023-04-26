from typing import Optional, AsyncGenerator, cast
from uuid import UUID

from asgiref.sync import sync_to_async
from django.core.exceptions import PermissionDenied
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID
import structlog

from bench import models
from bench.api import sync
from bench.api.auth import check_can_view_project_by_id
from bench.api.util import asafe_subscription
from bench.msg.core import subscribe, NMessage
from bench.msg.messages import ProjectVersionChangedPayload, NMessageType

logger = structlog.get_logger(__name__)

ModuleMutationType = gql.enum(sync.MMT)


@gql.type
class ModuleMutation:
    type: ModuleMutationType
    project_version_id: GlobalID
    file_id: Optional[GlobalID]
    statement_id: Optional[GlobalID]
    record_id: Optional[GlobalID]
    type_node_id: Optional[GlobalID]
    revision: Optional[int]
    input: Optional[JSON]


@gql.type
class ModuleChange:
    id: UUID
    client_id: Optional[GlobalID]
    mutations: list[ModuleMutation]


def to_global_id(type: str, id: UUID | None) -> GlobalID | None:
    if id is None:
        return None
    return GlobalID(type, str(id))


def rmap_mutation(mutation: sync.ModuleMutation) -> ModuleMutation:
    return ModuleMutation(
        type=mutation.type,
        project_version_id=to_global_id("ProjectVersion", mutation.project_version_id),
        file_id=to_global_id("File", mutation.file_id),
        statement_id=to_global_id("Statement", mutation.statement_id),
        record_id=to_global_id("Record", mutation.record_id),
        type_node_id=to_global_id("TypeNode", mutation.type_node_id),
        revision=mutation.revision,
        input=mutation.input,
    )


@gql.type
class ModuleSubscription:
    @asafe_subscription
    async def module_changed(
        self, info: Info, project_version_id: GlobalID
    ) -> AsyncGenerator[ModuleChange, None]:
        project_version_id = UUID(project_version_id.node_id)
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        client_id = info.context.request.scope["session"].get("client_id")
        log = logger.bind(
            project_version_id=project_version_id,
            user=user,
        )
        try:
            await sync_to_async(check_can_view_project_by_id)(
                user, project_version_id=project_version_id
            )
        except PermissionDenied:
            log.debug("runtime.subscribe_denied", project_version_id=project_version_id)
            return

        log.info("module.subscribe")
        module_sub = await subscribe(
            f"{NMessageType.PROJECT_VERSION_CHANGED}.{project_version_id}",
            payload_t=ProjectVersionChangedPayload,
        )

        log.info("module.listen")
        while True:
            change: NMessage[ProjectVersionChangedPayload] = await module_sub.next_msg()
            origin_type, origin_id = change.p.client
            if client_id == origin_id:
                continue  # skip self

            log.debug("module.update", id=change.id, origin_type=origin_type, origin_id=origin_id)
            client_id = to_global_id("Client", origin_id) if origin_type == "user" else None
            yield ModuleChange(
                id=change.id,
                client_id=client_id,
                mutations=[rmap_mutation(m) for m in change.p.mutations],
            )

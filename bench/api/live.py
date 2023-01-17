from typing import AsyncGenerator
from uuid import UUID

from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench.runtime.worker import ModuleState


@gql.type
class ModuleStateSubscription:
    @gql.subscription
    async def module_state_updated(
        self, project_version_id: GlobalID
    ) -> AsyncGenerator[ModuleState, None]:
        project_version_id = UUID(project_version_id.node_id)

        # TODO @Incomplete: get module state updates from the worker via zmq server/client then pub/sub

        for module_state in []:
            yield module_state
